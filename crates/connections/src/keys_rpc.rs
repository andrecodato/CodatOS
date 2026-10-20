//! Métodos `keys.*`: gerar, importar, renomear e apagar chaves SSH.

use std::sync::Arc;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::connections::{
    ConnectionId, GenerateKey, ImportKey, METHOD_KEY_DELETE, METHOD_KEY_GENERATE,
    METHOD_KEY_IMPORT, METHOD_KEY_RENAME, RenameKey, SshKey, errors,
};
use codatos_store::{Store, Vault};
use serde_json::{Value, json};
use ssh_key::PrivateKey;
use zeroize::Zeroizing;

use crate::keys::{self, KeyError};

const MAX_ID: usize = 64;
const MAX_NAME: usize = 200;
/// Uma chave RSA de 16384 bits em PPK cabe com folga.
const MAX_KEY_TEXT: usize = 64 * 1024;

/// Os métodos de chaves, para `ServerConfig::methods`.
pub fn key_methods(store: &Store, vault: &Vault) -> Vec<Arc<dyn Method>> {
    [
        METHOD_KEY_GENERATE,
        METHOD_KEY_IMPORT,
        METHOD_KEY_RENAME,
        METHOD_KEY_DELETE,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(KeyMethod {
            name,
            store: store.clone(),
            vault: vault.clone(),
        }) as Arc<dyn Method>
    })
    .collect()
}

#[derive(Debug)]
struct KeyMethod {
    name: &'static str,
    store: Store,
    vault: Vault,
}

impl Method for KeyMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            match self.name {
                METHOD_KEY_GENERATE => {
                    let p: GenerateKey = params(value)?;
                    check(&p.id, &p.name)?;
                    let comment = if p.comment.trim().is_empty() {
                        p.name.clone()
                    } else {
                        p.comment
                    };
                    // RSA 4096 leva segundos.
                    let key =
                        tokio::task::spawn_blocking(move || keys::generate(p.algorithm, &comment))
                            .await
                            .map_err(|_| MethodError::new("key_failed", "Falha ao gerar a chave"))?
                            .map_err(key_error)?;
                    Ok(json!(self.save(p.id, p.name, &key).await?))
                }
                METHOD_KEY_IMPORT => {
                    let p: ImportKey = params(value)?;
                    check(&p.id, &p.name)?;
                    let text = Zeroizing::new(p.private_key);
                    let passphrase = p.passphrase.map(Zeroizing::new);
                    if text.len() > MAX_KEY_TEXT {
                        return Err(key_error(KeyError::Invalid));
                    }
                    let key = keys::import(&text, passphrase.as_deref().map(String::as_str))
                        .map_err(key_error)?;
                    Ok(json!(self.save(p.id, p.name, &key).await?))
                }
                METHOD_KEY_RENAME => {
                    let p: RenameKey = params(value)?;
                    check(&p.id, &p.name)?;
                    found(
                        self.store
                            .rename_key(&p.id, &p.name)
                            .await
                            .map_err(internal)?,
                    )
                }
                METHOD_KEY_DELETE => {
                    let p: ConnectionId = params(value)?;
                    found(self.store.delete_key(&p.id).await.map_err(internal)?)
                }
                _ => unreachable!("método registrado sem implementação: {}", self.name),
            }
        })
    }
}

impl KeyMethod {
    /// Sela a chave privada no cofre (exige o cofre aberto) e grava.
    async fn save(
        &self,
        id: String,
        name: String,
        key: &PrivateKey,
    ) -> Result<SshKey, MethodError> {
        let (algorithm, public_key, fingerprint) = keys::describe(key).map_err(key_error)?;
        let private = keys::to_openssh(key).map_err(key_error)?;
        let envelope = self.vault.seal(&id, private.as_bytes())?;
        let sealed = serde_json::to_string(&envelope)
            .map_err(|_| MethodError::new("key_failed", "Falha ao guardar a chave"))?;
        let summary = SshKey {
            id,
            name,
            algorithm,
            public_key,
            fingerprint,
        };
        self.store
            .insert_key(&summary, &sealed)
            .await
            .map_err(|e| {
                if e.is_constraint() {
                    MethodError::invalid_params("Já existe uma chave com este id")
                } else {
                    internal(e)
                }
            })?;
        Ok(summary)
    }
}

fn check(id: &str, name: &str) -> Result<(), MethodError> {
    if id.is_empty() || id.len() > MAX_ID || name.trim().is_empty() || name.len() > MAX_NAME {
        return Err(MethodError::invalid_params("Id ou nome inválido"));
    }
    Ok(())
}

fn found(done: bool) -> Result<Value, MethodError> {
    if done {
        Ok(Value::Null)
    } else {
        Err(MethodError::new(errors::NOT_FOUND, "Chave não encontrada"))
    }
}

fn key_error(error: KeyError) -> MethodError {
    match error {
        KeyError::PassphraseRequired => {
            MethodError::new(errors::KEY_PASSPHRASE_REQUIRED, "Esta chave tem passphrase")
        }
        KeyError::WrongPassphrase => {
            MethodError::new(errors::KEY_WRONG_PASSPHRASE, "Passphrase incorreta")
        }
        KeyError::UnsupportedFormat => MethodError::new(
            errors::KEY_UNSUPPORTED_FORMAT,
            "Formato PEM antigo: converta com `ssh-keygen -p -f chave` e importe de novo",
        ),
        KeyError::Invalid => MethodError::new(
            errors::KEY_INVALID,
            "Não é uma chave privada OpenSSH ou PuTTY",
        ),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn internal(error: codatos_store::Error) -> MethodError {
    tracing::error!(%error, "erro no banco local");
    MethodError::new("store_failed", "Erro no banco local")
}

#[cfg(test)]
mod tests {
    use super::*;
    use codatos_protocol::vault::VaultProtection;
    use codatos_store::{Keychain, KeychainError};

    /// Modo sessão com a chave do cofre em memória: sem Argon2 nos testes.
    #[derive(Debug, Default)]
    struct MemoryKeychain(std::sync::Mutex<Option<String>>);

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

    async fn env(unlocked: bool) -> (Store, Vault) {
        let store = Store::in_memory().await.unwrap();
        let vault = Vault::open(store.clone(), Arc::new(MemoryKeychain::default()))
            .await
            .unwrap();
        if unlocked {
            vault
                .create(VaultProtection::OsSession, None)
                .await
                .unwrap();
        }
        (store, vault)
    }

    async fn call(
        store: &Store,
        vault: &Vault,
        name: &str,
        value: Value,
    ) -> Result<Value, MethodError> {
        let all = key_methods(store, vault);
        let method = all.iter().find(|m| m.name() == name).unwrap();
        method.call(value).await
    }

    #[tokio::test]
    async fn gera_lista_e_guarda_selada() {
        let (store, vault) = env(true).await;
        let key = call(
            &store,
            &vault,
            METHOD_KEY_GENERATE,
            json!({"id": "k1", "name": "Notebook", "algorithm": "ed25519"}),
        )
        .await
        .unwrap();
        assert_eq!(key["algorithm"], "ssh-ed25519");
        assert!(key["publicKey"].as_str().unwrap().ends_with(" Notebook"));

        let list = store.connections().await.unwrap();
        assert_eq!(list.keys.len(), 1);
        assert_eq!(list.keys[0].fingerprint, key["fingerprint"]);

        let sealed = store.key_private("k1").await.unwrap().unwrap();
        assert!(!sealed.contains("OPENSSH"));
        let envelope = serde_json::from_str(&sealed).unwrap();
        let private = vault.open_item("k1", &envelope).unwrap();
        let reopened = keys::import(std::str::from_utf8(&private).unwrap(), None).unwrap();
        assert_eq!(keys::describe(&reopened).unwrap().2, key["fingerprint"]);

        call(
            &store,
            &vault,
            METHOD_KEY_RENAME,
            json!({"id": "k1", "name": "PC"}),
        )
        .await
        .unwrap();
        call(&store, &vault, METHOD_KEY_DELETE, json!({"id": "k1"}))
            .await
            .unwrap();
        let err = call(&store, &vault, METHOD_KEY_DELETE, json!({"id": "k1"}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::NOT_FOUND);
    }

    #[tokio::test]
    async fn importa_putty_cifrada_e_exige_o_cofre() {
        let enc = include_str!("../tests/ed25519_enc.ppk");
        let (store, vault) = env(false).await;
        let import = |passphrase: Option<&str>| json!({"id": "k1", "name": "PuTTY", "privateKey": enc, "passphrase": passphrase});
        let err = call(&store, &vault, METHOD_KEY_IMPORT, import(Some("123")))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "vault_absent");
        assert_eq!(store.connections().await.unwrap().keys.len(), 0);

        vault
            .create(VaultProtection::OsSession, None)
            .await
            .unwrap();
        let err = call(&store, &vault, METHOD_KEY_IMPORT, import(None))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::KEY_PASSPHRASE_REQUIRED);
        let key = call(&store, &vault, METHOD_KEY_IMPORT, import(Some("123")))
            .await
            .unwrap();
        assert_eq!(key["algorithm"], "ssh-ed25519");

        // Mesmo id de novo: recusado sem apagar a primeira.
        let err = call(&store, &vault, METHOD_KEY_IMPORT, import(Some("123")))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid_params");
    }
}
