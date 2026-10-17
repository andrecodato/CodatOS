//! Layout da janela: sidebar à esquerda, topbar e conteúdo à direita.

use leptos::prelude::*;

use crate::api::{self, CoreStatus};
use crate::components::{sidebar::Sidebar, topbar::Topbar};

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal(CoreStatus::Connecting);
    api::connect(set_status);

    view! {
        <div class="flex h-screen overflow-hidden bg-base font-sans text-fg">
            <Sidebar />
            <div class="flex min-w-0 flex-1 flex-col">
                <Topbar status=status />
                <main class="flex-1 overflow-auto p-6">
                    <h1 class="text-[26px] leading-8 font-semibold">"Olá 👋"</h1>
                    <p class="mt-1 text-sm text-muted">"Seu universo, em um só lugar."</p>
                    <section class="mt-6 rounded-lg border border-line bg-card p-5 shadow-card">
                        <h2 class="text-sm font-semibold">"Dashboard"</h2>
                        <p class="mt-2 text-sm text-muted">
                            "Os widgets de sistema, clima e projetos chegam no M1."
                        </p>
                    </section>
                </main>
            </div>
        </div>
    }
}
