//! Layout da janela: sidebar à esquerda, topbar e conteúdo à direita.

use leptos::prelude::*;

use crate::api::{self, CoreStatus};
use crate::components::{sidebar::Sidebar, titlebar::Titlebar, topbar::Topbar};
use crate::dashboard::Dashboard;

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal(CoreStatus::Connecting);
    api::connect(set_status);
    // Os widgets mostram "desatualizado" quando o núcleo cai.
    provide_context(status);

    view! {
        <div class="flex h-screen flex-col overflow-hidden bg-base font-sans text-fg">
            {api::in_tauri().then(|| view! { <Titlebar /> })}
            <div class="flex min-h-0 flex-1">
                <Sidebar />
                <div class="flex min-w-0 flex-1 flex-col">
                    <Topbar status=status />
                    <main class="flex-1 overflow-auto p-6">
                        <h1 class="text-[26px] leading-8 font-semibold">"Olá 👋"</h1>
                        <p class="mt-1 text-sm text-muted">"Seu universo, em um só lugar."</p>
                        <Dashboard status=status />
                    </main>
                </div>
            </div>
        </div>
    }
}
