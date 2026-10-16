//! Menu lateral (RF-APP-006). No M0 é só visual: a navegação chega no M1.

use leptos::prelude::*;

use super::icon::{Icon, Svg};

const ITEMS: [(Icon, &str); 7] = [
    (Icon::Dashboard, "Dashboard"),
    (Icon::Wallet, "Financeiro"),
    (Icon::Kanban, "TaskHub"),
    (Icon::Server, "Conexões"),
    (Icon::FolderGit, "Projetos"),
    (Icon::Cpu, "Sistema"),
    (Icon::Settings, "Configurações"),
];

#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="flex w-[220px] shrink-0 flex-col border-r border-line bg-sidebar">
            <div class="flex items-center gap-2.5 px-5 py-5">
                <span class="grid size-8 place-content-center rounded-md bg-brand text-sm font-bold text-white">
                    "C"
                </span>
                <span class="text-[15px] font-semibold tracking-tight">"CodatOS"</span>
            </div>
            <nav aria-label="Principal" class="flex flex-1 flex-col gap-0.5 px-3">
                {ITEMS
                    .into_iter()
                    .enumerate()
                    .map(|(i, (icon, label))| {
                        let active = i == 0;
                        view! {
                            <a
                                href="#"
                                aria-current=active.then_some("page")
                                class="flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors duration-fast"
                                class=(["bg-primary-soft", "text-fg", "font-medium"], active)
                                class=(["text-muted", "hover:bg-card-hover", "hover:text-fg"], !active)
                            >
                                <Svg icon=icon />
                                {label}
                            </a>
                        }
                    })
                    .collect_view()}
            </nav>
            <div class="flex flex-col gap-2 border-t border-line p-3">
                <button
                    type="button"
                    class="flex items-center justify-between rounded-md border border-line bg-input px-3 py-2 text-left text-sm text-muted"
                >
                    <span>"Pessoal"</span>
                    <Svg icon=Icon::ChevronsUpDown size=14 />
                </button>
                <div class="flex items-center gap-3 rounded-md px-2 py-1.5">
                    <span class="grid size-8 place-content-center rounded-full bg-card-hover text-xs font-semibold">
                        "L"
                    </span>
                    <div class="min-w-0">
                        <p class="truncate text-sm font-medium">"Modo local"</p>
                        <p class="truncate text-xs text-subtle">"Sem conta"</p>
                    </div>
                </div>
            </div>
        </aside>
    }
}
