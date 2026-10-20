//! Canais binários no WebSocket `/rpc` (terminal, SFTP): [`ChannelMethod`] abre um canal
//! ligado à conexão que chamou, e os bytes vão e voltam em frames binários
//! (`codatos_protocol::channel`).

use std::fmt;
use std::sync::Arc;

use codatos_protocol::channel::{self, Frame, INITIAL_WINDOW, MAX_PAYLOAD};
use serde_json::Value;
use tokio::sync::{Semaphore, mpsc};

use crate::method::MethodFuture;

/// Frames da UI esperando o dono do canal.
const INBOX: usize = 64;

/// Método que abre um canal. Devolve o que a UI precisa saber; o núcleo acrescenta
/// `"channel": n` ao objeto devolvido.
pub trait ChannelMethod: fmt::Debug + Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// `channel` fica com quem abriu: o canal fecha quando ele solta [`Channel::incoming`]
    /// e todos os [`ChannelSender`]s, ou quando a UI fecha.
    fn open(&self, params: Value, channel: Channel) -> MethodFuture<'_>;
}

/// O que chega da UI num canal. O fim do canal é `None` no `recv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelEvent {
    Data(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

/// Canal aberto: o lado do núcleo.
#[derive(Debug)]
pub struct Channel {
    pub id: u32,
    pub incoming: mpsc::Receiver<ChannelEvent>,
    pub sender: ChannelSender,
}

/// O que sai pela conexão WebSocket.
#[derive(Debug)]
pub(crate) enum Outgoing {
    Text(String),
    Binary(Vec<u8>),
}

/// Envia dados à UI, respeitando a janela de crédito do canal.
#[derive(Debug, Clone)]
pub struct ChannelSender {
    id: u32,
    out: mpsc::Sender<Outgoing>,
    credit: Arc<Semaphore>,
}

/// A conexão WebSocket fechou.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a conexão com a UI fechou")]
pub struct Closed;

impl ChannelSender {
    /// Manda `data` em frames de até 64 KiB; espera quando a UI ainda não confirmou o que
    /// já recebeu.
    pub async fn send(&self, data: &[u8]) -> Result<(), Closed> {
        for chunk in data.chunks(MAX_PAYLOAD) {
            #[allow(clippy::cast_possible_truncation)] // pedaços de até 64 KiB
            let len = chunk.len() as u32;
            self.credit
                .acquire_many(len)
                .await
                .map_err(|_| Closed)?
                .forget();
            self.out
                .send(Outgoing::Binary(channel::encode(
                    self.id,
                    Frame::Data(chunk),
                )))
                .await
                .map_err(|_| Closed)?;
        }
        Ok(())
    }

    /// Avisa a UI que o canal acabou (o processo saiu, a sessão caiu).
    pub async fn close(&self) {
        let _ = self
            .out
            .send(Outgoing::Binary(channel::encode(self.id, Frame::Close)))
            .await;
    }
}

/// Lado da conexão: para onde vão os frames da UI e o crédito devolvido.
#[derive(Debug)]
pub(crate) struct Route {
    tx: mpsc::Sender<ChannelEvent>,
    credit: Arc<Semaphore>,
}

impl Route {
    pub(crate) fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }

    /// Repassa um frame da UI. `false` quando o canal acabou e a rota pode sair.
    pub(crate) async fn deliver(&self, frame: Frame<'_>) -> bool {
        let event = match frame {
            Frame::Data(data) => ChannelEvent::Data(data.to_vec()),
            Frame::Resize { cols, rows } => ChannelEvent::Resize { cols, rows },
            Frame::Ack(bytes) => {
                // Uma UI com defeito não abre a janela além do tamanho inicial.
                let room =
                    (INITIAL_WINDOW as usize).saturating_sub(self.credit.available_permits());
                self.credit.add_permits((bytes as usize).min(room));
                return true;
            }
            Frame::Close => return false,
        };
        self.tx.send(event).await.is_ok()
    }
}

/// Cria o par canal (para o método) e rota (para a conexão).
pub(crate) fn pair(id: u32, out: mpsc::Sender<Outgoing>) -> (Channel, Route) {
    let (tx, incoming) = mpsc::channel(INBOX);
    let credit = Arc::new(Semaphore::new(INITIAL_WINDOW as usize));
    (
        Channel {
            id,
            incoming,
            sender: ChannelSender {
                id,
                out,
                credit: Arc::clone(&credit),
            },
        },
        Route { tx, credit },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn janela_de_credito() {
        let (out, mut rx) = mpsc::channel(64);
        let (channel, route) = pair(3, out);
        // Enche a janela inteira.
        let big = vec![b'x'; INITIAL_WINDOW as usize];
        channel.sender.send(&big).await.unwrap();
        let mut frames = 0;
        while let Ok(Outgoing::Binary(frame)) = rx.try_recv() {
            assert_eq!(&frame[..5], [0, 0, 0, 3, 0]);
            frames += 1;
        }
        assert_eq!(frames, 4);

        // Sem confirmação, o próximo envio espera.
        let sender = channel.sender.clone();
        let pending = tokio::spawn(async move { sender.send(b"mais").await });
        tokio::task::yield_now().await;
        assert!(rx.try_recv().is_err());
        assert!(route.deliver(Frame::Ack(4)).await);
        pending.await.unwrap().unwrap();
        assert!(matches!(rx.recv().await, Some(Outgoing::Binary(f)) if f.ends_with(b"mais")));

        // Ack exagerado não passa do tamanho da janela.
        assert!(route.deliver(Frame::Ack(u32::MAX)).await);
        assert_eq!(route.credit.available_permits(), INITIAL_WINDOW as usize);
    }

    #[tokio::test]
    async fn eventos_e_fim() {
        let (out, _rx) = mpsc::channel(4);
        let (mut channel, route) = pair(1, out);
        assert!(route.deliver(Frame::Data(b"oi")).await);
        assert!(route.deliver(Frame::Resize { cols: 80, rows: 24 }).await);
        assert_eq!(
            channel.incoming.recv().await,
            Some(ChannelEvent::Data(b"oi".to_vec()))
        );
        assert_eq!(
            channel.incoming.recv().await,
            Some(ChannelEvent::Resize { cols: 80, rows: 24 })
        );
        assert!(!route.deliver(Frame::Close).await);
        drop(channel);
        assert!(route.is_closed());
        assert!(!route.deliver(Frame::Data(b"x")).await);
    }
}
