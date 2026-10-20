//! Tela do SFTP: escolha do host, os dois painéis, a fila e os diálogos.

use codatos_protocol::sftp::{Direction, Entry, EntryKind, TransferAction, TransferState};
use js_sys::{Object, Reflect};
use leptos::{ev, html, prelude::*, task::spawn_local};
use wasm_bindgen::JsValue;

use super::{Conflict, Phase, Row, Sftp, Side};
use crate::components::icon::{Icon, Svg};
use crate::connections::Data;
use crate::connections::ui::{BUTTON, DANGER, Dialog, FIELD, FormError, GHOST, LABEL, PRIMARY};
use crate::dashboard::widgets::format;
use crate::terminal::AskForm;

/// Ação de um painel que pede confirmação ou um nome.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Mkdir(Side),
    Rename(Side, String),
    Chmod(String, Option<u32>),
    Remove(Side, Vec<String>),
}

#[component]
pub fn SftpPanel() -> impl IntoView {
    let sftp = Sftp::current();
    move || {
        if sftp.phase.get() == Phase::Idle {
            view! { <HostPicker /> }.into_any()
        } else {
            view! { <Session /> }.into_any()
        }
    }
}

/// Nenhuma sessão: os hosts cadastrados.
#[component]
fn HostPicker() -> impl IntoView {
    let sftp = Sftp::current();
    let data = use_context::<Data>();
    let hosts = move || data.map(|d| d.get().hosts).unwrap_or_default();
    view! {
        <div class="flex flex-col items-center gap-4 py-10 text-sm">
            <Svg icon=Icon::Folder size=28 />
            <p class="text-muted">"Escolha um host para ver os arquivos dele ao lado dos deste PC."</p>
            {move || {
                let hosts = hosts();
                if hosts.is_empty() {
                    view! { <p class="text-subtle">"Nenhum host cadastrado. Cadastre um em SSH/SFTP."</p> }.into_any()
                } else {
                    view! {
                        <ul class="grid w-[min(40rem,100%)] gap-2 sm:grid-cols-2">
                            {hosts
                                .into_iter()
                                .map(|h| {
                                    let (id, name) = (h.id.clone(), h.name.clone());
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class="flex w-full items-center gap-3 rounded-lg border border-line bg-card px-3 py-2.5 text-left hover:bg-card-hover"
                                                on:click=move |_| sftp.open(id.clone(), name.clone())
                                            >
                                                <Svg icon=Icon::Server size=16 />
                                                <span class="min-w-0">
                                                    <span class="block truncate font-medium">{h.name}</span>
                                                    <span class="block truncate font-mono text-xs text-muted">{h.address}</span>
                                                </span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

#[component]
fn Session() -> impl IntoView {
    let sftp = Sftp::current();
    let action = RwSignal::new(None::<Action>);
    let open = move || sftp.phase.get() == Phase::Open;
    let has = move |side: Side| sftp.pane(side).selected.with(|s| !s.is_empty());
    let send = move |from: Side| {
        let names = sftp
            .pane(from)
            .selected
            .get_untracked()
            .into_iter()
            .collect();
        sftp.send(from, names);
    };
    let status = move || match sftp.phase.get() {
        Phase::Idle | Phase::Open => "",
        Phase::Connecting => "Conectando…",
        Phase::Ask(_) => "Esperando resposta",
        Phase::Failed(_) => "Falhou",
        Phase::Ended => "Desconectado",
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-3">
            <div class="flex items-center gap-3 text-sm">
                <Svg icon=Icon::Server size=16 />
                <span class="font-medium">{move || sftp.title.get()}</span>
                <span class="text-muted">{status}</span>
                <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| sftp.close()>
                    "Trocar de host"
                </button>
            </div>
            {move || sftp.notice.get().map(|message| view! {
                <div role="alert" class="flex items-start gap-3 rounded-md border border-line bg-card px-3 py-2 text-sm">
                    <span class="flex-1 text-warning">{message}</span>
                    <button type="button" class=GHOST aria-label="Dispensar" on:click=move |_| sftp.notice.set(None)>
                        <Svg icon=Icon::X size=14 />
                    </button>
                </div>
            })}
            <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] gap-3">
                <PaneView side=Side::Local action=action />
                <div class="flex flex-col items-center justify-center gap-2">
                    <button
                        type="button"
                        class=BUTTON
                        title="Enviar a seleção deste PC para o servidor"
                        disabled=move || !open() || !has(Side::Local)
                        on:click=move |_| send(Side::Local)
                    >
                        "Enviar"
                        <Svg icon=Icon::ArrowRight size=16 />
                    </button>
                    <button
                        type="button"
                        class=BUTTON
                        title="Baixar a seleção do servidor para este PC"
                        disabled=move || !open() || !has(Side::Remote)
                        on:click=move |_| send(Side::Remote)
                    >
                        <Svg icon=Icon::ArrowLeft size=16 />
                        "Baixar"
                    </button>
                </div>
                <PaneView side=Side::Remote action=action />
            </div>
            <Queue />
            <ActionDialog action=action />
            <ConflictDialog />
        </div>
    }
}

#[component]
fn PaneView(side: Side, action: RwSignal<Option<Action>>) -> impl IntoView {
    let sftp = Sftp::current();
    let pane = sftp.pane(side);
    let draft = RwSignal::new(String::new());
    Effect::new(move |_| draft.set(pane.path.get()));
    let ready = move || side == Side::Local || sftp.phase.get() == Phase::Open;
    let one = move || {
        pane.selected.with(|s| {
            if s.len() == 1 {
                s.iter().next().cloned()
            } else {
                None
            }
        })
    };
    let target = move || sftp.dragging.get() == Some(side.other());
    let tool = move |icon: Icon,
                     label: &'static str,
                     enabled: Signal<bool>,
                     run: Box<dyn Fn() + Send + Sync>| {
        view! {
            <button
                type="button"
                class=format!("{GHOST} disabled:opacity-40")
                aria-label=label
                title=label
                disabled=move || !ready() || !enabled.get()
                on:click=move |_| run()
            >
                <Svg icon=icon size=16 />
            </button>
        }
    };
    let always = Signal::derive(|| true);
    let has_one = Signal::derive(move || one().is_some());
    let has_any = Signal::derive(move || pane.selected.with(|s| !s.is_empty()));

    view! {
        <div
            role="region"
            aria-label=if side == Side::Local { "Este PC" } else { "Servidor" }
            class="relative flex min-h-0 flex-col overflow-hidden rounded-lg border bg-card"
            class=(["border-primary", "ring-1", "ring-primary"], target)
            class=("border-line", move || !target())
            on:dragover=move |e: ev::DragEvent| {
                if target() {
                    e.prevent_default();
                }
            }
            on:drop=move |e: ev::DragEvent| {
                e.prevent_default();
                if let Some(from) = sftp.dragging.get_untracked().filter(|f| *f != side) {
                    let names = sftp.pane(from).selected.get_untracked().into_iter().collect();
                    sftp.send(from, names);
                }
                sftp.dragging.set(None);
            }
        >
            <div class="flex items-center gap-1 border-b border-line px-2 py-1.5">
                <span class="px-1 text-xs font-medium tracking-wide text-muted uppercase">
                    {if side == Side::Local { "Este PC" } else { "Servidor" }}
                </span>
                <span class="flex-1"></span>
                {tool(Icon::ArrowUp, "Pasta de cima", always, Box::new(move || sftp.up(side)))}
                {tool(Icon::Refresh, "Atualizar", always, Box::new(move || sftp.refresh(side)))}
                {tool(Icon::FolderPlus, "Nova pasta", always, Box::new(move || action.set(Some(Action::Mkdir(side)))))}
                {tool(Icon::Pencil, "Renomear", has_one, Box::new(move || {
                    if let Some(name) = one() {
                        action.set(Some(Action::Rename(side, name)));
                    }
                }))}
                {(side == Side::Remote).then(|| tool(Icon::Lock, "Permissões", has_one, Box::new(move || {
                    if let Some(name) = one() {
                        let mode = pane.entry(&name).and_then(|e| e.mode);
                        action.set(Some(Action::Chmod(name, mode)));
                    }
                })))}
                {tool(Icon::Trash, "Apagar", has_any, Box::new(move || {
                    let mut names: Vec<String> = pane.selected.get_untracked().into_iter().collect();
                    names.sort();
                    action.set(Some(Action::Remove(side, names)));
                }))}
            </div>
            <form
                class="border-b border-line px-2 py-1.5"
                on:submit=move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    sftp.list(side, draft.get_untracked().trim());
                }
            >
                <input
                    type="text"
                    spellcheck="false"
                    aria-label="Caminho"
                    class=format!("{FIELD} py-1 font-mono text-xs")
                    disabled=move || !ready()
                    bind:value=draft
                />
            </form>
            <div class="min-h-0 flex-1 overflow-y-auto p-1 text-sm">
                {move || match pane.listing.get() {
                    None => view! { <p class="p-3 text-muted">"Carregando…"</p> }.into_any(),
                    Some(Err(message)) => view! {
                        <div class="space-y-2 p-3">
                            <p role="alert" class="text-danger">{message}</p>
                            <button type="button" class=BUTTON on:click=move |_| sftp.refresh(side)>
                                "Voltar para a pasta anterior"
                            </button>
                        </div>
                    }
                    .into_any(),
                    Some(Ok(entries)) if entries.is_empty() => {
                        view! { <p class="p-3 text-subtle">"Pasta vazia."</p> }.into_any()
                    }
                    Some(Ok(entries)) => view! {
                        <ul role="listbox" aria-multiselectable="true" aria-label="Arquivos">
                            {entries
                                .into_iter()
                                .map(|entry| view! { <EntryRow side=side entry=entry /> })
                                .collect_view()}
                        </ul>
                    }
                    .into_any(),
                }}
            </div>
            {(side == Side::Remote).then(|| view! { <RemoteCover /> })}
        </div>
    }
}

#[component]
fn EntryRow(side: Side, entry: Entry) -> impl IntoView {
    let sftp = Sftp::current();
    let pane = sftp.pane(side);
    let name = StoredValue::new(entry.name.clone());
    let selected = move || pane.selected.with(|s| name.with_value(|n| s.contains(n)));
    let folder = matches!(entry.kind, EntryKind::Dir | EntryKind::Symlink);
    let icon = if entry.kind == EntryKind::Dir {
        Icon::Folder
    } else if entry.kind == EntryKind::Symlink {
        Icon::Link
    } else {
        Icon::File
    };
    let size = (entry.kind != EntryKind::Dir).then(|| format::bytes(entry.size));
    let modified = entry.modified.map(date);

    view! {
        <li
            role="option"
            aria-selected=move || selected().to_string()
            draggable="true"
            class="grid cursor-default grid-cols-[minmax(0,1fr)_5.5rem_8.5rem] items-center gap-2 rounded px-2 py-1 select-none"
            class=(["bg-primary-soft", "text-fg"], selected)
            class=("hover:bg-card-hover", move || !selected())
            on:click=move |e: ev::MouseEvent| {
                let name = name.get_value();
                if e.ctrl_key() || e.meta_key() {
                    pane.selected.update(|s| {
                        if !s.remove(&name) {
                            s.insert(name);
                        }
                    });
                } else {
                    pane.selected.set(std::iter::once(name).collect());
                }
            }
            on:dblclick=move |_| {
                let name = name.get_value();
                if folder {
                    sftp.list(side, &sftp.child(side, &name));
                } else if sftp.phase.get_untracked() == Phase::Open {
                    sftp.send(side, vec![name]);
                }
            }
            on:dragstart=move |e: ev::DragEvent| {
                let name = name.get_value();
                if !pane.selected.with_untracked(|s| s.contains(&name)) {
                    pane.selected.set(std::iter::once(name.clone()).collect());
                }
                if let Some(dt) = e.data_transfer() {
                    let _ = dt.set_data("text/plain", &name);
                    dt.set_effect_allowed("copy");
                }
                sftp.dragging.set(Some(side));
            }
            on:dragend=move |_| sftp.dragging.set(None)
        >
            <span class="flex min-w-0 items-center gap-2">
                <span class=if entry.kind == EntryKind::Dir { "text-primary" } else { "text-muted" }>
                    <Svg icon=icon size=15 />
                </span>
                <span class="truncate">{entry.name}</span>
            </span>
            <span class="text-right text-xs text-muted tabular-nums">{size}</span>
            <span class="text-right text-xs text-muted tabular-nums">{modified}</span>
        </li>
    }
}

/// O que cobre o painel do servidor enquanto a sessão não está aberta.
#[component]
fn RemoteCover() -> impl IntoView {
    let sftp = Sftp::current();
    let cover =
        "absolute inset-0 flex flex-col items-center justify-center gap-4 bg-app/95 p-6 text-sm";
    move || match sftp.phase.get() {
        Phase::Idle | Phase::Open => ().into_any(),
        Phase::Connecting => view! {
            <div class=cover><p class="text-muted">"Conectando…"</p></div>
        }
        .into_any(),
        Phase::Ask(ask) => view! {
            <div class=cover>
                <AskForm
                    ask=ask
                    on_answer=Callback::new(move |answer| sftp.answer(answer))
                    on_cancel=Callback::new(move |()| sftp.close())
                />
            </div>
        }
        .into_any(),
        Phase::Failed(message) => view! {
            <div class=cover>
                <p role="alert" class="max-w-md text-center text-danger">{message}</p>
                <button type="button" class=PRIMARY on:click=move |_| sftp.reconnect()>
                    <Svg icon=Icon::Refresh size=16 />
                    "Tentar de novo"
                </button>
            </div>
        }
        .into_any(),
        Phase::Ended => view! {
            <div class=cover>
                <p class="text-muted">"A conexão com o servidor terminou."</p>
                <button type="button" class=PRIMARY on:click=move |_| sftp.reconnect()>
                    <Svg icon=Icon::Refresh size=16 />
                    "Reconectar"
                </button>
            </div>
        }
        .into_any(),
    }
}

/// Fila de transferências da sessão.
#[component]
fn Queue() -> impl IntoView {
    let sftp = Sftp::current();
    move || {
        let rows = sftp.transfers.get();
        (!rows.is_empty()).then(|| {
            let any_finished = rows.iter().any(Row::finished);
            view! {
                <section aria-label="Transferências" class="flex max-h-56 min-h-0 flex-col rounded-lg border border-line bg-card">
                    <header class="flex items-center gap-2 border-b border-line px-3 py-1.5 text-xs">
                        <span class="font-medium tracking-wide text-muted uppercase">"Transferências"</span>
                        <span class="text-subtle">{rows.len()}</span>
                        {any_finished.then(|| view! {
                            <button type="button" class="ml-auto text-muted hover:text-fg" on:click=move |_| sftp.clear_finished()>
                                "Limpar concluídas"
                            </button>
                        })}
                    </header>
                    <ul class="min-h-0 flex-1 divide-y divide-line overflow-y-auto">
                        {rows.into_iter().map(transfer_row).collect_view()}
                    </ul>
                </section>
            }
        })
    }
}

fn transfer_row(row: Row) -> impl IntoView {
    let sftp = Sftp::current();
    let upload = row.direction == Direction::Upload;
    let label = match row.state {
        TransferState::Queued => "Na fila",
        TransferState::Running if upload => "Enviando",
        TransferState::Running => "Baixando",
        TransferState::Paused => "Pausada",
        TransferState::Done => "Concluída",
        TransferState::Failed => "Falhou",
        TransferState::Canceled => "Cancelada",
    };
    let percent = if row.total == 0 {
        if row.state == TransferState::Done {
            100.0
        } else {
            0.0
        }
    } else {
        format::percent(row.done, row.total)
    };
    let amount = if row.total > 0 {
        format!(
            "{} de {}",
            format::bytes(row.done),
            format::bytes(row.total)
        )
    } else {
        String::new()
    };
    let destination = if upload {
        row.remote.clone()
    } else {
        row.local.clone()
    };
    let id = StoredValue::new(row.id.clone());
    let control = move |action: TransferAction| sftp.control(id.get_value(), action);
    let button = move |icon: Icon, label: &'static str, run: Box<dyn Fn() + Send + Sync>| {
        view! {
            <button type="button" class=GHOST aria-label=label title=label on:click=move |_| run()>
                <Svg icon=icon size=14 />
            </button>
        }
    };
    let buttons = match row.state {
        TransferState::Running => view! {
            {button(Icon::Pause, "Pausar", Box::new(move || control(TransferAction::Pause)))}
            {button(Icon::X, "Cancelar", Box::new(move || control(TransferAction::Cancel)))}
        }
        .into_any(),
        TransferState::Paused => view! {
            {button(Icon::Play, "Continuar", Box::new(move || control(TransferAction::Resume)))}
            {button(Icon::X, "Cancelar", Box::new(move || control(TransferAction::Cancel)))}
        }
        .into_any(),
        TransferState::Queued => button(
            Icon::X,
            "Cancelar",
            Box::new(move || control(TransferAction::Cancel)),
        )
        .into_any(),
        TransferState::Failed => button(
            Icon::Refresh,
            "Tentar de novo de onde parou",
            Box::new(move || sftp.retry(&id.get_value())),
        )
        .into_any(),
        TransferState::Done | TransferState::Canceled => ().into_any(),
    };
    let tone = match row.state {
        TransferState::Failed => "text-danger",
        TransferState::Done => "text-success",
        _ => "text-muted",
    };

    view! {
        <li class="flex items-center gap-3 px-3 py-1.5 text-sm">
            <span class="text-muted">
                <Svg icon=if upload { Icon::Upload } else { Icon::Download } size=14 />
            </span>
            <div class="min-w-0 flex-1">
                <div class="flex items-baseline gap-2">
                    <span class="truncate" title=destination>{row.name}</span>
                    <span class=format!("shrink-0 text-xs {tone}")>{label}</span>
                    <span class="ml-auto shrink-0 text-xs text-muted tabular-nums">{amount}</span>
                </div>
                {row.error.map(|e| view! { <p class="truncate text-xs text-danger" title=e.clone()>{e.clone()}</p> })}
                <div class="mt-1 h-1 overflow-hidden rounded bg-input">
                    <div class="h-full bg-primary" style=format!("width: {percent:.1}%")></div>
                </div>
            </div>
            <div class="flex shrink-0 gap-0.5">{buttons}</div>
        </li>
    }
}

#[component]
fn ActionDialog(action: RwSignal<Option<Action>>) -> impl IntoView {
    move || {
        action.get().map(
            |a| view! { <ActionForm action=a close=Callback::new(move |()| action.set(None)) /> },
        )
    }
}

#[component]
fn ActionForm(action: Action, close: Callback<()>) -> impl IntoView {
    let sftp = Sftp::current();
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let input = NodeRef::<html::Input>::new();
    let (title, label, initial, submit) = match &action {
        Action::Mkdir(_) => ("Nova pasta", "Nome", String::new(), "Criar"),
        Action::Rename(_, name) => ("Renomear", "Novo nome", name.clone(), "Renomear"),
        Action::Chmod(_, mode) => (
            "Permissões",
            "Modo em octal (ex.: 755)",
            mode.map(|m| format!("{:o}", m & 0o7777))
                .unwrap_or_default(),
            "Aplicar",
        ),
        Action::Remove(_, names) => (
            "Apagar",
            "",
            if names.len() == 1 {
                format!(
                    "Apagar “{}”? Pastas vão com tudo o que tem dentro.",
                    names[0]
                )
            } else {
                format!(
                    "Apagar {} itens? Pastas vão com tudo o que tem dentro.",
                    names.len()
                )
            },
            "Apagar",
        ),
    };
    let value = RwSignal::new(initial.clone());
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            // O `bind:value` ainda não pôs o valor; sem ele aqui, não há o que selecionar.
            // Num nome com extensão, seleciona só o nome, como o Explorer.
            let text = value.get_untracked();
            el.set_value(&text);
            let _ = el.focus();
            let end = text.rfind('.').filter(|&i| i > 0).unwrap_or(text.len());
            let end = u32::try_from(text[..end].encode_utf16().count()).unwrap_or(0);
            let _ = el.set_selection_range(0, end);
        }
    });
    let action = StoredValue::new(action);
    let run = move || {
        let value = value.get_untracked().trim().to_owned();
        let action = action.get_value();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = match action {
                Action::Mkdir(side) => match check_name(&value) {
                    Ok(()) => sftp.mkdir(side, &value).await,
                    Err(message) => Err(message),
                },
                Action::Rename(side, from) => match check_name(&value) {
                    Ok(()) => sftp.rename(side, &from, &value).await,
                    Err(message) => Err(message),
                },
                Action::Chmod(name, _) => match u32::from_str_radix(&value, 8) {
                    Ok(mode) if mode <= 0o7777 => sftp.chmod(&name, mode).await,
                    _ => Err("Use de 1 a 4 dígitos de 0 a 7, como 644 ou 755.".into()),
                },
                Action::Remove(side, names) => sftp.remove(side, &names).await,
            };
            busy.set(false);
            match result {
                Ok(()) => close.run(()),
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let removing = matches!(action.get_value(), Action::Remove(..));

    view! {
        <Dialog title=title.to_owned() on_close=close>
            <form
                class="space-y-4 p-5 text-sm"
                on:submit=move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    run();
                }
            >
                {if removing {
                    view! { <p>{initial}</p> }.into_any()
                } else {
                    view! {
                        <label class="block space-y-1">
                            <span class=LABEL>{label}</span>
                            <input type="text" spellcheck="false" node_ref=input class=FIELD bind:value=value />
                        </label>
                    }
                    .into_any()
                }}
                <FormError error=error />
                <div class="flex justify-end gap-2">
                    <button type="button" class=BUTTON on:click=move |_| close.run(())>"Cancelar"</button>
                    <button
                        type="submit"
                        class=if removing { DANGER } else { PRIMARY }
                        disabled=move || busy.get() || (!removing && value.with(|v| v.trim().is_empty()))
                    >
                        {submit}
                    </button>
                </div>
            </form>
        </Dialog>
    }
}

/// Nome de um item novo: sem separadores de pasta.
fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        Err("Use um nome sem / ou \\.".into())
    } else {
        Ok(())
    }
}

/// Destino que já existe: substituir ou pular, um de cada vez.
#[component]
fn ConflictDialog() -> impl IntoView {
    let sftp = Sftp::current();
    move || {
        let (first, count) = sftp.conflicts.with(|c| (c.first().cloned(), c.len()));
        first.map(|conflict| {
            let skip = Callback::new(move |()| sftp.conflicts.update(|c| {
                if !c.is_empty() {
                    c.remove(0);
                }
            }));
            let replace = move |all: bool| {
                let taken: Vec<Conflict> = sftp.conflicts.try_update(|c| {
                    if all { std::mem::take(c) } else if c.is_empty() { Vec::new() } else { vec![c.remove(0)] }
                }).unwrap_or_default();
                for Conflict { name, mut request } in taken {
                    request.overwrite = true;
                    sftp.start(name, request);
                }
            };
            let destination = match conflict.request.direction {
                Direction::Upload => conflict.request.remote.clone(),
                Direction::Download => conflict.request.local.clone(),
            };
            view! {
                <Dialog title="O destino já existe".to_owned() on_close=skip>
                    <div class="space-y-4 p-5 text-sm">
                        <p>"Já existe um arquivo em "<code class="font-mono text-xs break-all">{destination}</code>". Substituir?"</p>
                        <div class="flex justify-end gap-2">
                            <button type="button" class=BUTTON on:click=move |_| skip.run(())>"Pular"</button>
                            {(count > 1).then(|| view! {
                                <button type="button" class=BUTTON on:click=move |_| replace(true)>
                                    {format!("Substituir todos ({count})")}
                                </button>
                            })}
                            <button type="button" class=PRIMARY on:click=move |_| replace(false)>"Substituir"</button>
                        </div>
                    </div>
                </Dialog>
            }
        })
    }
}

/// Data e hora curtas no formato do Brasil.
fn date(seconds: i64) -> String {
    #[allow(clippy::cast_precision_loss)] // segundos até 2^53 cabem sem perda
    let date = js_sys::Date::new(&JsValue::from_f64(seconds as f64 * 1000.0));
    let options = Object::new();
    let _ = Reflect::set(&options, &"dateStyle".into(), &"short".into());
    let _ = Reflect::set(&options, &"timeStyle".into(), &"short".into());
    date.to_locale_string("pt-BR", &options).into()
}
