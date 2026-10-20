//! Túneis (RF-CON-040): encaminhamento de porta local (`-L`), remoto (`-R`) e dinâmico
//! (`-D`, SOCKS5) por um host cadastrado. Ficam salvos; o núcleo os mantém abertos,
//! reconecta quando a conexão cai e sobe com o app os marcados com `autoStart`.

use serde::{Deserialize, Serialize};

/// Lista os túneis com o estado de cada um; devolve [`Tunnels`].
pub const METHOD_TUNNELS_LIST: &str = "tunnels.list";
/// Cria ou atualiza ([`Tunnel`]); um túnel aberto reinicia com a configuração nova.
pub const METHOD_TUNNEL_SAVE: &str = "tunnels.save";
/// Fecha e apaga ([`TunnelId`]).
pub const METHOD_TUNNEL_DELETE: &str = "tunnels.delete";
/// Abre ([`StartTunnel`]). Responde quando o túnel está escutando, ou com o erro da
/// conexão (os mesmos do terminal SSH, para a UI pedir usuário, fingerprint ou senha).
pub const METHOD_TUNNEL_START: &str = "tunnels.start";
/// Fecha ([`TunnelId`]).
pub const METHOD_TUNNEL_STOP: &str = "tunnels.stop";

pub mod errors {
    pub const NOT_FOUND: &str = "tunnel_not_found";
    /// A porta local não abriu (em uso ou sem permissão).
    pub const BIND_FAILED: &str = "tunnel_bind_failed";
    /// O servidor recusou o encaminhamento remoto (`AllowTcpForwarding` ou porta em uso).
    pub const REMOTE_REFUSED: &str = "tunnel_remote_refused";
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TunnelKind {
    /// `-L`: uma porta deste PC leva a `target` visto do servidor.
    #[default]
    Local,
    /// `-R`: uma porta do servidor leva a `target` visto deste PC.
    Remote,
    /// `-D`: proxy SOCKS5 neste PC que sai pelo servidor.
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub host_id: String,
    #[serde(default)]
    pub kind: TunnelKind,
    /// Onde escutar: neste PC (`-L`, `-D`) ou no servidor (`-R`).
    #[serde(default = "loopback")]
    pub bind_host: String,
    pub bind_port: u16,
    /// Destino; vazio e `0` no dinâmico.
    #[serde(default)]
    pub target_host: String,
    #[serde(default)]
    pub target_port: u16,
    /// Abre sozinho quando o app sobe.
    #[serde(default)]
    pub auto_start: bool,
}

fn loopback() -> String {
    "127.0.0.1".to_owned()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TunnelState {
    #[default]
    Stopped,
    Starting,
    Running,
    /// A conexão caiu; tentando de novo.
    Reconnecting,
    /// Parou com erro (veja `error`); só volta com `tunnels.start`.
    Failed,
}

/// Um túnel e como ele está agora.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    pub state: TunnelState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Conexões passando pelo túnel agora.
    #[serde(default)]
    pub active: u32,
}

/// Resposta de `tunnels.list`, por nome.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tunnels {
    pub tunnels: Vec<TunnelStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelId {
    pub id: String,
}

/// Parâmetros de `tunnels.start`. Usuário, senha e fingerprint valem só para esta abertura
/// e para as reconexões dela.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTunnel {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub trust_host_key: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formatos() {
        let t: Tunnel = serde_json::from_value(json!({
            "id": "t1", "name": "Banco", "hostId": "h1", "bindPort": 5432,
            "targetHost": "localhost", "targetPort": 5432
        }))
        .unwrap();
        assert_eq!(t.kind, TunnelKind::Local);
        assert_eq!(t.bind_host, "127.0.0.1");
        assert!(!t.auto_start);
        let status = TunnelStatus {
            tunnel: t,
            state: TunnelState::Running,
            error: None,
            active: 2,
        };
        let v = serde_json::to_value(&status).unwrap();
        assert_eq!(v["state"], "running");
        assert_eq!(v["bindPort"], 5432);
        assert_eq!(v["active"], 2);
    }
}
