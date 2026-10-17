//! Discos: espaço usado e livre em cada volume (`disks`).

use codatos_protocol::system::{Disk, DiskKind, TOPIC_DISKS};
use leptos::prelude::*;
use serde_json::Value;

use super::format::{bytes, percent};
use super::parts::Bar;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_DISKS, None);
    sample_view(sample, |mut disks: Vec<Disk>| {
        disks.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
        if disks.is_empty() {
            return view! { <p class="text-sm text-muted">"Nenhum volume encontrado."</p> }
                .into_any();
        }
        view! {
            <ul class="h-full space-y-3 overflow-auto pr-1">
                {disks.into_iter().map(|d| view! { <DiskRow disk=d /> }).collect_view()}
            </ul>
        }
        .into_any()
    })
}

#[component]
fn DiskRow(disk: Disk) -> impl IntoView {
    let used = disk.total.saturating_sub(disk.available);
    let value = percent(used, disk.total);
    let kind = match disk.kind {
        DiskKind::Ssd => "SSD",
        DiskKind::Hdd => "HDD",
        DiskKind::Unknown => "",
    };
    let label = if disk.name.is_empty() || disk.name == disk.mount_point {
        disk.mount_point.clone()
    } else {
        format!("{} ({})", disk.mount_point, disk.name)
    };
    let tags: Vec<&str> = [
        kind,
        &disk.file_system,
        if disk.removable { "removível" } else { "" },
    ]
    .into_iter()
    .filter(|t| !t.is_empty())
    .collect();
    view! {
        <li class="space-y-1">
            <div class="flex items-baseline gap-2 text-sm">
                <span class="min-w-0 truncate font-medium">{label.clone()}</span>
                <span class="shrink-0 text-[11px] whitespace-nowrap text-subtle">{tags.join(" · ")}</span>
                <span class="ml-auto text-xs tabular-nums text-muted">{format!("{value:.0}%")}</span>
            </div>
            <Bar value=value label=format!("Uso de {label}") />
            <p class="text-xs text-subtle">
                {format!("{} livres de {}", bytes(disk.available), bytes(disk.total))}
            </p>
        </li>
    }
}
