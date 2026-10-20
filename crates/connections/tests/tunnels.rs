//! `tunnels.*` contra um servidor `russh` no próprio teste que faz `direct-tcpip` e
//! `tcpip-forward`, com um servidor de eco como destino: `-L`, `-D` (SOCKS5) e `-R`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codatos_connections::{TunnelManager, keys};
use codatos_core::Method;
use codatos_protocol::connections::KeyAlgorithm;
use codatos_protocol::vault::VaultProtection;
use codatos_store::{Keychain, KeychainError, Store, Vault, connection_methods};
use russh::Channel;
use russh::server::{Auth, Msg, Session};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use zeroize::Zeroizing;

// ---------- servidor SSH de teste ----------

struct SshServer;

#[allow(clippy::unused_async_trait_impl)]
impl russh::server::Handler for SshServer {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == "andre" && password == "segredo" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host: &str,
        port: u32,
        _originator: &str,
        _originator_port: u32,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let Ok(mut stream) = TcpStream::connect((host, u16::try_from(port).unwrap())).await else {
            return Ok(());
        };
        reply.accept().await;
        tokio::spawn(async move {
            let mut channel = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut stream, &mut channel).await;
        });
        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let listener = TcpListener::bind((address, u16::try_from(*port).unwrap()))
            .await
            .unwrap();
        let (address, port) = (address.to_owned(), *port);
        let handle = session.handle();
        tokio::spawn(async move {
            while let Ok((mut stream, origin)) = listener.accept().await {
                let Ok(channel) = handle
                    .channel_open_forwarded_tcpip(
                        address.clone(),
                        port,
                        origin.ip().to_string(),
                        u32::from(origin.port()),
                    )
                    .await
                else {
                    continue;
                };
                tokio::spawn(async move {
                    let mut channel = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut channel).await;
                });
            }
        });
        Ok(true)
    }
}

async fn ssh_server() -> u16 {
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
            tokio::spawn(async move {
                if let Ok(session) = russh::server::run_stream(config, stream, SshServer).await {
                    let _ = session.await;
                }
            });
        }
    });
    port
}

/// Devolve o que recebe, com "eco:" na frente.
async fn echo_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                while let Ok(n) = stream.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let mut reply = b"eco:".to_vec();
                    reply.extend_from_slice(&buf[..n]);
                    if stream.write_all(&reply).await.is_err() {
                        break;
                    }
                }
            });
        }
    });
    port
}

async fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

// ---------- núcleo ----------

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

struct Env {
    methods: Vec<Arc<dyn Method>>,
}

impl Env {
    async fn new(ssh_port: u16) -> Self {
        let store = Store::in_memory().await.unwrap();
        let vault = Vault::open(store.clone(), Arc::new(MemoryKeychain::default()))
            .await
            .unwrap();
        vault
            .create(VaultProtection::OsSession, None)
            .await
            .unwrap();
        let mut methods = connection_methods(&store, &vault);
        methods.extend(TunnelManager::new(&store, &vault).methods());
        let env = Self { methods };
        env.ok(
            "connections.saveIdentity",
            json!({"id": "i1", "name": "André", "username": "andre", "password": {"action": "set", "value": "segredo"}}),
        )
        .await;
        env.ok(
            "connections.saveHost",
            json!({"id": "h1", "name": "Servidor", "address": "127.0.0.1", "port": ssh_port, "identityId": "i1"}),
        )
        .await;
        env
    }

    async fn call(&self, name: &str, params: Value) -> Result<Value, codatos_core::MethodError> {
        let method = self.methods.iter().find(|m| m.name() == name).unwrap();
        method.call(params).await
    }

    async fn ok(&self, name: &str, params: Value) -> Value {
        self.call(name, params)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e:?}"))
    }

    /// Salva e abre, aceitando a chave do servidor na primeira vez.
    async fn open(&self, tunnel: Value) {
        let id = tunnel["id"].clone();
        self.ok("tunnels.save", tunnel).await;
        let fingerprint = match self.call("tunnels.start", json!({"id": id})).await {
            Ok(_) => return,
            Err(error) => {
                assert_eq!(error.code(), "ssh_host_key_unknown");
                error.detail().unwrap().to_owned()
            }
        };
        self.ok(
            "tunnels.start",
            json!({"id": id, "trustHostKey": fingerprint}),
        )
        .await;
    }

    async fn state(&self, id: &str) -> Value {
        let list = self.ok("tunnels.list", json!({})).await;
        list["tunnels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == id)
            .cloned()
            .unwrap()
    }
}

async fn roundtrip(stream: &mut TcpStream, text: &str) -> String {
    stream.write_all(text.as_bytes()).await.unwrap();
    let mut buf = vec![0u8; text.len() + 4];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut buf))
        .await
        .expect("eco demorou")
        .unwrap();
    String::from_utf8(buf).unwrap()
}

// ---------- testes ----------

#[tokio::test]
async fn local_dinamico_e_remoto() {
    let ssh = ssh_server().await;
    let echo = echo_server().await;
    let env = Env::new(ssh).await;

    // -L: porta daqui leva ao eco visto do servidor.
    let local = free_port().await;
    env.open(json!({
        "id": "l", "name": "Local", "hostId": "h1", "kind": "local",
        "bindPort": local, "targetHost": "127.0.0.1", "targetPort": echo
    }))
    .await;
    let mut stream = TcpStream::connect(("127.0.0.1", local)).await.unwrap();
    assert_eq!(roundtrip(&mut stream, "oi").await, "eco:oi");
    let state = env.state("l").await;
    assert_eq!(state["state"], "running");
    assert_eq!(state["active"], 1);
    drop(stream);

    // -D: SOCKS5 com nome de host.
    let socks = free_port().await;
    env.open(json!({
        "id": "d", "name": "SOCKS", "hostId": "h1", "kind": "dynamic", "bindPort": socks,
        "targetHost": "ignorado", "targetPort": 1
    }))
    .await;
    let mut stream = TcpStream::connect(("127.0.0.1", socks)).await.unwrap();
    stream.write_all(&[5, 1, 0]).await.unwrap();
    let mut reply = [0u8; 2];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply, [5, 0]);
    let mut request = vec![5, 1, 0, 3, 9];
    request.extend_from_slice(b"localhost");
    request.extend_from_slice(&echo.to_be_bytes());
    stream.write_all(&request).await.unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0, "SOCKS respondeu {reply:?}");
    assert_eq!(roundtrip(&mut stream, "socks").await, "eco:socks");
    // O dinâmico não guarda destino.
    assert_eq!(env.state("d").await["targetHost"], "");

    // -R: porta do servidor leva ao eco visto daqui.
    let remote = free_port().await;
    env.open(json!({
        "id": "r", "name": "Remoto", "hostId": "h1", "kind": "remote",
        "bindPort": remote, "targetHost": "127.0.0.1", "targetPort": echo
    }))
    .await;
    let mut stream = TcpStream::connect(("127.0.0.1", remote)).await.unwrap();
    assert_eq!(roundtrip(&mut stream, "volta").await, "eco:volta");
    drop(stream);

    // Fechar solta a porta.
    env.ok("tunnels.stop", json!({"id": "l"})).await;
    assert_eq!(env.state("l").await["state"], "stopped");
    let mut closed = false;
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", local)).await.is_err() {
            closed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(closed, "a porta do túnel fechado continuou aberta");

    // Porta ocupada vira erro claro.
    let busy = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let busy_port = busy.local_addr().unwrap().port();
    env.ok(
        "tunnels.save",
        json!({
            "id": "b", "name": "Ocupada", "hostId": "h1", "bindPort": busy_port,
            "targetHost": "127.0.0.1", "targetPort": echo
        }),
    )
    .await;
    let error = env
        .call("tunnels.start", json!({"id": "b"}))
        .await
        .unwrap_err();
    assert_eq!(error.code(), "tunnel_bind_failed");
    assert_eq!(env.state("b").await["state"], "failed");

    env.ok("tunnels.delete", json!({"id": "d"})).await;
    let error = env
        .call("tunnels.delete", json!({"id": "d"}))
        .await
        .unwrap_err();
    assert_eq!(error.code(), "tunnel_not_found");
}

#[tokio::test]
async fn host_inexistente_e_destino_obrigatorio() {
    let env = Env::new(free_port().await).await;
    let error = env
        .call(
            "tunnels.save",
            json!({"id": "x", "name": "x", "hostId": "nada", "bindPort": 9000, "targetHost": "a", "targetPort": 1}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), "invalid_params");
    let error = env
        .call(
            "tunnels.save",
            json!({"id": "x", "name": "x", "hostId": "h1", "bindPort": 9000}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), "invalid_params");
}
