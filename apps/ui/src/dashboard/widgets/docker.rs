//! Docker: containers de cada engine, agrupados por projeto Compose (`docker.containers`).

use std::collections::BTreeMap;

use codatos_protocol::system::{Container, DockerStatus, TOPIC_DOCKER};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, decimal};
use super::parts::Dot;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_DOCKER, None);
    sample_view(sample, |status: DockerStatus| {
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
                        .map(|(title, containers)| view! {
                            <section>
                                {title.map(|t| view! { <h4 class="mb-1 text-[11px] tracking-wide text-subtle uppercase">{t}</h4> })}
                                <ul class="space-y-1">
                                    {containers.into_iter().map(|c| view! { <Row container=c /> }).collect_view()}
                                </ul>
                            </section>
                        })
                        .collect_view()}
                </div>
            </div>
        }
    })
}

/// Grupos na ordem: projetos Compose por nome, depois os containers soltos.
/// Com mais de um engine, o nome do engine entra no título.
fn groups(status: DockerStatus, several: bool) -> Vec<(Option<String>, Vec<Container>)> {
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
        for (project, mut containers) in compose {
            containers.sort_by(|a, b| a.name.cmp(&b.name));
            out.push((title(Some(&project)), containers));
        }
        if !loose.is_empty() {
            loose.sort_by(|a, b| a.name.cmp(&b.name));
            let name = (!out.is_empty() || prefix.is_some()).then_some("Avulsos");
            out.push((title(name), loose));
        }
    }
    out
}

#[component]
fn Row(container: Container) -> impl IntoView {
    let running = container.state == "running";
    let usage = match (container.cpu, container.memory) {
        (Some(cpu), Some(mem)) => format!("{}% · {}", decimal(f64::from(cpu), 1), bytes(mem)),
        _ => container.status.clone(),
    };
    view! {
        <li class="flex items-center gap-2 text-sm" title=format!("{} · {}", container.image, container.status)>
            <Dot on=running />
            <span class="min-w-0 truncate">{container.name.clone()}</span>
            <span class="ml-auto shrink-0 text-xs text-subtle tabular-nums">{usage}</span>
        </li>
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
        assert_eq!(g[0].0.as_deref(), Some("app"));
        assert_eq!(g[0].1[0].name, "api");
        assert_eq!(g[1].0.as_deref(), Some("Avulsos"));
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
        assert_eq!(groups(status, false)[0].0, None);
    }
}
