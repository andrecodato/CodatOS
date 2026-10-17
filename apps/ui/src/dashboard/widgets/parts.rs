//! Peças visuais comuns aos widgets.

use leptos::prelude::*;

/// Cor da barra por faixa de uso. A cor acompanha um número, nunca aparece sozinha.
pub fn level(value: f64) -> &'static str {
    if value >= 90.0 {
        "bg-danger"
    } else if value >= 75.0 {
        "bg-warning"
    } else {
        "bg-primary"
    }
}

/// Barra fina de 0 a 100 %.
#[component]
pub fn Bar(value: f64, label: String) -> impl IntoView {
    let value = value.clamp(0.0, 100.0);
    view! {
        <div
            class="h-1.5 overflow-hidden rounded-full bg-card-hover"
            role="meter"
            aria-label=label
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow=format!("{value:.0}")
        >
            <div class=format!("h-full rounded-full {}", level(value)) style=format!("width: {value:.1}%")></div>
        </div>
    }
}

/// Percentual grande com barra e um detalhe embaixo.
#[component]
pub fn Meter(label: &'static str, value: f64, detail: String) -> impl IntoView {
    let value = value.clamp(0.0, 100.0);
    view! {
        <div class="flex min-w-0 flex-col justify-center gap-1.5">
            <p class="text-xs text-muted">{label}</p>
            <p class="text-2xl leading-none font-semibold tabular-nums">{format!("{value:.0}%")}</p>
            <Bar value=value label=label.to_owned() />
            <p class="truncate text-xs text-subtle">{detail}</p>
        </div>
    }
}

/// Ponto de estado: verde ligado, cinza parado. O texto ao lado diz o estado.
#[component]
pub fn Dot(on: bool) -> impl IntoView {
    let color = if on { "bg-success" } else { "bg-subtle" };
    view! { <span class=format!("size-2 shrink-0 rounded-full {color}") aria-hidden="true"></span> }
}
