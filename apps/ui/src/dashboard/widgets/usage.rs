//! Uso do PC: CPU e memória agora (`system.metrics`), com a temperatura da CPU quando
//! há sensor (`system.cpuSensors`, LibreHardwareMonitor no Windows).

use std::fmt::Write as _;

use codatos_protocol::system::{CpuSensors, SystemMetrics, TOPIC_CPU_SENSORS, TOPIC_METRICS};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, decimal, percent};
use super::parts::Meter;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_METRICS, None);
    let sensors = api::subscribe(TOPIC_CPU_SENSORS, None);
    sample_view(sample, move |m: SystemMetrics| {
        let mut cpu_detail = format!("{} núcleos", m.cpu.cores.len());
        if m.cpu.frequency_mhz > 0 {
            #[allow(clippy::cast_precision_loss)] // MHz cabem folgados num f64
            let ghz = m.cpu.frequency_mhz as f64 / 1000.0;
            let _ = write!(cpu_detail, " · {} GHz", decimal(ghz, 1));
        }
        // Sem sensor, a temperatura só não aparece: não é erro do widget.
        let temperature = sensors
            .get()
            .and_then(Result::ok)
            .and_then(|v| serde_json::from_value::<CpuSensors>(v).ok())
            .and_then(|s| s.temperature_c);
        if let Some(t) = temperature {
            let _ = write!(cpu_detail, " · {t:.0} °C");
        }
        view! {
            <div class="grid h-full grid-cols-2 gap-4">
                <Meter label="CPU" value=f64::from(m.cpu.total) detail=cpu_detail />
                <Meter
                    label="Memória"
                    value=percent(m.memory.used, m.memory.total)
                    detail=format!("{} de {}", bytes(m.memory.used), bytes(m.memory.total))
                />
            </div>
        }
    })
}
