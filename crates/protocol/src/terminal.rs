//! Terminais (`docs/modulos/conexoes.md#terminal`): `terminal.open` abre um canal binário
//! (`crate::channel`) com o processo do terminal.

use serde::{Deserialize, Serialize};

/// Abre um terminal ([`OpenTerminal`]); devolve `{"channel": n, "title": …}`.
pub const METHOD_TERMINAL_OPEN: &str = "terminal.open";

pub mod errors {
    /// O processo do terminal não iniciou (shell ausente, distro ou container inválido).
    pub const SPAWN_FAILED: &str = "terminal_spawn_failed";
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
    }
}
