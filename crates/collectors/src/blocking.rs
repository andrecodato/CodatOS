//! Coletas que bloqueiam a thread (sysinfo, comandos do SO) rodam fora do runtime.

use codatos_core::CollectError;
use serde::Serialize;
use serde_json::Value;

/// Roda `f` em `spawn_blocking` e serializa o resultado.
pub(crate) async fn run<T, F>(f: F) -> Result<Value, CollectError>
where
    T: Serialize,
    F: FnOnce() -> Result<T, CollectError> + Send + 'static,
    T: Send + 'static,
{
    let value = tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| CollectError::new("collector_panicked", "O coletor falhou"))??;
    serde_json::to_value(value)
        .map_err(|_| CollectError::new("collector_invalid", "Amostra inválida"))
}
