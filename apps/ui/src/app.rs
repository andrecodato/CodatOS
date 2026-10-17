//! Layout da janela: sidebar à esquerda, topbar e conteúdo à direita.

use leptos::prelude::*;

use crate::api::{self, CoreStatus};
use crate::components::greeting::{Greeting, Profile};
use crate::components::{sidebar::Sidebar, titlebar::Titlebar, topbar::Topbar};
use crate::dashboard::Dashboard;

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal(CoreStatus::Connecting);
    api::connect(set_status);
    // Os widgets mostram "desatualizado" quando o núcleo cai.
    provide_context(status);
    provide_context(Profile::load(status));

    view! {
        <div class="flex h-screen flex-col overflow-hidden bg-base font-sans text-fg">
            {api::in_tauri().then(|| view! { <Titlebar /> })}
            <div class="flex min-h-0 flex-1">
                <Sidebar />
                <div class="flex min-w-0 flex-1 flex-col">
                    <Topbar status=status />
                    <main class="flex-1 overflow-auto p-6">
                        <Greeting />
                        <Dashboard status=status />
                    </main>
                </div>
            </div>
        </div>
    }
}
