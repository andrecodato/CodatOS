//! Projetos dentro das distros WSL ligadas, lidos de dentro da distro (`docs/modulos/projetos.md`:
//! `\\wsl$` é lento demais). Uma chamada ao `wsl.exe` por distro faz a varredura e o
//! `git status` de todos os repositórios.

use codatos_protocol::projects::GitStatus;

use super::{git, language, scan};

/// Separadores que não aparecem em caminhos nem na saída do git.
const RECORD: char = '\u{1e}';
const SECTION: char = '\u{1d}';

/// Repositório lido de uma distro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WslRepo {
    pub(super) path: String,
    pub(super) git: GitStatus,
    pub(super) language: Option<String>,
}

/// Script `sh` que acha os repositórios sob `roots` e imprime, para cada um, o caminho, o
/// `status`, o `log -1`, o `origin` e os arquivos da raiz.
pub(super) fn script(roots: &[String], max_depth: u8) -> String {
    let roots: Vec<String> = roots.iter().map(|r| shell_path(r)).collect();
    let prune: Vec<String> = scan::IGNORED
        .iter()
        .map(|n| format!("-name '{n}'"))
        .collect();
    format!(
        "for r in {roots}; do find \"$r\" -maxdepth {depth} \\( {prune} \\) -prune -o -type d -name .git -prune -print 2>/dev/null; done \
         | while IFS= read -r g; do d=\"${{g%/.git}}\"; printf '\\036%s\\n' \"$d\"; \
         git -C \"$d\" --no-optional-locks status --porcelain=v2 --branch 2>/dev/null; printf '\\035\\n'; \
         git -C \"$d\" log -1 --format='%ct%x09%s' 2>/dev/null; printf '\\035\\n'; \
         git -C \"$d\" config --get remote.origin.url 2>/dev/null; printf '\\035\\n'; \
         ls -1A \"$d\" 2>/dev/null | head -100; done",
        roots = roots.join(" "),
        depth = u16::from(max_depth) + 1,
        prune = prune.join(" -o "),
    )
}

/// `~/x` → `"$HOME"'/x'`; o resto entre aspas simples.
fn shell_path(path: &str) -> String {
    let quote = |s: &str| format!("'{}'", s.replace('\'', r"'\''"));
    match path.strip_prefix('~') {
        Some("") => "\"$HOME\"".to_owned(),
        Some(rest) => format!("\"$HOME\"{}", quote(rest)),
        None => quote(path),
    }
}

/// Lê a saída do [`script`]. Repositórios dentro de outros (submódulos) ficam de fora.
pub(super) fn parse(output: &str) -> Vec<WslRepo> {
    let mut repos: Vec<WslRepo> = output
        .split(RECORD)
        .filter_map(|record| {
            let (path, rest) = record.split_once('\n')?;
            let mut sections = rest.split(SECTION).map(|s| s.trim_start_matches('\n'));
            let status = sections.next()?;
            let log = sections.next().unwrap_or_default();
            let remote = sections.next().unwrap_or_default();
            let files = sections.next().unwrap_or_default();
            Some(WslRepo {
                path: path.to_owned(),
                git: git::parse(status, log, remote),
                language: language::detect(files.lines()),
            })
        })
        .collect();
    repos.sort_by(|a, b| a.path.cmp(&b.path));
    let mut kept: Vec<WslRepo> = Vec::with_capacity(repos.len());
    for repo in repos {
        let nested = kept
            .last()
            .is_some_and(|p| repo.path.starts_with(&format!("{}/", p.path)));
        if !nested {
            kept.push(repo);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminhos_no_shell() {
        assert_eq!(shell_path("~"), "\"$HOME\"");
        assert_eq!(shell_path("~/dev"), "\"$HOME\"'/dev'");
        assert_eq!(shell_path("/srv/it's"), r"'/srv/it'\''s'");
    }

    #[test]
    fn script_roda_num_sh_de_verdade() {
        // Só onde houver sh e git (Linux/macOS e CI).
        let root = std::env::temp_dir().join(format!("codatos-wsl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        let ok = std::process::Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .arg(root.join("app"))
            .status()
            .is_ok_and(|s| s.success());
        if !ok || cfg!(windows) {
            return;
        }
        std::fs::write(root.join("app").join("Cargo.toml"), "").unwrap();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(script(&[root.to_string_lossy().into_owned()], 3))
            .output()
            .unwrap();
        let repos = parse(&String::from_utf8_lossy(&out.stdout));
        assert_eq!(repos.len(), 1, "{repos:?}");
        assert!(repos[0].path.ends_with("/app"));
        assert_eq!(repos[0].git.branch.as_deref(), Some("main"));
        assert_eq!(repos[0].git.changes, 1);
        assert_eq!(repos[0].language.as_deref(), Some("Rust"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn submodulos_ficam_de_fora() {
        let out = "\u{1e}/home/a/x\n# branch.head main\n\u{1d}\n\u{1d}\n\u{1d}\n\
                   \u{1e}/home/a/x/sub\n# branch.head main\n\u{1d}\n\u{1d}\n\u{1d}\n\
                   \u{1e}/home/a/xy\n# branch.head dev\n\u{1d}\n1760000000\tok\n\u{1d}\n\u{1d}\npackage.json\n";
        let repos = parse(out);
        let paths: Vec<&str> = repos.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, ["/home/a/x", "/home/a/xy"]);
        assert_eq!(repos[1].git.last_commit_at, Some(1_760_000_000));
        assert_eq!(repos[1].language.as_deref(), Some("JavaScript"));
    }
}
