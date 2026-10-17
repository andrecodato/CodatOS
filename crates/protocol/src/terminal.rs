//! Terminais (`docs/modulos/conexoes.md#terminal`): `terminal.open` abre um canal binário
//! (`crate::channel`) com o processo do terminal.

use serde::{Deserialize, Serialize};

/// Abre um terminal ([`OpenTerminal`]); devolve `{"channel": n, "title": …}`.
pub const METHOD_TERMINAL_OPEN: &str = "terminal.open";

pub mod errors {
    /// O processo do terminal não iniciou (shell ausente, distro ou container inválido).
    pub const SPAWN_FAILED: &str = "terminal_spawn_failed";
    /// Primeira conexão a este `host:porta`. `detail` traz a fingerprint SHA-256 da chave
    /// do servidor; a UI mostra e, se o usuário aceitar, repete o pedido com `trustHostKey`.
    pub const HOST_KEY_UNKNOWN: &str = "ssh_host_key_unknown";
    /// A chave do servidor não é a aceita antes (servidor trocado ou ataque). `detail` traz
    /// a fingerprint nova; aceitar substitui a antiga.
    pub const HOST_KEY_CHANGED: &str = "ssh_host_key_changed";
    /// Falha de rede, DNS, tempo esgotado ou na negociação SSH.
    pub const CONNECT_FAILED: &str = "ssh_connect_failed";
    /// O servidor recusou a chave e a senha.
    pub const AUTH_FAILED: &str = "ssh_auth_failed";
    /// Sem senha guardada e a chave (se houver) recusada: a UI pede a senha.
    pub const PASSWORD_REQUIRED: &str = "ssh_password_required";
    /// Host sem identidade e sem `username` no pedido.
    pub const USERNAME_REQUIRED: &str = "ssh_username_required";
}

/// Parâmetros de `terminal.open`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTerminal {
    pub target: TerminalTarget,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

/// Onde o terminal roda.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TerminalTarget {
    /// Shell deste PC. Sem `shell`, o padrão do SO (PowerShell no Windows, `$SHELL` fora).
    Local {
        #[serde(default)]
        shell: Option<LocalShell>,
    },
    /// Distro do WSL (`wsl.exe -d`), só no Windows.
    Wsl { distro: String },
    /// Shell dentro de um container (`docker exec -it`).
    Docker { container: String },
    /// Sessão SSH com um host cadastrado, com a identidade, a herança dos grupos e os jump
    /// hosts.
    Ssh {
        host_id: String,
        /// Usuário só desta conexão; sem ele, o da identidade.
        #[serde(default)]
        username: Option<String>,
        /// Senha digitada agora; não é guardada. Sem ela, a da identidade.
        #[serde(default)]
        password: Option<String>,
        /// Fingerprint (`SHA256:…`) que o usuário aceitou depois de
        /// [`errors::HOST_KEY_UNKNOWN`] ou [`errors::HOST_KEY_CHANGED`].
        #[serde(default)]
        trust_host_key: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalShell {
    /// Windows PowerShell 5.1.
    Powershell,
    /// PowerShell 7 (`pwsh`), se instalado.
    Pwsh,
    Cmd,
    /// `$SHELL` (Linux e macOS).
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn alvos() {
        let p: OpenTerminal =
            serde_json::from_value(json!({"target": {"kind": "wsl", "distro": "Ubuntu"}})).unwrap();
        assert_eq!((p.cols, p.rows), (80, 24));
        assert_eq!(
            p.target,
            TerminalTarget::Wsl {
                distro: "Ubuntu".into()
            }
        );
        let p: OpenTerminal =
            serde_json::from_value(json!({"target": {"kind": "local"}, "cols": 120, "rows": 40}))
                .unwrap();
        assert_eq!(p.target, TerminalTarget::Local { shell: None });
        let p: OpenTerminal = serde_json::from_value(
            json!({"target": {"kind": "ssh", "hostId": "h1", "trustHostKey": "SHA256:x"}}),
        )
        .unwrap();
        assert_eq!(
            p.target,
            TerminalTarget::Ssh {
                host_id: "h1".into(),
                username: None,
                password: None,
                trust_host_key: Some("SHA256:x".into()),
            }
        );
    }
}
