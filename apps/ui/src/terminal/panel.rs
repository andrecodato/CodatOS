//! Painel dos terminais: abas, o xterm da aba ativa e, por cima dele, o que a sessão
//! precisar perguntar (fingerprint, senha, usuário).

use std::cell::RefCell;

use codatos_protocol::system::{
    DockerStatus, SystemInfo, TOPIC_DOCKER, TOPIC_INFO, TOPIC_WSL, WslStatus,
};
use codatos_protocol::terminal::{LocalShell, TerminalTarget};
use leptos::{ev, html, prelude::*};
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;
use web_sys::{Element, ResizeObserver};

use super::{Answer, Ask, Phase, Tab, Terminals, attach, fit};
use crate::api;
use crate::components::icon::{Icon, Svg};
use crate::connections::Data;
use crate::connections::ui::{BUTTON, FIELD, GHOST, LABEL, PRIMARY};

type Observer = (ResizeObserver, Closure<dyn FnMut()>);

thread_local! {
    /// Observa o tamanho do painel para reajustar o xterm (uma instância do painel por vez).
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}

#[component]
pub fn TerminalPanel() -> impl IntoView {
    let terminals = Terminals::current();
    let panel = NodeRef::<html::Div>::new();
    let ids = Memo::new(move |_| {
        terminals
            .tabs
            .with(|tabs| tabs.iter().map(|t| t.id).collect::<Vec<_>>())
    });
    Effect::new(move |_| {
        ids.track();
        let active = terminals.active.get();
        if let Some(div) = panel.get() {
            attach(active, Some(&div));
            observe(&div);
        }
    });
    on_cleanup(|| {
        OBSERVER.with(|o| {
            if let Some((observer, _)) = o.borrow_mut().take() {
                observer.disconnect();
            }
        });
        attach(None, None);
    });

    let active_tab = Memo::new(move |_| {
        let active = terminals.active.get()?;
        terminals
            .tabs
            .with(|tabs| tabs.iter().find(|t| t.id == active).cloned())
    });

    view! {
        <div class="flex h-full min-h-[24rem] flex-col rounded-lg border border-line bg-app">
            <div class="flex items-center gap-1 border-b border-line px-2 py-1.5">
                // A rolagem fica só nas abas: o menu do + não pode ser cortado por ela.
                <div role="tablist" aria-label="Terminais" class="flex min-w-0 items-center gap-1 overflow-x-auto">
                    <For each=move || terminals.tabs.get() key=|tab| tab.id let:tab>
                        <TabButton id=tab.id />
                    </For>
                </div>
                <NewMenu />
                <super::snippets::SnippetMenu />
                {move || {
                    active_tab
                        .get()
                        .map(|tab| {
                            view! {
                                <button
                                    type="button"
                                    class=format!("ml-auto {GHOST}")
                                    title="Duplicar aba"
                                    aria-label="Duplicar aba"
                                    on:click=move |_| terminals.duplicate(tab.id)
                                >
                                    <Svg icon=Icon::Copy size=16 />
                                </button>
                            }
                        })
                }}
            </div>
            <div class="relative min-h-0 flex-1 overflow-hidden rounded-b-lg">
                <div node_ref=panel class="absolute inset-0 p-2"></div>
                {move || match active_tab.get() {
                    None => view! { <Empty /> }.into_any(),
                    Some(tab) => view! { <Overlay tab=tab /> }.into_any(),
                }}
            </div>
        </div>
    }
}

/// Reajusta o xterm da aba ativa quando o painel muda de tamanho.
fn observe(div: &Element) {
    OBSERVER.with(|o| {
        let mut o = o.borrow_mut();
        if o.is_some() {
            return;
        }
        let callback = Closure::<dyn FnMut()>::new(|| {
            fit(Terminals::current().active.get_untracked(), false);
        });
        if let Ok(observer) = ResizeObserver::new(callback.as_ref().unchecked_ref()) {
            observer.observe(div);
            *o = Some((observer, callback));
        }
    });
}

#[component]
fn TabButton(id: u64) -> impl IntoView {
    let terminals = Terminals::current();
    let tab = Memo::new(move |_| {
        terminals
            .tabs
            .with(|tabs| tabs.iter().find(|t| t.id == id).cloned())
    });
    let title = move || tab.get().map(|t| t.title).unwrap_or_default();
    let selected = move || terminals.active.get() == Some(id);
    let editing = RwSignal::new(false);
    let dot = move || match tab.get().map(|t| t.phase) {
        Some(Phase::Open) => "bg-success",
        Some(Phase::Connecting | Phase::Ask(_)) => "bg-warning",
        _ => "bg-subtle",
    };

    view! {
        <div
            class="group flex shrink-0 items-center gap-2 rounded-md py-1 pr-1 pl-2.5 text-sm"
            class=(["bg-card", "text-fg"], selected)
            class=(["text-muted", "hover:bg-card-hover", "hover:text-fg"], move || !selected())
        >
            <span class=move || format!("size-2 rounded-full {}", dot()) aria-hidden="true"></span>
            {move || {
                if editing.get() {
                    view! { <RenameInput id=id title=title() editing=editing /> }.into_any()
                } else {
                    view! {
                        <button
                            type="button"
                            role="tab"
                            aria-selected=move || selected().to_string()
                            class="max-w-48 truncate font-mono text-[13px]"
                            title="Clique duas vezes para renomear"
                            on:click=move |_| terminals.active.set(Some(id))
                            on:dblclick=move |_| editing.set(true)
                        >
                            {title}
                        </button>
                    }
                    .into_any()
                }
            }}
            <button
                type="button"
                class="rounded p-0.5 text-subtle opacity-60 group-hover:opacity-100 hover:bg-card-hover hover:text-fg"
                aria-label=move || format!("Fechar {}", title())
                on:click=move |_| terminals.close(id)
            >
                <Svg icon=Icon::X size=14 />
            </button>
        </div>
    }
}

#[component]
fn RenameInput(id: u64, title: String, editing: RwSignal<bool>) -> impl IntoView {
    let terminals = Terminals::current();
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
            el.select();
        }
    });
    // O Enter tira o campo da tela, e sair da tela dispara o `blur`: salva uma vez só.
    let save = move || {
        if !editing.get_untracked() {
            return;
        }
        if let Some(el) = input.try_get_untracked().flatten() {
            terminals.rename(id, &el.value());
        }
        editing.set(false);
    };
    view! {
        <input
            node_ref=input
            aria-label="Nome da aba"
            class="w-40 rounded border border-line bg-input px-1.5 py-0.5 font-mono text-[13px] focus:outline-none"
            value=title
            on:keydown=move |e: ev::KeyboardEvent| match e.key().as_str() {
                "Enter" => save(),
                "Escape" => editing.set(false),
                _ => {}
            }
            on:blur=move |_| save()
        />
    }
}

/// Botão "+" com os alvos: shells deste PC, distros WSL, containers e hosts SSH.
#[component]
fn NewMenu() -> impl IntoView {
    let open = RwSignal::new(false);
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || keys.remove());
    view! {
        <div class="relative shrink-0">
            <button
                type="button"
                class=GHOST
                aria-label="Novo terminal"
                title="Novo terminal"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|o| *o = !*o)
            >
                <Svg icon=Icon::Plus size=16 />
            </button>
            <Show when=move || open.get()>
                <div class="fixed inset-0 z-30" on:click=move |_| open.set(false)></div>
                <Targets on_pick=Callback::new(move |()| open.set(false)) />
            </Show>
        </div>
    }
}

/// Lê a amostra de um tópico já no tipo dela (erros e tópicos ausentes viram `None`).
fn sample<T: DeserializeOwned + Send + Sync + 'static>(topic: &'static str) -> Signal<Option<T>> {
    let raw = api::subscribe(topic, None);
    Signal::derive(move || {
        raw.with(|s| {
            s.as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|v| serde_json::from_value::<T>(v.clone()).ok())
        })
    })
}

#[component]
fn Targets(on_pick: Callback<()>) -> impl IntoView {
    let terminals = Terminals::current();
    // Assinados só enquanto o menu está aberto.
    let info = sample::<SystemInfo>(TOPIC_INFO);
    let wsl = sample::<WslStatus>(TOPIC_WSL);
    let docker = sample::<DockerStatus>(TOPIC_DOCKER);
    let data = use_context::<Data>();

    let pick = move |target: TerminalTarget, title: String| {
        on_pick.run(());
        terminals.open(target, title);
    };
    let item = move |icon: Icon, label: String, detail: Option<String>, target: TerminalTarget| {
        let title = label.clone();
        view! {
            <button
                type="button"
                role="menuitem"
                class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm hover:bg-card-hover"
                on:click=move |_| pick(target.clone(), title.clone())
            >
                <Svg icon=icon size=16 />
                <span class="truncate">{label}</span>
                {detail.map(|d| view! { <span class="ml-auto truncate text-xs text-subtle">{d}</span> })}
            </button>
        }
    };
    let heading = |text: &'static str| {
        view! { <p class="px-2.5 pt-2 pb-1 text-xs font-medium text-subtle">{text}</p> }
    };

    let local = move || {
        let windows = info
            .get()
            .is_some_and(|i| i.os.to_ascii_lowercase().contains("windows"));
        let shells: Vec<(LocalShell, &str)> = if windows {
            vec![
                (LocalShell::Powershell, "PowerShell"),
                (LocalShell::Pwsh, "PowerShell 7"),
                (LocalShell::Cmd, "Prompt de comando"),
            ]
        } else {
            vec![(LocalShell::Default, "Shell")]
        };
        shells
            .into_iter()
            .map(|(shell, label)| {
                item(
                    Icon::Terminal,
                    label.to_owned(),
                    None,
                    TerminalTarget::Local { shell: Some(shell) },
                )
            })
            .collect_view()
    };
    let distros = move || {
        let distros = wsl.get().map(|w| w.distros).unwrap_or_default();
        (!distros.is_empty()).then(|| {
            view! {
                {heading("WSL")}
                {distros
                    .into_iter()
                    .map(|d| {
                        let detail = d.running.then(|| "ligada".to_owned());
                        item(Icon::Monitor, d.name.clone(), detail, TerminalTarget::Wsl { distro: d.name })
                    })
                    .collect_view()}
            }
        })
    };
    let containers = move || {
        // `docker exec` daqui só alcança o engine do host, não os de dentro das distros.
        let running: Vec<_> = docker
            .get()
            .map(|d| d.engines)
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.wsl_distro.is_none())
            .flat_map(|e| e.containers)
            .filter(|c| c.state == "running")
            .collect();
        (!running.is_empty()).then(|| {
            view! {
                {heading("Docker")}
                {running
                    .into_iter()
                    .map(|c| {
                        item(Icon::Container, c.name.clone(), Some(c.image), TerminalTarget::Docker { container: c.name })
                    })
                    .collect_view()}
            }
        })
    };
    let hosts = move || {
        let hosts = data.map(|d| d.get().hosts).unwrap_or_default();
        (!hosts.is_empty()).then(|| {
            view! {
                {heading("SSH")}
                {hosts
                    .into_iter()
                    .map(|h| {
                        item(
                            Icon::Server,
                            h.name.clone(),
                            Some(h.address),
                            TerminalTarget::Ssh {
                                host_id: h.id,
                                username: None,
                                password: None,
                                trust_host_key: None,
                            },
                        )
                    })
                    .collect_view()}
            }
        })
    };

    view! {
        <div
            role="menu"
            aria-label="Novo terminal"
            class="absolute top-full left-0 z-40 mt-1 max-h-[60vh] w-72 overflow-y-auto rounded-lg border border-line bg-sidebar p-1.5 shadow-card"
        >
            {heading("Este PC")}
            {local}
            {distros}
            {containers}
            {hosts}
        </div>
    }
}

/// Nenhuma aba: atalho para o terminal deste PC.
#[component]
fn Empty() -> impl IntoView {
    let terminals = Terminals::current();
    view! {
        <div class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-sm text-muted">
            <Svg icon=Icon::Terminal size=28 />
            <p>"Nenhum terminal aberto."</p>
            <button
                type="button"
                class=PRIMARY
                on:click=move |_| terminals.open(TerminalTarget::Local { shell: None }, "Terminal".into())
            >
                <Svg icon=Icon::Plus size=16 />
                "Abrir terminal deste PC"
            </button>
            <p class="text-xs text-subtle">"Use o + para WSL, containers e hosts SSH."</p>
        </div>
    }
}

/// O que cobre o xterm enquanto a aba não está aberta.
#[component]
fn Overlay(tab: Tab) -> impl IntoView {
    let terminals = Terminals::current();
    let id = tab.id;
    match tab.phase {
        Phase::Open => ().into_any(),
        Phase::Connecting => view! {
            <Cover>
                <p class="text-sm text-muted">"Conectando…"</p>
            </Cover>
        }
        .into_any(),
        Phase::Failed(message) => view! {
            <Cover>
                <p role="alert" class="max-w-md text-center text-sm text-danger">{message}</p>
                <div class="flex gap-2">
                    <button type="button" class=PRIMARY on:click=move |_| terminals.reconnect(id)>
                        <Svg icon=Icon::Refresh size=16 />
                        "Tentar de novo"
                    </button>
                    <button type="button" class=BUTTON on:click=move |_| terminals.close(id)>
                        "Fechar aba"
                    </button>
                </div>
            </Cover>
        }
        .into_any(),
        // A saída fica à vista; só uma barra embaixo.
        Phase::Ended => view! {
            <div class="absolute inset-x-2 bottom-2 flex items-center gap-3 rounded-md border border-line bg-sidebar px-3 py-2 text-sm">
                <span class="text-muted">"Sessão encerrada."</span>
                <button type="button" class=format!("ml-auto {PRIMARY}") on:click=move |_| terminals.reconnect(id)>
                    <Svg icon=Icon::Refresh size=16 />
                    "Reconectar"
                </button>
                <button type="button" class=BUTTON on:click=move |_| terminals.close(id)>
                    "Fechar aba"
                </button>
            </div>
        }
        .into_any(),
        Phase::Ask(ask) => view! {
            <Cover>
                <AskForm
                    ask=ask
                    on_answer=Callback::new(move |answer| terminals.answer(id, answer))
                    on_cancel=Callback::new(move |()| terminals.close(id))
                />
            </Cover>
        }
        .into_any(),
    }
}

#[component]
fn Cover(children: Children) -> impl IntoView {
    view! {
        <div class="absolute inset-0 flex flex-col items-center justify-center gap-4 bg-app/95 p-6">
            {children()}
        </div>
    }
}

/// Pergunta de uma conexão SSH (chave do servidor, senha, usuário). Também serve ao SFTP.
#[component]
#[allow(clippy::too_many_lines)] // três formulários pequenos, um por pergunta
pub(crate) fn AskForm(
    ask: Ask,
    on_answer: Callback<Answer>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let cancel = move |_| on_cancel.run(());
    // `autofocus` só vale na carga da página; aqui o campo aparece depois.
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
        }
    });
    match ask {
        Ask::HostKey {
            fingerprint,
            changed,
        } => {
            let (title, text) = if changed {
                (
                    "A chave do servidor mudou",
                    "A chave não é a que você aceitou antes. Isso acontece quando o servidor foi reinstalado, mas também pode ser alguém no meio da conexão. Confira a fingerprint com quem cuida do servidor antes de aceitar.",
                )
            } else {
                (
                    "Primeira conexão a este servidor",
                    "Confira se a fingerprint abaixo é a do servidor. Depois de aceita, o CodatOS avisa se ela mudar.",
                )
            };
            let accept = fingerprint.clone();
            view! {
                <div role="alertdialog" aria-label=title class="w-[min(30rem,100%)] space-y-3 rounded-xl border border-line bg-sidebar p-5 text-sm">
                    <h2 class=if changed { "font-semibold text-danger" } else { "font-semibold" }>{title}</h2>
                    <p class="text-muted">{text}</p>
                    <code class="block rounded-md border border-line bg-input px-3 py-2 font-mono text-[13px] break-all">
                        {fingerprint}
                    </code>
                    <div class="flex justify-end gap-2 pt-1">
                        <button type="button" class=BUTTON on:click=cancel>"Cancelar"</button>
                        <button
                            type="button"
                            class=PRIMARY
                            on:click=move |_| on_answer.run(Answer::TrustHostKey(accept.clone()))
                        >
                            {if changed { "Aceitar a chave nova" } else { "Confiar e conectar" }}
                        </button>
                    </div>
                </div>
            }
            .into_any()
        }
        Ask::Password { rejected } => {
            let value = RwSignal::new(String::new());
            view! {
                <form
                    class="w-[min(24rem,100%)] space-y-3 rounded-xl border border-line bg-sidebar p-5 text-sm"
                    on:submit=move |e: ev::SubmitEvent| {
                        e.prevent_default();
                        let password = value.get_untracked();
                        if !password.is_empty() {
                            on_answer.run(Answer::Password(password));
                        }
                    }
                >
                    <h2 class="font-semibold">"Senha do servidor"</h2>
                    {rejected.then(|| view! { <p role="alert" class="text-danger">"O servidor recusou. Confira a senha."</p> })}
                    <label class="block space-y-1">
                        <span class=LABEL>"Senha (não fica guardada)"</span>
                        <input
                            type="password"
                            autocomplete="off"
                            node_ref=input
                            class=FIELD
                            bind:value=value
                        />
                    </label>
                    <div class="flex justify-end gap-2 pt-1">
                        <button type="button" class=BUTTON on:click=cancel>"Cancelar"</button>
                        <button type="submit" class=PRIMARY disabled=move || value.with(String::is_empty)>
                            "Conectar"
                        </button>
                    </div>
                </form>
            }
            .into_any()
        }
        Ask::Username => {
            let value = RwSignal::new(String::new());
            view! {
                <form
                    class="w-[min(24rem,100%)] space-y-3 rounded-xl border border-line bg-sidebar p-5 text-sm"
                    on:submit=move |e: ev::SubmitEvent| {
                        e.prevent_default();
                        let username = value.get_untracked().trim().to_owned();
                        if !username.is_empty() {
                            on_answer.run(Answer::Username(username));
                        }
                    }
                >
                    <h2 class="font-semibold">"Usuário"</h2>
                    <p class="text-muted">"Este host não tem identidade. Com qual usuário entrar?"</p>
                    <input
                        type="text"
                        autocomplete="off"
                        node_ref=input
                        aria-label="Usuário"
                        class=FIELD
                        bind:value=value
                    />
                    <div class="flex justify-end gap-2 pt-1">
                        <button type="button" class=BUTTON on:click=cancel>"Cancelar"</button>
                        <button type="submit" class=PRIMARY disabled=move || value.with(|v| v.trim().is_empty())>
                            "Conectar"
                        </button>
                    </div>
                </form>
            }
            .into_any()
        }
    }
}
