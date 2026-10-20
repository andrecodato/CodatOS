//! Servidor HTTP/WebSocket da API local.

use std::fmt::Write as _;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use codatos_protocol::api::{
    API_VERSION, Health, SESSION_COOKIE, SessionRequest, WS_PROTOCOL, WS_TOKEN_PROTOCOL_PREFIX,
};
use codatos_protocol::rpc::Problem;
use futures_util::{SinkExt as _, StreamExt as _};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::auth::Auth;
use crate::channel::{ChannelMethod, Outgoing};
use crate::collector::Collector;
use crate::guard::{self, Allowed};
use crate::hub::Hub;
use crate::method::Method;
use crate::rpc::Connection;

/// Porta padrão da API local (`docs/06-api-e-protocolos.md`).
pub const DEFAULT_PORT: u16 = codatos_protocol::api::DEFAULT_PORT;

/// Tamanho máximo de uma mensagem no WebSocket (1 MiB).
const MAX_WS_MESSAGE: usize = 1 << 20;

/// Mensagens esperando envio por conexão; eventos além disso são descartados.
const OUTBOX: usize = 64;

/// CSP da UI servida ao navegador: nada de CDN, só o próprio núcleo (Segurança §8).
/// `{scripts}` recebe os hashes dos scripts embutidos no `index.html` (ver [`csp`]).
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'{scripts}; \
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
    /// Coletores dos tópicos de `subscribe` (um por tópico).
    pub collectors: Vec<Arc<dyn Collector>>,
    /// Métodos JSON-RPC a mais (preferências, dashboards, ações).
    pub methods: Vec<Arc<dyn Method>>,
    /// Métodos que abrem canais binários (terminal, SFTP).
    pub channel_methods: Vec<Arc<dyn ChannelMethod>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            ui_dir: None,
            extra_origins: Vec::new(),
            collectors: Vec::new(),
            methods: Vec::new(),
            channel_methods: Vec::new(),
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
    let csp = csp(config.ui_dir.as_deref());
    let state = AppState {
        auth: Arc::clone(&auth),
        hub: Arc::new(Hub::new(
            &config.collectors,
            &config.methods,
            &config.channel_methods,
        )),
    };
    let app = router(state, allowed, config.ui_dir.as_deref(), &csp);

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
    hub: Arc<Hub>,
}

fn router(state: AppState, allowed: Arc<Allowed>, ui_dir: Option<&Path>, csp: &str) -> Router {
    let api = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/session", post(session))
        .route("/rpc", get(rpc_socket))
        .with_state(state);

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
        HeaderValue::from_str(csp).expect("CSP é ASCII"),
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

/// Monta a CSP liberando, por hash, só os scripts embutidos no `index.html` da UI.
/// O Trunk põe ali o `<script type="module">` que carrega o WASM; sem o hash, o
/// navegador bloqueia e a UI não sobe. Mesma ideia que o Tauri aplica na janela.
fn csp(ui_dir: Option<&Path>) -> String {
    let index = ui_dir.and_then(|dir| std::fs::read_to_string(dir.join("index.html")).ok());
    let mut scripts = String::new();
    for body in index.as_deref().map(inline_scripts).unwrap_or_default() {
        let hash = BASE64.encode(Sha256::digest(body));
        let _ = write!(scripts, " 'sha256-{hash}'");
    }
    CSP.replace("{scripts}", &scripts)
}

/// Conteúdo dos `<script>` sem `src` (o hash da CSP cobre o texto exato entre as tags).
fn inline_scripts(html: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        rest = &rest[start..];
        let Some(open_end) = rest.find('>') else {
            break;
        };
        let open_tag = &rest[..open_end];
        let body_start = open_end + 1;
        let Some(len) = rest[body_start..].find("</script>") else {
            break;
        };
        if !open_tag.contains("src=") {
            found.push(&rest[body_start..body_start + len]);
        }
        rest = &rest[body_start + len..];
    }
    found
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
        .on_upgrade(move |socket| serve_socket(socket, state.hub))
}

/// Lê pedidos e responde; respostas, eventos e frames dos canais saem pela mesma fila.
async fn serve_socket(socket: WebSocket, hub: Arc<Hub>) {
    let (mut sink, mut stream) = socket.split();
    let (out, mut outbox) = mpsc::channel::<Outgoing>(OUTBOX);
    let writer = tokio::spawn(async move {
        while let Some(message) = outbox.recv().await {
            let message = match message {
                Outgoing::Text(json) => Message::Text(json.into()),
                Outgoing::Binary(bytes) => Message::Binary(bytes.into()),
            };
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut connection = Connection::new(hub, out.clone());
    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Text(text) => {
                let Some(response) = connection.handle_text(text.as_str()) else {
                    continue;
                };
                let json = serde_json::to_string(&response).expect("Response sempre serializa");
                if out.send(Outgoing::Text(json)).await.is_err() {
                    break;
                }
            }
            Message::Binary(bytes) => connection.handle_binary(&bytes).await,
            Message::Close(_) => break,
            // Ping/pong o axum responde.
            _ => {}
        }
    }
    // Encerra assinaturas e canais antes do escritor, que acaba quando a fila fecha.
    drop(connection);
    drop(out);
    let _ = writer.await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_so_scripts_embutidos() {
        let html = r#"<script type="module">init()</script><script src="/a.js"></script>
            <script>x</script>"#;
        assert_eq!(inline_scripts(html), ["init()", "x"]);
    }

    #[test]
    fn sem_ui_a_csp_nao_libera_script_embutido() {
        assert!(!csp(None).contains("sha256-"));
        assert!(csp(None).contains("script-src 'self' 'wasm-unsafe-eval';"));
    }
}
