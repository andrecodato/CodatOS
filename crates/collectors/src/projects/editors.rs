//! Projetos recentes do VS Code e dos IDEs JetBrains (RF-PROJ-002).
//!
//! VS Code: cada pasta aberta tem um diretório em `User/workspaceStorage/<hash>/` com um
//! `workspace.json` (`{"folder": "file:///c%3A/dev/x"}`); a data de modificação do diretório
//! é o último uso. Pastas do WSL aparecem como `vscode-remote://wsl%2Bubuntu/home/...`.
//!
//! JetBrains: `<produto><versão>/options/recentProjects.xml`, com `activationTimestamp` (ms).

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Um projeto lembrado por um editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Recent {
    pub(super) path: String,
    pub(super) wsl_distro: Option<String>,
    pub(super) opened_at: i64,
    pub(super) editor: &'static str,
}

/// Recentes de todos os editores encontrados nesta máquina.
pub(super) fn recents() -> Vec<Recent> {
    let mut list = Vec::new();
    if let Some(dir) = config_dir() {
        list.extend(vscode(
            &dir.join("Code").join("User").join("workspaceStorage"),
        ));
        list.extend(jetbrains(&dir.join("JetBrains")));
    }
    list
}

/// `%APPDATA%` no Windows, `~/Library/Application Support` no macOS, `~/.config` no Linux.
fn config_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        super::scan::home_dir().map(|h| h.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| super::scan::home_dir().map(|h| h.join(".config")))
    }
}

fn vscode(storage: &Path) -> Vec<Recent> {
    let Ok(entries) = std::fs::read_dir(storage) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let json = std::fs::read_to_string(entry.path().join("workspace.json")).ok()?;
            let folder = serde_json::from_str::<serde_json::Value>(&json)
                .ok()?
                .get("folder")?
                .as_str()?
                .to_owned();
            let (path, wsl_distro) = from_vscode_uri(&folder)?;
            let opened_at = modified(&entry.path())?;
            Some(Recent {
                path,
                wsl_distro,
                opened_at,
                editor: "vscode",
            })
        })
        .collect()
}

/// `file:///c%3A/dev/x` → `C:\dev\x`; `vscode-remote://wsl%2Bubuntu/home/a/x` → `/home/a/x` na
/// distro `ubuntu`. Outros remotos (SSH, containers) ficam de fora.
pub(super) fn from_vscode_uri(uri: &str) -> Option<(String, Option<String>)> {
    if let Some(rest) = uri.strip_prefix("file://") {
        let path = percent_decode(rest);
        return Some((native_path(&path), None));
    }
    let rest = uri.strip_prefix("vscode-remote://")?;
    let (authority, path) = rest.split_once('/')?;
    let distro = percent_decode(authority).strip_prefix("wsl+")?.to_owned();
    Some((format!("/{}", percent_decode(path)), Some(distro)))
}

/// `/c:/dev/x` → `C:\dev\x` no Windows; nos outros SOs fica como está.
fn native_path(path: &str) -> String {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
        let mut p = path[1..].replace('/', "\\");
        p[..1].make_ascii_uppercase();
        return p;
    }
    path.to_owned()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn jetbrains(root: &Path) -> Vec<Recent> {
    let Ok(products) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let home = super::scan::home_dir()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_default();
    products
        .flatten()
        .filter_map(|p| {
            std::fs::read_to_string(p.path().join("options").join("recentProjects.xml")).ok()
        })
        .flat_map(|xml| parse_jetbrains(&xml, &home))
        .collect()
}

/// Lê os `<entry key="...">` com `activationTimestamp` do `recentProjects.xml`.
pub(super) fn parse_jetbrains(xml: &str, home: &str) -> Vec<Recent> {
    let mut list = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<entry key=\"") {
        rest = &rest[start + "<entry key=\"".len()..];
        let Some(end) = rest.find('"') else { break };
        let key = &rest[..end];
        // O bloco desta entrada vai até a próxima.
        let block_end = rest.find("<entry key=\"").unwrap_or(rest.len());
        let block = &rest[..block_end];
        let opened_ms = attribute(block, "activationTimestamp")
            .or_else(|| attribute(block, "projectOpenTimestamp"))
            .and_then(|v| v.parse::<i64>().ok());
        if let Some(ms) = opened_ms {
            let path = key.replace("$USER_HOME$", home);
            let (path, wsl_distro) = from_jetbrains_path(&path);
            list.push(Recent {
                path,
                wsl_distro,
                opened_at: ms / 1000,
                editor: "jetbrains",
            });
        }
        rest = &rest[block_end..];
    }
    list
}

/// `//wsl$/Ubuntu/home/a/x` ou `//wsl.localhost/Ubuntu/...` → caminho na distro.
fn from_jetbrains_path(path: &str) -> (String, Option<String>) {
    for prefix in ["//wsl$/", "//wsl.localhost/"] {
        if let Some(rest) = path.strip_prefix(prefix)
            && let Some((distro, inner)) = rest.split_once('/')
        {
            return (format!("/{inner}"), Some(distro.to_owned()));
        }
    }
    let native = if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.to_owned()
    };
    (native, None)
}

fn attribute<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("name=\"{name}\" value=\"");
    let start = block.find(&needle)? + needle.len();
    let len = block[start..].find('"')?;
    Some(&block[start..start + len])
}

fn modified(path: &Path) -> Option<i64> {
    let time = std::fs::metadata(path).ok()?.modified().ok()?;
    i64::try_from(time.duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_do_vscode() {
        assert_eq!(
            from_vscode_uri("file:///c%3A/dev/CodatOS"),
            Some((r"C:\dev\CodatOS".to_owned(), None))
        );
        assert_eq!(
            from_vscode_uri("file:///home/andre/x"),
            Some(("/home/andre/x".to_owned(), None))
        );
        assert_eq!(
            from_vscode_uri("vscode-remote://wsl%2Bubuntu-24.04/home/andre/api"),
            Some((
                "/home/andre/api".to_owned(),
                Some("ubuntu-24.04".to_owned())
            ))
        );
        assert_eq!(
            from_vscode_uri("vscode-remote://ssh-remote%2Bvps/srv/x"),
            None
        );
    }

    #[test]
    fn recent_projects_do_jetbrains() {
        let xml = r#"<application><component name="RecentProjectsManager"><option name="additionalInfo"><map>
            <entry key="$USER_HOME$/dev/loja">
              <value><RecentProjectMetaInfo frameTitle="loja">
                <option name="activationTimestamp" value="1760000000123" />
              </RecentProjectMetaInfo></value>
            </entry>
            <entry key="//wsl$/Ubuntu/home/andre/api">
              <value><RecentProjectMetaInfo><option name="projectOpenTimestamp" value="1750000000000" /></RecentProjectMetaInfo></value>
            </entry>
            <entry key="C:/sem/data"><value><RecentProjectMetaInfo /></value></entry>
        </map></option></component></application>"#;
        let list = parse_jetbrains(xml, "/home/andre");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].opened_at, 1_760_000_000);
        assert!(list[0].path.ends_with("loja"));
        assert_eq!(list[0].wsl_distro, None);
        assert_eq!(list[1].path, "/home/andre/api");
        assert_eq!(list[1].wsl_distro.as_deref(), Some("Ubuntu"));
    }

    #[test]
    fn workspace_storage_do_vscode() {
        let root = std::env::temp_dir().join(format!("codatos-vscode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("abc")).unwrap();
        std::fs::write(
            root.join("abc").join("workspace.json"),
            r#"{"folder":"file:///home/a/x"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.join("vazio")).unwrap();
        let list = vscode(&root);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/home/a/x");
        assert!(list[0].opened_at > 0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
