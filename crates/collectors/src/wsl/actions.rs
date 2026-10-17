//! Ações do WSL (Q-14): ligar, desligar uma distro, desligar tudo e trocar a padrão.
//!
//! O nome da distro só passa adiante se estiver no Registro: nada que venha do cliente vira
//! argumento do `wsl.exe` sem essa checagem.

use std::time::Duration;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::system::{
    METHOD_WSL_SET_DEFAULT, METHOD_WSL_SHUTDOWN, METHOD_WSL_START, METHOD_WSL_TERMINATE,
    WslDistroName,
};
use serde_json::Value;

use super::collector::registered;
use crate::command;

/// Ligar uma distro pode levar o boot da VM inteira.
const SLOW: Duration = Duration::from_mins(1);

#[derive(Debug, Clone, Copy)]
pub(crate) enum WslAction {
    Start,
    Terminate,
    Shutdown,
    SetDefault,
}

impl Method for WslAction {
    fn name(&self) -> &'static str {
        match self {
            Self::Start => METHOD_WSL_START,
            Self::Terminate => METHOD_WSL_TERMINATE,
            Self::Shutdown => METHOD_WSL_SHUTDOWN,
            Self::SetDefault => METHOD_WSL_SET_DEFAULT,
        }
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        let action = *self;
        Box::pin(async move {
            let result = if let Self::Shutdown = action {
                command::run("wsl.exe", &["--shutdown"], SLOW).await
            } else {
                let p: WslDistroName = params(value)?;
                let known = registered()
                    .map_err(|e| MethodError::new(e.code(), e.to_string()))?
                    .into_iter()
                    .any(|r| r.name == p.distro);
                if !known {
                    return Err(MethodError::invalid_params(format!(
                        "Distro desconhecida: {}",
                        p.distro
                    )));
                }
                let d = p.distro.as_str();
                match action {
                    // A distro fica ligada enquanto algo a usa (regras de ociosidade do WSL).
                    Self::Start => {
                        command::run("wsl.exe", &["-d", d, "--exec", "/bin/true"], SLOW).await
                    }
                    Self::Terminate => command::run("wsl.exe", &["--terminate", d], SLOW).await,
                    Self::SetDefault => {
                        command::run("wsl.exe", &["--set-default", d], command::TIMEOUT).await
                    }
                    Self::Shutdown => unreachable!("tratado acima"),
                }
            };
            result
                .map(|()| Value::Null)
                .map_err(|e| MethodError::new("wsl_failed", format!("O WSL recusou: {e}")))
        })
    }
}

pub(crate) fn methods() -> Vec<WslAction> {
    vec![
        WslAction::Start,
        WslAction::Terminate,
        WslAction::Shutdown,
        WslAction::SetDefault,
    ]
}
