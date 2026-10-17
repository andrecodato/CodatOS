//! WSL: distros, estado e disco; memória e CPU da VM do WSL 2 (`wsl.distros`, só Windows).

use codatos_protocol::system::{TOPIC_WSL, WslDistro, WslStatus};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, decimal};
use super::parts::Dot;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_WSL, None);
    sample_view(sample, |status: WslStatus| {
        let running = status.distros.iter().filter(|d| d.running).count();
        let vm = status.vm.map(|vm| {
            format!(
                "VM: {} · CPU {}%",
                bytes(vm.memory),
                decimal(f64::from(vm.cpu), 0)
            )
        });
        view! {
            <div class="flex h-full flex-col gap-2">
                <p class="text-xs text-muted">
                    {format!("{running} de {} ligadas", status.distros.len())}
                    {vm.map(|v| view! { <span class="text-subtle">{format!(" · {v}")}</span> })}
                </p>
                <ul class="min-h-0 flex-1 space-y-2 overflow-auto pr-1">
                    {status.distros.into_iter().map(|d| view! { <Distro distro=d /> }).collect_view()}
                </ul>
            </div>
        }
    })
}

#[component]
fn Distro(distro: WslDistro) -> impl IntoView {
    let disk = match (distro.disk_used, distro.disk_total) {
        (Some(used), Some(total)) => Some(format!("{} de {}", bytes(used), bytes(total))),
        _ => distro
            .vhdx_bytes
            .map(|v| format!("disco virtual de {}", bytes(v))),
    };
    view! {
        <li class="flex items-center gap-2 text-sm">
            <Dot on=distro.running />
            <span class="min-w-0 truncate font-medium">{distro.name.clone()}</span>
            {distro.default.then(|| view! {
                <span class="shrink-0 rounded bg-primary-soft px-1.5 text-[11px] text-primary">"padrão"</span>
            })}
            <span class="shrink-0 text-[11px] text-subtle">{format!("WSL {}", distro.version)}</span>
            <span class="ml-auto shrink-0 text-xs text-subtle tabular-nums">
                {if distro.running { "ligada" } else { "parada" }}
                {disk.map(|d| format!(" · {d}"))}
            </span>
        </li>
    }
}
