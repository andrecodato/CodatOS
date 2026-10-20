//! Snippets no terminal (RF-CON-023): menu para mandar um comando salvo à aba ativa ou a
//! todas, perguntando as variáveis `{{nome}}`, e o diálogo para cadastrar.

use codatos_protocol::snippets::{
    METHOD_SNIPPET_DELETE, METHOD_SNIPPET_SAVE, METHOD_SNIPPETS_LIST, Snippet, Snippets, fill,
    variables,
};
use leptos::{ev, html, prelude::*, task::spawn_local};
use serde_json::json;
use wasm_bindgen::JsCast;

use super::Terminals;
use crate::components::icon::{Icon, Svg};
use crate::connections::ui::{
    self, BUTTON, DANGER, Dialog, FIELD, FormError, GHOST, LABEL, PRIMARY,
};
use crate::ids::new_id;

/// Lista do núcleo; `None` enquanto carrega.
type List = RwSignal<Option<Result<Vec<Snippet>, String>>>;

fn reload(list: List) {
    spawn_local(async move {
        let result = ui::call(METHOD_SNIPPETS_LIST, &json!({}))
            .await
            .and_then(|v| serde_json::from_value::<Snippets>(v).map_err(|e| e.to_string()))
            .map(|s| s.snippets);
        list.try_set(Some(result));
    });
}

/// O texto que vai ao terminal: quebras de linha viram Enter, e um Enter no fim.
fn keystrokes(command: &str) -> String {
    let mut text = command.trim_end().replace("\r\n", "\r").replace('\n', "\r");
    text.push('\r');
    text
}

/// Botão do menu de snippets, na barra de abas.
#[component]
pub fn SnippetMenu() -> impl IntoView {
    let terminals = Terminals::current();
    let open = RwSignal::new(false);
    let list: List = RwSignal::new(None);
    let running = RwSignal::new(None::<Snippet>);
    let managing = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || keys.remove());

    let run = move |snippet: Snippet| {
        open.set(false);
        let targets = terminals.open_ids();
        if variables(&snippet.command).is_empty() && targets.len() == 1 {
            terminals.type_text(targets[0], &keystrokes(&snippet.command));
            super::fit(Some(targets[0]), true);
        } else {
            running.set(Some(snippet));
        }
    };
    view! {
        <div class="relative shrink-0">
            <button
                type="button"
                class=format!("{GHOST} disabled:opacity-40")
                aria-label="Snippets"
                title="Snippets"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| {
                    if !open.get_untracked() {
                        query.set(String::new());
                        reload(list);
                    }
                    open.update(|o| *o = !*o);
                }
            >
                <Svg icon=Icon::Code size=16 />
            </button>
            <Show when=move || open.get()>
                <div class="fixed inset-0 z-30" on:click=move |_| open.set(false)></div>
                <div
                    role="menu"
                    aria-label="Snippets"
                    class="absolute top-full left-0 z-40 mt-1 flex max-h-[60vh] w-80 flex-col rounded-lg border border-line bg-sidebar p-1.5 shadow-card"
                >
                    <input
                        type="search"
                        placeholder="Buscar snippet"
                        aria-label="Buscar snippet"
                        class=format!("{FIELD} mb-1 py-1.5")
                        bind:value=query
                    />
                    <Items list=list query=query run=Callback::new(run) />
                    <button
                        type="button"
                        class="mt-1 flex items-center gap-2 rounded-md border-t border-line px-2.5 pt-2 pb-1 text-left text-sm text-muted hover:text-fg"
                        on:click=move |_| {
                            open.set(false);
                            managing.set(true);
                        }
                    >
                        <Svg icon=Icon::Pencil size=14 />
                        "Gerenciar snippets"
                    </button>
                </div>
            </Show>
            {move || running.get().map(|snippet| view! {
                <RunDialog snippet=snippet close=Callback::new(move |()| running.set(None)) />
            })}
            <Show when=move || managing.get()>
                <ManageDialog close=Callback::new(move |()| managing.set(false)) />
            </Show>
        </div>
    }
}

/// Itens do menu que batem com a busca.
#[component]
fn Items(list: List, query: RwSignal<String>, run: Callback<Snippet>) -> impl IntoView {
    let terminals = Terminals::current();
    let has_open = move || {
        terminals
            .tabs
            .with(|tabs| tabs.iter().any(|t| t.phase == super::Phase::Open))
    };
    let shown = move || {
        let q = query.get().to_lowercase();
        list.get().map(|r| {
            r.map(|items| {
                items
                    .into_iter()
                    .filter(|s| {
                        q.is_empty()
                            || s.name.to_lowercase().contains(&q)
                            || s.command.to_lowercase().contains(&q)
                    })
                    .collect::<Vec<_>>()
            })
        })
    };

    view! {
                    <div class="min-h-0 flex-1 overflow-y-auto">
                        {move || match shown() {
                            None => view! { <p class="px-2.5 py-2 text-sm text-muted">"Carregando…"</p> }.into_any(),
                            Some(Err(message)) => view! { <p class="px-2.5 py-2 text-sm text-danger">{message}</p> }.into_any(),
                            Some(Ok(items)) if items.is_empty() => view! {
                                <p class="px-2.5 py-2 text-sm text-subtle">"Nenhum snippet ainda."</p>
                            }
                            .into_any(),
                            Some(Ok(items)) => items
                                .into_iter()
                                .map(|snippet| {
                                    let (name, command) = (snippet.name.clone(), snippet.command.clone());
                                    let snippet = StoredValue::new(snippet);
                                    view! {
                                        <button
                                            type="button"
                                            role="menuitem"
                                            class="block w-full rounded-md px-2.5 py-1.5 text-left hover:bg-card-hover disabled:opacity-50"
                                            disabled=move || !has_open()
                                            title=move || if has_open() { "" } else { "Abra um terminal primeiro" }
                                            on:click=move |_| run.run(snippet.get_value())
                                        >
                                            <span class="block truncate text-sm">{name}</span>
                                            <span class="block truncate font-mono text-xs text-subtle">{command}</span>
                                        </button>
                                    }
                                })
                                .collect_view()
                                .into_any(),
                        }}
                    </div>
    }
}

/// Pergunta as variáveis e para onde mandar.
#[component]
fn RunDialog(snippet: Snippet, close: Callback<()>) -> impl IntoView {
    let terminals = Terminals::current();
    let names = variables(&snippet.command);
    let values: Vec<(String, RwSignal<String>)> = names
        .iter()
        .map(|n| (n.clone(), RwSignal::new(String::new())))
        .collect();
    let values = StoredValue::new(values);
    let all = RwSignal::new(false);
    let count = terminals.open_ids().len();
    let command = StoredValue::new(snippet.command.clone());
    let form = NodeRef::<html::Form>::new();
    Effect::new(move |_| {
        // O primeiro campo, ou o botão de executar quando não há variáveis.
        if let Some(el) = form
            .get()
            .and_then(|f| {
                f.query_selector("input, button[type=submit]")
                    .ok()
                    .flatten()
            })
            .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = el.focus();
        }
    });
    let preview = move || {
        values.with_value(|values| {
            command.with_value(|c| {
                fill(c, |name| {
                    values
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| v.get())
                        .filter(|v| !v.is_empty())
                })
            })
        })
    };
    let send = move || {
        let text = keystrokes(&preview());
        let targets = terminals.open_ids();
        let targets = if all.get_untracked() {
            targets
        } else {
            targets.into_iter().take(1).collect()
        };
        for id in &targets {
            terminals.type_text(*id, &text);
        }
        close.run(());
        super::fit(targets.first().copied(), true);
    };
    let fields = values.with_value(|values| {
        values
            .iter()
            .map(|(name, value)| {
                let value = *value;
                view! {
                    <label class="block space-y-1">
                        <span class=LABEL>{name.clone()}</span>
                        <input type="text" spellcheck="false" class=format!("{FIELD} font-mono") bind:value=value />
                    </label>
                }
            })
            .collect_view()
    });

    view! {
        <Dialog title=snippet.name.clone() on_close=close>
            <form
                node_ref=form
                class="space-y-4 p-5 text-sm"
                on:submit=move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    send();
                }
            >
                {fields}
                <pre class="max-h-40 overflow-auto rounded-md border border-line bg-input px-3 py-2 font-mono text-xs whitespace-pre-wrap break-all">
                    {preview}
                </pre>
                {(count > 1).then(|| view! {
                    <label class="flex items-center gap-2">
                        <input type="checkbox" bind:checked=all />
                        {format!("Mandar para todas as abas abertas ({count})")}
                    </label>
                })}
                <div class="flex justify-end gap-2">
                    <button type="button" class=BUTTON on:click=move |_| close.run(())>"Cancelar"</button>
                    <button type="submit" class=PRIMARY>"Executar"</button>
                </div>
            </form>
        </Dialog>
    }
}

/// Cadastro: lista à esquerda, formulário à direita.
#[component]
fn ManageDialog(close: Callback<()>) -> impl IntoView {
    let list: List = RwSignal::new(None);
    reload(list);
    let editing = RwSignal::new(None::<Snippet>);
    let name = RwSignal::new(String::new());
    let command = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let edit = move |snippet: Option<Snippet>| {
        name.set(snippet.as_ref().map(|s| s.name.clone()).unwrap_or_default());
        command.set(
            snippet
                .as_ref()
                .map(|s| s.command.clone())
                .unwrap_or_default(),
        );
        error.set(None);
        editing.set(snippet);
    };
    let save = move || {
        let snippet = Snippet {
            id: editing.get_untracked().map_or_else(new_id, |s| s.id),
            name: name.get_untracked().trim().to_owned(),
            command: command.get_untracked(),
        };
        busy.set(true);
        spawn_local(async move {
            let result = ui::call(METHOD_SNIPPET_SAVE, &snippet).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    editing.set(Some(snippet));
                    error.set(None);
                    reload(list);
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };
    let remove = move || {
        let Some(snippet) = editing.get_untracked() else {
            return;
        };
        spawn_local(async move {
            match ui::call(METHOD_SNIPPET_DELETE, &json!({ "id": snippet.id })).await {
                Ok(_) => {
                    edit(None);
                    reload(list);
                }
                Err(message) => error.set(Some(message)),
            }
        });
    };

    view! {
        <Dialog title="Snippets".to_owned() on_close=close width="w-[min(48rem,calc(100vw-2rem))]">
            <div class="grid min-h-0 grid-cols-[14rem_minmax(0,1fr)] text-sm">
                <div class="flex min-h-0 flex-col border-r border-line p-2">
                    <button type="button" class=format!("{BUTTON} mb-2 justify-center") on:click=move |_| edit(None)>
                        <Svg icon=Icon::Plus size=14 />
                        "Novo snippet"
                    </button>
                    <ul class="min-h-0 max-h-[50vh] flex-1 overflow-y-auto">
                        {move || {
                            list.get()
                                .and_then(Result::ok)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|snippet| {
                                    let id = StoredValue::new(snippet.id.clone());
                                    let label = snippet.name.clone();
                                    let snippet = StoredValue::new(snippet);
                                    let selected = move || {
                                        editing.with(|e| e.as_ref().is_some_and(|e| id.with_value(|id| e.id == *id)))
                                    };
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                class="block w-full truncate rounded-md px-2.5 py-1.5 text-left"
                                                class=(["bg-card", "text-fg"], selected)
                                                class=(["text-muted", "hover:bg-card-hover"], move || !selected())
                                                on:click=move |_| edit(Some(snippet.get_value()))
                                            >
                                                {label}
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </div>
                <form
                    class="space-y-4 p-5"
                    on:submit=move |e: ev::SubmitEvent| {
                        e.prevent_default();
                        save();
                    }
                >
                    <label class="block space-y-1">
                        <span class=LABEL>"Nome"</span>
                        <input type="text" class=FIELD bind:value=name />
                    </label>
                    <label class="block space-y-1">
                        <span class=LABEL>"Comando"</span>
                        <textarea
                            rows="6"
                            spellcheck="false"
                            class=format!("{FIELD} font-mono")
                            prop:value=move || command.get()
                            on:input=move |e| command.set(event_target_value(&e))
                        ></textarea>
                        <span class="block text-xs text-subtle">
                            "Use {{nome}} para perguntar um valor na hora, como em ssh {{usuario}}@{{host}}."
                        </span>
                    </label>
                    <FormError error=error />
                    <div class="flex items-center gap-2">
                        {move || editing.get().is_some().then(|| view! {
                            <button type="button" class=DANGER on:click=move |_| remove()>
                                <Svg icon=Icon::Trash size=14 />
                                "Apagar"
                            </button>
                        })}
                        <button
                            type="submit"
                            class=format!("ml-auto {PRIMARY}")
                            disabled=move || busy.get() || name.with(|n| n.trim().is_empty()) || command.with(|c| c.trim().is_empty())
                        >
                            "Salvar"
                        </button>
                    </div>
                </form>
            </div>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_no_fim_e_entre_linhas() {
        assert_eq!(keystrokes("ls -la\n"), "ls -la\r");
        assert_eq!(keystrokes("cd /\r\nls"), "cd /\rls\r");
    }
}
