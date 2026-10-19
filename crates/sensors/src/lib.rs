//! Temperatura e potência da CPU no Windows sem programa externo (Q-13).
//!
//! O driver PawnIO só aceita quem roda como administrador, e o CodatOS roda como usuário
//! comum. Por isso a leitura fica num serviço pequeno do Windows (`codatos-sensors`, o
//! próprio `codatos.exe` copiado para `%ProgramFiles%` e iniciado com `--sensors-service`)
//! que abre o PawnIO, carrega o módulo assinado `AMDFamily17.bin` e responde cada conexão
//! no pipe [`PIPE_NAME`] com uma linha JSON ([`Reply`]). O serviço não lê nada do pipe:
//! não aceita comandos de ninguém.
//!
//! Por enquanto só Ryzen (família 17h em diante); Intel fica para depois.

pub mod ryzen;

#[cfg(windows)]
pub mod client;
#[cfg(windows)]
mod pawnio;
#[cfg(windows)]
mod sampler;
#[cfg(windows)]
pub mod service;

use serde::{Deserialize, Serialize};

/// Nome do serviço no Windows (`sc query codatos-sensors`).
pub const SERVICE_NAME: &str = "codatos-sensors";
/// Nome de exibição em `services.msc`.
pub const SERVICE_DISPLAY_NAME: &str = "CodatOS Sensores";
/// Argumento que faz o `codatos.exe` rodar como o serviço.
pub const SERVICE_ARG: &str = "--sensors-service";
/// Argumento que imprime uma leitura e sai (diagnóstico; precisa de administrador).
pub const ONCE_ARG: &str = "--sensors-once";
/// Pipe onde o serviço entrega as leituras. Só leitura para os clientes.
pub const PIPE_NAME: &str = r"\\.\pipe\codatos-sensors";

/// Erro do cliente quando o serviço não está instalado ou parado.
pub const NOT_INSTALLED: &str = "cpu_sensors_not_installed";
/// CPU sem módulo do PawnIO (por enquanto, tudo que não é Ryzen).
pub const UNSUPPORTED: &str = "cpu_unsupported";
/// O PawnIO não está instalado ou recusou o módulo.
pub const PAWNIO_UNAVAILABLE: &str = "pawnio_unavailable";

/// Uma linha do pipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Reply {
    Ok {
        temperature_c: Option<f32>,
        /// Ausente na primeira leitura: a potência sai da diferença entre duas.
        power_w: Option<f32>,
    },
    Error {
        code: String,
        title: String,
    },
}

impl Reply {
    pub fn error(code: &str, title: impl Into<String>) -> Self {
        Self::Error {
            code: code.to_owned(),
            title: title.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_da_linha() {
        let ok = Reply::Ok {
            temperature_c: Some(45.5),
            power_w: None,
        };
        assert_eq!(
            serde_json::to_string(&ok).unwrap(),
            r#"{"status":"ok","temperatureC":45.5,"powerW":null}"#
        );
        let err = Reply::error(UNSUPPORTED, "x");
        let line = serde_json::to_string(&err).unwrap();
        assert_eq!(
            line,
            r#"{"status":"error","code":"cpu_unsupported","title":"x"}"#
        );
        assert_eq!(serde_json::from_str::<Reply>(&line).unwrap(), err);
    }
}
