//! `terminal.open` de ponta a ponta: um `/bin/sh` num PTY atrás do WebSocket `/rpc`.
#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use codatos_connections::OpenTerminalMethod;
use codatos_core::{RunningServer, ServerConfig, start};
use codatos_protocol::channel::{Frame, decode, encode};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn server() -> RunningServer {
    start(ServerConfig {
        port: 0,
        channel_methods: vec![Arc::new(OpenTerminalMethod)],
        ..ServerConfig::default()
    })
    .await
    .expect("servidor sobe")
}

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

async fn open(socket: &mut Socket, params: Value) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "terminal.open", "params": params});
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    loop {
        if let Message::Text(text) = socket.next().await.unwrap().unwrap() {
            return serde_json::from_str(text.as_str()).unwrap();
        }
    }
}

async fn send(socket: &mut Socket, channel: u32, frame: Frame<'_>) {
    socket
        .send(Message::Binary(encode(channel, frame).into()))
        .await
        .unwrap();
}

/// Junta a saída até `want` aparecer ou o canal fechar. Devolve a saída e se fechou.
async fn read_until(socket: &mut Socket, channel: u32, want: &str) -> (String, bool) {
    let mut output = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let message = tokio::time::timeout_at(deadline, socket.next())
            .await
            .unwrap_or_else(|_| {
                panic!("sem {want:?} em 10 s: {}", String::from_utf8_lossy(&output))
            })
            .unwrap()
            .unwrap();
        let Message::Binary(bytes) = message else {
            continue;
        };
        let (id, frame) = decode(&bytes).unwrap();
        assert_eq!(id, channel);
        match frame {
            Frame::Data(data) => {
                output.extend_from_slice(data);
                let ack = u32::try_from(data.len()).unwrap();
                send(socket, channel, Frame::Ack(ack)).await;
                if String::from_utf8_lossy(&output).contains(want) {
                    return (String::from_utf8_lossy(&output).into_owned(), false);
                }
            }
            Frame::Close => return (String::from_utf8_lossy(&output).into_owned(), true),
            other => panic!("frame inesperado: {other:?}"),
        }
    }
}

#[tokio::test]
async fn shell_local_ecoa_redimensiona_e_fecha_ao_sair() {
    let s = server().await;
    let mut socket = connect(&s).await;
    let r = open(
        &mut socket,
        json!({"target": {"kind": "local"}, "cols": 100, "rows": 30}),
    )
    .await;
    let channel = u32::try_from(r["result"]["channel"].as_u64().expect("canal")).unwrap();
    assert!(r["result"]["title"].is_string());

    send(
        &mut socket,
        channel,
        Frame::Data(b"echo CODATOS-$((40+2))\n"),
    )
    .await;
    read_until(&mut socket, channel, "CODATOS-42").await;

    send(
        &mut socket,
        channel,
        Frame::Resize {
            cols: 120,
            rows: 40,
        },
    )
    .await;
    send(&mut socket, channel, Frame::Data(b"stty size\n")).await;
    read_until(&mut socket, channel, "40 120").await;

    send(&mut socket, channel, Frame::Data(b"exit\n")).await;
    let (_, closed) = read_until(&mut socket, channel, "\u{0}nunca\u{0}").await;
    assert!(closed);
}

#[tokio::test]
async fn alvo_invalido_nao_abre_canal() {
    let s = server().await;
    let mut socket = connect(&s).await;
    let r = open(
        &mut socket,
        json!({"target": {"kind": "docker", "container": "--privileged"}}),
    )
    .await;
    assert_eq!(r["error"]["data"]["code"], "invalid_params");
}
