//! Barra de título da janela sem moldura (RF-APP-005). Só aparece dentro do Tauri.
//! macOS: o semáforo nativo fica por cima, à esquerda. Windows/Linux: controles à
//! direita. Arrastar e o duplo clique para maximizar vêm do `data-tauri-drag-region`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;

use super::icon::{Icon, Svg};
use crate::api;

#[component]
pub fn Titlebar() -> impl IntoView {
    let mac = window()
        .navigator()
        .user_agent()
        .is_ok_and(|ua| ua.contains("Mac OS"));

    view! {
        <div
            data-tauri-drag-region
            class="flex h-8 shrink-0 items-center border-b border-line bg-sidebar select-none"
            class=("pl-20", mac)
        >
            <span data-tauri-drag-region class="px-3 text-xs text-subtle">"CodatOS"</span>
            {(!mac)
                .then(|| {
                    view! {
                        <div class="ml-auto flex h-full">
                            <Control icon=Icon::Minus label="Minimizar" cmd="minimize" />
                            <Control icon=Icon::Square label="Maximizar" cmd="toggle_maximize" />
                            <Control icon=Icon::X label="Fechar" cmd="close" danger=true />
                        </div>
                    }
                })}
        </div>
    }
}

#[component]
fn Control(
    icon: Icon,
    label: &'static str,
    cmd: &'static str,
    #[prop(optional)] danger: bool,
) -> impl IntoView {
    let on_click = move |_| {
        spawn_local(async move {
            let args = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&args, &"label".into(), &JsValue::from_str("main"));
            let _ = api::invoke(&format!("plugin:window|{cmd}"), &args).await;
        });
    };

    view! {
        <button
            type="button"
            aria-label=label
            title=label
            on:click=on_click
            class="grid w-11 place-content-center text-muted transition-colors duration-fast hover:text-fg"
            class=(["hover:bg-danger", "hover:text-white"], danger)
            class=("hover:bg-card-hover", !danger)
        >
            <Svg icon=icon size=14 />
        </button>
    }
}
