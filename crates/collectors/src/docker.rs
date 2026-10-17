//! Containers Docker (RF-SYS-030 a 032) pela CLI `docker`.
//!
//! A CLI funciona igual no host e dentro das distros WSL (`wsl -d X -e docker …`), por isso
//! foi escolhida em vez da API por socket/named pipe. No Windows, cada distro **ligada** com
//! `docker` instalado é consultada; o Docker Desktop com integração WSL responde com o mesmo
//! ID de engine no host e na distro, e aparece uma vez só.

mod parse;

use std::collections::HashSet;
use std::time::Duration;

use codatos_core::{CollectError, CollectFuture, Collector, Intervals};
use codatos_protocol::system::{DockerEngine, DockerStatus, TOPIC_DOCKER};

use crate::command;

const INFO: &[&str] = &["info", "--format", "{{json .}}"];
const PS: &[&str] = &["ps", "--all", "--no-trunc", "--format", "{{json .}}"];
const STATS: &[&str] = &["stats", "--no-stream", "--format", "{{json .}}"];
/// `docker stats` espera uma amostra de CPU; leva 1–2 s.
const STATS_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Default)]
pub struct DockerCollector;

impl Collector for DockerCollector {
    fn topic(&self) -> &'static str {
        TOPIC_DOCKER
    }

    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_secs(5),
            min: Duration::from_secs(2),
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async {
            let mut engines = Vec::new();
            let mut seen = HashSet::new();
            if let Some(engine) = engine(None).await {
                seen.insert(engine.id.clone());
                engines.push(engine);
            }
            #[cfg(windows)]
            for distro in crate::wsl::running().await {
                // A distro interna do Docker Desktop não tem a CLI; o engine dela já veio do host.
                if distro.starts_with("docker-desktop") {
                    continue;
                }
                if let Some(engine) = engine(Some(distro)).await
                    && seen.insert(engine.id.clone())
                {
                    engines.push(engine);
                }
            }
            if engines.is_empty() {
                return Err(CollectError::new(
                    "docker_not_running",
                    "Docker não está em execução",
                ));
            }
            Ok(serde_json::to_value(DockerStatus { engines })
                .expect("DockerStatus sempre serializa"))
        })
    }
}

/// Um engine e seus containers; `None` se não houver `docker` ou o engine estiver parado.
async fn engine(wsl_distro: Option<String>) -> Option<DockerEngine> {
    let info = parse::info(&docker(wsl_distro.as_deref(), INFO, command::TIMEOUT).await?)?;
    let mut containers = parse::containers(
        &docker(wsl_distro.as_deref(), PS, command::TIMEOUT)
            .await
            .unwrap_or_default(),
    );
    if containers.iter().any(|c| c.state == "running")
        && let Some(stats) = docker(wsl_distro.as_deref(), STATS, STATS_TIMEOUT).await
    {
        parse::apply_stats(&mut containers, &stats);
    }
    Some(DockerEngine {
        id: info.id,
        name: info.name,
        version: info.version,
        wsl_distro,
        containers,
    })
}

/// `docker …` no host ou `wsl -d <distro> -e docker …`. Só é chamado para distros que
/// acabaram de aparecer como ligadas (RF-SYS-024).
async fn docker(wsl_distro: Option<&str>, args: &[&str], timeout: Duration) -> Option<String> {
    match wsl_distro {
        None => command::output("docker", args, timeout).await,
        Some(distro) => {
            let mut full = vec!["-d", distro, "-e", "docker"];
            full.extend_from_slice(args);
            command::output("wsl.exe", &full, timeout).await
        }
    }
}
