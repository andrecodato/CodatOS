//! Aba Docker (RF-SYS-033): containers de todos os engines com iniciar, parar e reiniciar;
//! shell (`docker exec`) e logs ao vivo abrem como abas do terminal.

use codatos_protocol::system::{Container, DockerStatus, TOPIC_DOCKER};
use codatos_protocol::terminal::TerminalTarget;
use leptos::prelude::*;

use super::Tab;
use super::ui::GHOST;
use crate::api;
use crate::components::icon::{Icon, Svg};
use crate::dashboard::state::sample_view;
use crate::dashboard::widgets::docker::{ContainerActions, groups};
use crate::dashboard::widgets::format::{bytes, decimal};
use crate::dashboard::widgets::parts::Dot;
use crate::terminal::Terminals;

/// Mais rápido que o widget: depois de uma ação o estado novo aparece logo.
const INTERVAL_MS: u64 = 2_000;

#[component]
pub fn Docker() -> impl IntoView {
    let sample = api::subscribe(TOPIC_DOCKER, Some(INTERVAL_MS));
    let error = RwSignal::new(None::<String>);
    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-3">
            {move || error.get().map(|e| view! {
                <p role="alert" class="rounded-md border border-line bg-card px-3 py-2 text-sm text-danger">{e}</p>
            })}
            <div class="min-h-0 flex-1 overflow-auto">
                {sample_view(sample, move |status: DockerStatus| {
                    let several = status.engines.len() > 1;
                    let groups = groups(status, several);
                    if groups.is_empty() {
                        return view! { <p class="text-sm text-muted">"Nenhum container neste engine."</p> }.into_any();
                    }
                    groups
                        .into_iter()
                        .map(|group| {
                            let distro = group.wsl_distro;
                            view! {
                                <section class="mb-4">
                                    {group.title.map(|t| view! { <h2 class="mb-1.5 text-xs font-medium tracking-wide text-subtle uppercase">{t}</h2> })}
                                    <ul class="divide-y divide-line rounded-lg border border-line bg-card">
                                        {group
                                            .containers
                                            .into_iter()
                                            .map(|c| view! { <Row container=c wsl_distro=distro.clone() error=error /> })
                                            .collect_view()}
                                    </ul>
                                </section>
                            }
                        })
                        .collect_view()
                        .into_any()
                })}
            </div>
        </div>
    }
}

#[component]
fn Row(
    container: Container,
    wsl_distro: Option<String>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let running = container.state == "running";
    let usage = match (container.cpu, container.memory) {
        (Some(cpu), Some(mem)) => Some(format!(
            "CPU {}% · {}",
            decimal(f64::from(cpu), 1),
            bytes(mem)
        )),
        _ => None,
    };
    // `docker exec` e `docker logs` daqui só alcançam o engine do host.
    let local = wsl_distro.is_none();
    let name = container.name.clone();
    view! {
        <li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-sm">
            <Dot on=running />
            <div class="min-w-0 flex-1">
                <p class="truncate font-medium">{container.name.clone()}</p>
                <p class="truncate text-xs text-subtle">{container.image.clone()}</p>
            </div>
            <div class="min-w-0 text-right text-xs text-muted tabular-nums">
                <p class="truncate">{container.status.clone()}</p>
                {usage.map(|u| view! { <p class="truncate text-subtle">{u}</p> })}
            </div>
            {(!container.ports.is_empty()).then(|| view! {
                <p class="hidden max-w-56 truncate text-xs text-subtle md:block" title=container.ports.clone()>{container.ports.clone()}</p>
            })}
            <div class="flex shrink-0 items-center">
                {(local && running).then(|| view! {
                    <OpenButton icon=Icon::Terminal label=format!("Abrir terminal em {name}") target=TerminalTarget::Docker { container: name.clone() } title=name.clone() />
                })}
                {local.then(|| view! {
                    <OpenButton icon=Icon::File label=format!("Logs de {name}") target=TerminalTarget::DockerLogs { container: name.clone() } title=format!("logs · {name}") />
                })}
                <ContainerActions container=container wsl_distro=wsl_distro error=error />
            </div>
        </li>
    }
}

/// Abre o alvo numa aba do terminal e vai para ela.
#[component]
fn OpenButton(icon: Icon, label: String, target: TerminalTarget, title: String) -> impl IntoView {
    let tab = use_context::<RwSignal<Tab>>();
    let target = StoredValue::new(target);
    let title = StoredValue::new(title);
    view! {
        <button
            type="button"
            class=GHOST
            title=label.clone()
            aria-label=label
            on:click=move |_| {
                Terminals::current().open(target.get_value(), title.get_value());
                if let Some(tab) = tab {
                    tab.set(Tab::Terminal);
                }
            }
        >
            <Svg icon=icon size=14 />
        </button>
    }
}
