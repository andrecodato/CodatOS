//! Peças visuais comuns aos widgets.

use std::time::Duration;

use leptos::{prelude::*, task::spawn_local};
use serde_json::Value;

use crate::api;
use crate::components::icon::{Icon, Svg};

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

/// Botão de ação que chama um método do núcleo. Com `confirm`, o primeiro clique só pede
/// confirmação (por 3 s); ações que derrubam coisas em execução usam isso.
#[component]
pub fn ActionButton(
    icon: Icon,
    label: String,
    method: &'static str,
    params: Value,
    #[prop(optional)] confirm: bool,
    /// Mensagem de erro da última tentativa, para o widget mostrar.
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let armed = RwSignal::new(false);
    let params = StoredValue::new(params);
    let run = move |_| {
        if busy.get_untracked() {
            return;
        }
        if confirm && !armed.get_untracked() {
            armed.set(true);
            set_timeout(
                move || {
                    let _ = armed.try_set(false);
                },
                Duration::from_secs(3),
            );
            return;
        }
        armed.set(false);
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = api::call(method, params.get_value()).await;
            let _ = busy.try_set(false);
            if let Err(e) = result {
                let _ = error.try_set(Some(e.message));
            }
        });
    };
    let title = label.clone();
    view! {
        <button
            type="button"
            class=move || format!(
                "inline-flex items-center gap-1 rounded px-1.5 py-1 text-xs hover:bg-card-hover disabled:opacity-50 {}",
                if armed.get() { "text-danger" } else { "text-muted hover:text-fg" },
            )
            title=title
            aria-label=label
            disabled=move || busy.get()
            on:click=run
        >
            <Svg icon=icon size=14 />
            {move || armed.get().then_some("Confirmar?")}
        </button>
    }
}
