//! GPUs (RF-SYS-003): uso, memória, temperatura e potência.
//!
//! NVIDIA vem da NVML, carregada em tempo de execução. Sem ela, AMD e Intel vêm do
//! LibreHardwareMonitor no Windows (o mesmo da temperatura da CPU, Q-13) e do driver
//! `amdgpu` no Linux. Sem nenhuma fonte, o tópico responde `unavailable`.

use std::sync::{Arc, Mutex, PoisonError};

use codatos_core::{CollectError, CollectFuture, Collector};
use codatos_protocol::system::{Gpu, TOPIC_GPU};
use nvml_wrapper::Nvml;

#[cfg(any(not(windows), test))]
mod amdgpu;
#[cfg(any(windows, test))]
mod lhm;
mod nvml;

/// Estado da NVML entre coletas: tentada uma vez só, para não carregar a DLL a cada amostra
/// num PC sem NVIDIA.
#[derive(Debug, Default)]
enum NvmlState {
    #[default]
    Untried,
    Ready(Box<Nvml>),
    Missing,
}

/// `system.gpu`.
#[derive(Debug, Default)]
pub struct GpuCollector {
    nvml: Arc<Mutex<NvmlState>>,
}

impl Collector for GpuCollector {
    fn topic(&self) -> &'static str {
        TOPIC_GPU
    }

    fn collect(&self) -> CollectFuture<'_> {
        let nvml = Arc::clone(&self.nvml);
        Box::pin(async move {
            let nvidia = tokio::task::spawn_blocking(move || nvidia(&nvml))
                .await
                .map_err(|_| CollectError::new("collector_panicked", "O coletor falhou"))?;
            let gpus = if nvidia.is_empty() {
                others().await?
            } else {
                nvidia
            };
            serde_json::to_value(gpus)
                .map_err(|_| CollectError::new("collector_invalid", "Amostra inválida"))
        })
    }
}

fn nvidia(state: &Mutex<NvmlState>) -> Vec<Gpu> {
    let mut state = state.lock().unwrap_or_else(PoisonError::into_inner);
    if matches!(*state, NvmlState::Untried) {
        *state = nvml::init().map_or(NvmlState::Missing, |n| NvmlState::Ready(Box::new(n)));
    }
    match &*state {
        NvmlState::Ready(nvml) => nvml::gpus(nvml),
        _ => Vec::new(),
    }
}

/// GPUs que não são NVIDIA (ou NVIDIA sem NVML).
#[cfg(windows)]
async fn others() -> Result<Vec<Gpu>, CollectError> {
    lhm::read(crate::lhm::DEFAULT_ADDR).await
}

#[cfg(not(windows))]
async fn others() -> Result<Vec<Gpu>, CollectError> {
    let gpus = tokio::task::spawn_blocking(amdgpu::gpus)
        .await
        .map_err(|_| CollectError::new("collector_panicked", "O coletor falhou"))?;
    if gpus.is_empty() {
        return Err(CollectError::unavailable(
            "Nenhuma GPU NVIDIA ou AMD encontrada",
        ));
    }
    Ok(gpus)
}
