//! UI do CodatOS (ADR-0002): Leptos em CSR, compilada para WASM pelo Trunk.

mod api;
mod app;
mod components;
mod connections;
mod dashboard;
mod ids;
mod nav;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
