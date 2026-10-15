//! UI do CodatOS (ADR-0002). Por enquanto, o "hello" do spike do M0.

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (cliques, set_cliques) = signal(0u32);
    let user_agent = window().navigator().user_agent().unwrap_or_default();

    view! {
        <main style="display:grid;place-content:center;gap:12px;height:100vh;text-align:center">
            <h1 style="margin:0;background:linear-gradient(135deg,#7C5CFF,#3B82F6);\
                       -webkit-background-clip:text;color:transparent">
                "CodatOS"
            </h1>
            <p style="margin:0;color:#8B95AD">"Leptos rodando dentro do Tauri."</p>
            <button
                style="padding:8px 16px;border-radius:8px;border:0;background:#3B82F6;color:#fff"
                on:click=move |_| set_cliques.update(|n| *n += 1)
            >
                "Cliques: " {cliques}
            </button>
            <code style="font-size:12px;color:#5C6680">{user_agent}</code>
        </main>
    }
}
