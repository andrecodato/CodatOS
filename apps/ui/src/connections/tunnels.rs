//! Aba Túneis (RF-CON-040): lista com o estado de cada túnel, abrir e fechar, e o editor.
//! O núcleo mantém os túneis abertos mesmo com a tela fechada.

use std::time::Duration;

use codatos_protocol::tunnels::{
    METHOD_TUNNEL_DELETE, METHOD_TUNNEL_SAVE, METHOD_TUNNEL_START, METHOD_TUNNEL_STOP,
    METHOD_TUNNELS_LIST, Tunnel, TunnelKind, TunnelState, TunnelStatus, Tunnels,
};
use leptos::{ev, prelude::*, task::spawn_local};
use serde_json::json;

use super::Data;
use super::ui::{self, BUTTON, DANGER, Dialog, Edit, FIELD, FormError, GHOST, LABEL, PRIMARY};
use crate::components::icon::{Icon, Svg};
use crate::ids::new_id;
use crate::terminal::{Answer, Ask, AskForm};

/// Enquanto a aba está aberta, o estado é relido nesse intervalo.
const REFRESH: Duration = Duration::from_secs(3);

type List = RwSignal<Option<Result<Vec<TunnelStatus>, String>>>;

fn reload(list: List) {
    spawn_local(async move {
        let result = ui::call(METHOD_TUNNELS_LIST, &json!({}))
            .await
            .and_then(|v| serde_json::from_value::<Tunnels>(v).map_err(|e| e.to_string()))
            .map(|t| t.tunnels);
        list.try_set(Some(result));
    });
}

/// Abertura em andamento que esbarrou numa pergunta da conexão SSH.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    id: String,
    ask: Ask,
    username: Option<String>,
    password: Option<String>,
    trust_host_key: Option<String>,
}

#[component]
pub fn Tunnels() -> impl IntoView {
    let list: List = RwSignal::new(None);
    reload(list);
    let timer = set_interval_with_handle(move || reload(list), REFRESH).ok();
    on_cleanup(move || {
        if let Some(timer) = timer {
            timer.clear();
        }
    });
    let editing = RwSignal::new(None::<Edit<Tunnel>>);
    let pending = RwSignal::new(None::<Pending>);
    let notice = RwSignal::new(None::<String>);

    let start = move |p: Pending| {
        spawn_local(async move {
            let params = json!({
                "id": p.id,
                "username": p.username,
                "password": p.password,
                "trustHostKey": p.trust_host_key,
            });
            match api_start(&params).await {
                Ok(()) => pending.set(None),
                Err(error) => {
                    if let crate::terminal::Phase::Ask(ask) = crate::terminal::failure(&error) {
                        pending.set(Some(Pending { ask, ..p }));
                    } else {
                        pending.set(None);
                        notice.set(Some(error.message));
                    }
                }
            }
            reload(list);
        });
    };
    let start_new = Callback::new(move |id: String| {
        start(Pending {
            id,
            ask: Ask::Username,
            username: None,
            password: None,
            trust_host_key: None,
        });
    });

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-3">
            <div class="flex items-center gap-3">
                <p class="text-sm text-muted">
                    "Portas encaminhadas por SSH. Ficam abertas mesmo com esta tela fechada."
                </p>
                <button type="button" class=format!("ml-auto {PRIMARY}") on:click=move |_| editing.set(Some(Edit::New))>
                    <Svg icon=Icon::Plus size=16 />
                    "Novo túnel"
                </button>
            </div>
            {move || notice.get().map(|message| view! {
                <div role="alert" class="flex items-start gap-3 rounded-md border border-line bg-card px-3 py-2 text-sm">
                    <span class="flex-1 text-danger">{message}</span>
                    <button type="button" class=GHOST aria-label="Dispensar" on:click=move |_| notice.set(None)>
                        <Svg icon=Icon::X size=14 />
                    </button>
                </div>
            })}
            {move || match list.get() {
                None => view! { <p class="text-sm text-muted">"Carregando túneis…"</p> }.into_any(),
                Some(Err(message)) => view! { <p class="text-sm text-danger">{message}</p> }.into_any(),
                Some(Ok(items)) if items.is_empty() => view! {
                    <div class="rounded-lg border border-dashed border-line p-6 text-sm text-muted">
                        <p>"Nenhum túnel ainda."</p>
                        <p class="mt-1 text-subtle">
                            "Local (-L) traz uma porta do servidor para este PC, remoto (-R) leva uma porta deste PC para o servidor e dinâmico (-D) cria um proxy SOCKS que sai pelo servidor."
                        </p>
                    </div>
                }
                .into_any(),
                Some(Ok(items)) => view! {
                    <ul class="divide-y divide-line rounded-lg border border-line bg-card">
                        {items
                            .into_iter()
                            .map(|status| view! {
                                <Row status=status list=list editing=editing start=start_new notice=notice />
                            })
                            .collect_view()}
                    </ul>
                }
                .into_any(),
            }}
            {move || editing.get().map(|edit| view! {
                <Editor edit=edit list=list close=Callback::new(move |()| editing.set(None)) />
            })}
            <AskDialog pending=pending start=Callback::new(start) />
        </div>
    }
}

/// Pergunta da conexão SSH ao abrir um túnel; a resposta tenta de novo.
#[component]
fn AskDialog(pending: RwSignal<Option<Pending>>, start: Callback<Pending>) -> impl IntoView {
    view! {
            {move || pending.get().map(|p| {
                let retry = p.clone();
                view! {
                    <Dialog title="Abrir túnel".to_owned() on_close=Callback::new(move |()| pending.set(None))>
                        <div class="flex justify-center p-5">
                            <AskForm
                                ask=p.ask.clone()
                                on_answer=Callback::new(move |answer| {
                                    let mut next = retry.clone();
                                    match answer {
                                        Answer::TrustHostKey(f) => next.trust_host_key = Some(f),
                                        Answer::Password(s) => next.password = Some(s),
                                        Answer::Username(u) => next.username = Some(u),
                                    }
                                    start.run(next);
                                })
                                on_cancel=Callback::new(move |()| pending.set(None))
                            />
                        </div>
                    </Dialog>
                }
            })}
    }
}

async fn api_start(params: &serde_json::Value) -> Result<(), codatos_protocol::rpc::RpcError> {
    crate::api::call(METHOD_TUNNEL_START, params.clone())
        .await
        .map(|_| ())
}

fn kind_label(kind: TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "-L",
        TunnelKind::Remote => "-R",
        TunnelKind::Dynamic => "-D",
    }
}

/// "127.0.0.1:8080 → db:5432", no sentido em que os dados entram.
fn route(t: &Tunnel) -> String {
    match t.kind {
        TunnelKind::Local => format!(
            "{}:{} → {}:{} (pelo servidor)",
            t.bind_host, t.bind_port, t.target_host, t.target_port
        ),
        TunnelKind::Remote => format!(
            "servidor {}:{} → {}:{} (deste PC)",
            t.bind_host, t.bind_port, t.target_host, t.target_port
        ),
        TunnelKind::Dynamic => format!("SOCKS5 em {}:{}", t.bind_host, t.bind_port),
    }
}

#[component]
fn Row(
    status: TunnelStatus,
    list: List,
    editing: RwSignal<Option<Edit<Tunnel>>>,
    start: Callback<String>,
    notice: RwSignal<Option<String>>,
) -> impl IntoView {
    let data = use_context::<Data>();
    let tunnel = status.tunnel.clone();
    let host = data
        .and_then(|d| {
            d.get_untracked()
                .hosts
                .into_iter()
                .find(|h| h.id == tunnel.host_id)
        })
        .map_or_else(|| "host apagado".to_owned(), |h| h.name);
    let (dot, label) = match status.state {
        TunnelState::Running => ("bg-success", "Aberto"),
        TunnelState::Starting => ("bg-warning", "Abrindo…"),
        TunnelState::Reconnecting => ("bg-warning", "Reconectando…"),
        TunnelState::Failed => ("bg-danger", "Falhou"),
        TunnelState::Stopped => ("bg-subtle", "Fechado"),
    };
    let open = matches!(
        status.state,
        TunnelState::Running | TunnelState::Starting | TunnelState::Reconnecting
    );
    let id = StoredValue::new(tunnel.id.clone());
    let stored = StoredValue::new(tunnel.clone());
    let stop = move |_| {
        spawn_local(async move {
            if let Err(message) =
                ui::call(METHOD_TUNNEL_STOP, &json!({ "id": id.get_value() })).await
            {
                notice.set(Some(message));
            }
            reload(list);
        });
    };

    view! {
        <li class="flex items-center gap-3 px-4 py-3 text-sm">
            <span class=format!("size-2.5 shrink-0 rounded-full {dot}") aria-hidden="true"></span>
            <div class="min-w-0 flex-1">
                <div class="flex items-baseline gap-2">
                    <span class="truncate font-medium">{tunnel.name.clone()}</span>
                    <span class="rounded bg-input px-1.5 font-mono text-xs text-muted">{kind_label(tunnel.kind)}</span>
                    <span class="text-xs text-muted">{label}</span>
                    {(status.active > 0).then(|| view! {
                        <span class="text-xs text-subtle">
                            {format!("{} conexão(ões)", status.active)}
                        </span>
                    })}
                    {tunnel.auto_start.then(|| view! { <span class="text-xs text-subtle">"· sobe com o app"</span> })}
                </div>
                <p class="truncate font-mono text-xs text-muted">{format!("{} · {host}", route(&tunnel))}</p>
                {status.error.map(|e| view! { <p class="truncate text-xs text-danger" title=e.clone()>{e.clone()}</p> })}
            </div>
            {if open {
                view! {
                    <button type="button" class=BUTTON on:click=stop>
                        <Svg icon=Icon::Square size=14 />
                        "Fechar"
                    </button>
                }
                .into_any()
            } else {
                view! {
                    <button type="button" class=PRIMARY on:click=move |_| start.run(id.get_value())>
                        <Svg icon=Icon::Play size=14 />
                        "Abrir"
                    </button>
                }
                .into_any()
            }}
            <button
                type="button"
                class=GHOST
                aria-label="Editar túnel"
                title="Editar"
                on:click=move |_| editing.set(Some(Edit::Existing(stored.get_value())))
            >
                <Svg icon=Icon::Pencil size=16 />
            </button>
        </li>
    }
}

#[component]
fn Editor(edit: Edit<Tunnel>, list: List, close: Callback<()>) -> impl IntoView {
    let data = use_context::<Data>();
    let hosts = data.map(|d| d.get_untracked().hosts).unwrap_or_default();
    let existing = edit.clone().existing();
    let is_new = existing.is_none();
    let draft = RwSignal::new(existing.clone().unwrap_or_else(|| Tunnel {
        id: new_id(),
        name: String::new(),
        host_id: hosts.first().map(|h| h.id.clone()).unwrap_or_default(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".into(),
        bind_port: 0,
        target_host: "localhost".into(),
        target_port: 0,
        auto_start: false,
    }));
    let port = |value: u16| {
        if value == 0 {
            String::new()
        } else {
            value.to_string()
        }
    };
    let bind_port = RwSignal::new(port(draft.get_untracked().bind_port));
    let target_port = RwSignal::new(port(draft.get_untracked().target_port));
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let dynamic = move || draft.with(|d| d.kind == TunnelKind::Dynamic);
    let exposed = move || {
        draft.with(|d| {
            d.kind != TunnelKind::Remote
                && !matches!(d.bind_host.trim(), "127.0.0.1" | "localhost" | "::1")
        })
    };

    let save = move || {
        let mut tunnel = draft.get_untracked();
        let name = std::mem::take(&mut tunnel.name);
        name.trim().clone_into(&mut tunnel.name);
        let parse = |s: String| s.trim().parse::<u16>().ok().filter(|p| *p > 0);
        let Some(bp) = parse(bind_port.get_untracked()) else {
            error.set(Some("Porta de escuta entre 1 e 65535.".into()));
            return;
        };
        tunnel.bind_port = bp;
        if tunnel.kind == TunnelKind::Dynamic {
            tunnel.target_port = 0;
        } else {
            let Some(tp) = parse(target_port.get_untracked()) else {
                error.set(Some("Porta de destino entre 1 e 65535.".into()));
                return;
            };
            tunnel.target_port = tp;
        }
        busy.set(true);
        spawn_local(async move {
            let result = ui::call(METHOD_TUNNEL_SAVE, &tunnel).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    reload(list);
                    close.run(());
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let remove = move |_| {
        let id = draft.get_untracked().id;
        spawn_local(async move {
            match ui::call(METHOD_TUNNEL_DELETE, &json!({ "id": id })).await {
                Ok(_) => {
                    reload(list);
                    close.run(());
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let kind_button = move |kind: TunnelKind, label: &'static str| {
        view! {
            <button
                type="button"
                role="radio"
                aria-checked=move || draft.with(|d| d.kind == kind).to_string()
                class="flex-1 rounded-md border px-3 py-1.5 text-sm"
                class=(["border-primary", "bg-primary-soft", "text-fg"], move || draft.with(|d| d.kind == kind))
                class=(["border-line", "text-muted", "hover:bg-card-hover"], move || draft.with(|d| d.kind != kind))
                on:click=move |_| draft.update(|d| d.kind = kind)
            >
                {label}
            </button>
        }
    };
    let explain = move || match draft.with(|d| d.kind) {
        TunnelKind::Local => {
            "Uma porta deste PC leva a um endereço visto do servidor, como um banco que só escuta lá dentro."
        }
        TunnelKind::Remote => {
            "Uma porta do servidor leva a um endereço visto deste PC. O sshd precisa permitir (AllowTcpForwarding)."
        }
        TunnelKind::Dynamic => "Um proxy SOCKS5 neste PC: o que passar por ele sai pelo servidor.",
    };

    view! {
        <Dialog title=if is_new { "Novo túnel".to_owned() } else { "Editar túnel".to_owned() } on_close=close>
            <form
                class="space-y-4 overflow-y-auto p-5 text-sm"
                on:submit=move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    save();
                }
            >
                <label class="block space-y-1">
                    <span class=LABEL>"Nome"</span>
                    <input
                        type="text"
                        class=FIELD
                        prop:value=move || draft.with(|d| d.name.clone())
                        on:input=move |e| draft.update(|d| d.name = event_target_value(&e))
                    />
                </label>
                <label class="block space-y-1">
                    <span class=LABEL>"Host"</span>
                    <select
                        class=FIELD
                        prop:value=move || draft.with(|d| d.host_id.clone())
                        on:change=move |e| draft.update(|d| d.host_id = event_target_value(&e))
                    >
                        {hosts
                            .iter()
                            .map(|h| view! { <option value=h.id.clone()>{h.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </label>
                <div class="space-y-1">
                    <span class=LABEL>"Tipo"</span>
                    <div role="radiogroup" aria-label="Tipo" class="flex gap-2">
                        {kind_button(TunnelKind::Local, "Local (-L)")}
                        {kind_button(TunnelKind::Remote, "Remoto (-R)")}
                        {kind_button(TunnelKind::Dynamic, "Dinâmico (-D)")}
                    </div>
                    <p class="text-xs text-subtle">{explain}</p>
                </div>
                <div class="grid grid-cols-[minmax(0,1fr)_7rem] gap-2">
                    <label class="block space-y-1">
                        <span class=LABEL>
                            {move || if draft.with(|d| d.kind == TunnelKind::Remote) { "Escutar no servidor em" } else { "Escutar neste PC em" }}
                        </span>
                        <input
                            type="text"
                            class=format!("{FIELD} font-mono")
                            prop:value=move || draft.with(|d| d.bind_host.clone())
                            on:input=move |e| draft.update(|d| d.bind_host = event_target_value(&e))
                        />
                    </label>
                    <label class="block space-y-1">
                        <span class=LABEL>"Porta"</span>
                        <input type="text" inputmode="numeric" class=format!("{FIELD} font-mono") bind:value=bind_port />
                    </label>
                </div>
                {move || exposed().then(|| view! {
                    <p class="text-xs text-warning">"Fora de 127.0.0.1, outros aparelhos da rede poderão usar este túnel."</p>
                })}
                <Show when=move || !dynamic()>
                    <div class="grid grid-cols-[minmax(0,1fr)_7rem] gap-2">
                        <label class="block space-y-1">
                            <span class=LABEL>
                                {move || if draft.with(|d| d.kind == TunnelKind::Remote) { "Destino visto deste PC" } else { "Destino visto do servidor" }}
                            </span>
                            <input
                                type="text"
                                class=format!("{FIELD} font-mono")
                                prop:value=move || draft.with(|d| d.target_host.clone())
                                on:input=move |e| draft.update(|d| d.target_host = event_target_value(&e))
                            />
                        </label>
                        <label class="block space-y-1">
                            <span class=LABEL>"Porta"</span>
                            <input type="text" inputmode="numeric" class=format!("{FIELD} font-mono") bind:value=target_port />
                        </label>
                    </div>
                </Show>
                <label class="flex items-center gap-2">
                    <input
                        type="checkbox"
                        prop:checked=move || draft.with(|d| d.auto_start)
                        on:change=move |e| draft.update(|d| d.auto_start = event_target_checked(&e))
                    />
                    "Abrir sozinho quando o CodatOS iniciar"
                </label>
                <FormError error=error />
                <div class="flex items-center gap-2">
                    {(!is_new).then(|| view! {
                        <button type="button" class=DANGER on:click=remove>
                            <Svg icon=Icon::Trash size=14 />
                            "Apagar"
                        </button>
                    })}
                    <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| close.run(())>"Cancelar"</button>
                    <button
                        type="submit"
                        class=PRIMARY
                        disabled=move || busy.get() || draft.with(|d| d.name.trim().is_empty() || d.host_id.is_empty())
                    >
                        "Salvar"
                    </button>
                </div>
            </form>
        </Dialog>
    }
}
