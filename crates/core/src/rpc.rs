//! Despacho dos métodos JSON-RPC recebidos pelo WebSocket.

use codatos_protocol::api::{API_VERSION, Hello};
use codatos_protocol::rpc::{
    INVALID_REQUEST, JSONRPC_VERSION, METHOD_NOT_FOUND, PARSE_ERROR, Request, Response, RpcError,
};
use serde_json::{Value, json};

/// Trata uma mensagem de texto. `None` quando é notificação (sem `id`), que não tem resposta.
pub(crate) fn handle_text(text: &str) -> Option<Response> {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            return Some(Response::err(
                None,
                RpcError::new(PARSE_ERROR, "parse_error", "JSON inválido"),
            ));
        }
    };
    // Lotes (arrays) ficam para quando houver um uso real.
    let request: Request = match serde_json::from_value(value) {
        Ok(r) => r,
        Err(_) => return Some(invalid(None)),
    };
    if request.jsonrpc != JSONRPC_VERSION {
        return Some(invalid(request.id));
    }

    let result = dispatch(&request.method, request.params);
    let id = request.id?;
    Some(match result {
        Ok(value) => Response::ok(Some(id), value),
        Err(error) => Response::err(Some(id), error),
    })
}

fn dispatch(method: &str, _params: Option<Value>) -> Result<Value, RpcError> {
    match method {
        "system.ping" => Ok(json!("pong")),
        "session.hello" => Ok(serde_json::to_value(Hello {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            api: API_VERSION,
            capabilities: Vec::new(),
        })
        .expect("Hello sempre serializa")),
        _ => Err(RpcError::new(
            METHOD_NOT_FOUND,
            "method_not_found",
            format!("Método desconhecido: {method}"),
        )),
    }
}

fn invalid(id: Option<codatos_protocol::rpc::Id>) -> Response {
    Response::err(
        id,
        RpcError::new(
            INVALID_REQUEST,
            "invalid_request",
            "Pedido JSON-RPC inválido",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use codatos_protocol::rpc::{Id, Outcome};

    fn outcome(text: &str) -> Outcome {
        handle_text(text).expect("esperava resposta").outcome
    }

    #[test]
    fn ping() {
        let r = handle_text(r#"{"jsonrpc":"2.0","id":1,"method":"system.ping"}"#).unwrap();
        assert_eq!(r.id, Some(Id::Number(1)));
        assert_eq!(r.outcome, Outcome::Result(json!("pong")));
    }

    #[test]
    fn hello_traz_versao_e_api() {
        let Outcome::Result(v) = outcome(r#"{"jsonrpc":"2.0","id":"a","method":"session.hello"}"#)
        else {
            panic!("esperava resultado");
        };
        assert_eq!(v["api"], json!(API_VERSION));
        assert_eq!(v["version"], json!(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn erros_padrao() {
        let code = |text| match outcome(text) {
            Outcome::Error(e) => e.code,
            Outcome::Result(_) => panic!("esperava erro"),
        };
        assert_eq!(code("{"), PARSE_ERROR);
        assert_eq!(code(r#"{"id":1}"#), INVALID_REQUEST);
        assert_eq!(
            code(r#"{"jsonrpc":"1.0","id":1,"method":"system.ping"}"#),
            INVALID_REQUEST
        );
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":1,"method":"nada"}"#),
            METHOD_NOT_FOUND
        );
    }

    #[test]
    fn notificacao_nao_tem_resposta() {
        assert!(handle_text(r#"{"jsonrpc":"2.0","method":"system.ping"}"#).is_none());
    }
}
