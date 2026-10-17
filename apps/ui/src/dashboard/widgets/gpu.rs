//! GPU: uso, memória, temperatura e potência das GPUs NVIDIA (`system.gpu`, via NVML).

use codatos_protocol::system::{Gpu, TOPIC_GPU};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, percent};
use super::parts::Meter;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_GPU, None);
    sample_view(sample, |gpus: Vec<Gpu>| {
        view! {
            <div class="flex h-full flex-col gap-3 overflow-auto">
                {gpus.into_iter().map(|g| view! { <GpuRow gpu=g /> }).collect_view()}
            </div>
        }
    })
}

#[component]
fn GpuRow(gpu: Gpu) -> impl IntoView {
    let extras: Vec<String> = [
        gpu.temperature_c.map(|t| format!("{t:.0} °C")),
        gpu.power_w.map(|w| format!("{w:.0} W")),
    ]
    .into_iter()
    .flatten()
    .collect();
    view! {
        <div class="space-y-2">
            <p class="truncate text-xs text-muted" title=gpu.name.clone()>
                {gpu.name.clone()}
                {(!extras.is_empty()).then(|| view! { <span class="text-subtle">{format!(" · {}", extras.join(" · "))}</span> })}
            </p>
            <div class="grid grid-cols-2 gap-4">
                <Meter label="Uso" value=f64::from(gpu.usage) detail=String::new() />
                <Meter
                    label="Memória"
                    value=percent(gpu.memory_used, gpu.memory_total)
                    detail=format!("{} de {}", bytes(gpu.memory_used), bytes(gpu.memory_total))
                />
            </div>
        </div>
    }
}
