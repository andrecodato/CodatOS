//! Núcleo local do CodatOS (ADR-0003): uma API única para a janela, o navegador e o celular.
//!
//! [`start`] sobe a API em `127.0.0.1` (Axum): `GET /api/v1/health`, `POST /api/v1/session`
//! e o WebSocket JSON-RPC em `/rpc`. Toda requisição passa pela checagem de `Host` e `Origin`
//! (Segurança §8); o WebSocket exige o token do app ou um cookie de sessão.
//!
//! Pelo WebSocket, `subscribe`/`unsubscribe` ligam a UI aos [`Collector`]s: cada tópico
//! empurra notificações `event` enquanto alguém assina.

mod auth;
mod collector;
mod guard;
mod hub;
mod rpc;
mod server;

pub use collector::{CollectError, CollectFuture, Collector, Intervals, MAX_INTERVAL};

pub use server::{DEFAULT_PORT, Error, RunningServer, ServerConfig, start};
