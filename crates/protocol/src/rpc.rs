//! Envelope JSON-RPC 2.0 com erros no formato *problem details* (RFC 9457).

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const JSONRPC_VERSION: &str = "2.0";

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

/// Identificador de uma chamada; ausente em notificações.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Id {
    Number(i64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(
        id: impl Into<Option<Id>>,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// Resposta: `id` é `null` quando o pedido nem pôde ser lido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: Option<Id>,
    #[serde(flatten)]
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Result(Value),
    Error(RpcError),
}

impl Response {
    pub fn ok(id: Option<Id>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            outcome: Outcome::Result(result),
        }
    }

    pub fn err(id: Option<Id>, error: RpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            outcome: Outcome::Error(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<Problem>>,
}

impl RpcError {
    /// Erro com `data` em problem details; `code` é o código estável (ex.: `vault_locked`).
    pub fn new(code: i32, problem_code: &str, title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            code,
            message: title.clone(),
            data: Some(Box::new(Problem::new(problem_code, title))),
        }
    }
}

/// *Problem details* (RFC 9457), usado no `data` dos erros RPC e nas respostas HTTP de erro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub code: String,
}

impl Problem {
    pub fn new(code: &str, title: impl Into<String>) -> Self {
        Self {
            kind: format!("https://codatos.dev/problems/{code}"),
            title: title.into(),
            detail: None,
            code: code.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resposta_de_sucesso_segue_o_formato() {
        let r = Response::ok(Some(Id::Number(7)), json!("pong"));
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            json!({"jsonrpc": "2.0", "id": 7, "result": "pong"})
        );
    }

    #[test]
    fn erro_sem_id_serializa_id_null() {
        let r = Response::err(
            None,
            RpcError::new(PARSE_ERROR, "parse_error", "JSON inválido"),
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], json!(-32700));
        assert_eq!(v["error"]["data"]["code"], json!("parse_error"));
        assert_eq!(
            v["error"]["data"]["type"],
            json!("https://codatos.dev/problems/parse_error")
        );
    }

    #[test]
    fn pedido_sem_id_e_notificacao() {
        let r: Request = serde_json::from_str(r#"{"jsonrpc":"2.0","method":"x"}"#).unwrap();
        assert_eq!(r.id, None);
    }
}
