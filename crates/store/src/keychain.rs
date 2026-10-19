//! Keychain do SO para o modo sessão do cofre (Q-36): Gerenciador de Credenciais no Windows,
//! Keychain no macOS. No Linux o modo sessão fica para depois (o Secret Service precisa de
//! um daemon rodando e de mais dependências).

use std::fmt;
use std::sync::Arc;

use zeroize::Zeroizing;

/// Onde a chave do cofre fica no modo sessão. Um segredo só, por instalação.
pub trait Keychain: fmt::Debug + Send + Sync + 'static {
    /// `false` quando este SO não tem keychain suportado.
    fn available(&self) -> bool;
    /// `Ok(None)` quando não há nada guardado.
    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError>;
    fn set(&self, secret: &str) -> Result<(), KeychainError>;
    /// Apagar o que não existe não é erro.
    fn delete(&self) -> Result<(), KeychainError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeychainError {
    #[error("este sistema não tem keychain suportado")]
    Unavailable,
    #[error("o keychain do sistema falhou: {0}")]
    Failed(String),
}

/// Nome do serviço e do usuário da credencial (`vault-key.codatos` no Windows).
#[cfg(any(windows, target_os = "macos"))]
const SERVICE: &str = "codatos";
#[cfg(any(windows, target_os = "macos"))]
const USER: &str = "vault-key";

/// Keychain deste SO.
pub fn os_keychain() -> Arc<dyn Keychain> {
    Arc::new(OsKeychain)
}

#[derive(Debug)]
struct OsKeychain;

#[cfg(any(windows, target_os = "macos"))]
impl OsKeychain {
    fn entry() -> Result<keyring_core::Entry, KeychainError> {
        use keyring_core::api::CredentialStoreApi as _;
        #[cfg(windows)]
        let (store, modifiers) = (
            windows_native_keyring_store::Store::new(),
            // Local: não vai para o perfil móvel de um domínio.
            std::collections::HashMap::from([("persistence", "Local")]),
        );
        #[cfg(target_os = "macos")]
        let (store, modifiers) = (
            apple_native_keyring_store::keychain::Store::new(),
            std::collections::HashMap::new(),
        );
        store
            .map_err(failed)?
            .build(SERVICE, USER, Some(&modifiers))
            .map_err(failed)
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn failed(error: keyring_core::Error) -> KeychainError {
    KeychainError::Failed(error.to_string())
}

#[cfg(any(windows, target_os = "macos"))]
impl Keychain for OsKeychain {
    fn available(&self) -> bool {
        true
    }

    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError> {
        match Self::entry()?.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(failed(error)),
        }
    }

    fn set(&self, secret: &str) -> Result<(), KeychainError> {
        Self::entry()?.set_password(secret).map_err(failed)
    }

    fn delete(&self) -> Result<(), KeychainError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(failed(error)),
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
impl Keychain for OsKeychain {
    fn available(&self) -> bool {
        false
    }

    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError> {
        Ok(None)
    }

    fn set(&self, _secret: &str) -> Result<(), KeychainError> {
        Err(KeychainError::Unavailable)
    }

    fn delete(&self) -> Result<(), KeychainError> {
        Ok(())
    }
}

/// Keychain em memória, para testes.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct MemoryKeychain(std::sync::Mutex<Option<String>>);

#[cfg(test)]
impl Keychain for MemoryKeychain {
    fn available(&self) -> bool {
        true
    }

    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError> {
        Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
    }

    fn set(&self, secret: &str) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }

    fn delete(&self) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
