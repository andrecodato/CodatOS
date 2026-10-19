//! GPUs (RF-SYS-003): uso, memória, temperatura e potência.
//!
//! Tudo pelas bibliotecas que os próprios drivers instalam, sem programa externo: NVIDIA pela
//! NVML; AMD pela ADL no Windows e pelo driver `amdgpu` no Linux. Sem nenhuma, o tópico
//! responde `unavailable`.

use std::sync::{Arc, Mutex, PoisonError};

use codatos_core::{CollectError, CollectFuture, Collector};
use codatos_protocol::system::{Gpu, TOPIC_GPU};

#[cfg(any(windows, test))]
mod adl;
#[cfg(any(not(windows), test))]
mod amdgpu;
mod nvml;

/// Biblioteca de um driver entre coletas: tentada uma vez só, para não carregar a DLL a cada
/// amostra num PC sem aquela placa.
#[derive(Debug, Default)]
enum Lib<T> {
    #[default]
    Untried,
    Ready(Box<T>),
    Missing,
}

impl<T> Lib<T> {
    fn get_or_load(&mut self, load: impl FnOnce() -> Option<T>) -> Option<&T> {
        if matches!(self, Self::Untried) {
            *self = load().map_or(Self::Missing, |t| Self::Ready(Box::new(t)));
        }
        match self {
            Self::Ready(t) => Some(t),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
struct Libs {
    nvml: Lib<nvml_wrapper::Nvml>,
    #[cfg(windows)]
    adl: Lib<adl::Adl>,
}

/// `system.gpu`.
#[derive(Debug, Default)]
pub struct GpuCollector {
    libs: Arc<Mutex<Libs>>,
}

impl Collector for GpuCollector {
    fn topic(&self) -> &'static str {
        TOPIC_GPU
    }

    fn collect(&self) -> CollectFuture<'_> {
        let libs = Arc::clone(&self.libs);
        Box::pin(async move {
            let gpus = tokio::task::spawn_blocking(move || {
                let mut libs = libs.lock().unwrap_or_else(PoisonError::into_inner);
                gpus(&mut libs)
            })
            .await
            .map_err(|_| CollectError::new("collector_panicked", "O coletor falhou"))?;
            if gpus.is_empty() {
                return Err(CollectError::unavailable(
                    "Nenhuma GPU NVIDIA ou AMD encontrada",
                ));
            }
            serde_json::to_value(gpus)
                .map_err(|_| CollectError::new("collector_invalid", "Amostra inválida"))
        })
    }
}

/// NVIDIA e AMD juntas (um PC pode ter as duas).
fn gpus(libs: &mut Libs) -> Vec<Gpu> {
    let mut list = libs
        .nvml
        .get_or_load(nvml::init)
        .map(nvml::gpus)
        .unwrap_or_default();
    #[cfg(windows)]
    list.extend(
        libs.adl
            .get_or_load(adl::Adl::load)
            .map(adl::Adl::gpus)
            .unwrap_or_default(),
    );
    #[cfg(not(windows))]
    list.extend(amdgpu::gpus());
    list
}
