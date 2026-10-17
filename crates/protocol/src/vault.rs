//! Cofre local (`docs/04-seguranca.md` §7, Q-36): criar, desbloquear, bloquear.
//!
//! A UI nunca recebe a chave do cofre nem os segredos; só o estado e os erros.

use serde::{Deserialize, Serialize};

/// Métodos JSON-RPC.
pub const METHOD_VAULT_STATUS: &str = "vault.status";
pub const METHOD_VAULT_CREATE: &str = "vault.create";
pub const METHOD_VAULT_UNLOCK: &str = "vault.unlock";
pub const METHOD_VAULT_LOCK: &str = "vault.lock";
pub const METHOD_VAULT_SET_PROTECTION: &str = "vault.setProtection";

/// Tópico com o [`VaultStatus`], para a UI saber quando o cofre bloqueou sozinho.
pub const TOPIC_VAULT: &str = "vault.state";

/// Preferência `device`: minutos sem uso até bloquear (padrão 15, `0` desliga).
pub const SETTING_VAULT_AUTO_LOCK: &str = "vault.autoLockMinutes";

/// Tamanho mínimo da senha do cofre, em caracteres.
pub const MIN_PASSWORD_CHARS: usize = 8;

/// Códigos de erro estáveis dos métodos do cofre.
pub mod errors {
    /// Ainda não existe cofre neste PC.
    pub const ABSENT: &str = "vault_absent";
    /// `vault.create` com um cofre já criado.
    pub const EXISTS: &str = "vault_exists";
    /// A operação precisa do cofre aberto.
    pub const LOCKED: &str = "vault_locked";
    pub const WRONG_PASSWORD: &str = "wrong_password";
    /// Senha com menos de [`super::MIN_PASSWORD_CHARS`] caracteres.
    pub const WEAK_PASSWORD: &str = "weak_password";
    /// Modo sessão do SO pedido onde não há keychain suportado.
    pub const OS_SESSION_UNAVAILABLE: &str = "os_session_unavailable";
    /// O keychain do SO recusou ou falhou.
    pub const KEYCHAIN_FAILED: &str = "keychain_failed";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultState {
    /// Nenhum cofre criado ainda.
    Absent,
    Locked,
    Unlocked,
}

/// Como a chave do cofre fica protegida (Q-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultProtection {
    /// Senha do cofre (Argon2id).
    Password,
    /// Só a sessão do SO: a chave fica no keychain e o cofre abre sozinho.
    OsSession,
}

/// Resultado de `vault.status` e amostra de [`TOPIC_VAULT`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub state: VaultState,
    /// `None` enquanto não há cofre.
    pub protection: Option<VaultProtection>,
    /// Minutos sem uso até bloquear; `0` = nunca. Só vale com senha.
    pub auto_lock_minutes: u32,
    /// Este SO tem keychain suportado para o modo sessão.
    pub os_session_available: bool,
}

/// Parâmetros de `vault.create`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVault {
    pub protection: VaultProtection,
    /// Obrigatória com [`VaultProtection::Password`].
    #[serde(default)]
    pub password: Option<String>,
}

/// Parâmetros de `vault.unlock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockVault {
    pub password: String,
}

/// Parâmetros de `vault.setProtection`: troca a senha ou o modo. Pede o cofre aberto e, se
/// ele usa senha, a senha atual (reautenticação, §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProtection {
    pub protection: VaultProtection,
    #[serde(default)]
    pub current_password: Option<String>,
    /// Obrigatória com [`VaultProtection::Password`].
    #[serde(default)]
    pub new_password: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_em_camel_case() {
        let status = VaultStatus {
            state: VaultState::Unlocked,
            protection: Some(VaultProtection::OsSession),
            auto_lock_minutes: 15,
            os_session_available: true,
        };
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            json!({
                "state": "unlocked",
                "protection": "osSession",
                "autoLockMinutes": 15,
                "osSessionAvailable": true
            })
        );
        let create: CreateVault =
            serde_json::from_value(json!({"protection": "osSession"})).unwrap();
        assert_eq!(create.password, None);
    }
}
