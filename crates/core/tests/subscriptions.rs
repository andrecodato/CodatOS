//! Assinaturas de tópicos (`subscribe`/`unsubscribe`/`event`) contra um servidor de verdade.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use codatos_core::{
    CollectError, CollectFuture, Collector, Intervals, RunningServer, ServerConfig, start,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

/// Conta as coletas e devolve o número da coleta.
#[derive(Debug, Default)]
struct Counter(AtomicU64);

impl Collector for Counter {
    fn topic(&self) -> &'static str {
        "test.counter"
    }
    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_millis(20),
            min: Duration::from_millis(10),
        }
    }
    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async move { Ok(json!(self.0.fetch_add(1, Ordering::SeqCst) + 1)) })
    }
}

#[derive(Debug)]
struct Failing;

impl Collector for Failing {
    fn topic(&self) -> &'static str {
        "test.failing"
    }
    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async { Err(CollectError::new("boom", "Deu ruim")) })
    }
}

#[derive(Debug)]
struct Slow;

impl Collector for Slow {
    fn topic(&self) -> &'static str {
        "test.slow"
    }
    fn timeout(&self) -> Duration {
        Duration::from_millis(50)
    }
    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(json!(null))
        })
    }
}

async fn server(counter: Arc<Counter>) -> RunningServer {
    start(ServerConfig {
        port: 0,
        collectors: vec![counter, Arc::new(Failing), Arc::new(Slow)],
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

async fn next(socket: &mut Socket) -> Value {
    let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("mensagem em até 5 s");
    match message.unwrap().unwrap() {
        Message::Text(text) => serde_json::from_str(text.as_str()).unwrap(),
        other => panic!("mensagem inesperada: {other:?}"),
    }
}

/// Envia um pedido e devolve a resposta dele, guardando os eventos que chegarem antes.
async fn call(socket: &mut Socket, id: i64, method: &str, params: Value) -> (Value, Vec<Value>) {
    let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    let mut events = Vec::new();
    loop {
        let message = next(socket).await;
        if message["id"] == json!(id) {
            return (message, events);
        }
        events.push(message);
    }
}

async fn next_event(socket: &mut Socket) -> Value {
    let message = next(socket).await;
    assert_eq!(message["method"], json!("event"), "{message}");
    assert!(message.get("id").is_none(), "evento é notificação");
    message["params"].clone()
}

#[tokio::test]
async fn hello_lista_os_topicos() {
    let s = server(Arc::default()).await;
    let mut socket = connect(&s).await;
    let (r, _) = call(&mut socket, 1, "session.hello", json!(null)).await;
    assert_eq!(
        r["result"]["capabilities"],
        json!(["test.counter", "test.failing", "test.slow"])
    );
    s.shutdown().await;
}

#[tokio::test]
async fn assina_recebe_eventos_e_para_o_coletor_ao_cancelar() {
    let counter = Arc::new(Counter::default());
    let s = server(Arc::clone(&counter)).await;
    let mut socket = connect(&s).await;

    let (r, _) = call(
        &mut socket,
        1,
        "subscribe",
        json!({"topic": "test.counter", "intervalMs": 1}),
    )
    .await;
    // Abaixo do mínimo do tópico: vira o mínimo.
    assert_eq!(r["result"]["intervalMs"], json!(10));
    let sub = r["result"]["sub"].clone();

    let first = next_event(&mut socket).await;
    let second = next_event(&mut socket).await;
    assert_eq!(first["sub"], sub);
    assert!(second["data"].as_u64().unwrap() > first["data"].as_u64().unwrap());

    let (r, _) = call(&mut socket, 2, "unsubscribe", json!({"sub": sub})).await;
    assert_eq!(r["result"], json!(true));

    // Sem assinantes, o coletor para.
    tokio::time::sleep(Duration::from_millis(60)).await;
    let stopped = counter.0.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.0.load(Ordering::SeqCst), stopped);

    let (r, _) = call(&mut socket, 3, "unsubscribe", json!({"sub": sub})).await;
    assert_eq!(r["result"], json!(false));
    s.shutdown().await;
}

#[tokio::test]
async fn fechar_a_conexao_para_o_coletor() {
    let counter = Arc::new(Counter::default());
    let s = server(Arc::clone(&counter)).await;
    let mut socket = connect(&s).await;
    call(
        &mut socket,
        1,
        "subscribe",
        json!({"topic": "test.counter"}),
    )
    .await;
    next_event(&mut socket).await;
    socket.close(None).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    let stopped = counter.0.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.0.load(Ordering::SeqCst), stopped);
    s.shutdown().await;
}

#[tokio::test]
async fn dois_assinantes_dividem_a_mesma_coleta() {
    let counter = Arc::new(Counter::default());
    let s = server(Arc::clone(&counter)).await;
    let mut a = connect(&s).await;
    let mut b = connect(&s).await;
    call(&mut a, 1, "subscribe", json!({"topic": "test.counter"})).await;
    call(&mut b, 1, "subscribe", json!({"topic": "test.counter"})).await;
    // Com uma coleta por assinante, os números nunca se repetiriam entre as conexões.
    let mut seen_a = Vec::new();
    let mut seen_b = Vec::new();
    for _ in 0..5 {
        seen_a.push(next_event(&mut a).await["data"].as_u64().unwrap());
        seen_b.push(next_event(&mut b).await["data"].as_u64().unwrap());
    }
    assert!(
        seen_a.iter().any(|n| seen_b.contains(n)),
        "{seen_a:?} × {seen_b:?}"
    );
    s.shutdown().await;
}

#[tokio::test]
async fn erro_e_timeout_do_coletor_viram_eventos_de_erro() {
    let s = server(Arc::default()).await;
    let mut socket = connect(&s).await;

    call(
        &mut socket,
        1,
        "subscribe",
        json!({"topic": "test.failing"}),
    )
    .await;
    let event = next_event(&mut socket).await;
    assert!(event.get("data").is_none());
    assert_eq!(event["error"]["code"], json!("boom"));
    assert_eq!(event["error"]["title"], json!("Deu ruim"));

    call(&mut socket, 2, "subscribe", json!({"topic": "test.slow"})).await;
    let event = loop {
        let event = next_event(&mut socket).await;
        if event["error"]["code"] != json!("boom") {
            break event;
        }
    };
    assert_eq!(event["error"]["code"], json!("collector_timeout"));

    // A conexão continua respondendo.
    let (r, _) = call(&mut socket, 3, "system.ping", json!(null)).await;
    assert_eq!(r["result"], json!("pong"));
    s.shutdown().await;
}

#[tokio::test]
async fn topico_desconhecido_e_parametros_invalidos() {
    let s = server(Arc::default()).await;
    let mut socket = connect(&s).await;
    let (r, _) = call(&mut socket, 1, "subscribe", json!({"topic": "nada"})).await;
    assert_eq!(r["error"]["code"], json!(-32602));
    assert_eq!(r["error"]["data"]["code"], json!("unknown_topic"));

    let (r, _) = call(&mut socket, 2, "subscribe", json!({"intervalMs": 5})).await;
    assert_eq!(r["error"]["data"]["code"], json!("invalid_params"));
    s.shutdown().await;
}
