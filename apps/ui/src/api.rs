//! Conexão da UI com o núcleo (`docs/06-api-e-protocolos.md`).
//!
//! - **Janela do app (Tauri):** pede o token ao shell pelo IPC `get_session_token` e
//!   abre `ws://127.0.0.1:47800/rpc` mandando o token no subprotocolo (ADR-0003).
//! - **Navegador:** a UI vem do próprio núcleo; um `?token=` de uso único na URL é
//!   trocado por cookie em `POST /api/v1/session` e o WebSocket vai na mesma origem.
//!
//! Se a conexão cai, tenta de novo a cada 3 s.

use std::cell::RefCell;
use std::time::Duration;

use codatos_protocol::api::{
    DEFAULT_PORT, IPC_GET_SESSION_TOKEN, WS_PROTOCOL, WS_TOKEN_PROTOCOL_PREFIX,
};
use codatos_protocol::rpc::{Id, Outcome, Request, Response};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{MessageEvent, RequestCredentials, RequestInit, WebSocket};

const RETRY: Duration = Duration::from_secs(3);
const PING_ID: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreStatus {
    Connecting,
    Connected,
    Offline,
}

/// O WebSocket atual e os callbacks dele. Trocar a conexão derruba os callbacks
/// antigos fora deles mesmos (no timer de nova tentativa), sem vazar closures.
struct Conn {
    ws: WebSocket,
    _on_open: Closure<dyn FnMut()>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_close: Closure<dyn FnMut()>,
}

thread_local! {
    static CONN: RefCell<Option<Conn>> = const { RefCell::new(None) };
}

/// Conecta ao núcleo e mantém `status` atualizado.
pub fn connect(status: WriteSignal<CoreStatus>) {
    spawn_local(async move {
        if let Err(err) = open(status).await {
            leptos::logging::warn!("núcleo indisponível: {err:?}");
            offline(status);
        }
    });
}

async fn open(status: WriteSignal<CoreStatus>) -> Result<(), JsValue> {
    status.set(CoreStatus::Connecting);
    let protocols = js_sys::Array::of1(&WS_PROTOCOL.into());

    let url = if let Some(invoke) = tauri_invoke() {
        let token = JsFuture::from(
            invoke
                .call1(&JsValue::NULL, &IPC_GET_SESSION_TOKEN.into())?
                .dyn_into::<js_sys::Promise>()?,
        )
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
        let Some(text) = event.data().as_string() else {
            return;
        };
        if let Ok(resp) = serde_json::from_str::<Response>(&text)
            && resp.id == Some(Id::Number(PING_ID))
            && matches!(resp.outcome, Outcome::Result(_))
        {
            status.set(CoreStatus::Connected);
        }
    });
    // `close` vem depois de `error` também, então basta ele para tentar de novo.
    let on_close = Closure::<dyn FnMut()>::new(move || offline(status));

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

fn offline(status: WriteSignal<CoreStatus>) {
    status.set(CoreStatus::Offline);
    set_timeout(
        move || {
            replace(None);
            connect(status);
        },
        RETRY,
    );
}

fn replace(conn: Option<Conn>) {
    let old = CONN.with(|c| c.replace(conn));
    if let Some(old) = old {
        old.ws.set_onclose(None);
        let _ = old.ws.close();
    }
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
