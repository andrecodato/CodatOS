//! Canais binários (`ServerConfig::channel_methods`): um eco que devolve em maiúsculas.

use std::sync::Arc;
use std::time::Duration;

use codatos_core::{
    Channel, ChannelEvent, ChannelMethod, MethodError, MethodFuture, RunningServer, ServerConfig,
    start,
};
use codatos_protocol::channel::{Frame, decode, encode};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

#[derive(Debug)]
struct Shout;

impl ChannelMethod for Shout {
    fn name(&self) -> &'static str {
        "test.shout"
    }

    fn open(&self, params: Value, mut channel: Channel) -> MethodFuture<'_> {
        Box::pin(async move {
            if params["fail"] == json!(true) {
                return Err(MethodError::new("no", "Recusado"));
            }
            tokio::spawn(async move {
                while let Some(event) = channel.incoming.recv().await {
                    let reply = match event {
                        ChannelEvent::Data(data) => data.to_ascii_uppercase(),
                        ChannelEvent::Resize { cols, rows } => {
                            format!("{cols}x{rows}").into_bytes()
                        }
                    };
                    if reply == b"FIM" {
                        channel.sender.close().await;
                        return;
                    }
                    if channel.sender.send(&reply).await.is_err() {
                        return;
                    }
                }
            });
            Ok(json!({"kind": "shout"}))
        })
    }
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn server() -> RunningServer {
    start(ServerConfig {
        port: 0,
        channel_methods: vec![Arc::new(Shout)],
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

async fn next(socket: &mut Socket) -> Message {
    tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("mensagem em até 5 s")
        .unwrap()
        .unwrap()
}

async fn call(socket: &mut Socket, id: u32, params: Value) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": id, "method": "test.shout", "params": params});
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    match next(socket).await {
        Message::Text(text) => serde_json::from_str(text.as_str()).unwrap(),
        other => panic!("esperava resposta: {other:?}"),
    }
}

async fn send_frame(socket: &mut Socket, channel: u32, frame: Frame<'_>) {
    socket
        .send(Message::Binary(encode(channel, frame).into()))
        .await
        .unwrap();
}

async fn next_frame(socket: &mut Socket) -> (u32, Vec<u8>, bool) {
    match next(socket).await {
        Message::Binary(bytes) => match decode(&bytes).unwrap() {
            (channel, Frame::Data(data)) => (channel, data.to_vec(), false),
            (channel, Frame::Close) => (channel, Vec::new(), true),
            other => panic!("frame inesperado: {other:?}"),
        },
        other => panic!("esperava frame: {other:?}"),
    }
}

#[tokio::test]
async fn abre_canal_troca_bytes_e_fecha() {
    let s = server().await;
    let mut socket = connect(&s).await;

    let r = call(&mut socket, 1, json!({})).await;
    assert_eq!(r["result"]["kind"], "shout");
    let channel = u32::try_from(r["result"]["channel"].as_u64().unwrap()).unwrap();

    send_frame(&mut socket, channel, Frame::Data(b"ola")).await;
    assert_eq!(
        next_frame(&mut socket).await,
        (channel, b"OLA".to_vec(), false)
    );
    send_frame(
        &mut socket,
        channel,
        Frame::Resize {
            cols: 120,
            rows: 40,
        },
    )
    .await;
    assert_eq!(
        next_frame(&mut socket).await,
        (channel, b"120x40".to_vec(), false)
    );
    // Frames de canal inexistente são ignorados.
    send_frame(&mut socket, 999, Frame::Data(b"x")).await;
    send_frame(&mut socket, channel, Frame::Data(b"fim")).await;
    assert_eq!(next_frame(&mut socket).await, (channel, Vec::new(), true));

    // Erro ao abrir volta como erro JSON-RPC.
    let r = call(&mut socket, 2, json!({"fail": true})).await;
    assert_eq!(r["error"]["data"]["code"], "no");

    // Cada conexão tem seus canais; outro canal recebe outro número.
    let r = call(&mut socket, 3, json!({})).await;
    assert_ne!(r["result"]["channel"].as_u64().unwrap(), u64::from(channel));
    s.shutdown().await;
}
