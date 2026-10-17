//! Conexão da UI com o núcleo (`docs/06-api-e-protocolos.md`).
//!
//! - **Janela do app (Tauri):** pede o token ao shell pelo IPC `get_session_token` e
//!   abre `ws://127.0.0.1:47800/rpc` mandando o token no subprotocolo (ADR-0003).
//! - **Navegador:** a UI vem do próprio núcleo; um `?token=` de uso único na URL é
//!   trocado por cookie em `POST /api/v1/session` e o WebSocket vai na mesma origem.
//!
//! Se a conexão cai, tenta de novo a cada 3 s e refaz as assinaturas. Com a janela ou a aba
//! escondida, as assinaturas são canceladas no núcleo e voltam quando ela reaparece
//! (RF-DASH-007): nada é coletado para ninguém ver.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use codatos_protocol::api::{
    DEFAULT_PORT, EVENT_METHOD, EventParams, IPC_GET_SESSION_TOKEN, Subscribed, WS_PROTOCOL,
    WS_TOKEN_PROTOCOL_PREFIX,
};
use codatos_protocol::rpc::{Id, Outcome, Problem, Request, Response, RpcError};
use futures_channel::oneshot;
use leptos::prelude::*;
use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{MessageEvent, RequestCredentials, RequestInit, WebSocket};

const RETRY: Duration = Duration::from_secs(3);
const PING_ID: i64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreStatus {
    Connecting,
    Connected,
    Offline,
}

/// Última amostra de um tópico: os dados ou o erro do coletor (só daquele widget).
pub type Sample = Result<Value, Problem>;

/// O WebSocket atual e os callbacks dele. Trocar a conexão derruba os callbacks
/// antigos fora deles mesmos (no timer de nova tentativa), sem vazar closures.
struct Conn {
    ws: WebSocket,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut()>,
}

/// Assinatura feita por um componente. Sobrevive a quedas da conexão.
struct LocalSub {
    topic: &'static str,
    interval_ms: Option<u64>,
    set: WriteSignal<Option<Sample>>,
    /// Id no núcleo, enquanto estiver ativa lá.
    remote: Option<String>,
}

#[derive(Default)]
struct State {
    conn: Option<Conn>,
    connected: bool,
    next_id: i64,
    pending: HashMap<i64, Pending>,
    subs: HashMap<u64, LocalSub>,
    next_sub: u64,
}

enum Pending {
    Call(oneshot::Sender<Result<Value, RpcError>>),
    /// Resposta de um `subscribe` da assinatura local com essa chave.
    Subscribe(u64),
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static STATUS: RefCell<Option<WriteSignal<CoreStatus>>> = const { RefCell::new(None) };
}

/// Conecta ao núcleo e mantém `status` atualizado.
pub fn connect(status: WriteSignal<CoreStatus>) {
    STATUS.with(|s| *s.borrow_mut() = Some(status));
    watch_visibility();
    reconnect();
}

fn reconnect() {
    spawn_local(async move {
        if let Err(err) = open().await {
            leptos::logging::warn!("núcleo indisponível: {err:?}");
            offline();
        }
    });
}

fn set_status(value: CoreStatus) {
    STATUS.with(|s| {
        if let Some(set) = *s.borrow() {
            set.set(value);
        }
    });
}

async fn open() -> Result<(), JsValue> {
    set_status(CoreStatus::Connecting);
    let protocols = js_sys::Array::of1(&WS_PROTOCOL.into());

    let url = if in_tauri() {
        let token = invoke(IPC_GET_SESSION_TOKEN, &JsValue::UNDEFINED)
            .await?
            .as_string()
            .ok_or("token inválido")?;
        protocols.push(&format!("{WS_TOKEN_PROTOCOL_PREFIX}{token}").into());
        format!("ws://127.0.0.1:{DEFAULT_PORT}/rpc")
    } else {
        redeem_url_token().await?;
        let location = window().location();
        let scheme = if location.protocol()? == "https:" {
            "wss"
        } else {
            "ws"
        };
        format!("{scheme}://{}/rpc", location.host()?)
    };

    let ws = WebSocket::new_with_str_sequence(&url, &protocols)?;

    let on_open = {
        let ws = ws.clone();
        Closure::<dyn FnMut()>::new(move || {
            let ping = Request::new(Some(Id::Number(PING_ID)), "system.ping", None);
            if let Ok(text) = serde_json::to_string(&ping) {
                let _ = ws.send_with_str(&text);
            }
        })
    };
    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        if let Some(text) = event.data().as_string() {
            on_text(&text);
        }
    });
    // `close` vem depois de `error` também, então basta ele para tentar de novo.
    let on_close = Closure::<dyn FnMut()>::new(offline);

    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

    replace(Some(Conn {
        ws,
        _on_open: on_open,
        _on_message: on_message,
        _on_close: on_close,
    }));
    Ok(())
}

fn on_text(text: &str) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    if value.get("method").and_then(Value::as_str) == Some(EVENT_METHOD) {
        if let Ok(event) = serde_json::from_value::<EventParams>(value["params"].clone()) {
            on_event(event);
        }
        return;
    }
    let Ok(response) = serde_json::from_value::<Response>(value) else {
        return;
    };
    let Some(Id::Number(id)) = response.id else {
        return;
    };
    if id == PING_ID {
        if matches!(response.outcome, Outcome::Result(_)) {
            STATE.with(|s| s.borrow_mut().connected = true);
            set_status(CoreStatus::Connected);
            resubscribe_all();
        }
        return;
    }
    let result = match response.outcome {
        Outcome::Result(v) => Ok(v),
        Outcome::Error(e) => Err(e),
    };
    let pending = STATE.with(|s| s.borrow_mut().pending.remove(&id));
    match pending {
        Some(Pending::Call(tx)) => {
            let _ = tx.send(result);
        }
        Some(Pending::Subscribe(key)) => on_subscribed(key, result),
        None => {}
    }
}

fn on_event(event: EventParams) {
    let set = STATE.with(|s| {
        s.borrow()
            .subs
            .values()
            .find(|sub| sub.remote.as_deref() == Some(event.sub.as_str()))
            .map(|sub| sub.set)
    });
    let Some(set) = set else { return };
    let sample = match (event.data, event.error) {
        (_, Some(problem)) => Err(problem),
        (data, None) => Ok(data.unwrap_or(Value::Null)),
    };
    set.try_set(Some(sample));
}

fn on_subscribed(key: u64, result: Result<Value, RpcError>) {
    let subscribed = result.map(serde_json::from_value::<Subscribed>);
    match subscribed {
        Ok(Ok(sub)) => {
            // O componente pode ter saído enquanto a resposta vinha: cancela no núcleo.
            let gone = STATE.with(|s| match s.borrow_mut().subs.get_mut(&key) {
                Some(local) => {
                    local.remote = Some(sub.sub.clone());
                    false
                }
                None => true,
            });
            if gone {
                notify("unsubscribe", json!({ "sub": sub.sub }));
            }
        }
        Ok(Err(_)) => {}
        Err(error) => {
            // Tópico inexistente nesta máquina (ex.: WSL fora do Windows) vira erro no widget.
            let set = STATE.with(|s| s.borrow().subs.get(&key).map(|l| l.set));
            if let Some(set) = set {
                let problem = error
                    .data
                    .map_or_else(|| Problem::new("rpc_error", error.message), |d| *d);
                set.try_set(Some(Err(problem)));
            }
        }
    }
}

fn offline() {
    set_status(CoreStatus::Offline);
    let pending = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.connected = false;
        for sub in s.subs.values_mut() {
            sub.remote = None;
        }
        std::mem::take(&mut s.pending)
    });
    for (_, p) in pending {
        if let Pending::Call(tx) = p {
            let _ = tx.send(Err(offline_error()));
        }
    }
    set_timeout(
        move || {
            replace(None);
            reconnect();
        },
        RETRY,
    );
}

fn offline_error() -> RpcError {
    RpcError::new(-32000, "core_offline", "Núcleo offline")
}

fn replace(conn: Option<Conn>) {
    let old = STATE.with(|s| std::mem::replace(&mut s.borrow_mut().conn, conn));
    if let Some(old) = old {
        old.ws.set_onclose(None);
        let _ = old.ws.close();
    }
}

/// Manda um pedido; `None` se não houver conexão aberta.
fn send(id: Option<i64>, method: &str, params: Value) -> bool {
    let request = Request::new(id.map(Id::Number), method, Some(params));
    let Ok(text) = serde_json::to_string(&request) else {
        return false;
    };
    STATE.with(|s| {
        s.borrow()
            .conn
            .as_ref()
            .is_some_and(|c| c.ws.send_with_str(&text).is_ok())
    })
}

fn notify(method: &str, params: Value) {
    send(None, method, params);
}

fn next_id() -> i64 {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.next_id += 1;
        s.next_id
    })
}

/// Chama um método do núcleo.
pub async fn call(method: &str, params: Value) -> Result<Value, RpcError> {
    if !STATE.with(|s| s.borrow().connected) {
        return Err(offline_error());
    }
    let id = next_id();
    let (tx, rx) = oneshot::channel();
    STATE.with(|s| s.borrow_mut().pending.insert(id, Pending::Call(tx)));
    if !send(Some(id), method, params) {
        STATE.with(|s| s.borrow_mut().pending.remove(&id));
        return Err(offline_error());
    }
    rx.await.unwrap_or_else(|_| Err(offline_error()))
}

/// Assina um tópico enquanto o componente atual existir. O sinal começa em `None`
/// (carregando) e recebe cada amostra.
pub fn subscribe(topic: &'static str, interval_ms: Option<u64>) -> ReadSignal<Option<Sample>> {
    let (get, set) = signal(None);
    let key = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.next_sub += 1;
        let key = s.next_sub;
        s.subs.insert(
            key,
            LocalSub {
                topic,
                interval_ms,
                set,
                remote: None,
            },
        );
        key
    });
    if STATE.with(|s| s.borrow().connected) && visible() {
        subscribe_remote(key);
    }
    on_cleanup(move || {
        let removed = STATE.with(|s| s.borrow_mut().subs.remove(&key));
        if let Some(sub) = removed.and_then(|l| l.remote) {
            notify("unsubscribe", json!({ "sub": sub }));
        }
    });
    get
}

fn subscribe_remote(key: u64) {
    let Some((topic, interval_ms)) =
        STATE.with(|s| s.borrow().subs.get(&key).map(|l| (l.topic, l.interval_ms)))
    else {
        return;
    };
    let id = next_id();
    STATE.with(|s| s.borrow_mut().pending.insert(id, Pending::Subscribe(key)));
    let mut params = json!({ "topic": topic });
    if let Some(ms) = interval_ms {
        params["intervalMs"] = json!(ms);
    }
    if !send(Some(id), "subscribe", params) {
        STATE.with(|s| s.borrow_mut().pending.remove(&id));
    }
}

fn resubscribe_all() {
    if !visible() {
        return;
    }
    let keys: Vec<u64> = STATE.with(|s| {
        s.borrow()
            .subs
            .iter()
            .filter(|(_, l)| l.remote.is_none())
            .map(|(k, _)| *k)
            .collect()
    });
    for key in keys {
        subscribe_remote(key);
    }
}

fn unsubscribe_all_remote() {
    let remotes: Vec<String> = STATE.with(|s| {
        s.borrow_mut()
            .subs
            .values_mut()
            .filter_map(|l| l.remote.take())
            .collect()
    });
    for sub in remotes {
        notify("unsubscribe", json!({ "sub": sub }));
    }
}

fn visible() -> bool {
    window()
        .document()
        .is_none_or(|d| d.visibility_state() != web_sys::VisibilityState::Hidden)
}

/// Janela minimizada ou aba em segundo plano: cancela as assinaturas no núcleo.
fn watch_visibility() {
    let Some(document) = window().document() else {
        return;
    };
    let on_change = Closure::<dyn FnMut()>::new(|| {
        if visible() {
            if STATE.with(|s| s.borrow().connected) {
                resubscribe_all();
            }
        } else {
            unsubscribe_all_remote();
        }
    });
    let _ = document
        .add_event_listener_with_callback("visibilitychange", on_change.as_ref().unchecked_ref());
    // Vive enquanto a página existir.
    on_change.forget();
}

/// A UI está dentro da janela do app (e não num navegador)?
pub fn in_tauri() -> bool {
    tauri_invoke().is_some()
}

/// Chama um comando IPC do Tauri. Fora da janela do app, falha.
pub async fn invoke(cmd: &str, args: &JsValue) -> Result<JsValue, JsValue> {
    let invoke = tauri_invoke().ok_or("fora do Tauri")?;
    let promise = invoke
        .call2(&JsValue::NULL, &cmd.into(), args)?
        .dyn_into::<js_sys::Promise>()?;
    JsFuture::from(promise).await
}

/// `window.__TAURI_INTERNALS__.invoke`, presente só dentro da janela do Tauri.
fn tauri_invoke() -> Option<js_sys::Function> {
    let internals = js_sys::Reflect::get(&window(), &"__TAURI_INTERNALS__".into()).ok()?;
    if internals.is_undefined() {
        return None;
    }
    js_sys::Reflect::get(&internals, &"invoke".into())
        .ok()?
        .dyn_into()
        .ok()
}

/// No navegador, troca `?token=` (uso único, 60 s) pelo cookie de sessão e tira o
/// token da URL e do histórico.
async fn redeem_url_token() -> Result<(), JsValue> {
    let location = window().location();
    let params = web_sys::UrlSearchParams::new_with_str(&location.search()?)?;
    let Some(token) = params.get("token") else {
        return Ok(());
    };

    let body = serde_json::json!({ "token": token }).to_string();
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_credentials(RequestCredentials::SameOrigin);
    init.set_body(&body.into());
    let request = web_sys::Request::new_with_str_and_init("/api/v1/session", &init)?;
    request.headers().set("Content-Type", "application/json")?;
    let _ = JsFuture::from(window().fetch_with_request(&request)).await?;

    params.delete("token");
    let query = params.to_string().as_string().unwrap_or_default();
    let clean = if query.is_empty() {
        location.pathname()?
    } else {
        format!("{}?{query}", location.pathname()?)
    };
    window()
        .history()?
        .replace_state_with_url(&JsValue::NULL, "", Some(&clean))?;
    Ok(())
}
