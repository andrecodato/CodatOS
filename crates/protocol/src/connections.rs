//! Conexões (`docs/modulos/conexoes.md`): grupos, hosts e identidades.
//!
//! Ids são `UUIDv7` gerados pelo cliente (`docs/05-modelo-de-dados.md`). Senhas vão para o
//! núcleo uma vez, em [`SaveIdentity`], e nunca voltam: a UI só vê `hasPassword`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tudo de uma vez: [`Connections`].
pub const METHOD_CONNECTIONS_LIST: &str = "connections.list";
pub const METHOD_HOST_SAVE: &str = "connections.saveHost";
pub const METHOD_HOST_DELETE: &str = "connections.deleteHost";
pub const METHOD_GROUP_SAVE: &str = "connections.saveGroup";
/// Os filhos e os hosts do grupo sobem para o grupo pai.
pub const METHOD_GROUP_DELETE: &str = "connections.deleteGroup";
/// Senha nova exige o cofre aberto (`vault_locked`).
pub const METHOD_IDENTITY_SAVE: &str = "connections.saveIdentity";
pub const METHOD_IDENTITY_DELETE: &str = "connections.deleteIdentity";

/// Porta padrão do SSH.
pub const DEFAULT_SSH_PORT: u16 = 22;

/// Códigos de erro estáveis.
pub mod errors {
    pub const NOT_FOUND: &str = "connection_not_found";
    /// Um grupo dentro dele mesmo, ou um jump host que volta ao host.
    pub const CYCLE: &str = "connection_cycle";
}

/// Resultado de `connections.list`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connections {
    pub groups: Vec<HostGroup>,
    pub hosts: Vec<Host>,
    pub identities: Vec<Identity>,
}

/// Grupo (pasta) de hosts. Os hosts herdam o que não definirem (RF-CON-002).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostGroup {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub defaults: HostDefaults,
}

/// O que um grupo passa adiante para hosts e subgrupos.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump_host_id: Option<String>,
    /// Aba "Avançado": chaves do grupo valem quando o host não tem a mesma chave.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub advanced: Value,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Protocol {
    /// SSH com SFTP e túneis (Q-10: o único protocolo remoto do M2).
    #[default]
    Ssh,
}

impl Protocol {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        (s == "ssh").then_some(Self::Ssh)
    }
}

/// Host cadastrado (RF-CON-001).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub id: String,
    pub name: String,
    pub address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub protocol: Protocol,
    #[serde(default)]
    pub group_id: Option<String>,
    /// Vazio: herda do grupo.
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub jump_host_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub advanced: Value,
    /// Preenchido pelo núcleo a cada conexão; ignorado em `saveHost`.
    #[serde(default)]
    pub last_used_at: Option<String>,
}

fn default_port() -> u16 {
    DEFAULT_SSH_PORT
}

/// Identidade reutilizável: usuário e senha e/ou chave (RF-CON-010).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub username: String,
    #[serde(default)]
    pub key_id: Option<String>,
    /// Há senha guardada no cofre. A senha em si nunca sai do núcleo.
    #[serde(default)]
    pub has_password: bool,
}

/// Parâmetros de `connections.saveIdentity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveIdentity {
    pub id: String,
    pub name: String,
    pub username: String,
    #[serde(default)]
    pub key_id: Option<String>,
    #[serde(default)]
    pub password: PasswordChange,
}

/// O que fazer com a senha guardada.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action", content = "value")]
pub enum PasswordChange {
    #[default]
    Keep,
    Clear,
    Set(String),
}

/// Parâmetros dos métodos `delete*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionId {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn host_minimo_usa_os_padroes() {
        let host: Host = serde_json::from_value(json!({
            "id": "h1", "name": "web", "address": "web.exemplo.com"
        }))
        .unwrap();
        assert_eq!(host.port, 22);
        assert_eq!(host.protocol, Protocol::Ssh);
        assert!(host.tags.is_empty() && host.group_id.is_none());
    }

    #[test]
    fn mudanca_de_senha() {
        let p: SaveIdentity = serde_json::from_value(json!({
            "id": "i1", "name": "deploy", "username": "deploy"
        }))
        .unwrap();
        assert_eq!(p.password, PasswordChange::Keep);
        let set: PasswordChange =
            serde_json::from_value(json!({"action": "set", "value": "s3gredo"})).unwrap();
        assert_eq!(set, PasswordChange::Set("s3gredo".into()));
        let clear: PasswordChange = serde_json::from_value(json!({"action": "clear"})).unwrap();
        assert_eq!(clear, PasswordChange::Clear);
    }
}
