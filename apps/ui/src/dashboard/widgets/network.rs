//! Rede: download e upload agora, no total e por interface (`system.metrics`).

use codatos_protocol::system::{NetworkInterface, SystemMetrics, TOPIC_METRICS};
use leptos::prelude::*;
use serde_json::Value;

use super::format::bytes;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_METRICS, None);
    sample_view(sample, |mut m: SystemMetrics| {
        m.network.retain(|n| !is_loopback(&n.name));
        let rx: u64 = m.network.iter().map(|n| n.rx_bps).sum();
        let tx: u64 = m.network.iter().map(|n| n.tx_bps).sum();
        view! {
            <div class="flex h-full flex-col gap-3">
                <div class="flex gap-6">
                    <Rate label="Download" arrow="↓" value=rx />
                    <Rate label="Upload" arrow="↑" value=tx />
                </div>
                <ul class="min-h-0 flex-1 space-y-1 overflow-auto text-xs">
                    {m.network.into_iter().map(|n| view! { <Interface iface=n /> }).collect_view()}
                </ul>
            </div>
        }
    })
}

/// O tráfego da própria máquina não é rede.
fn is_loopback(name: &str) -> bool {
    name == "lo" || name.to_ascii_lowercase().starts_with("loopback")
}

/// Taxa em bytes por segundo: "1,2 MB/s".
pub fn rate(bytes_per_second: u64) -> String {
    bytes(bytes_per_second) + "/s"
}

#[component]
fn Rate(label: &'static str, arrow: &'static str, value: u64) -> impl IntoView {
    view! {
        <div>
            <p class="text-xs text-muted">{label}</p>
            <p class="text-xl leading-tight font-semibold tabular-nums">
                <span class="text-subtle" aria-hidden="true">{arrow}" "</span>
                {rate(value)}
            </p>
        </div>
    }
}

#[component]
fn Interface(iface: NetworkInterface) -> impl IntoView {
    let address = iface.addresses.first().cloned().unwrap_or_default();
    view! {
        <li class="flex items-baseline gap-2">
            <span class="truncate text-muted" title=iface.name.clone()>{iface.name.clone()}</span>
            <span class="truncate font-mono text-[11px] text-subtle">{address}</span>
            <span class="ml-auto shrink-0 tabular-nums text-subtle">
                {format!("↓ {}  ↑ {}", rate(iface.rx_bps), rate(iface.tx_bps))}
            </span>
        </li>
    }
}
