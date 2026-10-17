//! Coletores do CodatOS: cada um implementa [`Collector`] para um tópico de `subscribe`.
//!
//! O núcleo só os consulta enquanto alguém assina (`docs/02-arquitetura.md#coletores`).

mod blocking;
mod disks;
mod system;

use std::sync::Arc;

use codatos_core::Collector;

pub use disks::DisksCollector;
pub use system::{SystemInfoCollector, SystemMetricsCollector};

/// Coletores disponíveis nesta plataforma, na ordem em que aparecem em `session.hello`.
pub fn all() -> Vec<Arc<dyn Collector>> {
    vec![
        Arc::new(SystemMetricsCollector::default()),
        Arc::new(SystemInfoCollector),
        Arc::new(DisksCollector),
    ]
}
