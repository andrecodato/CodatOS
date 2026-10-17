//! GPUs NVIDIA via NVML (RF-SYS-003).
//!
//! A biblioteca do driver (`nvml.dll`, `libnvidia-ml.so`) é carregada em tempo de execução:
//! sem GPU NVIDIA, o tópico responde `unavailable` e o widget mostra "indisponível".

use std::sync::{Arc, Mutex, PoisonError};

use codatos_core::{CollectError, CollectFuture, Collector};
use codatos_protocol::system::{Gpu, TOPIC_GPU};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

use crate::blocking;

/// `system.gpu`. A NVML é iniciada na primeira coleta e reaproveitada.
#[derive(Debug, Default)]
pub struct GpuCollector {
    nvml: Arc<Mutex<Option<Nvml>>>,
}

impl Collector for GpuCollector {
    fn topic(&self) -> &'static str {
        TOPIC_GPU
    }

    fn collect(&self) -> CollectFuture<'_> {
        let nvml = Arc::clone(&self.nvml);
        Box::pin(blocking::run(move || {
            let mut guard = nvml.lock().unwrap_or_else(PoisonError::into_inner);
            if guard.is_none() {
                *guard = Some(
                    Nvml::init()
                        .map_err(|_| CollectError::unavailable("Nenhuma GPU NVIDIA encontrada"))?,
                );
            }
            let nvml = guard.as_ref().expect("acabou de ser iniciada");
            gpus(nvml)
        }))
    }
}

fn gpus(nvml: &Nvml) -> Result<Vec<Gpu>, CollectError> {
    let failed = |_| CollectError::new("nvml_failed", "Não consegui ler a GPU");
    let count = nvml.device_count().map_err(failed)?;
    let mut list = Vec::new();
    for index in 0..count {
        let device = nvml.device_by_index(index).map_err(failed)?;
        let memory = device.memory_info().map_err(failed)?;
        list.push(Gpu {
            name: device.name().unwrap_or_else(|_| format!("GPU {index}")),
            #[allow(clippy::cast_precision_loss)]
            usage: device.utilization_rates().map_or(0.0, |u| u.gpu as f32),
            memory_used: memory.used,
            memory_total: memory.total,
            #[allow(clippy::cast_precision_loss)]
            temperature_c: device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32),
            // NVML devolve miliwatts.
            #[allow(clippy::cast_precision_loss)]
            power_w: device.power_usage().ok().map(|mw| mw as f32 / 1000.0),
        });
    }
    Ok(list)
}
