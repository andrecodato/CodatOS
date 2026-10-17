//! Métodos JSON-RPC registrados de fora do núcleo (preferências, dashboards, ações do WSL…).
//!
//! Cada chamada roda numa tarefa própria: um método lento não segura os outros pedidos da
//! mesma conexão, e as respostas podem voltar fora de ordem (o `id` as identifica).

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use codatos_protocol::rpc::{INVALID_PARAMS, RpcError};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Futuro devolvido por [`Method::call`].
pub type MethodFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, MethodError>> + Send + 'a>>;

pub trait Method: fmt::Debug + Send + Sync + 'static {
    /// Nome no JSON-RPC, ex.: `dashboards.save`.
    fn name(&self) -> &'static str;

    /// `params` é `null` quando o pedido não trouxe parâmetros.
    fn call(&self, params: Value) -> MethodFuture<'_>;
}

/// Erros de aplicação usam o código JSON-RPC -32000 (faixa de erros do servidor).
pub const APP_ERROR: i32 = -32000;

/// Falha de um método, entregue como erro JSON-RPC com *problem details*.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{title}")]
pub struct MethodError {
    rpc_code: i32,
    code: String,
    title: String,
}

impl MethodError {
    /// Erro de aplicação; `code` é estável (ex.: `dashboard_not_found`).
    pub fn new(code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            rpc_code: APP_ERROR,
            code: code.into(),
            title: title.into(),
        }
    }

    pub fn invalid_params(title: impl Into<String>) -> Self {
        Self {
            rpc_code: INVALID_PARAMS,
            code: "invalid_params".to_owned(),
            title: title.into(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn into_rpc(self) -> RpcError {
        RpcError::new(self.rpc_code, &self.code, self.title)
    }
}

/// Lê os parâmetros de um método, com erro `invalid_params` legível.
pub fn params<T: DeserializeOwned>(params: Value) -> Result<T, MethodError> {
    serde_json::from_value(params)
        .map_err(|error| MethodError::invalid_params(format!("Parâmetros inválidos: {error}")))
}
