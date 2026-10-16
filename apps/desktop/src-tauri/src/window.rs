//! A janela principal. Fechar destrói a janela e a `WebView` (libera a memória do
//! WebView2, RF-APP-017); abrir de novo cria outra, com posição e tamanho salvos.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Wry};
use tauri_plugin_window_state::StateFlags;

pub const MAIN: &str = "main";

/// Lembra posição e tamanho (RF-APP-005). A visibilidade é decisão do app, não do plugin.
pub fn state_plugin() -> TauriPlugin<Wry> {
    tauri_plugin_window_state::Builder::default()
        .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
        .build()
}

/// Mostra a janela, criando-a se ela foi fechada.
pub fn show(app: &AppHandle) {
    let window = match app.get_webview_window(MAIN) {
        Some(window) => Ok(window),
        None => build(app),
    };
    match window {
        Ok(window) => {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
        Err(error) => tracing::error!(%error, "não consegui abrir a janela"),
    }
}

fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::default())
        .title("CodatOS")
        .inner_size(1100.0, 720.0)
        .min_inner_size(800.0, 560.0);

    // Barra de título customizada (RF-APP-005): no macOS o semáforo nativo fica por
    // cima da UI; no Windows/Linux a UI desenha os controles à direita.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false);

    builder.build()
}
