//! Iniciar com o SO. O app sobe com `--autostart`: fica só na bandeja, sem janela.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use crate::settings::{self, Settings};

pub const FLAG: &str = "--autostart";

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![FLAG]))
}

/// Na primeira execução de um build de release, liga o autostart (saída do M0:
/// "inicia com o sistema"). Builds de dev não se registram no SO.
pub fn configure_first_run(app: &AppHandle, prefs: &mut Settings) {
    if prefs.autostart_configured || cfg!(debug_assertions) {
        return;
    }
    match app.autolaunch().enable() {
        Ok(()) => {
            prefs.autostart_configured = true;
            settings::save(app, prefs);
        }
        Err(error) => tracing::warn!(%error, "não consegui ligar o autostart"),
    }
}
