//! Estado do git pela CLI: `status --porcelain=v2 --branch` e `log -1`.
//!
//! `--no-optional-locks` impede o `git status` de regravar o índice: sem isso, a própria
//! coleta mudaria a data de modificação do repositório.

use std::path::Path;
use std::time::Duration;

use codatos_protocol::projects::GitStatus;

use crate::command;

const TIMEOUT: Duration = Duration::from_secs(5);

/// Estado de um repositório do host. `None` se o git falhar ou passar do prazo.
pub(super) async fn status(repo: &Path) -> Option<GitStatus> {
    let dir = repo.to_str()?;
    let status = command::output(
        "git",
        &[
            "-C",
            dir,
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--branch",
        ],
        TIMEOUT,
    )
    .await?;
    let log = command::output(
        "git",
        &["-C", dir, "log", "-1", "--format=%ct%x09%s"],
        TIMEOUT,
    )
    .await
    .unwrap_or_default();
    let remote = command::output(
        "git",
        &["-C", dir, "config", "--get", "remote.origin.url"],
        TIMEOUT,
    )
    .await
    .unwrap_or_default();
    Some(parse(&status, &log, &remote))
}

/// Junta as três saídas num [`GitStatus`].
pub(super) fn parse(status: &str, log: &str, remote: &str) -> GitStatus {
    let mut git = GitStatus {
        branch: None,
        ahead: 0,
        behind: 0,
        changes: 0,
        last_commit_at: None,
        last_commit_subject: None,
        remote: Some(remote.trim().to_owned()).filter(|r| !r.is_empty()),
    };
    for line in status.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            git.branch = (head != "(detached)").then(|| head.to_owned());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut parts = ab.split_whitespace();
            git.ahead = parts
                .next()
                .and_then(|a| a.trim_start_matches('+').parse().ok())
                .unwrap_or(0);
            git.behind = parts
                .next()
                .and_then(|b| b.trim_start_matches('-').parse().ok())
                .unwrap_or(0);
        } else if !line.starts_with('#') && !line.is_empty() {
            // `1`/`2` alterados, `u` em conflito, `?` não rastreados.
            git.changes += 1;
        }
    }
    if let Some((ts, subject)) = log.trim_end().split_once('\t') {
        git.last_commit_at = ts.parse().ok();
        git.last_commit_subject = Some(subject.to_owned());
    }
    git
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_com_upstream_e_alteracoes() {
        let status = "# branch.oid 3f2a\n# branch.head main\n# branch.upstream origin/main\n\
            # branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/lib.rs\n? novo.txt\n";
        let git = parse(
            status,
            "1760000000\tfeat: algo\n",
            "git@github.com:andrecodato/CodatOS.git\n",
        );
        assert_eq!(git.branch.as_deref(), Some("main"));
        assert_eq!((git.ahead, git.behind, git.changes), (2, 1, 2));
        assert_eq!(git.last_commit_at, Some(1_760_000_000));
        assert_eq!(git.last_commit_subject.as_deref(), Some("feat: algo"));
        assert_eq!(
            git.remote.as_deref(),
            Some("git@github.com:andrecodato/CodatOS.git")
        );
    }

    #[test]
    fn repositorio_novo_e_head_destacado() {
        let git = parse("# branch.oid (initial)\n# branch.head main\n", "", "");
        assert_eq!(git.branch.as_deref(), Some("main"));
        assert_eq!(git.last_commit_at, None);
        assert_eq!(git.remote, None);
        let git = parse("# branch.head (detached)\n", "", "");
        assert_eq!(git.branch, None);
    }

    #[tokio::test]
    async fn repositorio_de_verdade() {
        let dir = std::env::temp_dir().join(format!("codatos-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .is_ok_and(|o| o.status.success())
        };
        if !git(&["init", "-q", "-b", "main"]) {
            return; // Sem git nesta máquina.
        }
        std::fs::write(dir.join("a.txt"), "x").unwrap();
        assert!(git(&["add", "."]));
        assert!(git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "primeiro"
        ]));
        std::fs::write(dir.join("b.txt"), "y").unwrap();

        let s = status(&dir).await.unwrap();
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.changes, 1);
        assert_eq!(s.last_commit_subject.as_deref(), Some("primeiro"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
