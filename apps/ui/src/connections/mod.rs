//! Conexões (`docs/modulos/conexoes.md`): hosts e pastas, identidades e chaves SSH,
//! importação do `~/.ssh/config`, túneis e containers Docker. Senhas e chaves privadas vão para o núcleo e nunca
//! voltam: a UI só vê `hasPassword` e a chave pública.

mod docker;
mod hosts;
mod import;
mod keychain;
mod tunnels;
pub(crate) mod ui;

use codatos_protocol::connections::{Connections as List, METHOD_CONNECTIONS_LIST};
use leptos::{prelude::*, task::spawn_local};
use serde_json::json;

use crate::api::{self, CoreStatus};
use crate::components::icon::{Icon, Svg};
use crate::sftp::{Phase as SftpPhase, Sftp, SftpPanel};
use crate::terminal::{TerminalPanel, Terminals};

/// Lista do núcleo, compartilhada pelas abas. `None` enquanto carrega.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Data {
    list: RwSignal<Option<Result<List, String>>>,
}

impl Data {
    /// Busca de novo a lista (depois de salvar ou apagar algo).
    pub(crate) fn reload(self) {
        spawn_local(async move {
            let result = api::call(METHOD_CONNECTIONS_LIST, json!({}))
                .await
                .map_err(|e| e.message)
                .and_then(|v| serde_json::from_value::<List>(v).map_err(|e| e.to_string()));
            self.list.set(Some(result));
        });
    }

    /// A lista atual, ou vazia se não carregou.
    pub(crate) fn get(self) -> List {
        self.list
            .with(|l| l.as_ref().and_then(|r| r.as_ref().ok()).cloned())
            .unwrap_or_default()
    }

    pub(crate) fn get_untracked(self) -> List {
        self.list
            .with_untracked(|l| l.as_ref().and_then(|r| r.as_ref().ok()).cloned())
            .unwrap_or_default()
    }
}

/// Estado da carga, separado da lista: salvar algo recarrega a lista sem desmontar a tela.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Load {
    Loading,
    Failed(String),
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Hosts,
    Keychain,
    Terminal,
    Files,
    Tunnels,
    Docker,
}

#[component]
pub fn Connections(status: ReadSignal<CoreStatus>) -> impl IntoView {
    let data = Data {
        list: RwSignal::new(None),
    };
    provide_context(data);
    // Carrega quando o núcleo conecta (e de novo se ele voltar de uma queda).
    Effect::new(move |_| {
        if status.get() == CoreStatus::Connected {
            data.reload();
        }
    });
    // Volta direto para os terminais ou para o SFTP se houver algo aberto.
    let tab = RwSignal::new(
        if !Terminals::current().tabs.with_untracked(Vec::is_empty) {
            Tab::Terminal
        } else if Sftp::current()
            .phase
            .with_untracked(|p| *p != SftpPhase::Idle)
        {
            Tab::Files
        } else {
            Tab::Hosts
        },
    );
    provide_context(tab);
    let load = Memo::new(move |_| {
        data.list.with(|l| match l {
            None => Load::Loading,
            Some(Err(message)) => Load::Failed(message.clone()),
            Some(Ok(_)) => Load::Ready,
        })
    });

    let tab_button = move |value: Tab, icon: Icon, label: &'static str| {
        view! {
            <button
                type="button"
                role="tab"
                aria-selected=move || (tab.get() == value).to_string()
                class="flex items-center gap-2 border-b-2 px-1 pb-2 text-sm transition-colors duration-fast"
                class=(["border-primary", "text-fg", "font-medium"], move || tab.get() == value)
                class=(["border-transparent", "text-muted", "hover:text-fg"], move || tab.get() != value)
                on:click=move |_| tab.set(value)
            >
                <Svg icon=icon size=16 />
                {label}
            </button>
        }
    };

    view! {
        <section class="flex h-full min-h-0 flex-col gap-4">
            <header class="flex items-end gap-6 border-b border-line">
                <h1 class="pb-2 text-lg font-semibold">"Conexões"</h1>
                <nav role="tablist" aria-label="Seções de conexões" class="flex gap-5">
                    {tab_button(Tab::Hosts, Icon::Server, "Hosts")}
                    {tab_button(Tab::Keychain, Icon::Key, "Chaveiro")}
                    {tab_button(Tab::Terminal, Icon::Terminal, "Terminal")}
                    {tab_button(Tab::Files, Icon::Folder, "SFTP")}
                    {tab_button(Tab::Tunnels, Icon::ArrowDownUp, "Túneis")}
                    {tab_button(Tab::Docker, Icon::Container, "Docker")}
                </nav>
            </header>
            {move || match load.get() {
                Load::Loading => view! { <p class="text-sm text-muted">"Carregando conexões…"</p> }.into_any(),
                Load::Failed(message) => view! {
                    <div class="rounded-lg border border-line bg-card p-4 text-sm">
                        <p class="text-danger">{format!("Não consegui carregar as conexões: {message}")}</p>
                        <button
                            type="button"
                            class=format!("mt-3 {}", ui::BUTTON)
                            on:click=move |_| data.reload()
                        >
                            "Tentar de novo"
                        </button>
                    </div>
                }
                .into_any(),
                Load::Ready => match tab.get() {
                    Tab::Hosts => view! { <hosts::Hosts /> }.into_any(),
                    Tab::Keychain => view! { <keychain::Keychain /> }.into_any(),
                    Tab::Terminal => view! { <TerminalPanel /> }.into_any(),
                    Tab::Files => view! { <SftpPanel /> }.into_any(),
                    Tab::Tunnels => view! { <tunnels::Tunnels /> }.into_any(),
                    Tab::Docker => view! { <docker::Docker /> }.into_any(),
                },
            }}
        </section>
    }
}
