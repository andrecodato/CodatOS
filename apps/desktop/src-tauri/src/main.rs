//! App desktop do CodatOS (binário `codatos`). A lógica fica em `lib.rs`.

// Sem console extra no Windows em release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    codatos_desktop::run();
}
