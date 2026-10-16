//! Servidor HTTP/WebSocket da API local.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use codatos_protocol::api::{
    API_VERSION, Health, SESSION_COOKIE, SessionRequest, WS_PROTOCOL, WS_TOKEN_PROTOCOL_PREFIX,
};
use codatos_protocol::rpc::Problem;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::auth::Auth;
use crate::guard::{self, Allowed};
use crate::rpc;

/// Porta padrão da API local (`docs/06-api-e-protocolos.md`).
pub const DEFAULT_PORT: u16 = 47800;

/// Tamanho máximo de uma mensagem no WebSocket (1 MiB).
const MAX_WS_MESSAGE: usize = 1 << 20;

/// CSP da UI servida ao navegador: nada de CDN, só o próprio núcleo (Segurança §8).
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
    style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; \
    frame-ancestors 'none'; base-uri 'none'; form-action 'self'";

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Porta em `127.0.0.1`; `0` escolhe uma livre (testes).
    pub port: u16,
    /// Pasta do bundle da UI (`apps/ui/dist`) servida em `/` para o navegador.
    pub ui_dir: Option<PathBuf>,
    /// Origens a mais aceitas, como a do `trunk serve` no modo dev.
    pub extra_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            ui_dir: None,
            extra_origins: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("não foi possível abrir a porta {port} em 127.0.0.1")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
}

/// API local em execução. Soltar sem [`RunningServer::shutdown`] deixa o servidor rodando
/// até o runtime Tokio acabar.
#[derive(Debug)]
pub struct RunningServer {
    addr: SocketAddr,
    auth: Arc<Auth>,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Token da janela do app, entregue à UI pelo IPC do Tauri; nunca vai em URL.
    pub fn app_token(&self) -> &str {
        self.auth.app_token()
    }

    /// Token de uso único (60 s) para "Abrir no navegador": a página troca por um cookie
    /// em `POST /api/v1/session`.
    pub fn issue_browser_token(&self) -> String {
        self.auth.issue_one_time()
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
        let _ = self.task.await;
    }
}

/// Sobe a API local em `127.0.0.1:{port}`.
pub async fn start(config: ServerConfig) -> Result<RunningServer, Error> {
    let bind_error = |source| Error::Bind {
        port: config.port,
        source,
    };
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port))
        .await
        .map_err(bind_error)?;
    let addr = listener.local_addr().map_err(bind_error)?;

    let auth = Arc::new(Auth::new());
    let allowed = Arc::new(Allowed::new(addr.port(), &config.extra_origins));
    let app = router(Arc::clone(&auth), allowed, config.ui_dir.as_deref());

    let (shutdown, stop) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let serve = axum::serve(listener, app).with_graceful_shutdown(async {
            let _ = stop.await;
        });
        if let Err(error) = serve.await {
            tracing::error!(%error, "a API local parou");
        }
    });
    tracing::info!(%addr, "API local no ar");

    Ok(RunningServer {
        addr,
        auth,
        shutdown,
        task,
    })
}

#[derive(Debug, Clone)]
struct AppState {
    auth: Arc<Auth>,
}

fn router(auth: Arc<Auth>, allowed: Arc<Allowed>, ui_dir: Option<&Path>) -> Router {
    let api = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/session", post(session))
        .route("/rpc", get(rpc_socket))
        .with_state(AppState { auth });

    let app = match ui_dir {
        Some(dir) => api
            .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")))),
        None => api,
    };

    app.layer(middleware::from_fn_with_state(
        allowed,
        guard::host_and_origin,
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    ))
}

async fn health() -> Json<Health> {
    Json(Health {
        name: "CodatOS".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        api: API_VERSION,
    })
}

async fn session(State(state): State<AppState>, Json(body): Json<SessionRequest>) -> Response {
    match state.auth.redeem(&body.token) {
        // Sem `Secure`: em 127.0.0.1 é HTTP puro. O modo LAN (TLS) acrescenta.
        Some(id) => (
            StatusCode::NO_CONTENT,
            [(
                header::SET_COOKIE,
                format!("{SESSION_COOKIE}={id}; Path=/; HttpOnly; SameSite=Strict"),
            )],
        )
            .into_response(),
        None => problem_response(
            StatusCode::UNAUTHORIZED,
            "token_invalid",
            "Token inválido ou expirado",
        ),
    }
}

async fn rpc_socket(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !authorized(&state.auth, &headers) {
        return problem_response(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Token ausente ou inválido",
        );
    }
    ws.protocols([WS_PROTOCOL])
        .max_message_size(MAX_WS_MESSAGE)
        .on_upgrade(serve_socket)
}

async fn serve_socket(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Text(text) => {
                let Some(response) = rpc::handle_text(text.as_str()) else {
                    continue;
                };
                let json = serde_json::to_string(&response).expect("Response sempre serializa");
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            // Frames binários (terminal/SFTP) chegam com os canais; ping/pong o axum responde.
            _ => {}
        }
    }
}

/// Janela do app: token por `Authorization: Bearer` ou subprotocolo `codatos.token.*`.
/// Navegador: cookie de sessão.
fn authorized(auth: &Auth, headers: &HeaderMap) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if bearer.is_some_and(|t| auth.is_app_token(t)) {
        return true;
    }

    let mut protocols = headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim);
    if protocols.any(|p| {
        p.strip_prefix(WS_TOKEN_PROTOCOL_PREFIX)
            .is_some_and(|t| auth.is_app_token(t))
    }) {
        return true;
    }

    let mut cookies = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='));
    cookies.any(|(name, value)| name == SESSION_COOKIE && auth.has_session(value))
}

pub(crate) fn problem_response(status: StatusCode, code: &str, title: &str) -> Response {
    let body = serde_json::to_string(&Problem::new(code, title)).expect("Problem sempre serializa");
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        body,
    )
        .into_response()
}
