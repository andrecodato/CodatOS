//! Layout da janela: sidebar à esquerda, topbar e conteúdo à direita.

use leptos::prelude::*;

use crate::api::{self, CoreStatus};
use crate::components::greeting::{Greeting, Profile};
use crate::components::icon::Svg;
use crate::components::{sidebar::Sidebar, titlebar::Titlebar, topbar::Topbar};
use crate::connections::Connections;
use crate::dashboard::Dashboard;
use crate::nav::{self, Page};

#[component]
pub fn App() -> impl IntoView {
    let (status, set_status) = signal(CoreStatus::Connecting);
    api::connect(set_status);
    // Os widgets mostram "desatualizado" quando o núcleo cai.
    provide_context(status);
    provide_context(Profile::load(status));
    let page = nav::page();

    view! {
        <div class="flex h-screen flex-col overflow-hidden bg-base font-sans text-fg">
            {api::in_tauri().then(|| view! { <Titlebar /> })}
            <div class="flex min-h-0 flex-1">
                <Sidebar page=page />
                <div class="flex min-w-0 flex-1 flex-col">
                    <Topbar status=status />
                    <main class="flex-1 overflow-auto p-6">
                        {move || match page.get() {
                            Page::Dashboard => view! {
                                <Greeting />
                                <Dashboard status=status />
                            }
                            .into_any(),
                            Page::Connections => view! { <Connections status=status /> }.into_any(),
                            other => view! { <ComingSoon page=other /> }.into_any(),
                        }}
                    </main>
                </div>
            </div>
        </div>
    }
}

/// Tela que ainda não existe: o módulo chega num marco seguinte (`docs/09-roadmap.md`).
#[component]
fn ComingSoon(page: Page) -> impl IntoView {
    view! {
        <section class="grid h-full place-content-center justify-items-center gap-3 text-center">
            <span class="rounded-lg bg-primary-soft p-3 text-primary">
                <Svg icon=page.icon() size=24 />
            </span>
            <h1 class="text-lg font-semibold">{page.label()}</h1>
            <p class="max-w-sm text-sm text-muted">"Esta tela chega num próximo marco do CodatOS."</p>
        </section>
    }
}
