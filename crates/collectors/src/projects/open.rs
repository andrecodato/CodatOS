//! `projects.open` (RF-PROJ-004): abre o projeto no VS Code, no Windows Terminal ou no
//! explorador. Os programas são chamados direto, sem shell; o Rust escapa os argumentos
//! (inclusive para o `code.cmd`, que é um script em lote).

use std::path::{Path, PathBuf};

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::projects::{METHOD_PROJECTS_OPEN, OpenProject, OpenTarget};
use serde_json::Value;

use crate::command;

#[derive(Debug)]
pub(crate) struct Open;

impl Method for Open {
    fn name(&self) -> &'static str {
        METHOD_PROJECTS_OPEN
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let p: OpenProject = params(value)?;
            let (program, args) = match &p.wsl_distro {
                None => host(&p.path, p.target).await?,
                Some(distro) => wsl(distro, &p.path, p.target)?,
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            command::launch(&program, &args).map_err(|e| {
                MethodError::new(
                    "open_failed",
                    format!("Não consegui abrir {}: {e}", program.display()),
                )
            })?;
            Ok(Value::Null)
        })
    }
}

fn missing(what: &str) -> MethodError {
    MethodError::new(
        "open_unavailable",
        format!("{what} não encontrado neste PC"),
    )
}

/// VS Code pelo `code` do `PATH` (no Windows, `code.cmd`).
fn vscode() -> Result<PathBuf, MethodError> {
    let name = if cfg!(windows) { "code.cmd" } else { "code" };
    command::which(name).ok_or_else(|| missing("VS Code"))
}

async fn host(path: &str, target: OpenTarget) -> Result<(PathBuf, Vec<String>), MethodError> {
    let is_dir = tokio::fs::metadata(path).await.is_ok_and(|m| m.is_dir());
    if !Path::new(path).is_absolute() || !is_dir {
        return Err(MethodError::invalid_params(format!(
            "Pasta inexistente: {path}"
        )));
    }
    let path = path.to_owned();
    Ok(match target {
        OpenTarget::Editor => (vscode()?, vec![path]),
        OpenTarget::Terminal if cfg!(windows) => (
            command::which("wt.exe").ok_or_else(|| missing("Windows Terminal"))?,
            vec!["-d".to_owned(), path],
        ),
        OpenTarget::Terminal => {
            return Err(MethodError::new(
                "open_unavailable",
                "Abrir o terminal só funciona no Windows por enquanto",
            ));
        }
        OpenTarget::Explorer => (file_manager()?, vec![path]),
    })
}

fn file_manager() -> Result<PathBuf, MethodError> {
    let name = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    command::which(name).ok_or_else(|| missing("Explorador de arquivos"))
}

/// Nome de distro que o `wsl.exe` aceita; o resto é recusado antes de virar argumento.
fn valid_distro(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Caminho Linux absoluto, sem caracteres de controle.
fn valid_linux_path(path: &str) -> bool {
    path.starts_with('/') && !path.chars().any(char::is_control)
}

fn wsl(
    distro: &str,
    path: &str,
    target: OpenTarget,
) -> Result<(PathBuf, Vec<String>), MethodError> {
    if !cfg!(windows) {
        return Err(MethodError::new(
            "open_unavailable",
            "WSL só existe no Windows",
        ));
    }
    if !valid_distro(distro) || !valid_linux_path(path) {
        return Err(MethodError::invalid_params("Projeto do WSL inválido"));
    }
    let (d, p) = (distro.to_owned(), path.to_owned());
    Ok(match target {
        OpenTarget::Editor => (
            vscode()?,
            vec!["--remote".to_owned(), format!("wsl+{d}"), p],
        ),
        OpenTarget::Terminal => (
            command::which("wt.exe").ok_or_else(|| missing("Windows Terminal"))?,
            vec![
                "wsl.exe".to_owned(),
                "-d".to_owned(),
                d,
                "--cd".to_owned(),
                p,
            ],
        ),
        OpenTarget::Explorer => (file_manager()?, vec![unc(&d, &p)]),
    })
}

/// `\\wsl.localhost\Ubuntu\home\eu\app` para o explorador do Windows.
fn unc(distro: &str, path: &str) -> String {
    format!(r"\\wsl.localhost\{distro}{}", path.replace('/', r"\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn so_aceita_distros_e_caminhos_seguros() {
        assert!(valid_distro("Ubuntu-24.04"));
        assert!(!valid_distro("Ubuntu & calc"));
        assert!(!valid_distro(""));
        assert!(valid_linux_path("/home/eu/app"));
        assert!(!valid_linux_path("home/eu"));
        assert!(!valid_linux_path("/home/eu\napp"));
    }

    #[test]
    fn caminho_unc_do_wsl() {
        assert_eq!(
            unc("Ubuntu", "/home/eu/app"),
            r"\\wsl.localhost\Ubuntu\home\eu\app"
        );
    }

    #[tokio::test]
    async fn pasta_inexistente_e_recusada() {
        let err = host("/nao/existe/mesmo", OpenTarget::Editor)
            .await
            .unwrap_err();
        assert_eq!(err.code(), "invalid_params");
        let err = host("relativo", OpenTarget::Explorer).await.unwrap_err();
        assert_eq!(err.code(), "invalid_params");
    }
}
