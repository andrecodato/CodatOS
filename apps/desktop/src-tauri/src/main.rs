//! App desktop do CodatOS. Por enquanto, o shell mínimo do spike do M0.

// Sem console extra no Windows em release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Tauri");
}
