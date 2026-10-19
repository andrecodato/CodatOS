//! Cofre em execução (`docs/04-seguranca.md` §7): o header fica no banco e a chave do cofre,
//! quando aberta, só em memória.
//!
//! - **Senha**: `vault.unlock` deriva a chave (Argon2id, ≈ 0,5 s fora do runtime) e o cofre
//!   bloqueia sozinho depois de [`SETTING_VAULT_AUTO_LOCK`] minutos sem uso (padrão 15).
//! - **Sessão do SO**: a chave fica no keychain e o cofre abre sozinho ao iniciar; sem
//!   bloqueio por inatividade, porque quem protege é a sessão do Windows.
//!
//! O bloqueio ao travar a sessão do SO e ao suspender vem do app desktop, que chama
//! [`Vault::lock`].

use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

use codatos_protocol::dashboard::SettingScope;
use codatos_protocol::vault::{
    MIN_PASSWORD_CHARS, SETTING_VAULT_AUTO_LOCK, VaultProtection, VaultState, VaultStatus,
};
use codatos_vault::{Aad, Envelope, Header, KdfParams, Key};
use tokio::time::Instant;
use zeroize::Zeroizing;

use crate::keychain::{Keychain, KeychainError};
use crate::{Error, Store};

/// Bloqueio por inatividade padrão (§7).
const DEFAULT_AUTO_LOCK_MINUTES: u32 = 15;
/// De quanto em quanto tempo o bloqueio automático confere a inatividade.
const AUTO_LOCK_TICK: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("ainda não existe cofre neste PC")]
    Absent,
    #[error("o cofre já foi criado")]
    Exists,
    #[error("o cofre está bloqueado")]
    Locked,
    #[error("senha do cofre incorreta")]
    WrongPassword,
    #[error("a senha do cofre precisa de pelo menos {MIN_PASSWORD_CHARS} caracteres")]
    WeakPassword,
    #[error("este sistema não tem keychain para o modo sessão")]
    OsSessionUnavailable,
    #[error(transparent)]
    Keychain(#[from] KeychainError),
    #[error(transparent)]
    Store(#[from] Error),
    #[error(transparent)]
    Crypto(codatos_vault::Error),
}

impl From<codatos_vault::Error> for VaultError {
    fn from(error: codatos_vault::Error) -> Self {
        match error {
            codatos_vault::Error::WrongPassword => Self::WrongPassword,
            other => Self::Crypto(other),
        }
    }
}

/// Cofre compartilhado entre os métodos, o tópico e o bloqueio automático. Clonar é barato.
#[derive(Debug, Clone)]
pub struct Vault {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    store: Store,
    keychain: Arc<dyn Keychain>,
    state: Mutex<State>,
}

#[derive(Debug)]
struct State {
    header: Option<Header>,
    key: Option<Key>,
    last_used: Instant,
}

impl Vault {
    /// Lê o header e, no modo sessão, já abre o cofre pelo keychain. Inicia o bloqueio
    /// automático (para quando o último clone do `Vault` sai de escopo).
    pub async fn open(store: Store, keychain: Arc<dyn Keychain>) -> Result<Self, Error> {
        let header = store.vault_header().await?;
        let vault = Self {
            inner: Arc::new(Inner {
                store,
                keychain,
                state: Mutex::new(State {
                    header,
                    key: None,
                    last_used: Instant::now(),
                }),
            }),
        };
        if vault.protection() == Some(VaultProtection::OsSession)
            && let Err(error) = vault.unlock_from_keychain()
        {
            tracing::warn!(%error, "cofre no modo sessão não abriu pelo keychain");
        }
        tokio::spawn(auto_lock(Arc::downgrade(&vault.inner)));
        Ok(vault)
    }

    pub async fn status(&self) -> VaultStatus {
        let auto_lock_minutes = self.auto_lock_minutes().await;
        let state = self.state();
        VaultStatus {
            state: match (&state.header, &state.key) {
                (None, _) => VaultState::Absent,
                (Some(_), None) => VaultState::Locked,
                (Some(_), Some(_)) => VaultState::Unlocked,
            },
            protection: state.header.as_ref().map(protection_of),
            auto_lock_minutes,
            os_session_available: self.inner.keychain.available(),
        }
    }

    /// Cria o cofre e o deixa aberto.
    pub async fn create(
        &self,
        protection: VaultProtection,
        password: Option<Zeroizing<String>>,
    ) -> Result<(), VaultError> {
        if self.state().header.is_some() {
            return Err(VaultError::Exists);
        }
        let vault_id = new_vault_id();
        let (header, key) = match protection {
            VaultProtection::Password => {
                let password = checked(password)?;
                blocking(move || Header::with_password(&vault_id, password.as_bytes(), kdf()))
                    .await?
            }
            VaultProtection::OsSession => {
                self.require_keychain()?;
                let (header, key) = Header::os_session(&vault_id);
                self.inner.keychain.set(&key.to_base64())?;
                (header, key)
            }
        };
        self.inner.store.save_vault_header(&header).await?;
        let mut state = self.state();
        state.header = Some(header);
        state.key = Some(key);
        state.last_used = Instant::now();
        Ok(())
    }

    /// Abre o cofre. No modo sessão a senha é ignorada e a chave vem do keychain.
    pub async fn unlock(&self, password: Zeroizing<String>) -> Result<(), VaultError> {
        let header = self.state().header.clone().ok_or(VaultError::Absent)?;
        if !header.uses_password() {
            return self.unlock_from_keychain();
        }
        let key = blocking(move || header.unlock(password.as_bytes())).await?;
        let mut state = self.state();
        state.key = Some(key);
        state.last_used = Instant::now();
        Ok(())
    }

    /// Esquece a chave do cofre. Sem efeito se já estava bloqueado.
    pub fn lock(&self) {
        if self.state().key.take().is_some() {
            tracing::info!("cofre bloqueado");
        }
    }

    /// Troca a senha ou o modo de proteção. Pede o cofre aberto e, se ele usa senha, a senha
    /// atual (reautenticação, §7). Os itens não são re-cifrados.
    pub async fn set_protection(
        &self,
        protection: VaultProtection,
        current_password: Option<Zeroizing<String>>,
        new_password: Option<Zeroizing<String>>,
    ) -> Result<(), VaultError> {
        let (mut header, key) = {
            let state = self.state();
            let header = state.header.clone().ok_or(VaultError::Absent)?;
            let key = state.key.as_ref().ok_or(VaultError::Locked)?;
            (header, Key::from_bytes(*key.as_bytes()))
        };
        let was_password = header.uses_password();
        if was_password {
            let current = current_password.unwrap_or_default();
            let check = header.clone();
            blocking(move || check.unlock(current.as_bytes()).map(drop)).await?;
        }
        match protection {
            VaultProtection::Password => {
                let password = checked(new_password)?;
                header = blocking(move || {
                    header.set_password(&key, password.as_bytes(), kdf())?;
                    Ok(header)
                })
                .await?;
                self.inner.store.save_vault_header(&header).await?;
                if !was_password {
                    self.inner.keychain.delete()?;
                }
            }
            VaultProtection::OsSession => {
                self.require_keychain()?;
                self.inner.keychain.set(&key.to_base64())?;
                header.set_os_session();
                self.inner.store.save_vault_header(&header).await?;
            }
        }
        let mut state = self.state();
        state.header = Some(header);
        state.last_used = Instant::now();
        Ok(())
    }

    /// Cifra um segredo do item `item_id` (uma identidade, uma chave SSH…).
    pub fn seal(&self, item_id: &str, plaintext: &[u8]) -> Result<Envelope, VaultError> {
        let mut state = self.state();
        state.last_used = Instant::now();
        let (header, key) = unlocked(&state)?;
        let aad = Aad {
            item_id,
            vault_id: &header.vault_id,
        };
        Ok(key.seal(&header.kid(), aad, plaintext)?)
    }

    /// Decifra um segredo selado por [`Vault::seal`] para o mesmo `item_id`.
    pub fn open_item(
        &self,
        item_id: &str,
        envelope: &Envelope,
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let mut state = self.state();
        state.last_used = Instant::now();
        let (header, key) = unlocked(&state)?;
        let aad = Aad {
            item_id,
            vault_id: &header.vault_id,
        };
        Ok(key.open(envelope, aad)?)
    }

    fn unlock_from_keychain(&self) -> Result<(), VaultError> {
        let secret = self.inner.keychain.get()?.ok_or(VaultError::Locked)?;
        let key = Key::from_base64(&secret)
            .ok_or_else(|| KeychainError::Failed("chave do cofre ilegível".to_owned()))?;
        let mut state = self.state();
        state.key = Some(key);
        state.last_used = Instant::now();
        Ok(())
    }

    fn require_keychain(&self) -> Result<(), VaultError> {
        if self.inner.keychain.available() {
            Ok(())
        } else {
            Err(VaultError::OsSessionUnavailable)
        }
    }

    fn protection(&self) -> Option<VaultProtection> {
        self.state().header.as_ref().map(protection_of)
    }

    async fn auto_lock_minutes(&self) -> u32 {
        auto_lock_minutes(&self.inner.store).await
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

fn unlocked(state: &State) -> Result<(&Header, &Key), VaultError> {
    let header = state.header.as_ref().ok_or(VaultError::Absent)?;
    let key = state.key.as_ref().ok_or(VaultError::Locked)?;
    Ok((header, key))
}

fn protection_of(header: &Header) -> VaultProtection {
    if header.uses_password() {
        VaultProtection::Password
    } else {
        VaultProtection::OsSession
    }
}

/// Senha presente e com o tamanho mínimo.
fn checked(password: Option<Zeroizing<String>>) -> Result<Zeroizing<String>, VaultError> {
    match password {
        Some(p) if p.chars().count() >= MIN_PASSWORD_CHARS => Ok(p),
        _ => Err(VaultError::WeakPassword),
    }
}

/// Parâmetros do Argon2id de um header novo: o mínimo do §3 (custo baixo nos testes, que
/// rodam sem otimização).
fn kdf() -> KdfParams {
    if cfg!(test) {
        KdfParams::with_cost(8, 1, 1)
    } else {
        KdfParams::generate()
    }
}

/// Id aleatório de 128 bits em hexadecimal.
fn new_vault_id() -> String {
    use std::fmt::Write as _;
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("gerador aleatório do SO indisponível");
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Argon2id e companhia fora do runtime.
async fn blocking<T, F>(f: F) -> Result<T, VaultError>
where
    F: FnOnce() -> Result<T, codatos_vault::Error> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .expect("tarefa do cofre não entra em pânico")
        .map_err(VaultError::from)
}

async fn auto_lock_minutes(store: &Store) -> u32 {
    store
        .setting(SETTING_VAULT_AUTO_LOCK, SettingScope::Device)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(DEFAULT_AUTO_LOCK_MINUTES)
}

/// Bloqueia o cofre com senha depois de `vault.autoLockMinutes` sem uso.
async fn auto_lock(inner: Weak<Inner>) {
    let mut tick = tokio::time::interval(AUTO_LOCK_TICK);
    loop {
        tick.tick().await;
        let Some(inner) = inner.upgrade() else {
            return;
        };
        lock_if_idle(&Vault { inner }).await;
    }
}

async fn lock_if_idle(vault: &Vault) {
    let minutes = vault.auto_lock_minutes().await;
    if minutes == 0 || vault.protection() != Some(VaultProtection::Password) {
        return;
    }
    let idle = vault.state().last_used.elapsed();
    if idle >= Duration::from_secs(u64::from(minutes) * 60) {
        vault.lock();
    }
}

impl Store {
    async fn vault_header(&self) -> Result<Option<Header>, Error> {
        let row: Option<(String,)> = sqlx::query_as("SELECT header FROM vault WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;
        row.map(|(h,)| serde_json::from_str(&h).map_err(Error::from))
            .transpose()
    }

    async fn save_vault_header(&self, header: &Header) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO vault (id, header) VALUES (1, ?)
             ON CONFLICT (id) DO UPDATE SET header = excluded.header,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(serde_json::to_string(header)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::MemoryKeychain;
    use serde_json::json;

    #[allow(clippy::unnecessary_wraps)]
    fn pw(text: &str) -> Option<Zeroizing<String>> {
        Some(Zeroizing::new(text.to_owned()))
    }

    async fn vault_with(keychain: Arc<dyn Keychain>) -> (Store, Vault) {
        let store = Store::in_memory().await.unwrap();
        let vault = Vault::open(store.clone(), keychain).await.unwrap();
        (store, vault)
    }

    async fn vault() -> (Store, Vault) {
        vault_with(Arc::new(MemoryKeychain::default())).await
    }

    #[tokio::test]
    async fn sem_cofre_tudo_responde_absent() {
        let (_, vault) = vault().await;
        let status = vault.status().await;
        assert_eq!(status.state, VaultState::Absent);
        assert_eq!(status.protection, None);
        assert_eq!(status.auto_lock_minutes, 15);
        assert!(matches!(vault.seal("x", b"y"), Err(VaultError::Absent)));
        assert!(matches!(
            vault.unlock(Zeroizing::new(String::new())).await,
            Err(VaultError::Absent)
        ));
    }

    #[tokio::test]
    async fn com_senha_cria_bloqueia_e_abre() {
        let (store, vault) = vault().await;
        assert!(matches!(
            vault.create(VaultProtection::Password, pw("curta")).await,
            Err(VaultError::WeakPassword)
        ));
        vault
            .create(VaultProtection::Password, pw("senha comprida"))
            .await
            .unwrap();
        assert_eq!(vault.status().await.state, VaultState::Unlocked);
        let item = vault.seal("identity-1", b"hunter2").unwrap();

        vault.lock();
        assert_eq!(vault.status().await.state, VaultState::Locked);
        assert!(matches!(
            vault.open_item("identity-1", &item),
            Err(VaultError::Locked)
        ));
        assert!(matches!(
            vault.unlock(Zeroizing::new("errada!!".to_owned())).await,
            Err(VaultError::WrongPassword)
        ));

        // Reabrir o banco (app reiniciado): continua bloqueado e abre com a senha.
        let reopened = Vault::open(store, Arc::new(MemoryKeychain::default()))
            .await
            .unwrap();
        assert_eq!(reopened.status().await.state, VaultState::Locked);
        reopened
            .unlock(Zeroizing::new("senha comprida".to_owned()))
            .await
            .unwrap();
        assert_eq!(
            reopened.open_item("identity-1", &item).unwrap().as_slice(),
            b"hunter2"
        );
        assert!(matches!(
            reopened
                .create(VaultProtection::Password, pw("outra senha"))
                .await,
            Err(VaultError::Exists)
        ));
    }

    #[tokio::test]
    async fn modo_sessao_abre_sozinho_ao_iniciar() {
        let keychain: Arc<dyn Keychain> = Arc::new(MemoryKeychain::default());
        let (store, vault) = vault_with(Arc::clone(&keychain)).await;
        vault
            .create(VaultProtection::OsSession, None)
            .await
            .unwrap();
        let item = vault.seal("key-1", b"PRIVATE KEY").unwrap();
        assert!(keychain.get().unwrap().is_some());

        let reopened = Vault::open(store, keychain).await.unwrap();
        let status = reopened.status().await;
        assert_eq!(status.state, VaultState::Unlocked);
        assert_eq!(status.protection, Some(VaultProtection::OsSession));
        assert_eq!(
            reopened.open_item("key-1", &item).unwrap().as_slice(),
            b"PRIVATE KEY"
        );
        reopened.lock();
        reopened
            .unlock(Zeroizing::new(String::new()))
            .await
            .unwrap();
        assert_eq!(reopened.status().await.state, VaultState::Unlocked);
    }

    #[tokio::test]
    async fn troca_de_protecao_mantem_os_itens() {
        let keychain: Arc<dyn Keychain> = Arc::new(MemoryKeychain::default());
        let (_, vault) = vault_with(Arc::clone(&keychain)).await;
        vault
            .create(VaultProtection::OsSession, None)
            .await
            .unwrap();
        let item = vault.seal("host-1", b"segredo").unwrap();

        // Sessão → senha: o keychain é limpo.
        vault
            .set_protection(VaultProtection::Password, None, pw("primeira senha"))
            .await
            .unwrap();
        assert!(keychain.get().unwrap().is_none());

        // Trocar a senha pede a atual.
        assert!(matches!(
            vault
                .set_protection(
                    VaultProtection::Password,
                    pw("chute errado"),
                    pw("segunda senha")
                )
                .await,
            Err(VaultError::WrongPassword)
        ));
        vault
            .set_protection(
                VaultProtection::Password,
                pw("primeira senha"),
                pw("segunda senha"),
            )
            .await
            .unwrap();
        vault.lock();
        vault
            .unlock(Zeroizing::new("segunda senha".to_owned()))
            .await
            .unwrap();
        assert_eq!(
            vault.open_item("host-1", &item).unwrap().as_slice(),
            b"segredo"
        );

        // Senha → sessão.
        vault
            .set_protection(VaultProtection::OsSession, pw("segunda senha"), None)
            .await
            .unwrap();
        assert!(keychain.get().unwrap().is_some());
        assert_eq!(
            vault.status().await.protection,
            Some(VaultProtection::OsSession)
        );
    }

    #[tokio::test]
    async fn sem_keychain_o_modo_sessao_e_recusado() {
        let (_, vault) = vault_with(crate::keychain::os_keychain()).await;
        if vault.status().await.os_session_available {
            return; // Windows/macOS: o teste vale só onde não há keychain.
        }
        assert!(matches!(
            vault.create(VaultProtection::OsSession, None).await,
            Err(VaultError::OsSessionUnavailable)
        ));
    }

    /// Finge que o último uso foi há `minutes` minutos.
    fn idle_for(vault: &Vault, minutes: u64) {
        vault.state().last_used = Instant::now()
            .checked_sub(Duration::from_secs(minutes * 60))
            .unwrap();
    }

    #[tokio::test]
    async fn bloqueia_sozinho_depois_de_15_minutos_sem_uso() {
        let (store, vault) = vault().await;
        vault
            .create(VaultProtection::Password, pw("senha comprida"))
            .await
            .unwrap();
        idle_for(&vault, 14);
        lock_if_idle(&vault).await;
        assert_eq!(vault.status().await.state, VaultState::Unlocked);
        // Usar o cofre reinicia a contagem.
        vault.seal("x", b"y").unwrap();
        lock_if_idle(&vault).await;
        assert_eq!(vault.status().await.state, VaultState::Unlocked);
        idle_for(&vault, 16);
        lock_if_idle(&vault).await;
        assert_eq!(vault.status().await.state, VaultState::Locked);

        // `0` desliga o bloqueio automático.
        store
            .set_setting(SETTING_VAULT_AUTO_LOCK, SettingScope::Device, &json!(0))
            .await
            .unwrap();
        vault
            .unlock(Zeroizing::new("senha comprida".to_owned()))
            .await
            .unwrap();
        idle_for(&vault, 600);
        lock_if_idle(&vault).await;
        assert_eq!(vault.status().await.state, VaultState::Unlocked);
    }

    #[tokio::test]
    async fn modo_sessao_nao_bloqueia_por_inatividade() {
        let (_, vault) = vault().await;
        vault
            .create(VaultProtection::OsSession, None)
            .await
            .unwrap();
        idle_for(&vault, 600);
        lock_if_idle(&vault).await;
        assert_eq!(vault.status().await.state, VaultState::Unlocked);
    }
}
