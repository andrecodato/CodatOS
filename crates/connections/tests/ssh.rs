//! `terminal.open` com `kind: "ssh"` contra um servidor `russh` no próprio teste: chave do
//! servidor, senha guardada, senha digitada, chave SSH e o shell pelo canal binário.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codatos_connections::{OpenTerminalMethod, key_methods, keys};
use codatos_core::{RunningServer, ServerConfig, start};
use codatos_protocol::channel::{Frame, decode, encode};
use codatos_protocol::connections::KeyAlgorithm;
use codatos_protocol::terminal::errors;
use codatos_protocol::vault::VaultProtection;
use codatos_store::{Keychain, KeychainError, Store, Vault, connection_methods};
use futures_util::{SinkExt, StreamExt};
use russh::keys::PublicKey;
use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use zeroize::Zeroizing;

// ---------- servidor SSH de teste ----------

/// Aceita `andre`/`segredo` ou a chave em `allowed`; o shell ecoa o que recebe.
#[derive(Clone)]
struct TestServer {
    allowed: Arc<Mutex<Option<PublicKey>>>,
}

// O trait pede `async fn`; o servidor de teste não espera nada.
#[allow(clippy::unused_async_trait_impl)]
impl russh::server::Handler for TestServer {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == "andre" && password == "segredo" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn auth_publickey(&mut self, user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        let allowed = self.allowed.lock().unwrap().clone();
        Ok(
            if user == "andre" && allowed.is_some_and(|k| k.key_data() == key.key_data()) {
                Auth::Accept
            } else {
                Auth::reject()
            },
        )
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        session.data(
            channel,
            format!("pty {term} {cols}x{rows}\r\n").into_bytes(),
        )?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        session.data(channel, b"bem-vindo\r\n".to_vec())?;
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if data == b"exit\n" {
            session.data(channel, b"tchau\r\n".to_vec())?;
            session.exit_status_request(channel, 0)?;
            session.eof(channel)?;
            session.close(channel)?;
        } else {
            let mut reply = b"eco:".to_vec();
            reply.extend_from_slice(data);
            session.data(channel, reply)?;
        }
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.data(channel, format!("{cols}x{rows}").into_bytes())?;
        Ok(())
    }
}

/// Sobe o servidor SSH numa porta livre; devolve a porta.
async fn ssh_server(allowed: Arc<Mutex<Option<PublicKey>>>) -> u16 {
    let host_key = keys::generate(KeyAlgorithm::Ed25519, "servidor").unwrap();
    let config = Arc::new(russh::server::Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..russh::server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let config = Arc::clone(&config);
            let handler = TestServer {
                allowed: Arc::clone(&allowed),
            };
            tokio::spawn(async move {
                if let Ok(session) = russh::server::run_stream(config, stream, handler).await {
                    let _ = session.await;
                }
            });
        }
    });
    port
}

// ---------- núcleo com banco e cofre ----------

#[derive(Debug, Default)]
struct MemoryKeychain(Mutex<Option<String>>);

impl Keychain for MemoryKeychain {
    fn available(&self) -> bool {
        true
    }
    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError> {
        Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
    }
    fn set(&self, secret: &str) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }
    fn delete(&self) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

async fn core() -> (RunningServer, Store) {
    let store = Store::in_memory().await.unwrap();
    let vault = Vault::open(store.clone(), Arc::new(MemoryKeychain::default()))
        .await
        .unwrap();
    vault
        .create(VaultProtection::OsSession, None)
        .await
        .unwrap();
    let mut methods = connection_methods(&store, &vault);
    methods.extend(key_methods(&store, &vault));
    let server = start(ServerConfig {
        port: 0,
        methods,
        channel_methods: vec![Arc::new(OpenTerminalMethod::new(Some((
            store.clone(),
            vault,
        ))))],
        ..ServerConfig::default()
    })
    .await
    .expect("servidor sobe");
    (server, store)
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

async fn call(socket: &mut Socket, method: &str, params: Value) -> Value {
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .unwrap();
    loop {
        let message = tokio::time::timeout(Duration::from_secs(20), socket.next())
            .await
            .expect("resposta em até 20 s")
            .unwrap()
            .unwrap();
        if let Message::Text(text) = message {
            let value: Value = serde_json::from_str(text.as_str()).unwrap();
            if value.get("id").is_some() {
                return value;
            }
        }
    }
}

fn ssh_target(host: &str, extra: &Value) -> Value {
    let mut target = json!({"kind": "ssh", "hostId": host});
    target
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    json!({"target": target, "cols": 100, "rows": 30})
}

async fn send(socket: &mut Socket, channel: u32, frame: Frame<'_>) {
    socket
        .send(Message::Binary(encode(channel, frame).into()))
        .await
        .unwrap();
}

/// Junta a saída até `want` aparecer (`None`: até o canal fechar).
async fn read_until(socket: &mut Socket, channel: u32, want: Option<&str>) -> String {
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
                send(
                    socket,
                    channel,
                    Frame::Ack(u32::try_from(data.len()).unwrap()),
                )
                .await;
                let text = String::from_utf8_lossy(&output).into_owned();
                if want.is_some_and(|w| text.contains(w)) {
                    return text;
                }
            }
            Frame::Close => {
                assert!(want.is_none(), "fechou antes de {want:?}");
                return String::from_utf8_lossy(&output).into_owned();
            }
            other => panic!("frame inesperado: {other:?}"),
        }
    }
}

fn channel_of(response: &Value) -> u32 {
    let id = response["result"]["channel"]
        .as_u64()
        .unwrap_or_else(|| panic!("sem canal: {response}"));
    u32::try_from(id).unwrap()
}

#[tokio::test]
async fn confere_a_chave_do_servidor_e_abre_o_shell() {
    let port = ssh_server(Arc::default()).await;
    let (server, store) = core().await;
    let mut socket = connect(&server).await;
    call(
        &mut socket,
        "connections.saveIdentity",
        json!({"id": "i1", "name": "André", "username": "andre", "password": {"action": "set", "value": "segredo"}}),
    )
    .await;
    let saved = call(
        &mut socket,
        "connections.saveHost",
        json!({"id": "h1", "name": "Servidor", "address": "127.0.0.1", "port": port, "identityId": "i1"}),
    )
    .await;
    assert!(saved.get("error").is_none(), "{saved}");

    // Primeira vez: a UI recebe a fingerprint para o usuário conferir.
    let r = call(&mut socket, "terminal.open", ssh_target("h1", &json!({}))).await;
    assert_eq!(r["error"]["data"]["code"], errors::HOST_KEY_UNKNOWN, "{r}");
    let fingerprint = r["error"]["data"]["detail"].as_str().unwrap().to_owned();
    assert!(fingerprint.starts_with("SHA256:"));

    // Outra fingerprint não serve.
    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"trustHostKey": "SHA256:outra"})),
    )
    .await;
    assert_eq!(r["error"]["data"]["code"], errors::HOST_KEY_UNKNOWN);
    assert_eq!(
        store
            .known_hosts(&format!("127.0.0.1:{port}"))
            .await
            .unwrap(),
        []
    );

    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"trustHostKey": fingerprint})),
    )
    .await;
    assert_eq!(r["result"]["title"], "Servidor", "{r}");
    let channel = channel_of(&r);
    let banner = read_until(&mut socket, channel, Some("bem-vindo")).await;
    assert!(banner.contains("pty xterm-256color 100x30"), "{banner}");

    send(&mut socket, channel, Frame::Data(b"oi")).await;
    read_until(&mut socket, channel, Some("eco:oi")).await;
    send(
        &mut socket,
        channel,
        Frame::Resize {
            cols: 120,
            rows: 40,
        },
    )
    .await;
    read_until(&mut socket, channel, Some("120x40")).await;
    send(&mut socket, channel, Frame::Data(b"exit\n")).await;
    assert!(
        read_until(&mut socket, channel, None)
            .await
            .contains("tchau")
    );

    // Aceita: a próxima conexão não pergunta, e o host fica marcado como usado.
    let known = store
        .known_hosts(&format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    assert_eq!(known, [("ssh-ed25519".to_owned(), fingerprint.clone())]);
    let r = call(&mut socket, "terminal.open", ssh_target("h1", &json!({}))).await;
    let channel = channel_of(&r);
    read_until(&mut socket, channel, Some("bem-vindo")).await;
    let list = call(&mut socket, "connections.list", json!({})).await;
    assert!(list["result"]["hosts"][0]["lastUsedAt"].is_string());

    // Chave diferente da aceita: aviso de troca.
    store
        .trust_host_key(&format!("127.0.0.1:{port}"), "ssh-ed25519", "SHA256:antiga")
        .await
        .unwrap();
    let r = call(&mut socket, "terminal.open", ssh_target("h1", &json!({}))).await;
    assert_eq!(r["error"]["data"]["code"], errors::HOST_KEY_CHANGED);
    assert_eq!(r["error"]["data"]["detail"], fingerprint.as_str());
}

#[tokio::test]
async fn senha_digitada_chave_e_recusas() {
    let allowed = Arc::new(Mutex::new(None));
    let port = ssh_server(Arc::clone(&allowed)).await;
    let (server, store) = core().await;
    let mut socket = connect(&server).await;
    let pattern = format!("127.0.0.1:{port}");

    // Aceita a chave do servidor sem passar pela pergunta.
    let r = call(
        &mut socket,
        "connections.saveHost",
        json!({"id": "h1", "name": "Sem identidade", "address": "127.0.0.1", "port": port}),
    )
    .await;
    assert!(r.get("error").is_none(), "{r}");
    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"username": "andre"})),
    )
    .await;
    let fingerprint = r["error"]["data"]["detail"].as_str().unwrap().to_owned();
    store
        .trust_host_key(&pattern, "ssh-ed25519", &fingerprint)
        .await
        .unwrap();

    // Sem identidade nem usuário.
    let r = call(&mut socket, "terminal.open", ssh_target("h1", &json!({}))).await;
    assert_eq!(r["error"]["data"]["code"], errors::USERNAME_REQUIRED);
    // Usuário sem senha: a UI pede a senha.
    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"username": "andre"})),
    )
    .await;
    assert_eq!(r["error"]["data"]["code"], errors::PASSWORD_REQUIRED);
    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"username": "andre", "password": "errada"})),
    )
    .await;
    assert_eq!(r["error"]["data"]["code"], errors::AUTH_FAILED);
    let r = call(
        &mut socket,
        "terminal.open",
        ssh_target("h1", &json!({"username": "andre", "password": "segredo"})),
    )
    .await;
    read_until(&mut socket, channel_of(&r), Some("bem-vindo")).await;

    // Chave do cofre, sem senha nenhuma.
    let key = call(
        &mut socket,
        "keys.generate",
        json!({"id": "k1", "name": "PC", "algorithm": "ed25519"}),
    )
    .await;
    let public = PublicKey::from_openssh(key["result"]["publicKey"].as_str().unwrap()).unwrap();
    *allowed.lock().unwrap() = Some(public);
    call(
        &mut socket,
        "connections.saveIdentity",
        json!({"id": "i1", "name": "Chave", "username": "andre", "keyId": "k1"}),
    )
    .await;
    call(
        &mut socket,
        "connections.saveHost",
        json!({"id": "h1", "name": "Com chave", "address": "127.0.0.1", "port": port, "identityId": "i1"}),
    )
    .await;
    let r = call(&mut socket, "terminal.open", ssh_target("h1", &json!({}))).await;
    read_until(&mut socket, channel_of(&r), Some("bem-vindo")).await;

    // Servidor que não responde.
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead = closed.local_addr().unwrap().port();
    drop(closed);
    call(
        &mut socket,
        "connections.saveHost",
        json!({"id": "h2", "name": "Fora", "address": "127.0.0.1", "port": dead, "identityId": "i1"}),
    )
    .await;
    let r = call(&mut socket, "terminal.open", ssh_target("h2", &json!({}))).await;
    assert_eq!(r["error"]["data"]["code"], errors::CONNECT_FAILED);
}
