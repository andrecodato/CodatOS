//! Sessão SSH (`russh`, backend `ring`) com um host cadastrado, ligada a um canal binário
//! como os terminais locais. Jump hosts (`ProxyJump`) encadeiam sessões: cada uma passa
//! por um túnel `direct-tcpip` da anterior, com chave e login conferidos em cada salto.
//! A primeira conexão pode sair por um proxy SOCKS5 ou HTTP ([`crate::proxy`]).
//!
//! Autenticação: a chave da identidade, depois a senha (digitada agora ou guardada) por
//! `password` e por `keyboard-interactive`. A chave do servidor é conferida com a tabela
//! `known_hosts` (RF-CON-013): a primeira conexão e uma chave trocada só seguem com a
//! fingerprint confirmada pelo usuário.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use codatos_core::{Channel, ChannelEvent, MethodError};
use codatos_protocol::connections::{Connections, errors as connection_errors};
use codatos_protocol::terminal::errors;
use codatos_store::{EffectiveHost, Store, Vault, effective};
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate};
use russh::{ChannelMsg, Disconnect};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::keys;
use crate::keys_rpc::internal;
use crate::proxy::Proxy;

/// Rede, DNS e negociação SSH até aqui; depois disso é falha de conexão.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Mantém a sessão viva atrás de NAT e percebe servidor sumido em ~90 s.
const KEEPALIVE: Duration = Duration::from_secs(30);
/// Jump hosts encadeados, contando o destino (o mesmo limite dos grupos).
const MAX_HOPS: usize = 16;
/// Rodadas de `keyboard-interactive` antes de desistir.
const MAX_PROMPT_ROUNDS: usize = 4;

/// O que a sessão SSH precisa do núcleo: hosts, identidades e o cofre para os segredos.
#[derive(Debug, Clone)]
pub(crate) struct Ssh {
    pub store: Store,
    pub vault: Vault,
}

/// Pedido de sessão, já lido do `terminal.open`.
pub(crate) struct Request {
    pub host_id: String,
    pub username: Option<String>,
    pub password: Option<Zeroizing<String>>,
    pub trust_host_key: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

impl Ssh {
    /// Conecta (pelos jump hosts, se houver), autentica e abre o shell; a sessão segue
    /// numa tarefa até um lado fechar.
    pub(crate) async fn open(
        &self,
        request: Request,
        channel: Channel,
    ) -> Result<Value, MethodError> {
        let all = self.store.connections().await.map_err(internal)?;
        let hops = chain(&all, &request.host_id)?;
        let mut sessions: Vec<Handle<HostKeyCheck>> = Vec::with_capacity(hops.len());
        for (i, hop) in hops.iter().enumerate() {
            let target = i + 1 == hops.len();
            // Usuário e senha do pedido valem só para o destino; jump hosts usam a identidade.
            let credentials = if target {
                self.credentials(
                    &all,
                    hop,
                    request.username.clone(),
                    request.password.clone(),
                )
                .await?
            } else {
                self.credentials(&all, hop, None, None).await?
            };
            let mut session = self
                .connect(hop, request.trust_host_key.clone(), sessions.last())
                .await?;
            authenticate(&mut session, credentials, &hop.host.name, target).await?;
            sessions.push(session);
        }
        let session = sessions.pop().expect("a cadeia tem ao menos o destino");
        let host = hops.last().expect("a cadeia tem ao menos o destino");

        let ssh_channel = session
            .channel_open_session()
            .await
            .map_err(connect_failed)?;
        ssh_channel
            .request_pty(
                true,
                "xterm-256color",
                u32::from(request.cols.clamp(10, 1000)),
                u32::from(request.rows.clamp(2, 500)),
                0,
                0,
                &[],
            )
            .await
            .map_err(connect_failed)?;
        ssh_channel
            .request_shell(true)
            .await
            .map_err(connect_failed)?;
        // Só para ordenar por "usado por último"; não impede a sessão.
        if let Err(error) = self.store.touch_host(&host.host.id).await {
            tracing::warn!(%error, "não foi possível marcar o host como usado");
        }
        tokio::spawn(pump(session, sessions, ssh_channel, channel));
        Ok(json!({ "title": host.host.name }))
    }

    /// Usuário, chave e senha: do pedido primeiro, depois da identidade (decifrados agora).
    async fn credentials(
        &self,
        all: &Connections,
        host: &EffectiveHost,
        username: Option<String>,
        password: Option<Zeroizing<String>>,
    ) -> Result<Credentials, MethodError> {
        let identity = host
            .identity_id
            .as_ref()
            .and_then(|id| all.identities.iter().find(|i| i.id == *id));
        let username = username
            .filter(|u| !u.trim().is_empty())
            .or_else(|| identity.map(|i| i.username.clone()))
            .filter(|u| !u.trim().is_empty())
            .ok_or_else(|| {
                MethodError::new(
                    errors::USERNAME_REQUIRED,
                    format!("Informe o usuário para conectar a {}", host.host.name),
                )
            })?;

        let key = match identity.and_then(|i| i.key_id.as_deref()) {
            Some(key_id) => self.private_key(key_id).await?,
            None => None,
        };
        let password = match (password, identity) {
            (Some(password), _) => Some(password),
            (None, Some(identity)) if identity.has_password => self
                .secret(
                    &identity.id,
                    self.store.identity_password(&identity.id).await,
                )?
                .map(|bytes| Zeroizing::new(String::from_utf8_lossy(&bytes).into_owned())),
            _ => None,
        };
        Ok(Credentials {
            username,
            key,
            password,
        })
    }

    async fn private_key(&self, key_id: &str) -> Result<Option<PrivateKey>, MethodError> {
        let Some(bytes) = self.secret(key_id, self.store.key_private(key_id).await)? else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| MethodError::new("key_invalid", "Chave guardada ilegível"))?;
        keys::import(text, None)
            .map(Some)
            .map_err(|_| MethodError::new("key_invalid", "Chave guardada ilegível"))
    }

    /// Abre um envelope do cofre gravado na linha `item_id`.
    fn secret(
        &self,
        item_id: &str,
        sealed: Result<Option<String>, codatos_store::Error>,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, MethodError> {
        let Some(sealed) = sealed.map_err(internal)? else {
            return Ok(None);
        };
        let envelope = serde_json::from_str(&sealed)
            .map_err(|_| MethodError::new("store_failed", "Segredo guardado ilegível"))?;
        Ok(Some(self.vault.open_item(item_id, &envelope)?))
    }

    /// TCP (ou túnel `direct-tcpip` pelo jump host `via`) e negociação SSH, conferindo a
    /// chave do servidor.
    async fn connect(
        &self,
        host: &EffectiveHost,
        trust: Option<String>,
        via: Option<&Handle<HostKeyCheck>>,
    ) -> Result<Handle<HostKeyCheck>, MethodError> {
        let name = &host.host.name;
        let address = host.host.address.trim().to_owned();
        let port = host.host.port;
        let pattern = known_hosts_pattern(&address, port);
        let known = self.store.known_hosts(&pattern).await.map_err(internal)?;
        let seen = Arc::new(Mutex::new(None));
        let check = HostKeyCheck {
            known: known.iter().map(|(_, f)| f.clone()).collect(),
            trust,
            seen: Arc::clone(&seen),
        };
        let config = Arc::new(client::Config {
            inactivity_timeout: None,
            keepalive_interval: Some(KEEPALIVE),
            keepalive_max: 3,
            nodelay: true,
            ..client::Config::default()
        });
        // Só a primeira conexão sai pelo proxy; as seguintes passam pelos jump hosts.
        let proxy = match via {
            None => Proxy::from_advanced(&host.advanced)?,
            Some(_) => None,
        };
        let result = tokio::time::timeout(CONNECT_TIMEOUT, async {
            match via {
                None => match &proxy {
                    None => client::connect(config, (address.as_str(), port), check).await,
                    Some(proxy) => {
                        let stream = proxy.connect(&address, port).await?;
                        client::connect_stream(config, stream, check).await
                    }
                },
                Some(jump) => {
                    let tunnel = jump
                        .channel_open_direct_tcpip(
                            address.as_str(),
                            u32::from(port),
                            "127.0.0.1",
                            0,
                        )
                        .await?;
                    client::connect_stream(config, tunnel.into_stream(), check).await
                }
            }
        })
        .await;
        let seen = seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        match (result, seen) {
            (Ok(Ok(session)), Some(key)) => {
                if !key.known {
                    self.store
                        .trust_host_key(&pattern, &key.key_type, &key.fingerprint)
                        .await
                        .map_err(internal)?;
                }
                Ok(session)
            }
            // Recusada pela conferência: a UI mostra a fingerprint e pergunta.
            (Ok(Err(_)), Some(key)) if !key.trusted => Err(if known.is_empty() {
                MethodError::new(
                    errors::HOST_KEY_UNKNOWN,
                    format!("Primeira conexão a {name} ({address}): confira a chave do servidor"),
                )
            } else {
                MethodError::new(
                    errors::HOST_KEY_CHANGED,
                    format!(
                        "A chave do servidor {name} ({address}) mudou. Se o servidor não foi \
                         trocado, alguém pode estar interceptando a conexão"
                    ),
                )
            }
            .with_detail(key.fingerprint)),
            (Ok(Err(error)), _) => Err(MethodError::new(
                errors::CONNECT_FAILED,
                match &proxy {
                    Some(proxy) => {
                        format!(
                            "Falha ao conectar a {name} pelo {}: {error}",
                            proxy.describe()
                        )
                    }
                    None => format!("Falha ao conectar a {name}: {error}"),
                },
            )),
            (Ok(Ok(_)), None) => Err(MethodError::new(
                errors::CONNECT_FAILED,
                "O servidor não apresentou chave",
            )),
            (Err(_), _) => Err(MethodError::new(
                errors::CONNECT_FAILED,
                format!("{name} ({address}:{port}) não respondeu"),
            )),
        }
    }
}

/// Hosts da conexão, do jump host mais externo ao destino.
fn chain(all: &Connections, host_id: &str) -> Result<Vec<EffectiveHost>, MethodError> {
    let mut hops = Vec::new();
    let mut next = Some(host_id.to_owned());
    while let Some(id) = next.take() {
        let host = effective(all, &id)
            .ok_or_else(|| MethodError::new(connection_errors::NOT_FOUND, "Host não encontrado"))?;
        if hops.len() >= MAX_HOPS || hops.iter().any(|h: &EffectiveHost| h.host.id == id) {
            return Err(MethodError::new(
                connection_errors::CYCLE,
                "A cadeia de jump hosts volta ao início ou é longa demais",
            ));
        }
        next.clone_from(&host.jump_host_id);
        hops.push(host);
    }
    hops.reverse();
    Ok(hops)
}

/// Chave da tabela `known_hosts`: `endereço:porta`, endereço em minúsculas.
fn known_hosts_pattern(address: &str, port: u16) -> String {
    format!("{}:{port}", address.to_ascii_lowercase())
}

struct Credentials {
    username: String,
    key: Option<PrivateKey>,
    password: Option<Zeroizing<String>>,
}

/// Chave que o servidor apresentou.
#[derive(Debug)]
struct SeenKey {
    key_type: String,
    fingerprint: String,
    /// Já estava na tabela.
    known: bool,
    /// Na tabela ou confirmada pelo usuário: a conexão seguiu.
    trusted: bool,
}

/// `client::Handler` que só confere a chave do servidor.
struct HostKeyCheck {
    known: Vec<String>,
    trust: Option<String>,
    seen: Arc<Mutex<Option<SeenKey>>>,
}

impl client::Handler for HostKeyCheck {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let public = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            // Sem CA de hosts: o certificado vale como a chave dentro dele.
            PublicKeyOrCertificate::Certificate(cert) => PublicKey::from(cert.public_key().clone()),
        };
        let fingerprint = public.fingerprint(HashAlg::Sha256).to_string();
        let known = self.known.contains(&fingerprint);
        let trusted = known || self.trust.as_deref() == Some(fingerprint.as_str());
        *self
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(SeenKey {
            key_type: public.algorithm().as_str().to_owned(),
            fingerprint,
            known,
            trusted,
        });
        std::future::ready(Ok(trusted))
    }
}

/// `target`: o destino, onde a UI pode pedir a senha; num jump host, ela precisa estar
/// guardada na identidade.
async fn authenticate(
    session: &mut Handle<HostKeyCheck>,
    credentials: Credentials,
    name: &str,
    target: bool,
) -> Result<(), MethodError> {
    let Credentials {
        username,
        key,
        password,
    } = credentials;
    if let Some(key) = key {
        let hash = session
            .best_supported_rsa_hash()
            .await
            .map_err(connect_failed)?
            .flatten();
        let result = session
            .authenticate_publickey(&username, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
            .map_err(connect_failed)?;
        if result.success() {
            return Ok(());
        }
    }
    let Some(password) = password else {
        return Err(if target {
            MethodError::new(errors::PASSWORD_REQUIRED, "Digite a senha para conectar")
        } else {
            MethodError::new(
                errors::AUTH_FAILED,
                format!(
                    "O jump host {name} pede senha: guarde a senha ou uma chave na identidade dele"
                ),
            )
        });
    };
    let result = session
        .authenticate_password(&username, password.as_str())
        .await
        .map_err(connect_failed)?;
    if result.success() {
        return Ok(());
    }
    // Servidores com PAM costumam aceitar a senha só por keyboard-interactive.
    let mut reply = session
        .authenticate_keyboard_interactive_start(&username, None)
        .await
        .map_err(connect_failed)?;
    for _ in 0..MAX_PROMPT_ROUNDS {
        match reply {
            KeyboardInteractiveAuthResponse::Success => return Ok(()),
            KeyboardInteractiveAuthResponse::Failure { .. } => break,
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                // Perguntas sem eco são a senha; as demais ficam em branco.
                let answers = prompts
                    .iter()
                    .map(|p| {
                        if p.echo {
                            String::new()
                        } else {
                            password.to_string()
                        }
                    })
                    .collect();
                reply = session
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(connect_failed)?;
            }
        }
    }
    Err(MethodError::new(
        errors::AUTH_FAILED,
        format!("{name} recusou o usuário ou a senha"),
    ))
}

/// Liga o shell remoto ao canal até um dos lados acabar.
async fn pump(
    session: Handle<HostKeyCheck>,
    jumps: Vec<Handle<HostKeyCheck>>,
    ssh_channel: russh::Channel<client::Msg>,
    mut channel: Channel,
) {
    let (mut read, write) = ssh_channel.split();
    let sender = channel.sender.clone();
    let mut output = tokio::spawn(async move {
        while let Some(message) = read.wait().await {
            match message {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    if sender.send(&data).await.is_err() {
                        return;
                    }
                }
                ChannelMsg::Eof | ChannelMsg::Close => return,
                _ => {}
            }
        }
    });

    let mut ui_closed = false;
    let mut output_done = false;
    loop {
        tokio::select! {
            event = channel.incoming.recv() => match event {
                Some(ChannelEvent::Data(data)) => {
                    if write.data_bytes(data).await.is_err() {
                        break;
                    }
                }
                Some(ChannelEvent::Resize { cols, rows }) => {
                    let _ = write
                        .window_change(
                            u32::from(cols.clamp(10, 1000)),
                            u32::from(rows.clamp(2, 500)),
                            0,
                            0,
                        )
                        .await;
                }
                None => {
                    ui_closed = true;
                    break;
                }
            },
            _ = &mut output => {
                output_done = true;
                break;
            }
        }
    }
    if ui_closed {
        output.abort();
    } else if !output_done {
        // Entrega o que o servidor mandou antes de fechar.
        let _ = output.await;
    }
    let _ = write.close().await;
    // Do destino para fora: cada jump host só cai depois de quem passa por ele.
    for session in std::iter::once(session).chain(jumps.into_iter().rev()) {
        let _ = session
            .disconnect(Disconnect::ByApplication, "", "pt-BR")
            .await;
    }
    channel.sender.close().await;
}

#[allow(clippy::needless_pass_by_value)]
fn connect_failed(error: russh::Error) -> MethodError {
    MethodError::new(
        errors::CONNECT_FAILED,
        format!("Falha na conexão SSH: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padrao_do_known_hosts() {
        assert_eq!(known_hosts_pattern("Srv.LAN", 22), "srv.lan:22");
        assert_eq!(known_hosts_pattern("::1", 2222), "::1:2222");
    }
}
