//! Coletores: fontes de dados que o núcleo consulta periodicamente e entrega por assinatura.
//!
//! Cada coletor responde por um tópico (`system.metrics`, `wsl.distros`…). O [`Hub`](crate::hub)
//! só o consulta enquanto houver alguém assinando, com *timeout* e *backoff*: um coletor
//! travado ou com erro não afeta os demais (`docs/modulos/sistema.md`, regras de negócio).

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use codatos_protocol::rpc::Problem;
use serde_json::Value;

/// Futuro devolvido por [`Collector::collect`].
pub type CollectFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, CollectError>> + Send + 'a>>;

/// Uma fonte de dados de um tópico.
///
/// Trabalho bloqueante (chamar `wsl.exe`, ler o `sysinfo`) vai para
/// `tokio::task::spawn_blocking` dentro do próprio `collect`.
pub trait Collector: fmt::Debug + Send + Sync + 'static {
    /// Nome do tópico em `subscribe`.
    fn topic(&self) -> &'static str;

    /// Intervalo padrão e mínimo entre coletas.
    fn intervals(&self) -> Intervals {
        Intervals::default()
    }

    /// Tempo máximo de uma coleta; passou disso, vira erro `collector_timeout`.
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }

    fn collect(&self) -> CollectFuture<'_>;
}

/// Intervalos de um tópico. O pedido do cliente fica entre `min` e [`MAX_INTERVAL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intervals {
    pub default: Duration,
    pub min: Duration,
}

impl Default for Intervals {
    fn default() -> Self {
        Self {
            default: Duration::from_secs(2),
            min: Duration::from_millis(500),
        }
    }
}

/// Maior intervalo aceito numa assinatura.
pub const MAX_INTERVAL: Duration = Duration::from_secs(3600);

impl Intervals {
    /// Intervalo efetivo para um pedido em milissegundos (ou o padrão).
    pub fn clamp(&self, requested_ms: Option<u64>) -> Duration {
        requested_ms
            .map_or(self.default, Duration::from_millis)
            .clamp(self.min, MAX_INTERVAL.max(self.min))
    }
}

/// Falha de uma coleta, entregue ao cliente como *problem details*.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{title}")]
pub struct CollectError {
    code: String,
    title: String,
}

impl CollectError {
    /// `code` é estável (ex.: `docker_not_running`); `title` aparece no widget.
    pub fn new(code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            title: title.into(),
        }
    }

    /// A fonte não existe nesta máquina ou não está rodando (ex.: sem GPU NVIDIA).
    pub fn unavailable(title: impl Into<String>) -> Self {
        Self::new("unavailable", title)
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn to_problem(&self) -> Problem {
        Problem::new(&self.code, self.title.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervalo_fica_entre_minimo_e_maximo() {
        let i = Intervals::default();
        assert_eq!(i.clamp(None), Duration::from_secs(2));
        assert_eq!(i.clamp(Some(10)), Duration::from_millis(500));
        assert_eq!(i.clamp(Some(5000)), Duration::from_secs(5));
        assert_eq!(i.clamp(Some(u64::MAX)), MAX_INTERVAL);
    }
}
