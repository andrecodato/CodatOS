//! Docker: containers de cada engine, agrupados por projeto Compose (`docker.containers`),
//! com iniciar, parar e reiniciar (RF-SYS-033).

use std::collections::BTreeMap;

use codatos_protocol::system::{
    Container, ContainerRef, DockerStatus, METHOD_DOCKER_RESTART, METHOD_DOCKER_START,
    METHOD_DOCKER_STOP, TOPIC_DOCKER,
};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, decimal};
use super::parts::{ActionButton, Dot};
use crate::api;
use crate::components::icon::Icon;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_DOCKER, None);
    let error = RwSignal::new(None::<String>);
    sample_view(sample, move |status: DockerStatus| {
        let several = status.engines.len() > 1;
        let total: usize = status.engines.iter().map(|e| e.containers.len()).sum();
        let running: usize = status
            .engines
            .iter()
            .flat_map(|e| &e.containers)
            .filter(|c| c.state == "running")
            .count();
        let groups = groups(status, several);
        view! {
            <div class="flex h-full flex-col gap-2">
                <p class="text-xs text-muted">{format!("{running} de {total} em execução")}</p>
                <div class="min-h-0 flex-1 space-y-3 overflow-auto pr-1">
                    {groups
                        .into_iter()
                        .map(|group| {
                            let distro = group.wsl_distro;
                            view! {
                                <section>
                                    {group.title.map(|t| view! { <h4 class="mb-1 text-[11px] tracking-wide text-subtle uppercase">{t}</h4> })}
                                    <ul class="space-y-1">
                                        {group
                                            .containers
                                            .into_iter()
                                            .map(|c| view! { <Row container=c wsl_distro=distro.clone() error=error /> })
                                            .collect_view()}
                                    </ul>
                                </section>
                            }
                        })
                        .collect_view()}
                </div>
                {move || error.get().map(|e| view! { <p role="alert" class="truncate text-xs text-danger" title=e.clone()>{e.clone()}</p> })}
            </div>
        }
    })
}

/// Containers de um projeto Compose (ou os soltos) de um engine.
pub(crate) struct Group {
    pub title: Option<String>,
    /// Distro do engine, para as ações.
    pub wsl_distro: Option<String>,
    pub containers: Vec<Container>,
}

/// Grupos na ordem: projetos Compose por nome, depois os containers soltos.
/// Com mais de um engine, o nome do engine entra no título.
pub(crate) fn groups(status: DockerStatus, several: bool) -> Vec<Group> {
    let mut out = Vec::new();
    for engine in status.engines {
        let prefix = several.then(|| engine.wsl_distro.clone().unwrap_or(engine.name.clone()));
        let mut compose: BTreeMap<String, Vec<Container>> = BTreeMap::new();
        let mut loose = Vec::new();
        for c in engine.containers {
            match c.compose_project.clone() {
                Some(project) => compose.entry(project).or_default().push(c),
                None => loose.push(c),
            }
        }
        let title = |name: Option<&str>| match (&prefix, name) {
            (Some(p), Some(n)) => Some(format!("{p} · {n}")),
            (Some(p), None) => Some(p.clone()),
            (None, Some(n)) => Some(n.to_owned()),
            (None, None) => None,
        };
        let first = out.len();
        for (project, mut containers) in compose {
            containers.sort_by(|a, b| a.name.cmp(&b.name));
            out.push(Group {
                title: title(Some(&project)),
                wsl_distro: engine.wsl_distro.clone(),
                containers,
            });
        }
        if !loose.is_empty() {
            loose.sort_by(|a, b| a.name.cmp(&b.name));
            let name = (out.len() > first || prefix.is_some()).then_some("Avulsos");
            out.push(Group {
                title: title(name),
                wsl_distro: engine.wsl_distro.clone(),
                containers: loose,
            });
        }
    }
    out
}

#[component]
fn Row(
    container: Container,
    wsl_distro: Option<String>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let running = container.state == "running";
    let usage = match (container.cpu, container.memory) {
        (Some(cpu), Some(mem)) => format!("{}% · {}", decimal(f64::from(cpu), 1), bytes(mem)),
        _ => container.status.clone(),
    };
    let title = format!("{} · {}", container.image, container.status);
    view! {
        <li class="group flex items-center gap-2 text-sm" title=title>
            <Dot on=running />
            <span class="min-w-0 truncate">{container.name.clone()}</span>
            <span class="ml-auto shrink-0 text-xs text-subtle tabular-nums">{usage}</span>
            // Ações aparecem no hover ou com foco (teclado).
            <span class="flex shrink-0 opacity-0 group-hover:opacity-100 focus-within:opacity-100">
                <ContainerActions container=container wsl_distro=wsl_distro error=error />
            </span>
        </li>
    }
}

/// Iniciar, ou parar e reiniciar, conforme o estado. Parar pede um segundo clique.
#[component]
pub(crate) fn ContainerActions(
    container: Container,
    wsl_distro: Option<String>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let params = serde_json::to_value(ContainerRef {
        container: container.id.clone(),
        wsl_distro,
    })
    .expect("ContainerRef sempre serializa");
    let name = container.name;
    // `paused` também para; só `docker start` não serve para ele.
    let up = matches!(
        container.state.as_str(),
        "running" | "paused" | "restarting"
    );
    if up {
        view! {
            <ActionButton
                icon=Icon::Refresh
                label=format!("Reiniciar {name}")
                method=METHOD_DOCKER_RESTART
                params=params.clone()
                error=error
            />
            <ActionButton
                icon=Icon::Square
                label=format!("Parar {name}")
                method=METHOD_DOCKER_STOP
                params=params
                confirm=true
                error=error
            />
        }
        .into_any()
    } else {
        view! {
            <ActionButton
                icon=Icon::Play
                label=format!("Iniciar {name}")
                method=METHOD_DOCKER_START
                params=params
                error=error
            />
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use codatos_protocol::system::DockerEngine;

    use super::*;

    fn c(name: &str, project: Option<&str>) -> Container {
        Container {
            id: name.into(),
            name: name.into(),
            image: "img".into(),
            state: "running".into(),
            status: "Up".into(),
            ports: String::new(),
            compose_project: project.map(Into::into),
            cpu: None,
            memory: None,
        }
    }

    #[test]
    fn agrupa_por_compose_e_deixa_avulsos_no_fim() {
        let status = DockerStatus {
            engines: vec![DockerEngine {
                id: "e".into(),
                name: "docker-desktop".into(),
                version: "27".into(),
                wsl_distro: None,
                containers: vec![
                    c("solto", None),
                    c("db", Some("app")),
                    c("api", Some("app")),
                ],
            }],
        };
        let g = groups(status, false);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].title.as_deref(), Some("app"));
        assert_eq!(g[0].containers[0].name, "api");
        assert_eq!(g[1].title.as_deref(), Some("Avulsos"));
    }

    #[test]
    fn so_avulsos_sem_titulo() {
        let status = DockerStatus {
            engines: vec![DockerEngine {
                id: "e".into(),
                name: "x".into(),
                version: "27".into(),
                wsl_distro: None,
                containers: vec![c("a", None)],
            }],
        };
        assert_eq!(groups(status, false)[0].title, None);
    }
}
