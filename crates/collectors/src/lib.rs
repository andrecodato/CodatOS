//! Coletores do CodatOS: cada um implementa [`Collector`] para um tópico de `subscribe`.
//!
//! O núcleo só os consulta enquanto alguém assina (`docs/02-arquitetura.md#coletores`).

mod blocking;
mod command;
mod cpu_sensors;
mod disks;
mod docker;
mod gpu;
mod system;
mod wsl;

use std::sync::Arc;

use codatos_core::Collector;

pub use cpu_sensors::CpuSensorsCollector;
pub use disks::DisksCollector;
pub use docker::DockerCollector;
pub use gpu::GpuCollector;
pub use system::{SystemInfoCollector, SystemMetricsCollector};
#[cfg(windows)]
pub use wsl::WslCollector;

/// Coletores disponíveis nesta plataforma, na ordem em que aparecem em `session.hello`.
pub fn all() -> Vec<Arc<dyn Collector>> {
    #[allow(unused_mut)]
    let mut list: Vec<Arc<dyn Collector>> = vec![
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(SystemInfoCollector),
        Arc::new(DisksCollector),
        Arc::new(GpuCollector::default()),
        Arc::new(CpuSensorsCollector),
        Arc::new(DockerCollector),
    ];
    #[cfg(windows)]
    list.push(Arc::new(WslCollector::default()));
    list
}
