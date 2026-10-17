//! Ícone na bandeja: clique abre a janela; o menu tem "Abrir", "Abrir no navegador" e "Sair".

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::{Core, window};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir CodatOS", true, None::<&str>)?;
    let browser = MenuItem::with_id(app, "browser", "Abrir no navegador", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &browser, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("CodatOS")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => window::show(app),
            "browser" => open_in_browser(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Abre a UI no navegador padrão com um token de uso único (vale 60 s, troca por cookie).
fn open_in_browser(app: &AppHandle) {
    let Some(url) = app.state::<Core>().browser_url() else {
        tracing::warn!("núcleo ainda subindo; tente de novo em instantes");
        return;
    };
    if let Err(error) = app.opener().open_url(url, None::<&str>) {
        tracing::warn!(%error, "não consegui abrir o navegador");
    }
}
