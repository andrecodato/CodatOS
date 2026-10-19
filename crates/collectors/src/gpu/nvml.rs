//! GPUs NVIDIA via NVML.
//!
//! A biblioteca do driver (`nvml.dll`, `libnvidia-ml.so`) é carregada em tempo de execução.

use codatos_protocol::system::Gpu;
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;

/// `None` quando a NVML não existe nesta máquina (sem driver NVIDIA).
pub(super) fn init() -> Option<Nvml> {
    Nvml::init().ok()
}

/// GPUs vistas pela NVML. Falha de leitura vira lista vazia: quem chama tenta a próxima fonte.
pub(super) fn gpus(nvml: &Nvml) -> Vec<Gpu> {
    let Ok(count) = nvml.device_count() else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let Ok(memory) = device.memory_info() else {
            continue;
        };
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
    list
}
