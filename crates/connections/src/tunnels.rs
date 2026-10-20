//! Túneis (`tunnels.*`, RF-CON-040): `-L` escuta neste PC e abre `direct-tcpip` pelo
//! servidor; `-D` faz o mesmo atrás de um SOCKS5; `-R` pede ao servidor que escute
//! (`tcpip-forward`) e liga cada canal que chega a uma conexão TCP daqui.
//!
//! Cada túnel aberto tem uma tarefa que o serve e o vigia: se a conexão SSH cai, tenta de
//! novo com espera crescente, com o mesmo login da abertura. Erro que pede o usuário
//! (senha, fingerprint) para as tentativas e deixa o túnel em `failed`.

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codatos_core::{Method, MethodError, MethodFuture, params};
use codatos_protocol::terminal::errors as ssh_errors;
use codatos_protocol::tunnels::{
    METHOD_TUNNEL_DELETE, METHOD_TUNNEL_SAVE, METHOD_TUNNEL_START, METHOD_TUNNEL_STOP,
    METHOD_TUNNELS_LIST, StartTunnel, Tunnel, TunnelId, TunnelKind, TunnelState, TunnelStatus,
    Tunnels, errors,
};
use codatos_store::{Store, Vault};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use zeroize::Zeroizing;

use crate::keys_rpc::internal;
use crate::ssh::{Connected, Login, Ssh};

/// Vigia da conexão SSH: um servidor sumido aparece em até isso.
const CHECK: Duration = Duration::from_secs(5);
/// Esperas entre as tentativas de reconexão.
const BACKOFF: [u64; 5] = [2, 5, 10, 30, 60];
/// O cliente SOCKS tem isso para dizer aonde quer ir.
const SOCKS_HANDSHAKE: Duration = Duration::from_secs(10);
const MAX_ID: usize = 64;
const MAX_NAME: usize = 200;
const MAX_ADDRESS: usize = 255;

/// Túneis abertos e o que falta para abrir os outros. Uma instância por núcleo.
#[derive(Debug, Clone)]
pub struct TunnelManager {
    ssh: Ssh,
    running: Arc<Mutex<HashMap<String, Entry>>>,
}

/// Login de uma abertura, reaproveitado nas reconexões.
#[derive(Debug, Clone, Default)]
struct Extras {
    username: Option<String>,
    password: Option<Zeroizing<String>>,
    trust_host_key: Option<String>,
}

#[derive(Debug)]
struct Entry {
    state: TunnelState,
    error: Option<String>,
    active: Arc<AtomicU32>,
    stop: watch::Sender<bool>,
    /// Muda a cada abertura: a tarefa de uma abertura velha não mexe no estado da nova.
    generation: u64,
    extras: Extras,
}

/// O que está aberto para um túnel: a sessão e onde ele recebe conexões.
struct Open {
    connected: Arc<Connected>,
    incoming: Incoming,
}

enum Incoming {
    Listener(TcpListener),
    /// Canais `forwarded-tcpip` do servidor (túnel remoto).
    Forwarded(mpsc::UnboundedReceiver<russh::Channel<russh::client::Msg>>),
}

impl TunnelManager {
    #[must_use]
    pub fn new(store: &Store, vault: &Vault) -> Self {
        Self {
            ssh: Ssh {
                store: store.clone(),
                vault: vault.clone(),
            },
            running: Arc::default(),
        }
    }

    /// Os métodos `tunnels.*`, para `ServerConfig::methods`.
    #[must_use]
    pub fn methods(&self) -> Vec<Arc<dyn Method>> {
        [
            METHOD_TUNNELS_LIST,
            METHOD_TUNNEL_SAVE,
            METHOD_TUNNEL_DELETE,
            METHOD_TUNNEL_START,
            METHOD_TUNNEL_STOP,
        ]
        .into_iter()
        .map(|name| {
            Arc::new(TunnelMethod {
                name,
                manager: self.clone(),
            }) as Arc<dyn Method>
        })
        .collect()
    }

    /// Abre os túneis marcados para subir com o app. Precisa de um runtime tokio.
    pub fn autostart(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let tunnels = match manager.ssh.store.tunnels().await {
                Ok(tunnels) => tunnels,
                Err(error) => {
                    tracing::warn!(%error, "não foi possível ler os túneis");
                    return;
                }
            };
            for tunnel in tunnels.into_iter().filter(|t| t.auto_start) {
                if let Err(error) = manager.start(tunnel, Extras::default(), true).await {
                    tracing::info!(code = error.code(), "túnel automático não abriu");
                }
            }
        });
    }

    async fn list(&self) -> Result<Tunnels, MethodError> {
        let tunnels = self.ssh.store.tunnels().await.map_err(internal)?;
        let running = self.lock();
        Ok(Tunnels {
            tunnels: tunnels
                .into_iter()
                .map(|tunnel| {
                    let entry = running.get(&tunnel.id);
                    TunnelStatus {
                        state: entry.map_or(TunnelState::Stopped, |e| e.state),
                        error: entry.and_then(|e| e.error.clone()),
                        active: entry.map_or(0, |e| e.active.load(Ordering::Relaxed)),
                        tunnel,
                    }
                })
                .collect(),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Fecha o túnel (se aberto). Devolve o login da abertura, para reabrir igual.
    fn stop(&self, id: &str) -> Option<Extras> {
        let entry = self.lock().remove(id)?;
        let _ = entry.stop.send(true);
        Some(entry.extras)
    }

    /// Abre e deixa uma tarefa servindo. Com `retry_first`, uma primeira tentativa que
    /// falha por rede ou cofre trancado entra nas reconexões em vez de parar (autostart).
    async fn start(
        &self,
        tunnel: Tunnel,
        extras: Extras,
        retry_first: bool,
    ) -> Result<(), MethodError> {
        self.stop(&tunnel.id);
        let (stop, stopped) = watch::channel(false);
        let active = Arc::new(AtomicU32::new(0));
        let generation = {
            let mut running = self.lock();
            let generation = running.values().map(|e| e.generation).max().unwrap_or(0) + 1;
            running.insert(
                tunnel.id.clone(),
                Entry {
                    state: TunnelState::Starting,
                    error: None,
                    active: Arc::clone(&active),
                    stop,
                    generation,
                    extras: extras.clone(),
                },
            );
            generation
        };
        let first = self.open(&tunnel, &extras).await;
        let open = match first {
            Ok(open) => Some(open),
            Err(error) if retry_first && retryable(&error) => {
                self.set(
                    &tunnel.id,
                    generation,
                    TunnelState::Reconnecting,
                    Some(error.to_string()),
                );
                None
            }
            Err(error) => {
                self.set(
                    &tunnel.id,
                    generation,
                    TunnelState::Failed,
                    Some(error.to_string()),
                );
                return Err(error);
            }
        };
        if open.is_some() {
            self.set(&tunnel.id, generation, TunnelState::Running, None);
        }
        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .supervise(tunnel, extras, open, generation, stopped, active)
                .await;
        });
        Ok(())
    }

    fn set(&self, id: &str, generation: u64, state: TunnelState, error: Option<String>) {
        if let Some(entry) = self.lock().get_mut(id)
            && entry.generation == generation
        {
            entry.state = state;
            entry.error = error;
        }
    }

    /// Serve enquanto a conexão durar; reconecta quando ela cai; sai ao fechar o túnel.
    async fn supervise(
        &self,
        tunnel: Tunnel,
        extras: Extras,
        mut open: Option<Open>,
        generation: u64,
        mut stopped: watch::Receiver<bool>,
        active: Arc<AtomicU32>,
    ) {
        let mut attempt = 0usize;
        loop {
            if let Some(current) = open.take() {
                attempt = 0;
                let ended = serve(&tunnel, current, &mut stopped, &active).await;
                if ended == Ended::Stopped {
                    return;
                }
                self.set(
                    &tunnel.id,
                    generation,
                    TunnelState::Reconnecting,
                    Some("A conexão caiu".into()),
                );
            }
            let wait = Duration::from_secs(BACKOFF[attempt.min(BACKOFF.len() - 1)]);
            attempt += 1;
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                _ = stopped.changed() => return,
            }
            match self.open(&tunnel, &extras).await {
                Ok(next) => {
                    self.set(&tunnel.id, generation, TunnelState::Running, None);
                    open = Some(next);
                }
                Err(error) if retryable(&error) => {
                    self.set(
                        &tunnel.id,
                        generation,
                        TunnelState::Reconnecting,
                        Some(error.to_string()),
                    );
                }
                Err(error) => {
                    self.set(
                        &tunnel.id,
                        generation,
                        TunnelState::Failed,
                        Some(error.to_string()),
                    );
                    return;
                }
            }
        }
    }

    /// Conecta ao host e abre a escuta (aqui ou no servidor).
    async fn open(&self, tunnel: &Tunnel, extras: &Extras) -> Result<Open, MethodError> {
        let connected = self
            .ssh
            .connect_host(Login {
                host_id: tunnel.host_id.clone(),
                username: extras.username.clone(),
                password: extras.password.clone(),
                trust_host_key: extras.trust_host_key.clone(),
            })
            .await?;
        let incoming = match tunnel.kind {
            TunnelKind::Local | TunnelKind::Dynamic => {
                match TcpListener::bind((tunnel.bind_host.as_str(), tunnel.bind_port)).await {
                    Ok(listener) => Incoming::Listener(listener),
                    Err(error) => {
                        connected.disconnect().await;
                        return Err(MethodError::new(
                            errors::BIND_FAILED,
                            format!(
                                "A porta {}:{} não abriu neste PC: {error}",
                                tunnel.bind_host, tunnel.bind_port
                            ),
                        ));
                    }
                }
            }
            TunnelKind::Remote => {
                let (sender, receiver) = mpsc::unbounded_channel();
                *connected
                    .forwarded
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = Some(sender);
                if let Err(error) = connected
                    .session
                    .tcpip_forward(tunnel.bind_host.as_str(), u32::from(tunnel.bind_port))
                    .await
                {
                    connected.disconnect().await;
                    return Err(MethodError::new(
                        errors::REMOTE_REFUSED,
                        format!(
                            "O servidor não abriu a porta {}:{} ({error}). Confira o \
                             AllowTcpForwarding do sshd e se a porta está livre",
                            tunnel.bind_host, tunnel.bind_port
                        ),
                    ));
                }
                Incoming::Forwarded(receiver)
            }
        };
        Ok(Open {
            connected: Arc::new(connected),
            incoming,
        })
    }
}

/// Erros que outra tentativa pode resolver sozinha. Os que pedem o usuário param.
fn retryable(error: &MethodError) -> bool {
    !matches!(
        error.code(),
        ssh_errors::PASSWORD_REQUIRED
            | ssh_errors::AUTH_FAILED
            | ssh_errors::USERNAME_REQUIRED
            | ssh_errors::HOST_KEY_UNKNOWN
            | ssh_errors::HOST_KEY_CHANGED
            | codatos_protocol::connections::errors::NOT_FOUND
            | codatos_protocol::connections::errors::CYCLE
            | errors::BIND_FAILED
            | errors::REMOTE_REFUSED
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Stopped,
    Lost,
}

async fn serve(
    tunnel: &Tunnel,
    open: Open,
    stopped: &mut watch::Receiver<bool>,
    active: &Arc<AtomicU32>,
) -> Ended {
    let Open {
        connected,
        mut incoming,
    } = open;
    let mut check = tokio::time::interval(CHECK);
    let ended = loop {
        tokio::select! {
            _ = stopped.changed() => break Ended::Stopped,
            _ = check.tick() => {
                if connected.session.is_closed() {
                    break Ended::Lost;
                }
            }
            accepted = next(&mut incoming) => match accepted {
                Some(Accepted::Tcp(stream, origin)) => {
                    let connected = Arc::clone(&connected);
                    let tunnel = tunnel.clone();
                    let guard = Active::new(active);
                    tokio::spawn(async move {
                        let _guard = guard;
                        let result = match tunnel.kind {
                            TunnelKind::Dynamic => socks(&connected, stream, origin).await,
                            _ => forward_local(&connected, &tunnel, stream, origin).await,
                        };
                        if let Err(error) = result {
                            tracing::debug!(%error, "conexão do túnel terminou com erro");
                        }
                    });
                }
                Some(Accepted::Forwarded(channel)) => {
                    let target = (tunnel.target_host.clone(), tunnel.target_port);
                    let guard = Active::new(active);
                    tokio::spawn(async move {
                        let _guard = guard;
                        match TcpStream::connect((target.0.as_str(), target.1)).await {
                            Ok(mut stream) => {
                                let mut channel = channel.into_stream();
                                let _ = tokio::io::copy_bidirectional(&mut stream, &mut channel).await;
                            }
                            Err(error) => {
                                tracing::debug!(%error, "destino do túnel remoto não respondeu");
                            }
                        }
                    });
                }
                None => break Ended::Lost,
            },
        }
    };
    if tunnel.kind == TunnelKind::Remote && ended == Ended::Stopped {
        let _ = connected
            .session
            .cancel_tcpip_forward(tunnel.bind_host.as_str(), u32::from(tunnel.bind_port))
            .await;
    }
    connected.close().await;
    ended
}

enum Accepted {
    Tcp(TcpStream, std::net::SocketAddr),
    Forwarded(russh::Channel<russh::client::Msg>),
}

async fn next(incoming: &mut Incoming) -> Option<Accepted> {
    match incoming {
        Incoming::Listener(listener) => loop {
            match listener.accept().await {
                Ok((stream, origin)) => {
                    let _ = stream.set_nodelay(true);
                    return Some(Accepted::Tcp(stream, origin));
                }
                // Falta de descritores e afins: espera um pouco e segue escutando.
                Err(error) => {
                    tracing::warn!(%error, "túnel não aceitou uma conexão");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        },
        Incoming::Forwarded(receiver) => receiver.recv().await.map(Accepted::Forwarded),
    }
}

/// Conta as conexões passando pelo túnel.
struct Active(Arc<AtomicU32>);

impl Active {
    fn new(counter: &Arc<AtomicU32>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(counter))
    }
}

impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

async fn forward_local(
    connected: &Connected,
    tunnel: &Tunnel,
    mut stream: TcpStream,
    origin: std::net::SocketAddr,
) -> std::io::Result<()> {
    let channel = connected
        .session
        .channel_open_direct_tcpip(
            tunnel.target_host.as_str(),
            u32::from(tunnel.target_port),
            origin.ip().to_string(),
            u32::from(origin.port()),
        )
        .await
        .map_err(std::io::Error::other)?;
    let mut channel = channel.into_stream();
    tokio::io::copy_bidirectional(&mut stream, &mut channel).await?;
    Ok(())
}

/// Proxy SOCKS5 (RFC 1928) sem autenticação, só `CONNECT`, saindo pelo servidor.
async fn socks(
    connected: &Connected,
    mut stream: TcpStream,
    origin: std::net::SocketAddr,
) -> std::io::Result<()> {
    let target = tokio::time::timeout(SOCKS_HANDSHAKE, socks_request(&mut stream))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "SOCKS lento"))??;
    let Some((host, port)) = target else {
        return Ok(());
    };
    let channel = match connected
        .session
        .channel_open_direct_tcpip(
            host,
            u32::from(port),
            origin.ip().to_string(),
            u32::from(origin.port()),
        )
        .await
    {
        Ok(channel) => channel,
        Err(error) => {
            // 0x05: conexão recusada.
            stream.write_all(&socks_reply(5)).await?;
            return Err(std::io::Error::other(error));
        }
    };
    stream.write_all(&socks_reply(0)).await?;
    let mut channel = channel.into_stream();
    tokio::io::copy_bidirectional(&mut stream, &mut channel).await?;
    Ok(())
}

/// Lê a saudação e o pedido. `None`: pedido recusado (a resposta já foi mandada).
async fn socks_request<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
) -> std::io::Result<Option<(String, u16)>> {
    let mut head = [0u8; 2];
    stream.read_exact(&mut head).await?;
    if head[0] != 5 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "só SOCKS5",
        ));
    }
    let mut methods = vec![0u8; usize::from(head[1])];
    stream.read_exact(&mut methods).await?;
    if !methods.contains(&0) {
        // Nenhum método aceitável.
        stream.write_all(&[5, 0xFF]).await?;
        return Ok(None);
    }
    stream.write_all(&[5, 0]).await?;
    let mut request = [0u8; 4];
    stream.read_exact(&mut request).await?;
    let host = match request[3] {
        1 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await?;
            Ipv4Addr::from(ip).to_string()
        }
        3 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            let mut name = vec![0u8; usize::from(len[0])];
            stream.read_exact(&mut name).await?;
            String::from_utf8(name).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "nome inválido")
            })?
        }
        4 => {
            let mut ip = [0u8; 16];
            stream.read_exact(&mut ip).await?;
            Ipv6Addr::from(ip).to_string()
        }
        _ => {
            // 0x08: tipo de endereço não suportado.
            stream.write_all(&socks_reply(8)).await?;
            return Ok(None);
        }
    };
    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await?;
    if request[1] != 1 {
        // 0x07: comando não suportado (só CONNECT).
        stream.write_all(&socks_reply(7)).await?;
        return Ok(None);
    }
    Ok(Some((host, u16::from_be_bytes(port))))
}

/// Resposta SOCKS5 com endereço vazio (0.0.0.0:0).
fn socks_reply(code: u8) -> [u8; 10] {
    [5, code, 0, 1, 0, 0, 0, 0, 0, 0]
}

fn validate(tunnel: &Tunnel) -> Result<(), MethodError> {
    if tunnel.id.is_empty() || tunnel.id.len() > MAX_ID {
        return Err(MethodError::invalid_params("Id inválido"));
    }
    if tunnel.name.trim().is_empty() || tunnel.name.len() > MAX_NAME {
        return Err(MethodError::invalid_params("Nome vazio ou grande demais"));
    }
    let address = |a: &str| {
        !a.trim().is_empty() && a.len() <= MAX_ADDRESS && !a.contains(char::is_whitespace)
    };
    if !address(&tunnel.bind_host) || tunnel.bind_port == 0 {
        return Err(MethodError::invalid_params(
            "Endereço ou porta de escuta inválidos",
        ));
    }
    if tunnel.kind != TunnelKind::Dynamic
        && (!address(&tunnel.target_host) || tunnel.target_port == 0)
    {
        return Err(MethodError::invalid_params("Destino inválido"));
    }
    Ok(())
}

#[derive(Debug)]
struct TunnelMethod {
    name: &'static str,
    manager: TunnelManager,
}

impl Method for TunnelMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let manager = &self.manager;
            let store = &manager.ssh.store;
            match self.name {
                METHOD_TUNNELS_LIST => return Ok(json!(manager.list().await?)),
                METHOD_TUNNEL_SAVE => {
                    let mut tunnel: Tunnel = params(value)?;
                    tunnel.bind_host = tunnel.bind_host.trim().to_owned();
                    tunnel.target_host = tunnel.target_host.trim().to_owned();
                    if tunnel.kind == TunnelKind::Dynamic {
                        tunnel.target_host.clear();
                        tunnel.target_port = 0;
                    }
                    validate(&tunnel)?;
                    if store
                        .effective_host(&tunnel.host_id)
                        .await
                        .map_err(internal)?
                        .is_none()
                    {
                        return Err(MethodError::invalid_params("Host inexistente"));
                    }
                    store.save_tunnel(&tunnel).await.map_err(internal)?;
                    // Aberto, reabre com a configuração nova e o mesmo login. O resultado
                    // aparece no estado do túnel; o salvamento já valeu.
                    let open = manager.lock().get(&tunnel.id).is_some_and(|e| {
                        matches!(
                            e.state,
                            TunnelState::Starting
                                | TunnelState::Running
                                | TunnelState::Reconnecting
                        )
                    });
                    if let Some(extras) = manager.stop(&tunnel.id).filter(|_| open) {
                        let _ = manager.start(tunnel, extras, true).await;
                    }
                }
                METHOD_TUNNEL_DELETE => {
                    let p: TunnelId = params(value)?;
                    manager.stop(&p.id);
                    if !store.delete_tunnel(&p.id).await.map_err(internal)? {
                        return Err(not_found());
                    }
                }
                METHOD_TUNNEL_START => {
                    let p: StartTunnel = params(value)?;
                    let tunnel = store
                        .tunnel(&p.id)
                        .await
                        .map_err(internal)?
                        .ok_or_else(not_found)?;
                    let extras = Extras {
                        username: p.username,
                        password: p.password.map(Zeroizing::new),
                        trust_host_key: p.trust_host_key,
                    };
                    manager.start(tunnel, extras, false).await?;
                }
                METHOD_TUNNEL_STOP => {
                    let p: TunnelId = params(value)?;
                    manager.stop(&p.id);
                }
                _ => unreachable!("método registrado sem implementação: {}", self.name),
            }
            Ok(Value::Null)
        })
    }
}

fn not_found() -> MethodError {
    MethodError::new(errors::NOT_FOUND, "Túnel não encontrado")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pedido_socks() {
        let (mut client, mut server) = tokio::io::duplex(256);
        let task = tokio::spawn(async move { socks_request(&mut server).await });
        client.write_all(&[5, 1, 0]).await.unwrap();
        let mut reply = [0u8; 2];
        client.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [5, 0]);
        client
            .write_all(&[
                5, 1, 0, 3, 7, b'e', b'x', b'.', b'c', b'o', b'm', b'.', 0x01, 0xBB,
            ])
            .await
            .unwrap();
        assert_eq!(
            task.await.unwrap().unwrap(),
            Some(("ex.com.".to_owned(), 443))
        );

        // Só com autenticação: recusa.
        let (mut client, mut server) = tokio::io::duplex(256);
        let task = tokio::spawn(async move { socks_request(&mut server).await });
        client.write_all(&[5, 1, 2]).await.unwrap();
        let mut reply = [0u8; 2];
        client.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [5, 0xFF]);
        assert_eq!(task.await.unwrap().unwrap(), None);

        // BIND (2) não é suportado.
        let (mut client, mut server) = tokio::io::duplex(256);
        let task = tokio::spawn(async move { socks_request(&mut server).await });
        client.write_all(&[5, 1, 0]).await.unwrap();
        client.read_exact(&mut reply).await.unwrap();
        client
            .write_all(&[5, 2, 0, 1, 127, 0, 0, 1, 0, 80])
            .await
            .unwrap();
        let mut refused = [0u8; 10];
        client.read_exact(&mut refused).await.unwrap();
        assert_eq!(refused[1], 7);
        assert_eq!(task.await.unwrap().unwrap(), None);
    }

    #[test]
    fn validacao() {
        let base = Tunnel {
            id: "t".into(),
            name: "x".into(),
            host_id: "h".into(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".into(),
            bind_port: 8080,
            target_host: "localhost".into(),
            target_port: 80,
            auto_start: false,
        };
        assert!(validate(&base).is_ok());
        assert!(
            validate(&Tunnel {
                target_port: 0,
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&Tunnel {
                bind_port: 0,
                ..base.clone()
            })
            .is_err()
        );
        assert!(
            validate(&Tunnel {
                kind: TunnelKind::Dynamic,
                target_host: String::new(),
                target_port: 0,
                ..base
            })
            .is_ok()
        );
    }
}
