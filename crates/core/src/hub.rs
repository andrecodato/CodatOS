//! Registro do que a API oferece além do básico: tópicos (um coletor por tópico, consultado
//! só enquanto houver assinantes) e métodos.
//!
//! Todos os assinantes de um tópico (janela, navegador) dividem a mesma coleta. O intervalo
//! da coleta é o menor pedido entre eles; quem pediu um intervalo maior recebe menos
//! amostras. Sem assinantes, o coletor para: com a janela fechada, nada é coletado
//! (RNF-DES-01).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codatos_protocol::rpc::Problem;
use serde_json::Value;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::channel::ChannelMethod;
use crate::collector::Collector;
use crate::method::Method;

/// Teto do *backoff* depois de falhas seguidas.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Resultado de uma coleta, compartilhado entre os assinantes.
pub(crate) type Sample = Arc<Result<Value, Problem>>;

#[derive(Debug, Default)]
pub(crate) struct Hub {
    topics: BTreeMap<&'static str, Arc<Topic>>,
    methods: BTreeMap<&'static str, Arc<dyn Method>>,
    channel_methods: BTreeMap<&'static str, Arc<dyn ChannelMethod>>,
    next_id: AtomicU64,
}

impl Hub {
    pub(crate) fn new(
        collectors: &[Arc<dyn Collector>],
        methods: &[Arc<dyn Method>],
        channel_methods: &[Arc<dyn ChannelMethod>],
    ) -> Self {
        let mut topics = BTreeMap::new();
        for collector in collectors {
            let name = collector.topic();
            if topics.contains_key(name) {
                tracing::warn!(topic = name, "tópico repetido; fica o primeiro coletor");
                continue;
            }
            topics.insert(name, Arc::new(Topic::new(Arc::clone(collector))));
        }
        let mut by_name = BTreeMap::new();
        for method in methods {
            let name = method.name();
            if by_name.contains_key(name) {
                tracing::warn!(method = name, "método repetido; fica o primeiro");
                continue;
            }
            by_name.insert(name, Arc::clone(method));
        }
        let mut channels = BTreeMap::new();
        for method in channel_methods {
            let name = method.name();
            if by_name.contains_key(name) || channels.contains_key(name) {
                tracing::warn!(method = name, "método repetido; fica o primeiro");
                continue;
            }
            channels.insert(name, Arc::clone(method));
        }
        Self {
            topics,
            methods: by_name,
            channel_methods: channels,
            next_id: AtomicU64::new(1),
        }
    }

    pub(crate) fn channel_method(&self, name: &str) -> Option<Arc<dyn ChannelMethod>> {
        self.channel_methods.get(name).cloned()
    }

    pub(crate) fn method(&self, name: &str) -> Option<Arc<dyn Method>> {
        self.methods.get(name).cloned()
    }

    /// Nomes dos métodos registrados (com os que abrem canal), em ordem.
    pub(crate) fn methods(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.methods
            .keys()
            .chain(self.channel_methods.keys())
            .copied()
    }

    /// Nomes dos tópicos disponíveis, em ordem.
    pub(crate) fn topics(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.topics.keys().copied()
    }

    /// `None` quando o tópico não existe nesta máquina.
    pub(crate) fn subscribe(&self, topic: &str, requested_ms: Option<u64>) -> Option<Subscription> {
        let topic = Arc::clone(self.topics.get(topic)?);
        let interval = topic.collector.intervals().clamp(requested_ms);
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let rx = topic.add(id, interval);
        Some(Subscription {
            id,
            interval,
            rx,
            topic,
        })
    }
}

/// Uma assinatura ativa. Soltar remove o assinante (e para o coletor se era o último).
pub(crate) struct Subscription {
    id: u64,
    pub(crate) interval: Duration,
    pub(crate) rx: watch::Receiver<Option<Sample>>,
    topic: Arc<Topic>,
}

impl Subscription {
    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("id", &self.id)
            .field("topic", &self.topic.collector.topic())
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.topic.remove(self.id);
    }
}

#[derive(Debug)]
struct Topic {
    collector: Arc<dyn Collector>,
    tx: watch::Sender<Option<Sample>>,
    state: Mutex<TopicState>,
}

#[derive(Debug, Default)]
struct TopicState {
    /// Intervalo pedido por assinante.
    subscribers: HashMap<u64, Duration>,
    poller: Option<JoinHandle<()>>,
}

impl Topic {
    fn new(collector: Arc<dyn Collector>) -> Self {
        Self {
            collector,
            tx: watch::Sender::new(None),
            state: Mutex::default(),
        }
    }

    fn add(self: &Arc<Self>, id: u64, interval: Duration) -> watch::Receiver<Option<Sample>> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.subscribers.insert(id, interval);
        if state.poller.is_none() {
            state.poller = Some(tokio::spawn(poll(Arc::clone(self))));
        }
        self.tx.subscribe()
    }

    fn remove(&self, id: u64) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.subscribers.remove(&id);
        if state.subscribers.is_empty() {
            if let Some(poller) = state.poller.take() {
                poller.abort();
            }
            // O próximo assinante não recebe uma amostra velha.
            self.tx.send_replace(None);
        }
    }

    /// Menor intervalo pedido entre os assinantes atuais.
    fn interval(&self) -> Duration {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .subscribers
            .values()
            .min()
            .copied()
            .unwrap_or_else(|| self.collector.intervals().default)
    }
}

async fn poll(topic: Arc<Topic>) {
    let mut failures = 0u32;
    loop {
        let collector = &topic.collector;
        let sample = match tokio::time::timeout(collector.timeout(), collector.collect()).await {
            Ok(Ok(value)) => {
                failures = 0;
                Ok(value)
            }
            Ok(Err(error)) => {
                failures = failures.saturating_add(1);
                tracing::debug!(topic = collector.topic(), %error, "coleta falhou");
                Err(error.to_problem())
            }
            Err(_) => {
                failures = failures.saturating_add(1);
                tracing::warn!(topic = collector.topic(), "coleta passou do tempo");
                Err(Problem::new("collector_timeout", "A coleta demorou demais"))
            }
        };
        topic.tx.send_replace(Some(Arc::new(sample)));
        tokio::time::sleep(backoff(topic.interval(), failures)).await;
    }
}

/// A primeira falha mantém o intervalo; as seguintes dobram a espera até [`MAX_BACKOFF`].
fn backoff(interval: Duration, failures: u32) -> Duration {
    if failures <= 1 {
        return interval;
    }
    let factor = 1u32 << (failures - 1).min(16);
    interval
        .saturating_mul(factor)
        .min(MAX_BACKOFF.max(interval))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_dobra_ate_o_teto() {
        let s = Duration::from_secs(2);
        assert_eq!(backoff(s, 0), s);
        assert_eq!(backoff(s, 1), s);
        assert_eq!(backoff(s, 2), Duration::from_secs(4));
        assert_eq!(backoff(s, 3), Duration::from_secs(8));
        assert_eq!(backoff(s, 40), MAX_BACKOFF);
        // Um intervalo maior que o teto não encolhe.
        let h = Duration::from_secs(600);
        assert_eq!(backoff(h, 5), h);
    }
}
