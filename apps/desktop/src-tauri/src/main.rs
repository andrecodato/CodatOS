//! App desktop do CodatOS (binário `codatos`). A lógica fica em `lib.rs`.

// Sem console extra no Windows em release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // A cópia em `%ProgramFiles%` roda como o serviço de sensores, sem janela nem núcleo.
    #[cfg(windows)]
    match std::env::args().nth(1).as_deref() {
        Some(codatos_sensors::SERVICE_ARG) => {
            let _ = codatos_sensors::service::run();
            return;
        }
        Some(codatos_sensors::ONCE_ARG) => {
            println!("{}", codatos_sensors::service::once());
            return;
        }
        _ => {}
    }
    codatos_desktop::run();
}
