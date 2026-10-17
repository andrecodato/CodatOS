//! Despacho dos métodos JSON-RPC recebidos pelo WebSocket.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use codatos_protocol::api::{
    API_VERSION, EVENT_METHOD, EventParams, Hello, SubscribeParams, Subscribed, UnsubscribeParams,
};
use codatos_protocol::rpc::{
    INVALID_PARAMS, INVALID_REQUEST, Id, JSONRPC_VERSION, METHOD_NOT_FOUND, PARSE_ERROR, Request,
    Response, RpcError,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::hub::{Hub, Subscription};

/// Assinaturas simultâneas por conexão.
const MAX_SUBSCRIPTIONS: usize = 64;

/// Estado de uma conexão WebSocket: as assinaturas dela e a fila de saída.
#[derive(Debug)]
pub(crate) struct Connection {
    hub: Arc<Hub>,
    out: mpsc::Sender<String>,
    subs: HashMap<String, JoinHandle<()>>,
    /// Chamadas de métodos em andamento; canceladas se a conexão fechar.
    calls: Vec<JoinHandle<()>>,
}

impl Connection {
    pub(crate) fn new(hub: Arc<Hub>, out: mpsc::Sender<String>) -> Self {
        Self {
            hub,
            out,
            subs: HashMap::new(),
            calls: Vec::new(),
        }
    }

    /// Trata uma mensagem de texto. `None` quando é notificação (sem `id`), que não tem resposta.
    pub(crate) fn handle_text(&mut self, text: &str) -> Option<Response> {
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

        if let Some(method) = self.hub.method(&request.method) {
            self.spawn_call(method, request.id, request.params.unwrap_or(Value::Null));
            return None;
        }
        let result = self.dispatch(&request.method, request.params);
        let id = request.id?;
        Some(match result {
            Ok(value) => Response::ok(Some(id), value),
            Err(error) => Response::err(Some(id), error),
        })
    }

    fn dispatch(&mut self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
        match method {
            "system.ping" => Ok(json!("pong")),
            "session.hello" => Ok(to_value(&Hello {
                version: env!("CARGO_PKG_VERSION").to_owned(),
                api: API_VERSION,
                capabilities: self
                    .hub
                    .topics()
                    .chain(self.hub.methods())
                    .map(str::to_owned)
                    .collect(),
            })),
            "subscribe" => self.subscribe(&parse(params)?),
            "unsubscribe" => {
                let params: UnsubscribeParams = parse(params)?;
                let found = self.subs.remove(&params.sub).inspect(JoinHandle::abort);
                Ok(json!(found.is_some()))
            }
            _ => Err(RpcError::new(
                METHOD_NOT_FOUND,
                "method_not_found",
                format!("Método desconhecido: {method}"),
            )),
        }
    }

    /// Roda um método registrado numa tarefa; a resposta sai pela fila da conexão.
    fn spawn_call(
        &mut self,
        method: Arc<dyn crate::method::Method>,
        id: Option<Id>,
        params: Value,
    ) {
        let out = self.out.clone();
        let task = tokio::spawn(async move {
            let result = method.call(params).await;
            let Some(id) = id else { return };
            let response = match result {
                Ok(value) => Response::ok(Some(id), value),
                Err(error) => Response::err(Some(id), error.into_rpc()),
            };
            let json = serde_json::to_string(&response).expect("Response sempre serializa");
            let _ = out.send(json).await;
        });
        self.calls.retain(|t| !t.is_finished());
        self.calls.push(task);
    }

    fn subscribe(&mut self, params: &SubscribeParams) -> Result<Value, RpcError> {
        // Assinaturas cujo envio já acabou (cliente lento demais) não contam.
        self.subs.retain(|_, task| !task.is_finished());
        if self.subs.len() >= MAX_SUBSCRIPTIONS {
            return Err(RpcError::new(
                INVALID_REQUEST,
                "too_many_subscriptions",
                format!("Limite de {MAX_SUBSCRIPTIONS} assinaturas por conexão"),
            ));
        }
        let Some(subscription) = self.hub.subscribe(&params.topic, params.interval_ms) else {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "unknown_topic",
                format!("Tópico desconhecido: {}", params.topic),
            ));
        };
        let sub = format!("s{}", subscription.id());
        let interval_ms = u64::try_from(subscription.interval.as_millis()).unwrap_or(u64::MAX);
        let task = tokio::spawn(forward(subscription, sub.clone(), self.out.clone()));
        self.subs.insert(sub.clone(), task);
        Ok(to_value(&Subscribed { sub, interval_ms }))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        for task in self.subs.values().chain(&self.calls) {
            task.abort();
        }
    }
}

/// Repassa as amostras de uma assinatura para a fila de saída da conexão, respeitando o
/// intervalo pedido. Se a fila estiver cheia, a amostra é descartada: a próxima chega logo.
async fn forward(mut subscription: Subscription, sub: String, out: mpsc::Sender<String>) {
    // Folga para uma amostra que chega um pouco antes do intervalo não ser pulada.
    let slack = subscription.interval / 10;
    let mut last_sent: Option<Instant> = None;
    loop {
        let sample = subscription.rx.borrow_and_update().clone();
        if let Some(sample) = sample {
            let due = last_sent.is_none_or(|t| t.elapsed() + slack >= subscription.interval);
            if due {
                let (data, error) = match sample.as_ref() {
                    Ok(value) => (Some(value.clone()), None),
                    Err(problem) => (None, Some(problem.clone())),
                };
                let params = EventParams {
                    sub: sub.clone(),
                    data,
                    error,
                };
                let event = Request::new(None, EVENT_METHOD, Some(json!(params)));
                let json = serde_json::to_string(&event).expect("evento sempre serializa");
                match out.try_send(json) {
                    Ok(()) => last_sent = Some(Instant::now()),
                    Err(mpsc::error::TrySendError::Full(_)) => {}
                    Err(mpsc::error::TrySendError::Closed(_)) => break,
                }
            }
        }
        if subscription.rx.changed().await.is_err() {
            break;
        }
    }
}

fn parse<T: DeserializeOwned>(params: Option<Value>) -> Result<T, RpcError> {
    serde_json::from_value(params.unwrap_or(Value::Null)).map_err(|error| {
        RpcError::new(
            INVALID_PARAMS,
            "invalid_params",
            format!("Parâmetros inválidos: {error}"),
        )
    })
}

fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("tipos do protocolo sempre serializam")
}

fn invalid(id: Option<Id>) -> Response {
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
    use codatos_protocol::rpc::Outcome;

    fn handle_text(text: &str) -> Option<Response> {
        let (out, _rx) = mpsc::channel(1);
        Connection::new(Arc::new(Hub::default()), out).handle_text(text)
    }

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
