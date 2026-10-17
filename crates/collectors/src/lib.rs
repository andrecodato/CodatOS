//! Coletores do CodatOS: cada um implementa [`Collector`] para um tópico de `subscribe`.
//!
//! O núcleo só os consulta enquanto alguém assina (`docs/02-arquitetura.md#coletores`).

mod blocking;
mod command;
mod cpu_sensors;
mod disks;
mod docker;
mod gpu;
mod projects;
mod system;
mod wsl;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use codatos_core::Collector;

pub use cpu_sensors::CpuSensorsCollector;
pub use disks::DisksCollector;
pub use docker::DockerCollector;
pub use gpu::GpuCollector;
pub use projects::ProjectsCollector;
pub use system::{SystemInfoCollector, SystemMetricsCollector};
#[cfg(windows)]
pub use wsl::WslCollector;

/// Lê uma preferência de escopo `device` (o banco local fica no app, não aqui).
pub type DeviceSettings = Arc<
    dyn Fn(&'static str) -> Pin<Box<dyn Future<Output = Option<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// Coletores disponíveis nesta plataforma. `settings` traz a configuração dos projetos.
pub fn all(settings: Option<DeviceSettings>) -> Vec<Arc<dyn Collector>> {
    #[allow(unused_mut)]
    let mut list: Vec<Arc<dyn Collector>> = vec![
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(SystemInfoCollector),
        Arc::new(DisksCollector),
        Arc::new(GpuCollector::default()),
        Arc::new(CpuSensorsCollector),
        Arc::new(DockerCollector),
        Arc::new(ProjectsCollector::new(settings)),
    ];
    #[cfg(windows)]
    list.push(Arc::new(WslCollector::default()));
    list
}
