//! xterm.js (`apps/ui/assets/xterm/`) pelo `wasm-bindgen`. Os scripts entram na página só
//! quando o primeiro terminal abre: quem nunca abre um terminal não paga os ~500 KB.

use std::cell::RefCell;

use js_sys::{Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Element;

#[wasm_bindgen]
extern "C" {
    /// `Terminal` do xterm.js (global do bundle UMD).
    #[derive(Debug, Clone)]
    pub type Terminal;

    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> Terminal;

    #[wasm_bindgen(method)]
    pub fn open(this: &Terminal, parent: &Element);

    /// `callback` roda depois que o xterm processou os dados: é a hora de confirmar ao núcleo.
    #[wasm_bindgen(method)]
    pub fn write(this: &Terminal, data: &Uint8Array, callback: &Function);

    #[wasm_bindgen(method, js_name = write)]
    pub fn write_str(this: &Terminal, data: &str);

    #[wasm_bindgen(method, js_name = onData)]
    pub fn on_data(this: &Terminal, listener: &Function) -> JsValue;

    /// Sequências fora do UTF-8 (alguns relatórios de mouse): cada caractere é um byte.
    #[wasm_bindgen(method, js_name = onBinary)]
    pub fn on_binary(this: &Terminal, listener: &Function) -> JsValue;

    /// Recebe `{cols, rows}`.
    #[wasm_bindgen(method, js_name = onResize)]
    pub fn on_resize(this: &Terminal, listener: &Function) -> JsValue;

    #[wasm_bindgen(method, js_name = loadAddon)]
    pub fn load_addon(this: &Terminal, addon: &FitAddon);

    #[wasm_bindgen(method)]
    pub fn focus(this: &Terminal);

    #[wasm_bindgen(method)]
    pub fn dispose(this: &Terminal);

    #[wasm_bindgen(method, getter)]
    pub fn cols(this: &Terminal) -> u16;

    #[wasm_bindgen(method, getter)]
    pub fn rows(this: &Terminal) -> u16;
}

#[wasm_bindgen]
extern "C" {
    /// `FitAddon.FitAddon`: ajusta colunas e linhas ao tamanho do elemento.
    /// O global do bundle é o módulo; a classe é `FitAddon.FitAddon` ([`FitAddon::create`]).
    #[derive(Debug, Clone)]
    pub type FitAddon;

    #[wasm_bindgen(method)]
    pub fn fit(this: &FitAddon);
}

impl FitAddon {
    pub fn create() -> Result<Self, JsValue> {
        let module = Reflect::get(&js_sys::global(), &"FitAddon".into())?;
        let class: Function = Reflect::get(&module, &"FitAddon".into())?.dyn_into()?;
        Ok(Reflect::construct(&class, &js_sys::Array::new())?.unchecked_into())
    }
}

/// Opções do terminal no tema do app (`styles/tokens.css`, doc 07: JetBrains Mono 13/20).
pub fn options() -> JsValue {
    let theme = Object::new();
    for (key, value) in [
        ("background", "#060a14"),
        ("foreground", "#e6eaf2"),
        ("cursor", "#3b82f6"),
        ("cursorAccent", "#060a14"),
        ("selectionBackground", "rgba(59, 130, 246, 0.35)"),
        ("black", "#1c2640"),
        ("red", "#f43f5e"),
        ("green", "#22c55e"),
        ("yellow", "#f59e0b"),
        ("blue", "#3b82f6"),
        ("magenta", "#a855f7"),
        ("cyan", "#06b6d4"),
        ("white", "#e6eaf2"),
        ("brightBlack", "#5c6680"),
        ("brightRed", "#fb7185"),
        ("brightGreen", "#4ade80"),
        ("brightYellow", "#fbbf24"),
        ("brightBlue", "#60a5fa"),
        ("brightMagenta", "#c084fc"),
        ("brightCyan", "#22d3ee"),
        ("brightWhite", "#ffffff"),
    ] {
        set(&theme, key, &value.into());
    }
    let options = Object::new();
    set(
        &options,
        "fontFamily",
        &"\"JetBrains Mono Variable\", ui-monospace, \"Cascadia Mono\", monospace".into(),
    );
    set(&options, "fontSize", &13.into());
    set(&options, "lineHeight", &1.2.into());
    set(&options, "cursorBlink", &true.into());
    set(&options, "scrollback", &5000.into());
    set(&options, "theme", &theme);
    options.into()
}

fn set(target: &Object, key: &str, value: &JsValue) {
    let _ = Reflect::set(target, &key.into(), value);
}

thread_local! {
    /// Carga dos scripts, compartilhada por quem pedir enquanto ela anda.
    static LOADING: RefCell<Option<Promise>> = const { RefCell::new(None) };
}

/// Põe o xterm.js na página (uma vez só). Falha se os arquivos não carregarem.
pub async fn load() -> Result<(), JsValue> {
    let promise = LOADING.with(|l| {
        l.borrow_mut()
            .get_or_insert_with(|| {
                let css = inject(
                    "link",
                    &[("rel", "stylesheet"), ("href", "/xterm/xterm.css")],
                );
                let main = inject("script", &[("src", "/xterm/xterm.js")]);
                let fit = inject("script", &[("src", "/xterm/addon-fit.js")]);
                Promise::all(&js_sys::Array::of3(&css, &main, &fit))
            })
            .clone()
    });
    let result = JsFuture::from(promise).await.map(|_| ());
    if result.is_ok() {
        load_font().await;
    }
    if result.is_err() {
        // Deixa tentar de novo na próxima aba.
        LOADING.with(|l| l.borrow_mut().take());
    }
    result
}

/// O xterm mede a largura do caractere ao abrir: com a fonte ainda carregando, mediria a de
/// reserva. Sem a fonte (falhou), segue com a de reserva mesmo.
async fn load_font() {
    let document = leptos::prelude::document();
    let Ok(fonts) = Reflect::get(&document, &"fonts".into()) else {
        return;
    };
    let Ok(load) = Reflect::get(&fonts, &"load".into()).and_then(JsCast::dyn_into::<Function>)
    else {
        return;
    };
    if let Ok(promise) = load.call1(&fonts, &"13px \"JetBrains Mono Variable\"".into()) {
        let _ = JsFuture::from(Promise::resolve(&promise)).await;
    }
}

/// Acrescenta um `<link>` ou `<script>` ao `<head>`; a promessa resolve no `load`.
fn inject(tag: &str, attrs: &[(&str, &str)]) -> Promise {
    let document = leptos::prelude::document();
    let element = match document.create_element(tag) {
        Ok(element) => element,
        Err(err) => return Promise::reject(&err),
    };
    for (name, value) in attrs {
        let _ = element.set_attribute(name, value);
    }
    let target = element.clone();
    let promise = Promise::new(&mut |resolve, reject| {
        let _ = target.add_event_listener_with_callback("load", &resolve);
        let _ = target.add_event_listener_with_callback("error", &reject);
    });
    if let Some(head) = document.head() {
        let _ = head.append_child(&element);
    }
    promise
}
