//! Projetos recentes (RF-PROJ-001 a 003): repositórios das pastas raiz, do host e das distros
//! WSL ligadas, mais os recentes do VS Code e dos IDEs JetBrains, com o estado do git.
//!
//! A varredura das pastas fica em cache por alguns minutos; o `git status` roda a cada coleta,
//! com prazo e no máximo 8 de cada vez. O *watcher* de arquivos (RF-PROJ-008) vem depois.

mod editors;
mod git;
mod language;
mod scan;
#[cfg(any(windows, test))]
mod wsl;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use codatos_core::{CollectError, CollectFuture, Collector, Intervals};

use crate::DeviceSettings;
use codatos_protocol::projects::{Project, ProjectsConfig, SETTING_PROJECTS, TOPIC_PROJECTS};
use futures_util::StreamExt as _;

/// De quanto em quanto tempo as pastas raiz são varridas de novo.
const RESCAN: Duration = Duration::from_secs(300);
/// `git status` simultâneos.
const PARALLEL_GIT: usize = 8;
/// Teto da lista.
const MAX_PROJECTS: usize = 300;

/// `projects`. Lê a configuração em `settings` (escopo `device`) a cada coleta.
#[derive(Default)]
pub struct ProjectsCollector {
    settings: Option<DeviceSettings>,
    cache: Arc<Mutex<Option<ScanCache>>>,
}

#[derive(Debug)]
struct ScanCache {
    at: Instant,
    config: ProjectsConfig,
    /// Repositório → linguagem.
    repos: Vec<(PathBuf, Option<String>)>,
}

impl std::fmt::Debug for ProjectsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectsCollector").finish_non_exhaustive()
    }
}

impl ProjectsCollector {
    pub fn new(settings: Option<DeviceSettings>) -> Self {
        Self {
            settings,
            cache: Arc::default(),
        }
    }

    async fn config(&self) -> ProjectsConfig {
        let Some(settings) = &self.settings else {
            return ProjectsConfig::default();
        };
        settings(SETTING_PROJECTS)
            .await
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    /// Repositórios do host, do cache ou de uma varredura nova.
    async fn host_repos(&self, config: &ProjectsConfig) -> Vec<(PathBuf, Option<String>)> {
        {
            let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(c) = cache.as_ref()
                && c.at.elapsed() < RESCAN
                && c.config == *config
            {
                return c.repos.clone();
            }
        }
        let roots = config.roots.clone();
        let depth = config.max_depth;
        let repos = tokio::task::spawn_blocking(move || {
            roots
                .iter()
                .filter_map(|r| scan::expand_home(r))
                .flat_map(|root| scan::repositories(&root, depth))
                .map(|repo| {
                    let language = read_names(&repo);
                    (repo, language)
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        *self.cache.lock().unwrap_or_else(PoisonError::into_inner) = Some(ScanCache {
            at: Instant::now(),
            config: config.clone(),
            repos: repos.clone(),
        });
        repos
    }
}

impl Collector for ProjectsCollector {
    fn topic(&self) -> &'static str {
        TOPIC_PROJECTS
    }

    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_secs(30),
            min: Duration::from_secs(10),
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async move {
            let config = self.config().await;
            let mut projects: Vec<Project> =
                futures_util::stream::iter(self.host_repos(&config).await)
                    .map(|(path, language)| async move {
                        Project {
                            name: name_of(&path.to_string_lossy()),
                            git: git::status(&path).await,
                            path: path.to_string_lossy().into_owned(),
                            wsl_distro: None,
                            last_opened_at: None,
                            editor: None,
                            language,
                        }
                    })
                    .buffer_unordered(PARALLEL_GIT)
                    .collect()
                    .await;

            #[cfg(windows)]
            projects.extend(wsl_projects(&config).await);

            if config.editors {
                let recents = tokio::task::spawn_blocking(editors::recents)
                    .await
                    .unwrap_or_default();
                merge_recents(&mut projects, recents).await;
            }

            if projects.is_empty() {
                return Err(CollectError::new(
                    "no_projects",
                    "Nenhum projeto encontrado nas pastas configuradas",
                ));
            }
            sort(&mut projects);
            projects.truncate(MAX_PROJECTS);
            Ok(serde_json::to_value(projects).expect("Project sempre serializa"))
        })
    }
}

#[cfg(windows)]
async fn wsl_projects(config: &ProjectsConfig) -> Vec<Project> {
    let mut list = Vec::new();
    if config.wsl_roots.is_empty() {
        return list;
    }
    let script = wsl::script(&config.wsl_roots, config.max_depth);
    for distro in crate::wsl::running().await {
        if distro.starts_with("docker-desktop") {
            continue;
        }
        // RF-SYS-024: só distros que acabaram de aparecer como ligadas.
        let Some(output) = crate::command::output(
            "wsl.exe",
            &["-d", &distro, "-e", "sh", "-c", &script],
            Duration::from_secs(20),
        )
        .await
        else {
            continue;
        };
        list.extend(wsl::parse(&output).into_iter().map(|repo| Project {
            name: name_of(&repo.path),
            path: repo.path,
            wsl_distro: Some(distro.clone()),
            git: Some(repo.git),
            last_opened_at: None,
            editor: None,
            language: repo.language,
        }));
    }
    list
}

/// Aplica a data de uso dos editores. Pastas do editor que não estão na lista entram também
/// (com git, se forem repositórios do host).
async fn merge_recents(projects: &mut Vec<Project>, recents: Vec<editors::Recent>) {
    let key = |path: &str, distro: Option<&str>| {
        let path = path.trim_end_matches(['/', '\\']);
        let path = if cfg!(windows) && distro.is_none() {
            path.to_lowercase()
        } else {
            path.to_owned()
        };
        (distro.map(str::to_lowercase), path)
    };
    let mut index: HashMap<(Option<String>, String), usize> = projects
        .iter()
        .enumerate()
        .map(|(i, p)| (key(&p.path, p.wsl_distro.as_deref()), i))
        .collect();

    for recent in recents {
        let k = key(&recent.path, recent.wsl_distro.as_deref());
        let i = if let Some(&i) = index.get(&k) {
            i
        } else {
            let host = Path::new(&recent.path);
            // Pasta do host que não existe mais: fica de fora.
            if recent.wsl_distro.is_none() && !host.is_dir() {
                continue;
            }
            let is_host_repo = recent.wsl_distro.is_none() && host.join(".git").exists();
            projects.push(Project {
                name: name_of(&recent.path),
                path: recent.path.clone(),
                wsl_distro: recent.wsl_distro.clone(),
                git: if is_host_repo {
                    git::status(host).await
                } else {
                    None
                },
                last_opened_at: None,
                editor: None,
                language: if recent.wsl_distro.is_none() {
                    read_names(host)
                } else {
                    None
                },
            });
            index.insert(k, projects.len() - 1);
            projects.len() - 1
        };
        let project = &mut projects[i];
        if project.last_opened_at.is_none_or(|t| recent.opened_at > t) {
            project.last_opened_at = Some(recent.opened_at);
            project.editor = Some(recent.editor.to_owned());
        }
    }
}

/// Mais recente primeiro: o maior entre o último uso no editor e o último commit.
fn sort(projects: &mut [Project]) {
    let recency = |p: &Project| {
        p.last_opened_at
            .max(p.git.as_ref().and_then(|g| g.last_commit_at))
            .unwrap_or(0)
    };
    projects.sort_by(|a, b| {
        recency(b)
            .cmp(&recency(a))
            .then_with(|| a.name.cmp(&b.name))
    });
}

fn name_of(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_owned()
}

/// Linguagem pelos nomes de arquivo na raiz do projeto.
fn read_names(dir: &Path) -> Option<String> {
    let names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .take(500)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    language::detect(names.iter().map(String::as_str))
}

#[cfg(test)]
mod tests {
    use codatos_protocol::projects::GitStatus;

    use super::*;

    fn project(name: &str, opened: Option<i64>, commit: Option<i64>) -> Project {
        Project {
            name: name.to_owned(),
            path: format!("/dev/{name}"),
            wsl_distro: None,
            git: commit.map(|c| GitStatus {
                branch: None,
                ahead: 0,
                behind: 0,
                changes: 0,
                last_commit_at: Some(c),
                last_commit_subject: None,
                remote: None,
            }),
            last_opened_at: opened,
            editor: None,
            language: None,
        }
    }

    #[test]
    fn ordena_pelo_uso_mais_recente() {
        let mut list = vec![
            project("a", None, Some(10)),
            project("b", Some(30), Some(5)),
            project("c", None, None),
            project("d", Some(20), None),
        ];
        sort(&mut list);
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["b", "d", "a", "c"]);
    }

    #[test]
    fn nome_pelo_caminho() {
        assert_eq!(name_of(r"C:\dev\CodatOS\"), "CodatOS");
        assert_eq!(name_of("/home/a/api"), "api");
    }

    #[tokio::test]
    async fn recentes_dos_editores_se_juntam_aos_repositorios() {
        let mut list = vec![project("a", None, Some(10))];
        let wsl = |path: &str, at| editors::Recent {
            path: path.to_owned(),
            wsl_distro: Some("Ubuntu".to_owned()),
            opened_at: at,
            editor: "vscode",
        };
        merge_recents(
            &mut list,
            vec![
                editors::Recent {
                    path: "/dev/a/".to_owned(),
                    wsl_distro: None,
                    opened_at: 50,
                    editor: "jetbrains",
                },
                wsl("/home/u/api", 40),
                wsl("/home/u/api", 45),
                // Pasta do host que não existe: ignorada.
                editors::Recent {
                    path: "/nao/existe/mesmo".to_owned(),
                    wsl_distro: None,
                    opened_at: 99,
                    editor: "vscode",
                },
            ],
        )
        .await;
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].last_opened_at, Some(50));
        assert_eq!(list[0].editor.as_deref(), Some("jetbrains"));
        assert_eq!(list[1].name, "api");
        assert_eq!(list[1].last_opened_at, Some(45));
        assert_eq!(list[1].git, None);
    }
}
