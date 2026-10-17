//! Métodos `vault.*` e o tópico `vault.state`.

use std::sync::Arc;

use codatos_core::{
    CollectError, CollectFuture, Collector, Method, MethodError, MethodFuture, params,
};
use codatos_protocol::vault::{
    CreateVault, METHOD_VAULT_CREATE, METHOD_VAULT_LOCK, METHOD_VAULT_SET_PROTECTION,
    METHOD_VAULT_STATUS, METHOD_VAULT_UNLOCK, SetProtection, TOPIC_VAULT, UnlockVault, errors,
};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::vault::{Vault, VaultError};

/// Os métodos do cofre, para `ServerConfig::methods`.
pub fn vault_methods(vault: &Vault) -> Vec<Arc<dyn Method>> {
    [
        METHOD_VAULT_STATUS,
        METHOD_VAULT_CREATE,
        METHOD_VAULT_UNLOCK,
        METHOD_VAULT_LOCK,
        METHOD_VAULT_SET_PROTECTION,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(VaultMethod {
            name,
            vault: vault.clone(),
        }) as Arc<dyn Method>
    })
    .collect()
}

#[derive(Debug)]
struct VaultMethod {
    name: &'static str,
    vault: Vault,
}

impl Method for VaultMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let vault = &self.vault;
            match self.name {
                METHOD_VAULT_STATUS => {}
                METHOD_VAULT_CREATE => {
                    let p: CreateVault = params(value)?;
                    vault
                        .create(p.protection, p.password.map(Zeroizing::new))
                        .await?;
                }
                METHOD_VAULT_UNLOCK => {
                    let p: UnlockVault = params(value)?;
                    vault.unlock(Zeroizing::new(p.password)).await?;
                }
                METHOD_VAULT_LOCK => vault.lock(),
                METHOD_VAULT_SET_PROTECTION => {
                    let p: SetProtection = params(value)?;
                    vault
                        .set_protection(
                            p.protection,
                            p.current_password.map(Zeroizing::new),
                            p.new_password.map(Zeroizing::new),
                        )
                        .await?;
                }
                _ => unreachable!("método registrado sem implementação: {}", self.name),
            }
            // Todos devolvem o estado novo, para a UI não precisar de outra chamada.
            Ok(json!(vault.status().await))
        })
    }
}

/// Erro do cofre como erro JSON-RPC com código estável.
impl From<VaultError> for MethodError {
    fn from(error: VaultError) -> Self {
        let code = match &error {
            VaultError::Absent => errors::ABSENT,
            VaultError::Exists => errors::EXISTS,
            VaultError::Locked => errors::LOCKED,
            VaultError::WrongPassword => errors::WRONG_PASSWORD,
            VaultError::WeakPassword => errors::WEAK_PASSWORD,
            VaultError::OsSessionUnavailable => errors::OS_SESSION_UNAVAILABLE,
            VaultError::Keychain(_) => errors::KEYCHAIN_FAILED,
            VaultError::Store(_) | VaultError::Crypto(_) => {
                tracing::error!(%error, "erro no cofre");
                return MethodError::new("vault_failed", "Erro no cofre local");
            }
        };
        MethodError::new(code, error.to_string())
    }
}

/// `vault.state`: o estado do cofre, para a UI notar o bloqueio automático.
#[derive(Debug)]
pub struct VaultCollector(pub Vault);

impl Collector for VaultCollector {
    fn topic(&self) -> &'static str {
        TOPIC_VAULT
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async move {
            serde_json::to_value(self.0.status().await)
                .map_err(|_| CollectError::new("collector_invalid", "Amostra inválida"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use crate::keychain::MemoryKeychain;

    async fn call(vault: &Vault, name: &'static str, params: Value) -> Result<Value, MethodError> {
        let methods = vault_methods(vault);
        let method = methods.iter().find(|m| m.name() == name).unwrap();
        method.call(params).await
    }

    #[tokio::test]
    async fn fluxo_pelos_metodos() {
        let store = Store::in_memory().await.unwrap();
        let vault = Vault::open(store, Arc::new(MemoryKeychain::default()))
            .await
            .unwrap();

        let status = call(&vault, METHOD_VAULT_STATUS, Value::Null)
            .await
            .unwrap();
        assert_eq!(status["state"], "absent");

        let weak = call(
            &vault,
            METHOD_VAULT_CREATE,
            json!({"protection": "password", "password": "123"}),
        )
        .await
        .unwrap_err();
        assert_eq!(weak.code(), errors::WEAK_PASSWORD);

        let status = call(
            &vault,
            METHOD_VAULT_CREATE,
            json!({"protection": "password", "password": "uma senha boa"}),
        )
        .await
        .unwrap();
        assert_eq!(status["state"], "unlocked");
        assert_eq!(status["protection"], "password");

        let status = call(&vault, METHOD_VAULT_LOCK, Value::Null).await.unwrap();
        assert_eq!(status["state"], "locked");

        let wrong = call(
            &vault,
            METHOD_VAULT_UNLOCK,
            json!({"password": "nope nope"}),
        )
        .await
        .unwrap_err();
        assert_eq!(wrong.code(), errors::WRONG_PASSWORD);

        let status = call(
            &vault,
            METHOD_VAULT_UNLOCK,
            json!({"password": "uma senha boa"}),
        )
        .await
        .unwrap();
        assert_eq!(status["state"], "unlocked");

        let sample = VaultCollector(vault).collect().await.unwrap();
        assert_eq!(sample["state"], "unlocked");
    }
}
