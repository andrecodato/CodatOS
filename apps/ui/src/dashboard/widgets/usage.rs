//! Uso do PC: CPU e memória agora (`system.metrics`), com a temperatura da CPU quando
//! há sensor (`system.cpuSensors`). No Windows, sem o serviço de sensores, oferece
//! instalá-lo (`system.enableCpuSensors`, um pedido de administrador).

use std::fmt::Write as _;

use codatos_protocol::system::{
    CPU_SENSORS_NOT_INSTALLED, CpuSensors, METHOD_ENABLE_CPU_SENSORS, SystemMetrics,
    TOPIC_CPU_SENSORS, TOPIC_METRICS,
};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};

use super::format::{bytes, decimal, percent};
use super::parts::Meter;
use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_METRICS, None);
    let sensors = api::subscribe(TOPIC_CPU_SENSORS, None);
    let not_installed = move || {
        sensors
            .get()
            .is_some_and(|s| s.is_err_and(|p| p.code == CPU_SENSORS_NOT_INSTALLED))
    };
    sample_view(sample, move |m: SystemMetrics| {
        let mut cpu_detail = format!("{} núcleos", m.cpu.cores.len());
        if m.cpu.frequency_mhz > 0 {
            #[allow(clippy::cast_precision_loss)] // MHz cabem folgados num f64
            let ghz = m.cpu.frequency_mhz as f64 / 1000.0;
            let _ = write!(cpu_detail, " · {} GHz", decimal(ghz, 1));
        }
        // Sem sensor, a temperatura só não aparece: não é erro do widget.
        let temperature = sensors
            .get()
            .and_then(Result::ok)
            .and_then(|v| serde_json::from_value::<CpuSensors>(v).ok())
            .and_then(|s| s.temperature_c);
        if let Some(t) = temperature {
            let _ = write!(cpu_detail, " · {t:.0} °C");
        }
        view! {
            <div class="grid h-full grid-cols-2 gap-4">
                <Meter label="CPU" value=f64::from(m.cpu.total) detail=cpu_detail />
                <Meter
                    label="Memória"
                    value=percent(m.memory.used, m.memory.total)
                    detail=format!("{} de {}", bytes(m.memory.used), bytes(m.memory.total))
                />
                {move || not_installed().then(|| view! { <EnableSensors /> })}
            </div>
        }
    })
}

/// Instala o PawnIO e o serviço de sensores. O Windows pede permissão de administrador; a
/// próxima amostra de `system.cpuSensors` já traz a temperatura.
#[component]
fn EnableSensors() -> impl IntoView {
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let run = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = api::call(METHOD_ENABLE_CPU_SENSORS, json!({})).await;
            let _ = busy.try_set(false);
            if let Err(e) = result {
                let _ = error.try_set(Some(e.message));
            }
        });
    };
    view! {
        <div class="col-span-2 flex min-w-0 items-center gap-2 text-xs">
            <button
                type="button"
                class="rounded px-1.5 py-1 text-primary hover:bg-card-hover disabled:opacity-50"
                title="Instala o driver PawnIO e o serviço de sensores do CodatOS (pede administrador uma vez)"
                disabled=move || busy.get()
                on:click=run
            >
                {move || if busy.get() { "Aguardando o Windows…" } else { "Ativar temperatura da CPU" }}
            </button>
            {move || error.get().map(|e| view! { <span class="truncate text-danger" title=e.clone()>{e.clone()}</span> })}
        </div>
    }
}
