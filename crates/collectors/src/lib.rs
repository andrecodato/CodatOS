//! Coletores do CodatOS: cada um implementa [`Collector`] para um tópico de `subscribe`.
//!
//! O núcleo só os consulta enquanto alguém assina (`docs/02-arquitetura.md#coletores`).

mod blocking;
mod command;
mod cpu_sensors;
mod disks;
mod docker;
mod gpu;
mod http;
mod projects;
mod system;
mod weather;
mod wsl;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use codatos_core::{Collector, Method};
use codatos_protocol::dashboard::SettingScope;

pub use cpu_sensors::CpuSensorsCollector;
pub use disks::DisksCollector;
pub use docker::DockerCollector;
pub use gpu::GpuCollector;
pub use projects::ProjectsCollector;
pub use system::{SystemInfoCollector, SystemMetricsCollector};
pub use weather::WeatherCollector;
#[cfg(windows)]
pub use wsl::WslCollector;

/// Lê uma preferência do banco local (que fica no app, não aqui).
pub type SettingsReader = Arc<
    dyn Fn(
            &'static str,
            SettingScope,
        ) -> Pin<Box<dyn Future<Output = Option<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// Coletores disponíveis nesta plataforma. `settings` traz a configuração dos projetos e a
/// cidade do clima.
pub fn all(settings: Option<SettingsReader>) -> Vec<Arc<dyn Collector>> {
    #[allow(unused_mut)]
    let mut list: Vec<Arc<dyn Collector>> = vec![
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(SystemInfoCollector),
        Arc::new(DisksCollector),
        Arc::new(GpuCollector::default()),
        Arc::new(CpuSensorsCollector),
        Arc::new(DockerCollector),
        Arc::new(ProjectsCollector::new(settings.clone())),
        Arc::new(WeatherCollector::new(settings)),
    ];
    #[cfg(windows)]
    list.push(Arc::new(WslCollector::default()));
    list
}

/// Métodos JSON-RPC dos coletores: busca de cidade do clima, abrir projeto e, no Windows,
/// as ações do WSL.
pub fn methods() -> Vec<Arc<dyn Method>> {
    #[allow(unused_mut)]
    let mut list: Vec<Arc<dyn Method>> =
        vec![Arc::new(weather::SearchCity), Arc::new(projects::Open)];
    #[cfg(windows)]
    list.extend(
        wsl::methods()
            .into_iter()
            .map(|m| Arc::new(m) as Arc<dyn Method>),
    );
    list
}
