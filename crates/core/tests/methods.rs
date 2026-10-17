//! Métodos registrados de fora do núcleo (`ServerConfig::methods`).

use std::sync::Arc;
use std::time::Duration;

use codatos_core::{Method, MethodError, MethodFuture, RunningServer, ServerConfig, params, start};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

#[derive(Debug)]
struct Echo;

impl Method for Echo {
    fn name(&self) -> &'static str {
        "test.echo"
    }
    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            #[derive(Deserialize)]
            struct P {
                text: String,
            }
            let p: P = params(value)?;
            if p.text.is_empty() {
                return Err(MethodError::new("empty_text", "Texto vazio"));
            }
            Ok(json!(p.text))
        })
    }
}

#[derive(Debug)]
struct Slow;

impl Method for Slow {
    fn name(&self) -> &'static str {
        "test.slow"
    }
    fn call(&self, _: Value) -> MethodFuture<'_> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Ok(json!("lento"))
        })
    }
}

async fn server() -> RunningServer {
    start(ServerConfig {
        port: 0,
        methods: vec![Arc::new(Echo), Arc::new(Slow)],
        ..ServerConfig::default()
    })
    .await
    .expect("servidor sobe")
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn connect(server: &RunningServer) -> Socket {
    let mut request = format!("ws://127.0.0.1:{}/rpc", server.local_addr().port())
        .into_client_request()
        .unwrap();
    let protocol = format!("codatos.v1, codatos.token.{}", server.app_token());
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str(&protocol).unwrap(),
    );
    tokio_tungstenite::connect_async(request).await.unwrap().0
}

async fn send(socket: &mut Socket, request: Value) {
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
}

async fn next(socket: &mut Socket) -> Value {
    let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("mensagem em até 5 s");
    match message.unwrap().unwrap() {
        Message::Text(text) => serde_json::from_str(text.as_str()).unwrap(),
        other => panic!("mensagem inesperada: {other:?}"),
    }
}

#[tokio::test]
async fn chama_metodo_e_mapeia_erros() {
    let s = server().await;
    let mut socket = connect(&s).await;

    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 1, "method": "test.echo", "params": {"text": "oi"}}),
    )
    .await;
    assert_eq!(
        next(&mut socket).await,
        json!({"jsonrpc": "2.0", "id": 1, "result": "oi"})
    );

    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 2, "method": "test.echo", "params": {"text": ""}}),
    )
    .await;
    let r = next(&mut socket).await;
    assert_eq!(r["error"]["code"], json!(-32000));
    assert_eq!(r["error"]["data"]["code"], json!("empty_text"));

    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 3, "method": "test.echo"}),
    )
    .await;
    let r = next(&mut socket).await;
    assert_eq!(r["error"]["code"], json!(-32602));
    assert_eq!(r["error"]["data"]["code"], json!("invalid_params"));

    let r = {
        send(
            &mut socket,
            json!({"jsonrpc": "2.0", "id": 4, "method": "session.hello"}),
        )
        .await;
        next(&mut socket).await
    };
    assert_eq!(
        r["result"]["capabilities"],
        json!(["test.echo", "test.slow"])
    );
    s.shutdown().await;
}

#[tokio::test]
async fn metodo_lento_nao_segura_os_outros() {
    let s = server().await;
    let mut socket = connect(&s).await;
    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 1, "method": "test.slow"}),
    )
    .await;
    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 2, "method": "system.ping"}),
    )
    .await;
    assert_eq!(next(&mut socket).await["id"], json!(2));
    assert_eq!(
        next(&mut socket).await,
        json!({"jsonrpc": "2.0", "id": 1, "result": "lento"})
    );
    s.shutdown().await;
}

#[tokio::test]
async fn notificacao_de_metodo_nao_tem_resposta() {
    let s = server().await;
    let mut socket = connect(&s).await;
    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "method": "test.echo", "params": {"text": "x"}}),
    )
    .await;
    send(
        &mut socket,
        json!({"jsonrpc": "2.0", "id": 9, "method": "system.ping"}),
    )
    .await;
    assert_eq!(next(&mut socket).await["id"], json!(9));
    s.shutdown().await;
}
