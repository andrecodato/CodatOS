//! WSL: distros, estado e disco; memória e CPU da VM do WSL 2 (`wsl.distros`, só Windows).

use codatos_protocol::system::{
    METHOD_WSL_SET_DEFAULT, METHOD_WSL_SHUTDOWN, METHOD_WSL_START, METHOD_WSL_TERMINATE, TOPIC_WSL,
    WslDistro, WslStatus,
};
use leptos::prelude::*;
use serde_json::{Value, json};

use super::format::{bytes, decimal};
use super::parts::{ActionButton, Dot};
use crate::api;
use crate::components::icon::Icon;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_WSL, None);
    let error = RwSignal::new(None::<String>);
    sample_view(sample, move |status: WslStatus| {
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
                <div class="flex items-center gap-2 text-xs text-muted">
                    <p class="min-w-0 truncate">
                        {format!("{running} de {} ligadas", status.distros.len())}
                        {vm.map(|v| view! { <span class="text-subtle">{format!(" · {v}")}</span> })}
                    </p>
                    {(running > 0).then(|| view! {
                        <span class="ml-auto">
                            <ActionButton
                                icon=Icon::Power
                                label="Desligar o WSL inteiro".to_owned()
                                method=METHOD_WSL_SHUTDOWN
                                params=json!({})
                                confirm=true
                                error=error
                            />
                        </span>
                    })}
                </div>
                <ul class="min-h-0 flex-1 space-y-1 overflow-auto pr-1">
                    {status.distros.into_iter().map(|d| view! { <Distro distro=d error=error /> }).collect_view()}
                </ul>
                {move || error.get().map(|e| view! { <p role="alert" class="truncate text-xs text-danger" title=e.clone()>{e.clone()}</p> })}
            </div>
        }
    })
}

#[component]
fn Distro(distro: WslDistro, error: RwSignal<Option<String>>) -> impl IntoView {
    let params = json!({ "distro": distro.name });
    let name = distro.name.clone();
    let disk = match (distro.disk_used, distro.disk_total) {
        (Some(used), Some(total)) => Some(format!("{} de {}", bytes(used), bytes(total))),
        _ => distro
            .vhdx_bytes
            .map(|v| format!("disco virtual de {}", bytes(v))),
    };
    view! {
        <li class="group flex items-center gap-2 text-sm">
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
            // Ações aparecem no hover ou com foco (teclado).
            <span class="flex shrink-0 opacity-0 group-hover:opacity-100 focus-within:opacity-100">
                {if distro.running {
                    view! {
                        <ActionButton
                            icon=Icon::Power
                            label=format!("Desligar {name}")
                            method=METHOD_WSL_TERMINATE
                            params=params.clone()
                            confirm=true
                            error=error
                        />
                    }
                    .into_any()
                } else {
                    view! {
                        <ActionButton
                            icon=Icon::Play
                            label=format!("Ligar {name}")
                            method=METHOD_WSL_START
                            params=params.clone()
                            error=error
                        />
                    }
                    .into_any()
                }}
                {(!distro.default).then(|| view! {
                    <ActionButton
                        icon=Icon::Star
                        label=format!("Tornar {name} a distro padrão")
                        method=METHOD_WSL_SET_DEFAULT
                        params=params.clone()
                        error=error
                    />
                })}
            </span>
        </li>
    }
}
