//! Arquivos deste PC para o painel local do SFTP (`files.*`): listar, criar pasta, renomear
//! e apagar. Quem pode abrir um terminal aqui já pode tudo isso; os métodos só poupam a UI
//! de passar por um shell.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::files::{
    LocalPath, LocalRemove, LocalRename, METHOD_FILES_LIST, METHOD_FILES_MKDIR,
    METHOD_FILES_REMOVE, METHOD_FILES_RENAME,
};
use codatos_protocol::sftp::{Entry, EntryKind, Listing, errors};
use serde_json::{Value, json};

/// Itens de uma pasta numa listagem (o mesmo limite do SFTP).
const MAX_ITEMS: usize = 10_000;

#[must_use]
pub fn local_file_methods() -> Vec<Arc<dyn Method>> {
    [
        METHOD_FILES_LIST,
        METHOD_FILES_MKDIR,
        METHOD_FILES_RENAME,
        METHOD_FILES_REMOVE,
    ]
    .into_iter()
    .map(|name| Arc::new(LocalFiles { name }) as Arc<dyn Method>)
    .collect()
}

#[derive(Debug)]
struct LocalFiles {
    name: &'static str,
}

impl Method for LocalFiles {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            match self.name {
                METHOD_FILES_LIST => {
                    let p: LocalPath = params(value)?;
                    let path = if p.path.trim().is_empty() {
                        home().ok_or_else(|| {
                            MethodError::new(errors::NOT_FOUND, "Pasta do usuário desconhecida")
                        })?
                    } else {
                        PathBuf::from(p.path)
                    };
                    Ok(json!(list(&path).await?))
                }
                METHOD_FILES_MKDIR => {
                    let p: LocalPath = params(value)?;
                    tokio::fs::create_dir(&p.path).await.map_err(failed)?;
                    Ok(Value::Null)
                }
                METHOD_FILES_RENAME => {
                    let p: LocalRename = params(value)?;
                    if tokio::fs::try_exists(&p.to).await.unwrap_or(false) {
                        return Err(MethodError::new(
                            errors::EXISTS,
                            "Já existe um item com esse nome",
                        ));
                    }
                    tokio::fs::rename(&p.from, &p.to).await.map_err(failed)?;
                    Ok(Value::Null)
                }
                METHOD_FILES_REMOVE => {
                    let p: LocalRemove = params(value)?;
                    let meta = tokio::fs::symlink_metadata(&p.path).await.map_err(failed)?;
                    if !meta.is_dir() {
                        tokio::fs::remove_file(&p.path).await.map_err(failed)?;
                    } else if p.recursive {
                        // Não segue links: um link para fora da pasta some sem levar o alvo.
                        tokio::fs::remove_dir_all(&p.path).await.map_err(failed)?;
                    } else if tokio::fs::read_dir(&p.path)
                        .await
                        .map_err(failed)?
                        .next_entry()
                        .await
                        .map_err(failed)?
                        .is_some()
                    {
                        return Err(MethodError::new(
                            errors::NOT_EMPTY,
                            "A pasta não está vazia",
                        ));
                    } else {
                        tokio::fs::remove_dir(&p.path).await.map_err(failed)?;
                    }
                    Ok(Value::Null)
                }
                _ => unreachable!("método de arquivos registrado sem implementação"),
            }
        })
    }
}

fn home() -> Option<PathBuf> {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).map(PathBuf::from)
}

async fn list(path: &Path) -> Result<Listing, MethodError> {
    let path = tokio::fs::canonicalize(path).await.map_err(failed)?;
    let mut dir = tokio::fs::read_dir(&path).await.map_err(failed)?;
    let mut entries = Vec::new();
    while let Some(item) = dir.next_entry().await.map_err(failed)? {
        if entries.len() >= MAX_ITEMS {
            break;
        }
        // Itens que somem ou negam acesso no meio da listagem ficam de fora.
        let Ok(link) = item.metadata().await else {
            continue;
        };
        let kind = if link.is_symlink() {
            EntryKind::Symlink
        } else if link.is_dir() {
            EntryKind::Dir
        } else if link.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        // O tamanho de um link é o do alvo.
        let meta = if link.is_symlink() {
            tokio::fs::metadata(item.path()).await.unwrap_or(link)
        } else {
            link
        };
        entries.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            kind,
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .and_then(|d| i64::try_from(d.as_secs()).ok()),
            mode: mode(&meta),
        });
    }
    entries.sort_by(|a, b| {
        (a.kind != EntryKind::Dir)
            .cmp(&(b.kind != EntryKind::Dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(Listing {
        path: display(&path),
        entries,
    })
}

#[cfg(unix)]
#[allow(clippy::unnecessary_wraps)] // o Windows não tem modo
fn mode(meta: &std::fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(meta.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn mode(_: &std::fs::Metadata) -> Option<u32> {
    None
}

/// O `canonicalize` do Windows devolve `\\?\C:\…`; a tela mostra `C:\…`.
fn display(path: &Path) -> String {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => rest.to_owned(),
        _ => text.into_owned(),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn failed(error: std::io::Error) -> MethodError {
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            MethodError::new(errors::NOT_FOUND, "Arquivo ou pasta não encontrado")
        }
        std::io::ErrorKind::PermissionDenied => {
            MethodError::new(errors::PERMISSION_DENIED, "Sem permissão neste PC")
        }
        std::io::ErrorKind::AlreadyExists => {
            MethodError::new(errors::EXISTS, "Já existe um item com esse nome")
        }
        _ => MethodError::new(errors::FAILED, format!("Erro no disco deste PC: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminho_do_windows_sem_prefixo() {
        assert_eq!(display(Path::new(r"\\?\C:\Users\andre")), r"C:\Users\andre");
        assert_eq!(display(Path::new(r"\\?\UNC\srv\x")), r"\\?\UNC\srv\x");
        assert_eq!(display(Path::new("/home/andre")), "/home/andre");
    }

    #[tokio::test]
    async fn lista_cria_renomeia_e_apaga() {
        let root = std::env::temp_dir().join(format!("codatos-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let methods = local_file_methods();
        let call = |name: &'static str, params: Value| {
            let method = methods.iter().find(|m| m.name() == name).unwrap().clone();
            async move { method.call(params).await }
        };
        let path = |p: &str| root.join(p).to_string_lossy().into_owned();

        call(METHOD_FILES_MKDIR, json!({"path": path("b")}))
            .await
            .unwrap();
        std::fs::write(root.join("A.txt"), b"abc").unwrap();
        let listing = call(METHOD_FILES_LIST, json!({"path": path("")}))
            .await
            .unwrap();
        let names: Vec<&str> = listing["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["b", "A.txt"]);
        assert_eq!(listing["entries"][1]["size"], 3);

        let err = call(
            METHOD_FILES_RENAME,
            json!({"from": path("A.txt"), "to": path("b")}),
        )
        .await
        .unwrap_err();
        assert_eq!(err.code(), errors::EXISTS);
        call(
            METHOD_FILES_RENAME,
            json!({"from": path("A.txt"), "to": path("b/a.txt")}),
        )
        .await
        .unwrap();
        let err = call(METHOD_FILES_REMOVE, json!({"path": path("b")}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::NOT_EMPTY);
        call(
            METHOD_FILES_REMOVE,
            json!({"path": path("b"), "recursive": true}),
        )
        .await
        .unwrap();
        assert!(!root.join("b").exists());
        let err = call(METHOD_FILES_LIST, json!({"path": path("sumiu")}))
            .await
            .unwrap_err();
        assert_eq!(err.code(), errors::NOT_FOUND);
        let _ = std::fs::remove_dir_all(&root);
    }
}
