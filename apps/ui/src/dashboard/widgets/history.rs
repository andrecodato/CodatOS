//! Histórico: CPU e memória na última hora (RF-SYS-001/002).
//!
//! O núcleo só coleta enquanto alguém assina (consumo em repouso), então o histórico vive na
//! UI: amostras de `system.metrics` enquanto a janela está aberta e visível. Com a janela
//! escondida o gráfico mostra o intervalo vazio em vez de inventar uma linha.

use std::collections::VecDeque;
use std::fmt::Write as _;

use codatos_protocol::system::{SystemMetrics, TOPIC_METRICS};
use leptos::prelude::*;
use serde_json::Value;

use super::format::percent;
use crate::api;
use crate::dashboard::state::sample_view;

/// Janela do gráfico.
const WINDOW_MS: f64 = 60.0 * 60.0 * 1000.0;
/// Um intervalo maior que isso sem amostras quebra a linha.
const GAP_MS: f64 = 10_000.0;
/// Largura do `viewBox`; a altura é 100 (o próprio percentual).
const WIDTH: f64 = 600.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    t: f64,
    cpu: f64,
    memory: f64,
}

thread_local! {
    // Fora de qualquer componente: sobrevive ao modo edição e a widgets removidos.
    static BUFFER: ArcRwSignal<VecDeque<Point>> = ArcRwSignal::new(VecDeque::new());
}

fn buffer() -> ArcRwSignal<VecDeque<Point>> {
    BUFFER.with(Clone::clone)
}

fn record(buffer: &mut VecDeque<Point>, point: Point) {
    // Dois widgets de histórico na grade recebem a mesma amostra: guarda uma só.
    if buffer.back().is_some_and(|last| point.t - last.t < 1000.0) {
        return;
    }
    buffer.push_back(point);
    while buffer
        .front()
        .is_some_and(|first| point.t - first.t > WINDOW_MS)
    {
        buffer.pop_front();
    }
}

/// Pontos de `polyline` em trechos contínuos, do mais antigo (x = 0) até `now` (x = 600).
fn segments(points: &VecDeque<Point>, now: f64, value: fn(&Point) -> f64) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut previous: Option<f64> = None;
    for p in points {
        if previous.is_none_or(|t| p.t - t > GAP_MS) {
            out.push(String::new());
        }
        previous = Some(p.t);
        let x = WIDTH * (1.0 - (now - p.t) / WINDOW_MS);
        let y = 100.0 - value(p).clamp(0.0, 100.0);
        if let Some(last) = out.last_mut() {
            if !last.is_empty() {
                last.push(' ');
            }
            let _ = write!(last, "{x:.1},{y:.1}");
        }
    }
    out
}

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_METRICS, None);
    let buf = buffer();
    Effect::new({
        let buf = buf.clone();
        move |_| {
            let Some(Ok(value)) = sample.get() else {
                return;
            };
            let Ok(m) = serde_json::from_value::<SystemMetrics>(value) else {
                return;
            };
            let point = Point {
                t: js_sys::Date::now(),
                cpu: f64::from(m.cpu.total),
                memory: percent(m.memory.used, m.memory.total),
            };
            buf.update(|b| record(b, point));
        }
    });
    sample_view(sample, move |_: SystemMetrics| {
        let now = js_sys::Date::now();
        let (cpu, memory, last) = buf.with(|b| {
            (
                segments(b, now, |p| p.cpu),
                segments(b, now, |p| p.memory),
                b.back().copied(),
            )
        });
        view! { <Chart cpu=cpu memory=memory last=last /> }
    })
}

#[component]
fn Chart(cpu: Vec<String>, memory: Vec<String>, last: Option<Point>) -> impl IntoView {
    let line = |points: Vec<String>, class: &'static str| {
        points
            .into_iter()
            .map(|p| {
                view! {
                    <polyline
                        points=p
                        fill="none"
                        stroke-width="2"
                        stroke-linejoin="round"
                        vector-effect="non-scaling-stroke"
                        class=class
                    />
                }
            })
            .collect_view()
    };
    let value =
        |f: fn(&Point) -> f64| last.map_or_else(|| "—".to_owned(), |p| format!("{:.0}%", f(&p)));
    view! {
        <div class="flex h-full flex-col gap-2">
            <div class="flex gap-4 text-xs text-muted">
                <span class="flex items-center gap-1.5">
                    <span class="size-2 rounded-full bg-chart-1"></span>
                    "CPU "
                    <span class="text-fg tabular-nums">{value(|p| p.cpu)}</span>
                </span>
                <span class="flex items-center gap-1.5">
                    <span class="size-2 rounded-full bg-chart-4"></span>
                    "Memória "
                    <span class="text-fg tabular-nums">{value(|p| p.memory)}</span>
                </span>
            </div>
            <svg
                viewBox="0 0 600 100"
                preserveAspectRatio="none"
                class="min-h-0 w-full flex-1"
                role="img"
                aria-label="Uso de CPU e memória na última hora"
            >
                {[25, 50, 75]
                    .map(|y| view! {
                        <line x1="0" x2="600" y1=y y2=y class="stroke-line" stroke-width="1" vector-effect="non-scaling-stroke" />
                    })}
                {line(memory, "stroke-chart-4")}
                {line(cpu, "stroke-chart-1")}
            </svg>
            <div class="flex justify-between text-[11px] text-subtle">
                <span>"60 min"</span>
                <span>"agora"</span>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(t: f64, cpu: f64) -> Point {
        Point {
            t,
            cpu,
            memory: 0.0,
        }
    }

    #[test]
    fn guarda_so_a_ultima_hora_sem_repetir() {
        let mut b = VecDeque::new();
        record(&mut b, p(0.0, 1.0));
        record(&mut b, p(500.0, 2.0)); // mesma amostra vinda de outro widget
        record(&mut b, p(2000.0, 3.0));
        record(&mut b, p(WINDOW_MS + 1500.0, 4.0));
        assert_eq!(b.iter().map(|x| x.cpu).collect::<Vec<_>>(), [3.0, 4.0]);
    }

    #[test]
    fn intervalo_sem_amostras_quebra_a_linha() {
        let now = WINDOW_MS;
        let b: VecDeque<Point> = [p(0.0, 0.0), p(2000.0, 50.0), p(60_000.0, 100.0)].into();
        let s = segments(&b, now, |x| x.cpu);
        assert_eq!(s, ["0.0,100.0 0.3,50.0", "10.0,0.0"]);
    }
}
