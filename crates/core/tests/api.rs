//! Regras de segurança da API local (Segurança §8), contra um servidor de verdade em 127.0.0.1.

use std::fmt::Write as _;
use std::net::SocketAddr;

use codatos_core::{RunningServer, ServerConfig, start};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::{self, Message};

async fn server() -> RunningServer {
    start(ServerConfig {
        port: 0,
        ..ServerConfig::default()
    })
    .await
    .expect("servidor sobe")
}

struct HttpResponse {
    status: u16,
    head: String,
    body: String,
}

/// HTTP/1.1 cru, para controlar exatamente o `Host` e o `Origin` enviados.
async fn http(
    addr: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> HttpResponse {
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        let _ = write!(request, "{name}: {value}\r\n");
    }
    request.push_str("\r\n");
    request.push_str(body);

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).await.unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    HttpResponse {
        status,
        head: head.to_ascii_lowercase(),
        body: body.to_owned(),
    }
}

fn host(addr: SocketAddr) -> String {
    format!("127.0.0.1:{}", addr.port())
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn ws(addr: SocketAddr, headers: &[(&str, &str)]) -> Result<Socket, tungstenite::Error> {
    let mut request = format!("ws://127.0.0.1:{}/rpc", addr.port())
        .into_client_request()
        .unwrap();
    for (name, value) in headers {
        request.headers_mut().insert(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(socket, _)| socket)
}

fn status_of(result: Result<Socket, tungstenite::Error>) -> u16 {
    match result {
        Err(tungstenite::Error::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("erro inesperado: {other}"),
        Ok(_) => panic!("conexão deveria ter sido recusada"),
    }
}

async fn call(socket: &mut Socket, request: Value) -> Value {
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    loop {
        match socket.next().await.unwrap().unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("mensagem inesperada: {other:?}"),
        }
    }
}

fn token_protocol(server: &RunningServer) -> String {
    format!("codatos.v1, codatos.token.{}", server.app_token())
}

#[tokio::test]
async fn health_responde_e_traz_cabecalhos_de_seguranca() {
    let s = server().await;
    let r = http(
        s.local_addr(),
        "GET",
        "/api/v1/health",
        &[("Host", &host(s.local_addr()))],
        "",
    )
    .await;
    assert_eq!(r.status, 200);
    let body: Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(body["name"], json!("CodatOS"));
    assert_eq!(body["api"], json!(1));
    assert!(
        r.head
            .contains("content-security-policy: default-src 'self'")
    );
    assert!(r.head.contains("x-content-type-options: nosniff"));
    assert!(!r.head.contains("access-control-allow-origin"), "sem CORS");
    s.shutdown().await;
}

#[tokio::test]
async fn recusa_host_de_dns_rebinding() {
    let s = server().await;
    let port = s.local_addr().port();
    let r = http(
        s.local_addr(),
        "GET",
        "/api/v1/health",
        &[("Host", &format!("evil.example:{port}"))],
        "",
    )
    .await;
    assert_eq!(r.status, 403);
    assert!(r.body.contains("host_invalid"));
    s.shutdown().await;
}

#[tokio::test]
async fn recusa_origin_de_site_estranho() {
    let s = server().await;
    let r = http(
        s.local_addr(),
        "GET",
        "/api/v1/health",
        &[
            ("Host", &host(s.local_addr())),
            ("Origin", "https://evil.example"),
        ],
        "",
    )
    .await;
    assert_eq!(r.status, 403);
    assert!(r.body.contains("origin_invalid"));
    s.shutdown().await;
}

#[tokio::test]
async fn websocket_sem_token_e_recusado() {
    let s = server().await;
    assert_eq!(
        status_of(ws(s.local_addr(), &[("Origin", "tauri://localhost")]).await),
        401
    );
    let errado = format!("codatos.v1, codatos.token.{}", "0".repeat(64));
    assert_eq!(
        status_of(ws(s.local_addr(), &[("Sec-WebSocket-Protocol", &errado)]).await),
        401
    );
    s.shutdown().await;
}

#[tokio::test]
async fn websocket_de_site_estranho_e_recusado_mesmo_com_token() {
    let s = server().await;
    let protocol = token_protocol(&s);
    let result = ws(
        s.local_addr(),
        &[
            ("Origin", "https://evil.example"),
            ("Sec-WebSocket-Protocol", &protocol),
        ],
    )
    .await;
    assert_eq!(status_of(result), 403);
    s.shutdown().await;
}

#[tokio::test]
async fn janela_do_tauri_conecta_com_o_token_do_app() {
    let s = server().await;
    let protocol = token_protocol(&s);
    for origin in [
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
    ] {
        let mut socket = ws(
            s.local_addr(),
            &[("Origin", origin), ("Sec-WebSocket-Protocol", &protocol)],
        )
        .await
        .unwrap_or_else(|e| panic!("{origin}: {e}"));
        let r = call(
            &mut socket,
            json!({"jsonrpc": "2.0", "id": 1, "method": "system.ping"}),
        )
        .await;
        assert_eq!(
            r,
            json!({"jsonrpc": "2.0", "id": 1, "result": "pong"}),
            "{origin}"
        );
    }
    s.shutdown().await;
}

#[tokio::test]
async fn rpc_responde_erros_padrao() {
    let s = server().await;
    let protocol = token_protocol(&s);
    let mut socket = ws(s.local_addr(), &[("Sec-WebSocket-Protocol", &protocol)])
        .await
        .unwrap();

    let r = call(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 2, "method": "nao.existe"}),
    )
    .await;
    assert_eq!(r["error"]["code"], json!(-32601));
    assert_eq!(r["error"]["data"]["code"], json!("method_not_found"));

    socket.send(Message::Text("{".into())).await.unwrap();
    let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
        panic!("esperava texto");
    };
    let r: Value = serde_json::from_str(text.as_str()).unwrap();
    assert_eq!(r["error"]["code"], json!(-32700));
    assert_eq!(r["id"], Value::Null);

    let r = call(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": "h", "method": "session.hello"}),
    )
    .await;
    assert_eq!(r["result"]["api"], json!(1));
    s.shutdown().await;
}

#[tokio::test]
async fn navegador_troca_token_de_uso_unico_por_cookie() {
    let s = server().await;
    let addr = s.local_addr();
    let origin = format!("http://{}", host(addr));
    let token = s.issue_browser_token();
    let body = json!({ "token": token }).to_string();
    let headers = [
        ("Host", host(addr)),
        ("Origin", origin.clone()),
        ("Content-Type", "application/json".to_owned()),
    ];
    let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let r = http(addr, "POST", "/api/v1/session", &headers, &body).await;
    assert_eq!(r.status, 204);
    let cookie_line = r
        .head
        .lines()
        .find(|l| l.starts_with("set-cookie:"))
        .expect("set-cookie");
    assert!(cookie_line.contains("httponly"));
    assert!(cookie_line.contains("samesite=strict"));
    let cookie = cookie_line
        .trim_start_matches("set-cookie:")
        .trim()
        .split(';')
        .next()
        .unwrap()
        .to_owned();

    // O mesmo token não vale de novo.
    let again = http(addr, "POST", "/api/v1/session", &headers, &body).await;
    assert_eq!(again.status, 401);

    let mut socket = ws(addr, &[("Origin", &origin), ("Cookie", &cookie)])
        .await
        .unwrap();
    let r = call(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 1, "method": "system.ping"}),
    )
    .await;
    assert_eq!(r["result"], json!("pong"));

    assert_eq!(
        status_of(ws(addr, &[("Cookie", "codatos_session=forjado")]).await),
        401
    );
    s.shutdown().await;
}

#[tokio::test]
async fn porta_ocupada_vira_erro() {
    let s = server().await;
    let err = start(ServerConfig {
        port: s.local_addr().port(),
        ..ServerConfig::default()
    })
    .await
    .unwrap_err();
    assert!(err.to_string().contains(&s.local_addr().port().to_string()));
    s.shutdown().await;
}

#[tokio::test]
async fn serve_a_ui_com_fallback_para_index() {
    let dir = std::env::temp_dir().join(format!("codatos-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<h1>CodatOS</h1>").unwrap();
    let s = start(ServerConfig {
        port: 0,
        ui_dir: Some(dir.clone()),
        ..ServerConfig::default()
    })
    .await
    .unwrap();
    let h = host(s.local_addr());
    for path in ["/", "/financeiro"] {
        let r = http(s.local_addr(), "GET", path, &[("Host", &h)], "").await;
        assert_eq!(r.status, 200, "{path}");
        assert!(r.body.contains("CodatOS"), "{path}");
    }
    let r = http(s.local_addr(), "GET", "/", &[("Host", "evil.example")], "").await;
    assert_eq!(r.status, 403, "a UI também passa pela checagem de Host");
    s.shutdown().await;
    std::fs::remove_dir_all(dir).unwrap();
}
