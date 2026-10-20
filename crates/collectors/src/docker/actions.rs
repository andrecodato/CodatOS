//! Ações dos containers (RF-SYS-033): iniciar, parar e reiniciar.
//!
//! Nada do cliente vira argumento sem checagem: o container precisa ser um ID
//! hexadecimal e a distro, uma das registradas no WSL.

use std::time::Duration;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::system::{
    ContainerRef, METHOD_DOCKER_RESTART, METHOD_DOCKER_START, METHOD_DOCKER_STOP,
};
use serde_json::Value;

use super::invocation;
use crate::command;

/// `stop` espera até 10 s o container sair sozinho; `restart` soma o tempo de subir.
const SLOW: Duration = Duration::from_mins(1);

#[derive(Debug, Clone, Copy)]
pub(crate) enum DockerAction {
    Start,
    Stop,
    Restart,
}

impl DockerAction {
    fn verb(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
        }
    }
}

impl Method for DockerAction {
    fn name(&self) -> &'static str {
        match self {
            Self::Start => METHOD_DOCKER_START,
            Self::Stop => METHOD_DOCKER_STOP,
            Self::Restart => METHOD_DOCKER_RESTART,
        }
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        let action = *self;
        Box::pin(async move {
            let p: ContainerRef = params(value)?;
            if !container_id(&p.container) {
                return Err(MethodError::invalid_params("ID de container inválido"));
            }
            if let Some(distro) = &p.wsl_distro {
                check_distro(distro)?;
            }
            let (program, args) =
                invocation(p.wsl_distro.as_deref(), &[action.verb(), &p.container]);
            command::run(program, &args, SLOW)
                .await
                .map(|()| Value::Null)
                .map_err(|e| MethodError::new("docker_failed", format!("O Docker recusou: {e}")))
        })
    }
}

/// O ID do `docker ps`, completo (64) ou curto (12 ou mais).
fn container_id(id: &str) -> bool {
    (12..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(windows)]
fn check_distro(distro: &str) -> Result<(), MethodError> {
    let known = crate::wsl::registered()
        .map_err(|e| MethodError::new(e.code(), e.to_string()))?
        .into_iter()
        .any(|r| r.name == distro);
    if known {
        Ok(())
    } else {
        Err(MethodError::invalid_params(format!(
            "Distro desconhecida: {distro}"
        )))
    }
}

/// Fora do Windows não há WSL: só o engine local.
#[cfg(not(windows))]
fn check_distro(distro: &str) -> Result<(), MethodError> {
    Err(MethodError::invalid_params(format!(
        "Distro desconhecida: {distro}"
    )))
}

pub(crate) fn methods() -> Vec<DockerAction> {
    vec![
        DockerAction::Start,
        DockerAction::Stop,
        DockerAction::Restart,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_aceita_id_hexadecimal() {
        assert!(container_id("0123456789ab"));
        assert!(container_id(&"f".repeat(64)));
        for bad in [
            "",
            "abc",
            "--privileged",
            "nome-do-container",
            &"a".repeat(65),
        ] {
            assert!(!container_id(bad), "{bad}");
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn sem_wsl_nenhuma_distro_vale() {
        assert_eq!(check_distro("Ubuntu").unwrap_err().code(), "invalid_params");
    }
}
