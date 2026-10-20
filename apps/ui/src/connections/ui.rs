//! Peças comuns das telas de conexões: estilos de campo e botão, diálogo e chamadas ao
//! núcleo com mensagens legíveis.

use leptos::{ev, prelude::*};
use serde::Serialize;
use serde_json::Value;

use crate::api;
use crate::components::icon::{Icon, Svg};

pub const FIELD: &str = "w-full rounded-md border border-line bg-input px-3 py-2 text-sm placeholder:text-subtle focus:outline-none";
pub const LABEL: &str = "block text-xs text-muted";
pub const BUTTON: &str = "inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-card-hover disabled:opacity-50";
pub const PRIMARY: &str = "inline-flex items-center gap-2 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-hover disabled:opacity-50";
pub const DANGER: &str = "inline-flex items-center gap-2 rounded-md px-3 py-1.5 text-sm text-danger hover:bg-card-hover disabled:opacity-50";
pub const GHOST: &str = "rounded-md p-1.5 text-muted hover:bg-card-hover hover:text-fg";

/// O que um diálogo de edição abre: um item novo ou um existente.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit<T> {
    New,
    Existing(T),
}

impl<T> Edit<T> {
    pub fn existing(self) -> Option<T> {
        match self {
            Self::New => None,
            Self::Existing(item) => Some(item),
        }
    }
}

/// Chama um método do núcleo. O erro já vem como frase para mostrar; cofre trancado ou
/// ausente vira um pedido para abrir o cofre.
pub async fn call(method: &str, params: &impl Serialize) -> Result<Value, String> {
    call_coded(method, params)
        .await
        .map_err(|(_, message)| message)
}

/// Como [`call`], com o código estável do erro junto (ex.: `key_passphrase_required`).
pub async fn call_coded(method: &str, params: &impl Serialize) -> Result<Value, (String, String)> {
    let params = serde_json::to_value(params).map_err(|e| (String::new(), e.to_string()))?;
    api::call(method, params).await.map_err(|e| {
        let code = e.data.as_ref().map(|p| p.code.clone()).unwrap_or_default();
        let message = match code.as_str() {
            "vault_locked" => {
                "O cofre está trancado. Desbloqueie o cofre para guardar senhas e chaves."
                    .to_owned()
            }
            "vault_absent" => "Crie o cofre antes de guardar senhas e chaves.".to_owned(),
            _ => e.message,
        };
        (code, message)
    })
}

/// Copia para a área de transferência (sem resposta: o navegador pode negar em silêncio).
pub fn copy(text: &str) {
    let _ = window().navigator().clipboard().write_text(text);
}

/// Diálogo modal centrado. Fecha no Esc e no clique fora.
#[component]
pub fn Dialog(
    title: String,
    on_close: Callback<()>,
    #[prop(default = "w-[min(32rem,calc(100vw-2rem))]")] width: &'static str,
    children: Children,
) -> impl IntoView {
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            on_close.run(());
        }
    });
    on_cleanup(move || keys.remove());
    let label = title.clone();
    view! {
        <div class="fixed inset-0 z-40 bg-app/60" on:click=move |_| on_close.run(())></div>
        <div
            role="dialog"
            aria-modal="true"
            aria-label=label
            class=format!("fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl border border-line bg-sidebar shadow-card {width}")
        >
            <header class="flex items-center gap-2 border-b border-line px-5 py-4">
                <h2 class="text-sm font-semibold">{title}</h2>
                <button
                    type="button"
                    class=format!("ml-auto {GHOST}")
                    aria-label="Fechar"
                    on:click=move |_| on_close.run(())
                >
                    <Svg icon=Icon::X size=16 />
                </button>
            </header>
            {children()}
        </div>
    }
}

/// Mensagem de erro de um formulário, quando houver.
#[component]
pub fn FormError(error: RwSignal<Option<String>>) -> impl IntoView {
    move || {
        error.get().map(|e| {
            view! { <p role="alert" class="text-sm text-danger">{e}</p> }
        })
    }
}
