//! Uso do PC: CPU e memória agora (`system.metrics`).

use codatos_protocol::system::{SystemMetrics, TOPIC_METRICS};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, decimal, percent};
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_METRICS, None);
    sample_view(sample, |m: SystemMetrics| {
        #[allow(clippy::cast_precision_loss)] // MHz cabem folgados num f64
        let ghz = m.cpu.frequency_mhz as f64 / 1000.0;
        let cpu_detail = if m.cpu.frequency_mhz == 0 {
            format!("{} núcleos", m.cpu.cores.len())
        } else {
            format!("{} núcleos · {} GHz", m.cpu.cores.len(), decimal(ghz, 1))
        };
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

/// Percentual grande com barra. A cor acompanha o texto, nunca sozinha.
#[component]
fn Meter(label: &'static str, value: f64, detail: String) -> impl IntoView {
    let value = value.clamp(0.0, 100.0);
    let bar = if value >= 90.0 {
        "bg-danger"
    } else if value >= 75.0 {
        "bg-warning"
    } else {
        "bg-primary"
    };
    view! {
        <div class="flex min-w-0 flex-col justify-center gap-1.5">
            <p class="text-xs text-muted">{label}</p>
            <p class="text-2xl leading-none font-semibold tabular-nums">{format!("{value:.0}%")}</p>
            <div
                class="h-1.5 overflow-hidden rounded-full bg-card-hover"
                role="meter"
                aria-label=label
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow=format!("{value:.0}")
            >
                <div class=format!("h-full rounded-full {bar}") style=format!("width: {value:.1}%")></div>
            </div>
            <p class="truncate text-xs text-subtle">{detail}</p>
        </div>
    }
}
