//! Relógio: hora e data deste PC. Não depende do núcleo.

use std::time::Duration;

use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::JsValue;

pub fn render(_config: &Value) -> AnyView {
    view! { <Clock /> }.into_any()
}

#[component]
fn Clock() -> impl IntoView {
    let (now, set_now) = signal(read());
    let handle = set_interval_with_handle(move || set_now.set(read()), Duration::from_secs(1)).ok();
    on_cleanup(move || {
        if let Some(handle) = handle {
            handle.clear();
        }
    });

    view! {
        <div class="flex h-full flex-col justify-center">
            <p class="text-4xl leading-none font-semibold tabular-nums">{move || now.get().0}</p>
            <p class="mt-2 text-sm text-muted first-letter:uppercase">{move || now.get().1}</p>
        </div>
    }
}

/// ("14:05:09", "sexta-feira, 17 de outubro").
fn read() -> (String, String) {
    let date = js_sys::Date::new_0();
    let opts = |pairs: &[(&str, &str)]| {
        let o = js_sys::Object::new();
        for (k, v) in pairs {
            let _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), &JsValue::from_str(v));
        }
        o
    };
    let time = date.to_locale_time_string_with_options(
        "pt-BR",
        &opts(&[
            ("hour", "2-digit"),
            ("minute", "2-digit"),
            ("second", "2-digit"),
        ]),
    );
    let day = date.to_locale_date_string(
        "pt-BR",
        &opts(&[("weekday", "long"), ("day", "numeric"), ("month", "long")]),
    );
    (String::from(time), String::from(day))
}
