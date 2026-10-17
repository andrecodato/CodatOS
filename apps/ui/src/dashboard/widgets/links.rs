//! Links rápidos: atalhos para sites e painéis, configurados por instância (RF-DASH-004).
//!
//! Config: `{"links": [{"title": "GitHub", "url": "https://github.com"}]}`. Só `http(s)`
//! entra; nada de ícones buscados na internet (a CSP só libera o próprio núcleo).

use leptos::{ev, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api;
use crate::components::icon::{Icon, Svg};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Link {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
}

fn links(config: &Value) -> Vec<Link> {
    config
        .get("links")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Nome curto para um link sem título: o host da URL.
fn label(link: &Link) -> String {
    if !link.title.trim().is_empty() {
        return link.title.trim().to_owned();
    }
    let rest = link.url.split("://").nth(1).unwrap_or(&link.url);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    host.trim_start_matches("www.").to_owned()
}

pub fn render(config: &Value) -> AnyView {
    let list: Vec<Link> = links(config)
        .into_iter()
        .filter(|l| api::is_web_url(&l.url))
        .collect();
    if list.is_empty() {
        return view! {
            <p class="flex h-full items-center text-xs text-subtle">
                "Nenhum link ainda. No modo edição, use a engrenagem deste widget para adicionar."
            </p>
        }
        .into_any();
    }
    view! {
        <ul class="grid h-full auto-rows-min grid-cols-[repeat(auto-fill,minmax(7rem,1fr))] gap-2 overflow-auto">
            {list
                .into_iter()
                .map(|link| {
                    let name = label(&link);
                    let initial = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                    let url = link.url.clone();
                    view! {
                        <li>
                            <a
                                href=link.url.clone()
                                target="_blank"
                                rel="noopener noreferrer"
                                title=link.url
                                class="flex items-center gap-2 rounded-md border border-line px-2 py-1.5 text-sm hover:bg-card-hover"
                                on:click=move |e: ev::MouseEvent| {
                                    e.prevent_default();
                                    api::open_external(&url);
                                }
                            >
                                <span class="grid size-6 shrink-0 place-items-center rounded bg-primary-soft text-xs font-semibold text-primary">
                                    {initial}
                                </span>
                                <span class="truncate">{name}</span>
                            </a>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

/// Editor da lista de links, usado no diálogo de configuração do widget.
pub fn editor(draft: RwSignal<Value>) -> AnyView {
    let rows = RwSignal::new(links(&draft.get_untracked()));
    // Cada mudança na lista vira a config em rascunho; o diálogo decide se aplica.
    Effect::new(move |_| {
        let list = rows.get();
        draft.update(|c| {
            if !c.is_object() {
                *c = json!({});
            }
            c["links"] = serde_json::to_value(&list).unwrap_or_default();
        });
    });
    let input = "min-w-0 rounded-md border bg-input px-2 py-1 text-sm placeholder:text-subtle focus:outline-none";
    view! {
        <div class="space-y-2">
            <For
                each=move || 0..rows.with(Vec::len)
                key=|i| *i
                let:i
            >
                <div class="flex items-center gap-2">
                    <input
                        class=format!("w-32 border-line {input}")
                        placeholder="Título"
                        aria-label=format!("Título do link {}", i + 1)
                        prop:value=move || rows.with(|r| r.get(i).map(|l| l.title.clone()).unwrap_or_default())
                        on:input=move |e| rows.update(|r| if let Some(l) = r.get_mut(i) { l.title = event_target_value(&e) })
                    />
                    <input
                        class=move || {
                            let bad = rows.with(|r| r.get(i).is_some_and(|l| !l.url.is_empty() && !api::is_web_url(&l.url)));
                            format!("flex-1 {input}{}", if bad { " border-danger" } else { " border-line" })
                        }
                        type="url"
                        placeholder="https://"
                        aria-label=format!("Endereço do link {}", i + 1)
                        prop:value=move || rows.with(|r| r.get(i).map(|l| l.url.clone()).unwrap_or_default())
                        on:input=move |e| rows.update(|r| if let Some(l) = r.get_mut(i) { l.url = event_target_value(&e) })
                    />
                    <button
                        type="button"
                        class="rounded p-1 text-muted hover:bg-card-hover hover:text-danger"
                        aria-label=format!("Remover link {}", i + 1)
                        on:click=move |_| rows.update(|r| { if i < r.len() { r.remove(i); } })
                    >
                        <Svg icon=Icon::Trash size=14 />
                    </button>
                </div>
            </For>
            <p class="text-xs text-subtle">"Só endereços http:// ou https://."</p>
            <button
                type="button"
                class="inline-flex items-center gap-1.5 rounded-md border border-line px-2 py-1 text-xs hover:bg-card-hover"
                on:click=move |_| rows.update(|r| r.push(Link::default()))
            >
                <Svg icon=Icon::Plus size=12 />
                "Adicionar link"
            </button>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titulo_ou_host() {
        let l = |title: &str, url: &str| Link {
            title: title.into(),
            url: url.into(),
        };
        assert_eq!(label(&l("GitHub", "https://github.com")), "GitHub");
        assert_eq!(label(&l(" ", "https://www.example.com/a?b")), "example.com");
        assert_eq!(label(&l("", "http://192.168.0.2:8123")), "192.168.0.2:8123");
    }

    #[test]
    fn config_invalida_vira_lista_vazia() {
        assert_eq!(links(&json!({})), Vec::<Link>::new());
        assert_eq!(links(&json!({"links": 3})), Vec::<Link>::new());
        assert_eq!(
            links(&json!({"links": [{"url": "https://a.b"}]}))[0].url,
            "https://a.b"
        );
    }
}
