//! Métodos `connections.*`: cadastro de grupos, hosts e identidades.

use std::sync::Arc;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::connections::{
    ConnectionId, Connections, Host, HostGroup, Identity, METHOD_CONNECTIONS_LIST,
    METHOD_GROUP_DELETE, METHOD_GROUP_SAVE, METHOD_HOST_DELETE, METHOD_HOST_SAVE,
    METHOD_IDENTITY_DELETE, METHOD_IDENTITY_SAVE, PasswordChange, SaveIdentity, errors,
};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::connections::{group_cycle, jump_cycle};
use crate::{Error, Store, Vault};

const MAX_ID: usize = 64;
const MAX_NAME: usize = 200;
const MAX_ADDRESS: usize = 255;
const MAX_TAGS: usize = 32;
const MAX_TAG: usize = 50;
const MAX_NOTES: usize = 10_000;
const MAX_JSON_BYTES: usize = 64 * 1024;
const MAX_PASSWORD: usize = 4096;

/// Os métodos de conexões, para `ServerConfig::methods`. Senhas passam pelo `vault`.
pub fn connection_methods(store: &Store, vault: &Vault) -> Vec<Arc<dyn Method>> {
    [
        METHOD_CONNECTIONS_LIST,
        METHOD_HOST_SAVE,
        METHOD_HOST_DELETE,
        METHOD_GROUP_SAVE,
        METHOD_GROUP_DELETE,
        METHOD_IDENTITY_SAVE,
        METHOD_IDENTITY_DELETE,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(ConnectionMethod {
            name,
            store: store.clone(),
            vault: vault.clone(),
        }) as Arc<dyn Method>
    })
    .collect()
}

#[derive(Debug)]
struct ConnectionMethod {
    name: &'static str,
    store: Store,
    vault: Vault,
}

impl Method for ConnectionMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let store = &self.store;
            match self.name {
                METHOD_CONNECTIONS_LIST => {
                    return Ok(json!(store.connections().await.map_err(internal)?));
                }
                METHOD_GROUP_SAVE => {
                    let group: HostGroup = params(value)?;
                    validate_group(&group)?;
                    let mut all = store.connections().await.map_err(internal)?;
                    if group_cycle(&all, &group.id, group.parent_id.as_deref()) {
                        return Err(MethodError::new(
                            errors::CYCLE,
                            "Um grupo não pode ficar dentro dele mesmo",
                        ));
                    }
                    replace(&mut all.groups, group.clone(), |g| &g.id);
                    check_jumps(&all)?;
                    store.save_group(&group).await.map_err(write_error)?;
                }
                METHOD_HOST_SAVE => {
                    let host: Host = params(value)?;
                    validate_host(&host)?;
                    let mut all = store.connections().await.map_err(internal)?;
                    replace(&mut all.hosts, host.clone(), |h| &h.id);
                    check_jumps(&all)?;
                    store.save_host(&host).await.map_err(write_error)?;
                }
                METHOD_IDENTITY_SAVE => {
                    let p: SaveIdentity = params(value)?;
                    self.save_identity(p).await?;
                }
                METHOD_HOST_DELETE | METHOD_GROUP_DELETE | METHOD_IDENTITY_DELETE => {
                    let p: ConnectionId = params(value)?;
                    let deleted = match self.name {
                        METHOD_HOST_DELETE => store.delete_host(&p.id).await,
                        METHOD_GROUP_DELETE => store.delete_group(&p.id).await,
                        _ => store.delete_identity(&p.id).await,
                    }
                    .map_err(internal)?;
                    if !deleted {
                        return Err(not_found());
                    }
                }
                _ => unreachable!("método registrado sem implementação: {}", self.name),
            }
            Ok(Value::Null)
        })
    }
}

impl ConnectionMethod {
    async fn save_identity(&self, p: SaveIdentity) -> Result<(), MethodError> {
        check_id(&p.id)?;
        check_name(&p.name)?;
        if p.username.trim().is_empty()
            || p.username.len() > MAX_NAME
            || p.username.chars().any(char::is_whitespace)
        {
            return Err(MethodError::invalid_params("Usuário inválido"));
        }
        // O envelope é selado antes de gravar: com o cofre bloqueado, nada muda.
        let sealed = match p.password {
            PasswordChange::Keep => None,
            PasswordChange::Clear => Some(None),
            PasswordChange::Set(password) => {
                let password = Zeroizing::new(password);
                if password.is_empty() || password.len() > MAX_PASSWORD {
                    return Err(MethodError::invalid_params("Senha vazia ou grande demais"));
                }
                let envelope = self.vault.seal(&p.id, password.as_bytes())?;
                Some(Some(
                    serde_json::to_string(&envelope).map_err(|e| internal(e.into()))?,
                ))
            }
        };
        let identity = Identity {
            id: p.id,
            name: p.name,
            username: p.username,
            key_id: p.key_id,
            has_password: false,
        };
        self.store
            .save_identity(&identity, sealed.as_ref().map(Option::as_deref))
            .await
            .map_err(write_error)
    }
}

/// Troca (ou acrescenta) o item com o mesmo id, para validar o estado como ficaria.
fn replace<T>(items: &mut Vec<T>, item: T, id: fn(&T) -> &String) {
    match items.iter().position(|i| id(i) == id(&item)) {
        Some(pos) => items[pos] = item,
        None => items.push(item),
    }
}

fn check_jumps(all: &Connections) -> Result<(), MethodError> {
    match jump_cycle(all) {
        Some(name) => Err(MethodError::new(
            errors::CYCLE,
            format!("Os jump hosts de {name} formam um ciclo"),
        )),
        None => Ok(()),
    }
}

fn validate_group(group: &HostGroup) -> Result<(), MethodError> {
    check_id(&group.id)?;
    check_name(&group.name)?;
    check_json(&group.defaults.advanced)
}

fn validate_host(host: &Host) -> Result<(), MethodError> {
    check_id(&host.id)?;
    check_name(&host.name)?;
    if host.address.is_empty()
        || host.address.len() > MAX_ADDRESS
        || host
            .address
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(MethodError::invalid_params("Endereço inválido"));
    }
    if host.port == 0 {
        return Err(MethodError::invalid_params("Porta inválida"));
    }
    if host.jump_host_id.as_deref() == Some(host.id.as_str()) {
        return Err(MethodError::new(
            errors::CYCLE,
            "Um host não pode ser o próprio jump host",
        ));
    }
    if host.tags.len() > MAX_TAGS
        || host
            .tags
            .iter()
            .any(|t| t.trim().is_empty() || t.len() > MAX_TAG)
    {
        return Err(MethodError::invalid_params(format!(
            "Até {MAX_TAGS} tags, cada uma com até {MAX_TAG} caracteres"
        )));
    }
    if host.notes.len() > MAX_NOTES || host.color.as_ref().is_some_and(|c| c.len() > 32) {
        return Err(MethodError::invalid_params("Notas ou cor grandes demais"));
    }
    check_json(&host.advanced)
}

fn check_id(id: &str) -> Result<(), MethodError> {
    if id.is_empty() || id.len() > MAX_ID {
        return Err(MethodError::invalid_params("Id inválido"));
    }
    Ok(())
}

fn check_name(name: &str) -> Result<(), MethodError> {
    if name.trim().is_empty() || name.len() > MAX_NAME {
        return Err(MethodError::invalid_params("Nome vazio ou grande demais"));
    }
    Ok(())
}

/// "Avançado" é um objeto (ou vazio) de tamanho limitado.
fn check_json(value: &Value) -> Result<(), MethodError> {
    if !(value.is_null() || value.is_object()) || value.to_string().len() > MAX_JSON_BYTES {
        return Err(MethodError::invalid_params(
            "Configuração avançada inválida",
        ));
    }
    Ok(())
}

fn not_found() -> MethodError {
    MethodError::new(errors::NOT_FOUND, "Conexão não encontrada")
}

/// Referência a grupo, identidade, chave ou host que não existe vira erro do cliente.
#[allow(clippy::needless_pass_by_value)]
fn write_error(error: Error) -> MethodError {
    if let Error::Query(sqlx::Error::Database(db)) = &error
        && db.is_foreign_key_violation()
    {
        return MethodError::invalid_params("Grupo, identidade, chave ou host inexistente");
    }
    internal(error)
}

#[allow(clippy::needless_pass_by_value)]
fn internal(error: Error) -> MethodError {
    tracing::error!(%error, "erro no banco local");
    MethodError::new("store_failed", "Erro no banco local")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::MemoryKeychain;
    use codatos_protocol::vault::VaultProtection;

    struct Env {
        store: Store,
        vault: Vault,
    }

    impl Env {
        async fn new() -> Self {
            let store = Store::in_memory().await.unwrap();
            let vault = Vault::open(store.clone(), Arc::new(MemoryKeychain::default()))
                .await
                .unwrap();
            Self { store, vault }
        }

        async fn call(&self, name: &str, value: Value) -> Result<Value, MethodError> {
            let all = connection_methods(&self.store, &self.vault);
            let method = all.iter().find(|m| m.name() == name).unwrap();
            method.call(value).await
        }
    }

    #[tokio::test]
    async fn cadastro_e_heranca() {
        let env = Env::new().await;
        env.call(
            METHOD_IDENTITY_SAVE,
            json!({"id": "i1", "name": "deploy", "username": "deploy"}),
        )
        .await
        .unwrap();
        env.call(
            METHOD_GROUP_SAVE,
            json!({"id": "g1", "name": "Produção", "defaults": {"identityId": "i1"}}),
        )
        .await
        .unwrap();
        env.call(
            METHOD_HOST_SAVE,
            json!({"id": "h1", "name": "web", "address": "10.0.0.5", "groupId": "g1", "tags": ["prod"]}),
        )
        .await
        .unwrap();

        let list = env
            .call(METHOD_CONNECTIONS_LIST, Value::Null)
            .await
            .unwrap();
        assert_eq!(list["hosts"][0]["port"], 22);
        assert_eq!(list["identities"][0]["hasPassword"], false);
        let web = env.store.effective_host("h1").await.unwrap().unwrap();
        assert_eq!(web.identity_id.as_deref(), Some("i1"));

        // Apagar o grupo leva o host para a raiz e a herança some.
        env.call(METHOD_GROUP_DELETE, json!({"id": "g1"}))
            .await
            .unwrap();
        let web = env.store.effective_host("h1").await.unwrap().unwrap();
        assert_eq!(web.host.group_id, None);
        assert_eq!(web.identity_id, None);

        let err = env
            .call(METHOD_HOST_DELETE, json!({"id": "nada"}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::NOT_FOUND);
    }

    #[tokio::test]
    async fn recusa_referencias_e_ciclos() {
        let env = Env::new().await;
        let err = env
            .call(
                METHOD_HOST_SAVE,
                json!({"id": "h1", "name": "web", "address": "web", "identityId": "nada"}),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid_params");

        env.call(
            METHOD_HOST_SAVE,
            json!({"id": "a", "name": "a", "address": "a"}),
        )
        .await
        .unwrap();
        env.call(
            METHOD_HOST_SAVE,
            json!({"id": "b", "name": "b", "address": "b", "jumpHostId": "a"}),
        )
        .await
        .unwrap();
        let err = env
            .call(
                METHOD_HOST_SAVE,
                json!({"id": "a", "name": "a", "address": "a", "jumpHostId": "b"}),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::CYCLE);

        env.call(METHOD_GROUP_SAVE, json!({"id": "g1", "name": "g1"}))
            .await
            .unwrap();
        env.call(
            METHOD_GROUP_SAVE,
            json!({"id": "g2", "name": "g2", "parentId": "g1"}),
        )
        .await
        .unwrap();
        let err = env
            .call(
                METHOD_GROUP_SAVE,
                json!({"id": "g1", "name": "g1", "parentId": "g2"}),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::CYCLE);

        let err = env
            .call(
                METHOD_HOST_SAVE,
                json!({"id": "c", "name": "c", "address": "com espaço"}),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid_params");
    }

    #[tokio::test]
    async fn senha_vai_selada_e_exige_o_cofre() {
        let env = Env::new().await;
        let save = |password: Value| json!({"id": "i1", "name": "root", "username": "root", "password": password});
        // Sem cofre aberto, senha nova é recusada e nada é gravado.
        let err = env
            .call(
                METHOD_IDENTITY_SAVE,
                save(json!({"action": "set", "value": "s3gredo!"})),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "vault_absent");
        assert_eq!(env.store.connections().await.unwrap().identities.len(), 0);

        env.vault
            .create(
                VaultProtection::Password,
                Some(Zeroizing::new("senha-do-cofre".into())),
            )
            .await
            .unwrap();
        env.call(
            METHOD_IDENTITY_SAVE,
            save(json!({"action": "set", "value": "s3gredo!"})),
        )
        .await
        .unwrap();
        let sealed = env.store.identity_password("i1").await.unwrap().unwrap();
        assert!(!sealed.contains("s3gredo!"));
        let envelope = serde_json::from_str(&sealed).unwrap();
        assert_eq!(
            env.vault.open_item("i1", &envelope).unwrap().as_slice(),
            b"s3gredo!"
        );
        // Outra identidade não abre o envelope desta.
        assert!(env.vault.open_item("i2", &envelope).is_err());

        // Sem `password`, mantém; com `clear`, apaga.
        env.call(
            METHOD_IDENTITY_SAVE,
            json!({"id": "i1", "name": "root", "username": "admin"}),
        )
        .await
        .unwrap();
        let list = env.store.connections().await.unwrap();
        assert!(list.identities[0].has_password);
        assert_eq!(list.identities[0].username, "admin");
        env.call(METHOD_IDENTITY_SAVE, save(json!({"action": "clear"})))
            .await
            .unwrap();
        assert!(env.store.identity_password("i1").await.unwrap().is_none());
    }
}
