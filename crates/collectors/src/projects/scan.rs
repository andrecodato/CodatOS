//! Varredura das pastas raiz do host atrás de repositórios git (RF-PROJ-001).

use std::path::{Path, PathBuf};

/// Pastas que nunca têm projetos dentro e podem ser enormes.
pub(super) const IGNORED: &[&str] = &[
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    "build",
    "vendor",
    ".cache",
    ".gradle",
    ".idea",
    ".vscode",
    "bin",
    "obj",
];

/// Repositórios até `max_depth` níveis abaixo de `root`. Não desce dentro de um repositório.
pub(super) fn repositories(root: &Path, max_depth: u8) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, max_depth, &mut found);
    found.sort();
    found
}

fn walk(dir: &Path, depth_left: u8, found: &mut Vec<PathBuf>) {
    if dir.join(".git").exists() {
        found.push(dir.to_path_buf());
        return;
    }
    if depth_left == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        // `file_type` não segue links simbólicos: evita laços.
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || IGNORED.contains(&name.as_ref()) {
            continue;
        }
        walk(&entry.path(), depth_left - 1, found);
    }
}

/// `~` e `~/x` viram a pasta pessoal do usuário.
pub(super) fn expand_home(path: &str) -> Option<PathBuf> {
    match path.strip_prefix('~') {
        Some(rest) => {
            let home = home_dir()?;
            Some(home.join(rest.trim_start_matches(['/', '\\'])))
        }
        None => Some(PathBuf::from(path)),
    }
}

pub(super) fn home_dir() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_repositorios_e_respeita_profundidade_e_ignorados() {
        let root = std::env::temp_dir().join(format!("codatos-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in [
            "a/.git",
            "grupo/b/.git",
            "grupo/b/sub/.git", // dentro de outro repositório: não conta
            "x/node_modules/c/.git",
            "fundo/1/2/3/d/.git", // além da profundidade
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        let found: Vec<String> = repositories(&root, 3)
            .iter()
            .map(|p| {
                p.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(found, ["a", "grupo/b"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
