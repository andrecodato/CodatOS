//! Núcleo local do CodatOS (ADR-0003): uma API única para a janela, o navegador e o celular.
//!
//! [`start`] sobe a API em `127.0.0.1` (Axum): `GET /api/v1/health`, `POST /api/v1/session`
//! e o WebSocket JSON-RPC em `/rpc`. Toda requisição passa pela checagem de `Host` e `Origin`
//! (Segurança §8); o WebSocket exige o token do app ou um cookie de sessão.
//!
//! Pelo WebSocket, `subscribe`/`unsubscribe` ligam a UI aos [`Collector`]s: cada tópico
//! empurra notificações `event` enquanto alguém assina. Outros módulos acrescentam
//! métodos JSON-RPC com [`Method`], e canais binários (terminal, SFTP) com
//! [`ChannelMethod`].

mod auth;
mod channel;
mod collector;
mod guard;
mod hub;
mod method;
mod rpc;
mod server;

pub use channel::{Channel, ChannelEvent, ChannelMethod, ChannelSender, Closed};
pub use collector::{CollectError, CollectFuture, Collector, Intervals, MAX_INTERVAL};
pub use method::{APP_ERROR, Method, MethodError, MethodFuture, params};

pub use server::{DEFAULT_PORT, Error, RunningServer, ServerConfig, start};
