//! Barra superior: busca (Ctrl + K no M1), avisos, estado do núcleo, data e hora.

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::JsValue;

use super::icon::{Icon, Svg};
use crate::api::CoreStatus;

#[component]
pub fn Topbar(status: ReadSignal<CoreStatus>) -> impl IntoView {
    let (now, set_now) = signal(agora());
    // Vive enquanto a janela existir; o Tauri destrói a WebView inteira ao fechar.
    let _ = set_interval_with_handle(move || set_now.set(agora()), Duration::from_secs(15));

    view! {
        <header class="flex h-14 shrink-0 items-center gap-4 border-b border-line px-6">
            <label class="flex w-full max-w-sm items-center gap-2 rounded-md border border-line bg-input px-3 py-1.5 text-sm text-subtle">
                <Svg icon=Icon::Search size=16 />
                <input
                    type="search"
                    placeholder="Buscar… (Ctrl + K)"
                    disabled
                    class="w-full bg-transparent placeholder:text-subtle focus:outline-none"
                />
            </label>
            <div class="ml-auto flex items-center gap-4 text-muted">
                <CoreIndicator status=status />
                <button type="button" aria-label="Notificações" class="rounded-md p-1.5 hover:bg-card-hover hover:text-fg">
                    <Svg icon=Icon::Bell />
                </button>
                <button type="button" aria-label="Alertas" class="rounded-md p-1.5 hover:bg-card-hover hover:text-fg">
                    <Svg icon=Icon::Alert />
                </button>
                <div class="text-right text-xs leading-4 tabular-nums">
                    <p class="text-fg">{move || now.get().1}</p>
                    <p>{move || now.get().0}</p>
                </div>
            </div>
        </header>
    }
}

/// Ponto de status + texto (a cor nunca é o único sinal).
#[component]
fn CoreIndicator(status: ReadSignal<CoreStatus>) -> impl IntoView {
    let label = move || match status.get() {
        CoreStatus::Connecting => "Conectando ao núcleo…",
        CoreStatus::Connected => "Núcleo conectado",
        CoreStatus::Offline => "Núcleo offline",
    };
    let dot = move || match status.get() {
        CoreStatus::Connecting => "bg-warning",
        CoreStatus::Connected => "bg-success",
        CoreStatus::Offline => "bg-subtle",
    };

    view! {
        <span role="status" class="flex items-center gap-2 text-xs">
            <span class=move || format!("size-2 rounded-full {}", dot())></span>
            {label}
        </span>
    }
}

/// Data ("qui., 16 de out.") e hora ("04:43") no fuso e no formato pt-BR.
fn agora() -> (String, String) {
    let date = js_sys::Date::new_0();
    let opts = js_sys::Object::new();
    let set = |k: &str, v: &str| {
        let _ = js_sys::Reflect::set(&opts, &JsValue::from_str(k), &JsValue::from_str(v));
    };
    set("weekday", "short");
    set("day", "numeric");
    set("month", "short");
    let dia = date.to_locale_date_string("pt-BR", &opts);

    let opts = js_sys::Object::new();
    for k in ["hour", "minute"] {
        let _ = js_sys::Reflect::set(&opts, &JsValue::from_str(k), &JsValue::from_str("2-digit"));
    }
    let hora = date.to_locale_time_string_with_options("pt-BR", &opts);

    (String::from(dia), String::from(hora))
}
