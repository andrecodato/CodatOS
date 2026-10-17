//! Tipos das rotas HTTP e dos métodos da API local.

use serde::{Deserialize, Serialize};

/// Versão da API local exposta em `/api/v1`.
pub const API_VERSION: u32 = 1;

/// Porta padrão do núcleo em `127.0.0.1`. A CSP da janela do app aponta para ela.
pub const DEFAULT_PORT: u16 = 47800;

/// Comando IPC do Tauri que entrega à janela o token do app (ADR-0003).
pub const IPC_GET_SESSION_TOKEN: &str = "get_session_token";

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

/// Método das notificações que o núcleo empurra para cada assinatura.
pub const EVENT_METHOD: &str = "event";

/// Parâmetros de `subscribe`. `interval_ms` é ajustado ao mínimo e ao máximo do tópico.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeParams {
    pub topic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,
}

/// Resultado de `subscribe`: o id usado nos eventos e em `unsubscribe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscribed {
    pub sub: String,
    /// Intervalo efetivo depois do ajuste.
    pub interval_ms: u64,
}

/// Parâmetros de `unsubscribe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeParams {
    pub sub: String,
}

/// Parâmetros da notificação `event`: `data` com a amostra ou `error` quando o coletor
/// falhou. O erro vale só para o widget daquele tópico (RF-DASH-009).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventParams {
    pub sub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::rpc::Problem>,
}
