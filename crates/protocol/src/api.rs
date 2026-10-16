//! Tipos das rotas HTTP e dos métodos da API local.

use serde::{Deserialize, Serialize};

/// Versão da API local exposta em `/api/v1`.
pub const API_VERSION: u32 = 1;

/// Subprotocolo WebSocket negociado em `/rpc`.
pub const WS_PROTOCOL: &str = "codatos.v1";

/// Prefixo do subprotocolo que carrega o token do app (janela Tauri), para o token
/// não aparecer na URL nem em logs de acesso.
pub const WS_TOKEN_PROTOCOL_PREFIX: &str = "codatos.token.";

/// Nome do cookie de sessão do navegador.
pub const SESSION_COOKIE: &str = "codatos_session";

/// Resposta de `GET /api/v1/health`. Pública: nada sensível aqui.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub name: String,
    pub version: String,
    pub api: u32,
}

/// Corpo de `POST /api/v1/session`: troca o token de uso único por um cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub token: String,
}

/// Resultado de `session.hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub version: String,
    pub api: u32,
    pub capabilities: Vec<String>,
}
