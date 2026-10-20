//! Aba SSH/SFTP: hosts em pastas, busca e o editor do host (Geral, Autenticação,
//! Avançado). Permissões chegam com as contas no M3.

use codatos_protocol::connections::{
    ConnectionId, Connections as List, DEFAULT_SSH_PORT, Host, HostDefaults, HostGroup,
    METHOD_GROUP_DELETE, METHOD_GROUP_SAVE, METHOD_HOST_DELETE, METHOD_HOST_SAVE, Protocol,
};
use codatos_protocol::terminal::TerminalTarget;
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Map, Value, json};

use super::Data;
use super::import::{ExportDialog, ImportDialog};
use super::ui::{self, BUTTON, DANGER, Dialog, Edit, FIELD, FormError, GHOST, LABEL, PRIMARY};
use crate::components::icon::{Icon, Svg};
use crate::ids::new_id;
use crate::terminal::Terminals;

/// Cores para marcar hosts (as séries de gráfico do design system).
const COLORS: [&str; 6] = [
    "#3b82f6", "#f43f5e", "#f59e0b", "#22c55e", "#a855f7", "#06b6d4",
];

/// O host aberto no painel da direita. `new` marca um host que ainda não foi salvo; o id
/// já nasce aqui para que salvar não troque a seleção (e não remonte o editor).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Selected {
    id: String,
    new: bool,
}

#[component]
pub fn Hosts() -> impl IntoView {
    let data = expect_context::<Data>();
    let query = RwSignal::new(String::new());
    let selected = RwSignal::new(None::<Selected>);
    let group_dialog = RwSignal::new(None::<Edit<HostGroup>>);
    let importing = RwSignal::new(false);
    let exporting = RwSignal::new(false);

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-4 lg:flex-row">
            <div class="flex min-h-0 flex-col gap-3 lg:w-80 lg:shrink-0">
                <div class="flex items-center gap-2">
                    <label class="flex flex-1 items-center gap-2 rounded-md border border-line bg-input px-3 py-1.5 text-sm text-subtle">
                        <Svg icon=Icon::Search size=16 />
                        <input
                            type="search"
                            placeholder="Buscar host, endereço ou tag"
                            aria-label="Buscar hosts"
                            class="w-full bg-transparent text-fg placeholder:text-subtle focus:outline-none"
                            prop:value=query
                            on:input=move |e| query.set(event_target_value(&e))
                        />
                    </label>
                    <button
                        type="button"
                        class=GHOST
                        title="Nova pasta"
                        aria-label="Nova pasta"
                        on:click=move |_| group_dialog.set(Some(Edit::New))
                    >
                        <Svg icon=Icon::FolderPlus size=16 />
                    </button>
                </div>
                <div class="flex flex-wrap gap-2">
                    <button type="button" class=PRIMARY on:click=move |_| selected.set(Some(Selected { id: new_id(), new: true }))>
                        <Svg icon=Icon::Plus size=16 />
                        "Nova conexão"
                    </button>
                    <button type="button" class=BUTTON on:click=move |_| importing.set(true)>
                        <Svg icon=Icon::Download size=16 />
                        "Importar"
                    </button>
                    <button type="button" class=BUTTON on:click=move |_| exporting.set(true)>
                        <Svg icon=Icon::Upload size=16 />
                        "Exportar"
                    </button>
                </div>
                <HostList query=query selected=selected group_dialog=group_dialog />
            </div>
            <div class="min-h-0 min-w-0 flex-1 overflow-auto">
                {move || match selected.get() {
                    None => view! {
                        <div class="grid h-full place-content-center rounded-lg border border-dashed border-line p-8 text-center text-sm text-muted">
                            "Escolha um host na lista ou crie uma conexão nova."
                        </div>
                    }
                    .into_any(),
                    Some(sel) => {
                        let host = if sel.new {
                            None
                        } else {
                            data.get_untracked().hosts.into_iter().find(|h| h.id == sel.id)
                        };
                        view! { <HostEditor id=sel.id host=host selected=selected /> }.into_any()
                    }
                }}
            </div>
        </div>
        {move || group_dialog.get().map(|group| view! { <GroupDialog group=group open=group_dialog /> })}
        {move || importing.get().then(|| view! { <ImportDialog open=importing /> })}
        {move || exporting.get().then(|| view! { <ExportDialog open=exporting /> })}
    }
}

/// Texto de busca bate com nome, endereço ou tag?
fn matches(host: &Host, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty()
        || host.name.to_lowercase().contains(&q)
        || host.address.to_lowercase().contains(&q)
        || host.tags.iter().any(|t| t.to_lowercase().contains(&q))
}

/// Pastas na ordem da árvore, com a profundidade de cada uma.
fn tree(groups: &[HostGroup]) -> Vec<(HostGroup, usize)> {
    fn walk(
        groups: &[HostGroup],
        parent: Option<&str>,
        depth: usize,
        out: &mut Vec<(HostGroup, usize)>,
    ) {
        let mut children: Vec<&HostGroup> = groups
            .iter()
            .filter(|g| g.parent_id.as_deref() == parent)
            .collect();
        children.sort_by_key(|g| g.name.to_lowercase());
        for group in children {
            if out.iter().any(|(g, _)| g.id == group.id) || depth > 16 {
                continue;
            }
            out.push((group.clone(), depth));
            walk(groups, Some(&group.id), depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(groups, None, 0, &mut out);
    // Pastas cujo pai sumiu aparecem na raiz.
    for group in groups {
        if !out.iter().any(|(g, _)| g.id == group.id) {
            out.push((group.clone(), 0));
        }
    }
    out
}

/// `usuário@endereço:porta`, como se digitaria no `ssh`.
fn target(host: &Host, list: &List) -> String {
    let user = host
        .identity_id
        .as_ref()
        .and_then(|id| list.identities.iter().find(|i| i.id == *id))
        .map(|i| format!("{}@", i.username))
        .unwrap_or_default();
    let port = if host.port == DEFAULT_SSH_PORT {
        String::new()
    } else {
        format!(":{}", host.port)
    };
    format!("{user}{}{port}", host.address)
}

#[component]
fn HostList(
    query: RwSignal<String>,
    selected: RwSignal<Option<Selected>>,
    group_dialog: RwSignal<Option<Edit<HostGroup>>>,
) -> impl IntoView {
    let data = expect_context::<Data>();
    let row = move |host: Host, list: &List, depth: usize| {
        let id = host.id.clone();
        let active = {
            let id = id.clone();
            move || selected.with(|s| s.as_ref().is_some_and(|s| s.id == id))
        };
        let subtitle = target(&host, list);
        let color = host
            .color
            .clone()
            .unwrap_or_else(|| "transparent".to_owned());
        view! {
            <li>
                <button
                    type="button"
                    class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors duration-fast"
                    class=(["bg-primary-soft"], active.clone())
                    class=(["hover:bg-card-hover"], move || !active())
                    style=format!("padding-left: {}rem", 0.75 + 1.0 * f64::from(u32::try_from(depth).unwrap_or(0)))
                    on:click=move |_| selected.set(Some(Selected { id: id.clone(), new: false }))
                >
                    <span class="size-2 shrink-0 rounded-full border border-line" style=format!("background: {color}")></span>
                    <span class="min-w-0 flex-1">
                        <span class="block truncate text-sm font-medium">{host.name.clone()}</span>
                        <span class="block truncate font-mono text-xs text-muted">{subtitle}</span>
                    </span>
                    {host.favorite.then(|| view! {
                        <span class="text-warning" title="Favorito"><Svg icon=Icon::Star size=14 /></span>
                    })}
                </button>
            </li>
        }
    };

    view! {
        <ul class="min-h-0 flex-1 space-y-0.5 overflow-auto" aria-label="Hosts">
            {move || {
                let list = data.get();
                let q = query.get();
                let mut hosts: Vec<Host> = list.hosts.iter().filter(|h| matches(h, &q)).cloned().collect();
                // Favoritos primeiro, depois os usados por último, depois por nome.
                hosts.sort_by(|a, b| {
                    b.favorite
                        .cmp(&a.favorite)
                        .then_with(|| b.last_used_at.cmp(&a.last_used_at))
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
                if list.hosts.is_empty() {
                    return view! {
                        <li class="rounded-lg border border-dashed border-line p-4 text-sm text-muted">
                            "Nenhum host ainda. Crie uma conexão ou importe o seu ~/.ssh/config."
                        </li>
                    }
                    .into_any();
                }
                if !q.trim().is_empty() {
                    if hosts.is_empty() {
                        return view! { <li class="px-3 py-2 text-sm text-muted">"Nada encontrado."</li> }.into_any();
                    }
                    return hosts.into_iter().map(|h| row(h, &list, 0)).collect_view().into_any();
                }
                let loose: Vec<_> = hosts
                    .iter()
                    .filter(|h| h.group_id.as_ref().is_none_or(|g| !list.groups.iter().any(|x| x.id == *g)))
                    .cloned()
                    .map(|h| row(h, &list, 0))
                    .collect();
                let folders: Vec<_> = tree(&list.groups)
                    .into_iter()
                    .map(|(group, depth)| {
                        let inside: Vec<_> = hosts
                            .iter()
                            .filter(|h| h.group_id.as_deref() == Some(group.id.as_str()))
                            .cloned()
                            .map(|h| row(h, &list, depth + 1))
                            .collect();
                        let edit = group.clone();
                        view! {
                            <li class="mt-2">
                                <button
                                    type="button"
                                    class="flex w-full items-center gap-2 rounded-md px-3 py-1 text-left text-xs font-medium tracking-wide text-muted uppercase hover:text-fg"
                                    style=format!("padding-left: {}rem", 0.75 + f64::from(u32::try_from(depth).unwrap_or(0)))
                                    title="Editar pasta"
                                    on:click=move |_| group_dialog.set(Some(Edit::Existing(edit.clone())))
                                >
                                    <Svg icon=Icon::Folder size=14 />
                                    {group.name.clone()}
                                </button>
                                <ul class="space-y-0.5">{inside}</ul>
                            </li>
                        }
                    })
                    .collect();
                view! { {loose} {folders} }.into_any()
            }}
        </ul>
    }
}

/// Formulário do host: textos como digitados, convertidos ao salvar.
#[derive(Debug, Clone, Default, PartialEq)]
struct Draft {
    id: String,
    name: String,
    address: String,
    port: String,
    group_id: String,
    identity_id: String,
    jump_host_id: String,
    tags: String,
    favorite: bool,
    color: Option<String>,
    notes: String,
    proxy_kind: String,
    proxy_host: String,
    proxy_port: String,
    /// O resto de `advanced` (ex.: `sshConfig` da importação), devolvido como veio.
    advanced: Map<String, Value>,
    last_used_at: Option<String>,
}

impl Draft {
    fn new(id: String) -> Self {
        Self {
            id,
            port: DEFAULT_SSH_PORT.to_string(),
            ..Self::default()
        }
    }

    fn from_host(host: &Host) -> Self {
        let mut advanced = host.advanced.as_object().cloned().unwrap_or_default();
        let proxy = advanced.remove("proxy").unwrap_or(Value::Null);
        let text = |key: &str| match &proxy[key] {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        Self {
            id: host.id.clone(),
            name: host.name.clone(),
            address: host.address.clone(),
            port: host.port.to_string(),
            group_id: host.group_id.clone().unwrap_or_default(),
            identity_id: host.identity_id.clone().unwrap_or_default(),
            jump_host_id: host.jump_host_id.clone().unwrap_or_default(),
            tags: host.tags.join(", "),
            favorite: host.favorite,
            color: host.color.clone(),
            notes: host.notes.clone(),
            proxy_kind: text("type"),
            proxy_host: text("host"),
            proxy_port: text("port"),
            advanced,
            last_used_at: host.last_used_at.clone(),
        }
    }

    fn to_host(&self) -> Result<Host, String> {
        let name = self.name.trim();
        let address = self.address.trim();
        if name.is_empty() || address.is_empty() {
            return Err("Preencha o nome e o endereço.".to_owned());
        }
        let port = parse_port(&self.port).ok_or("Porta inválida: use um número de 1 a 65535.")?;
        let mut advanced = self.advanced.clone();
        if !self.proxy_kind.is_empty() {
            let proxy_port = parse_port(&self.proxy_port)
                .ok_or("Porta do proxy inválida: use um número de 1 a 65535.")?;
            if self.proxy_host.trim().is_empty() {
                return Err("Preencha o endereço do proxy.".to_owned());
            }
            advanced.insert(
                "proxy".to_owned(),
                json!({"type": self.proxy_kind, "host": self.proxy_host.trim(), "port": proxy_port}),
            );
        }
        let some = |s: &str| (!s.is_empty()).then(|| s.to_owned());
        Ok(Host {
            id: self.id.clone(),
            name: name.to_owned(),
            address: address.to_owned(),
            port,
            protocol: Protocol::Ssh,
            group_id: some(&self.group_id),
            identity_id: some(&self.identity_id),
            jump_host_id: some(&self.jump_host_id),
            tags: self
                .tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect(),
            favorite: self.favorite,
            color: self.color.clone(),
            notes: self.notes.clone(),
            advanced: if advanced.is_empty() {
                Value::Null
            } else {
                Value::Object(advanced)
            },
            last_used_at: self.last_used_at.clone(),
        })
    }
}

fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<u16>().ok().filter(|p| *p != 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    General,
    Auth,
    Advanced,
}

#[component]
fn HostEditor(
    id: String,
    host: Option<Host>,
    selected: RwSignal<Option<Selected>>,
) -> impl IntoView {
    let data = expect_context::<Data>();
    let is_new = RwSignal::new(host.is_none());
    let draft = RwSignal::new(
        host.as_ref()
            .map_or_else(|| Draft::new(id), Draft::from_host),
    );
    let tab = RwSignal::new(EditorTab::General);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let saved = RwSignal::new(false);

    let save = move |_| {
        let host = match draft.with_untracked(Draft::to_host) {
            Ok(h) => h,
            Err(e) => {
                error.set(Some(e));
                return;
            }
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call(METHOD_HOST_SAVE, &host).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    saved.set(true);
                    is_new.set(false);
                    data.reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete = move |_| {
        let id = draft.with_untracked(|d| d.id.clone());
        let name = draft.with_untracked(|d| d.name.clone());
        if !window()
            .confirm_with_message(&format!("Apagar o host {name}?"))
            .unwrap_or(false)
        {
            return;
        }
        spawn_local(async move {
            match ui::call(METHOD_HOST_DELETE, &ConnectionId { id }).await {
                Ok(_) => {
                    selected.set(None);
                    data.reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // Abre o terminal com o host como está salvo no núcleo.
    let page_tab = expect_context::<RwSignal<super::Tab>>();
    let connect = move |_| {
        let (id, name) = draft.with_untracked(|d| (d.id.clone(), d.name.trim().to_owned()));
        Terminals::current().open(
            TerminalTarget::Ssh {
                host_id: id,
                username: None,
                password: None,
                trust_host_key: None,
            },
            name,
        );
        page_tab.set(super::Tab::Terminal);
    };
    let files = move |_| {
        let (id, name) = draft.with_untracked(|d| (d.id.clone(), d.name.trim().to_owned()));
        crate::sftp::Sftp::current().open(id, name);
        page_tab.set(super::Tab::Files);
    };

    let tab_button = move |value: EditorTab, label: &'static str| {
        view! {
            <button
                type="button"
                role="tab"
                aria-selected=move || (tab.get() == value).to_string()
                class="rounded-md px-3 py-1.5 text-sm"
                class=(["bg-card-hover", "text-fg"], move || tab.get() == value)
                class=(["text-muted", "hover:text-fg"], move || tab.get() != value)
                on:click=move |_| tab.set(value)
            >
                {label}
            </button>
        }
    };

    view! {
        <article class="flex flex-col rounded-lg border border-line bg-card">
            <header class="flex items-center gap-3 border-b border-line px-5 py-4">
                <span class="rounded-md bg-primary-soft p-2 text-primary"><Svg icon=Icon::Server size=18 /></span>
                <div class="min-w-0 flex-1">
                    <h2 class="truncate text-[1rem] font-semibold">
                        {move || {
                            let name = draft.with(|d| d.name.trim().to_owned());
                            if name.is_empty() { "Nova conexão".to_owned() } else { name }
                        }}
                    </h2>
                    <p class="truncate font-mono text-xs text-muted">
                        {move || draft.with(|d| format!("ssh {}:{}", d.address.trim(), d.port.trim()))}
                    </p>
                </div>
                <button
                    type="button"
                    class=GHOST
                    aria-label=move || if draft.with(|d| d.favorite) { "Tirar dos favoritos" } else { "Marcar como favorito" }
                    aria-pressed=move || draft.with(|d| d.favorite).to_string()
                    class=("text-warning", move || draft.with(|d| d.favorite))
                    on:click=move |_| draft.update(|d| d.favorite = !d.favorite)
                >
                    <Svg icon=Icon::Star size=18 />
                </button>
                {move || (!is_new.get()).then(|| view! {
                    <button type="button" class=BUTTON on:click=files>
                        <Svg icon=Icon::Folder size=16 />
                        "Arquivos"
                    </button>
                    <button type="button" class=PRIMARY on:click=connect>
                        <Svg icon=Icon::Terminal size=16 />
                        "Conectar"
                    </button>
                })}
            </header>
            <nav role="tablist" aria-label="Seções do host" class="flex gap-1 border-b border-line px-4 py-2">
                {tab_button(EditorTab::General, "Geral")}
                {tab_button(EditorTab::Auth, "Autenticação")}
                {tab_button(EditorTab::Advanced, "Avançado")}
            </nav>
            <div class="space-y-4 p-5" on:input=move |_| saved.set(false)>
                {move || match tab.get() {
                    EditorTab::General => view! { <GeneralTab draft=draft /> }.into_any(),
                    EditorTab::Auth => view! { <AuthTab draft=draft /> }.into_any(),
                    EditorTab::Advanced => view! { <AdvancedTab draft=draft /> }.into_any(),
                }}
                <FormError error=error />
            </div>
            <footer class="flex items-center gap-2 border-t border-line px-5 py-3">
                {move || (!is_new.get()).then(|| view! {
                    <button type="button" class=DANGER on:click=delete>
                        <Svg icon=Icon::Trash size=16 />
                        "Apagar"
                    </button>
                })}
                {move || saved.get().then(|| view! {
                    <span role="status" class="ml-auto flex items-center gap-1 text-xs text-success">
                        <Svg icon=Icon::Check size=14 />
                        "Salvo"
                    </span>
                })}
                <button
                    type="button"
                    class=move || if saved.get() { BUTTON.to_owned() } else { format!("ml-auto {BUTTON}") }
                    on:click=move |_| selected.set(None)
                >
                    "Fechar"
                </button>
                <button type="button" class=PRIMARY disabled=busy on:click=save>
                    {move || if busy.get() { "Salvando…" } else { "Salvar" }}
                </button>
            </footer>
        </article>
    }
}

/// Campo de texto ligado a um pedaço do rascunho.
fn text_field(
    draft: RwSignal<Draft>,
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    get: fn(&Draft) -> String,
    set: fn(&mut Draft, String),
) -> impl IntoView {
    view! {
        <div>
            <label class=LABEL for=id>{label}</label>
            <input
                id=id
                class=format!("mt-1 {FIELD}")
                placeholder=placeholder
                prop:value=move || draft.with(get)
                on:input=move |e| draft.update(|d| set(d, event_target_value(&e)))
            />
        </div>
    }
}

#[component]
fn GeneralTab(draft: RwSignal<Draft>) -> impl IntoView {
    let data = expect_context::<Data>();
    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            {text_field(draft, "host-name", "Nome", "Servidor de produção", |d| d.name.clone(), |d, v| d.name = v)}
            {text_field(draft, "host-address", "Endereço", "192.168.1.10 ou srv.exemplo.com", |d| d.address.clone(), |d, v| d.address = v)}
            {text_field(draft, "host-port", "Porta", "22", |d| d.port.clone(), |d, v| d.port = v)}
            <div>
                <label class=LABEL for="host-group">"Pasta"</label>
                <select
                    id="host-group"
                    class=format!("mt-1 {FIELD}")
                    prop:value=move || draft.with(|d| d.group_id.clone())
                    on:change=move |e| draft.update(|d| d.group_id = event_target_value(&e))
                >
                    <option value="">"Sem pasta"</option>
                    {move || tree(&data.get().groups)
                        .into_iter()
                        .map(|(g, depth)| view! {
                            <option value=g.id.clone()>{format!("{}{}", "  ".repeat(depth), g.name)}</option>
                        })
                        .collect_view()}
                </select>
            </div>
            <div class="sm:col-span-2">
                {text_field(draft, "host-tags", "Tags (separadas por vírgula)", "prod, web", |d| d.tags.clone(), |d, v| d.tags = v)}
            </div>
            <fieldset class="sm:col-span-2">
                <legend class=LABEL>"Cor"</legend>
                <div class="mt-1 flex gap-2">
                    <button
                        type="button"
                        aria-label="Sem cor"
                        aria-pressed=move || draft.with(|d| d.color.is_none()).to_string()
                        class="size-6 rounded-full border border-line"
                        class=("ring-2", move || draft.with(|d| d.color.is_none()))
                        on:click=move |_| draft.update(|d| d.color = None)
                    ></button>
                    {COLORS
                        .into_iter()
                        .map(|c| view! {
                            <button
                                type="button"
                                aria-label=format!("Cor {c}")
                                aria-pressed=move || draft.with(|d| d.color.as_deref() == Some(c)).to_string()
                                class="size-6 rounded-full"
                                class=("ring-2", move || draft.with(|d| d.color.as_deref() == Some(c)))
                                style=format!("background: {c}")
                                on:click=move |_| draft.update(|d| d.color = Some(c.to_owned()))
                            ></button>
                        })
                        .collect_view()}
                </div>
            </fieldset>
            <div class="sm:col-span-2">
                <label class=LABEL for="host-notes">"Notas"</label>
                <textarea
                    id="host-notes"
                    rows="3"
                    class=format!("mt-1 {FIELD}")
                    prop:value=move || draft.with(|d| d.notes.clone())
                    on:input=move |e| draft.update(|d| d.notes = event_target_value(&e))
                ></textarea>
            </div>
        </div>
    }
}

#[component]
fn AuthTab(draft: RwSignal<Draft>) -> impl IntoView {
    let data = expect_context::<Data>();
    view! {
        <div class="space-y-3">
            <div>
                <label class=LABEL for="host-identity">"Identidade"</label>
                <select
                    id="host-identity"
                    class=format!("mt-1 {FIELD}")
                    prop:value=move || draft.with(|d| d.identity_id.clone())
                    on:change=move |e| draft.update(|d| d.identity_id = event_target_value(&e))
                >
                    <option value="">"Herdar da pasta (ou pedir na conexão)"</option>
                    {move || data
                        .get()
                        .identities
                        .into_iter()
                        .map(|i| view! { <option value=i.id.clone()>{format!("{} ({})", i.name, i.username)}</option> })
                        .collect_view()}
                </select>
            </div>
            <p class="text-xs text-muted">
                "A identidade guarda o usuário, a senha e a chave. Crie e edite identidades na aba Chaveiro; a senha fica no cofre e nunca volta para a tela."
            </p>
        </div>
    }
}

#[component]
fn AdvancedTab(draft: RwSignal<Draft>) -> impl IntoView {
    let data = expect_context::<Data>();
    view! {
        <div class="grid gap-4 sm:grid-cols-2">
            <div class="sm:col-span-2">
                <label class=LABEL for="host-jump">"Jump host"</label>
                <select
                    id="host-jump"
                    class=format!("mt-1 {FIELD}")
                    prop:value=move || draft.with(|d| d.jump_host_id.clone())
                    on:change=move |e| draft.update(|d| d.jump_host_id = event_target_value(&e))
                >
                    <option value="">"Herdar da pasta (ou direto)"</option>
                    {move || {
                        let me = draft.with(|d| d.id.clone());
                        data.get()
                            .hosts
                            .into_iter()
                            .filter(|h| h.id != me)
                            .map(|h| view! { <option value=h.id.clone()>{format!("{} ({})", h.name, h.address)}</option> })
                            .collect_view()
                    }}
                </select>
                <p class="mt-1 text-xs text-muted">"A conexão passa por este host antes de chegar ao destino (ProxyJump)."</p>
            </div>
            <div>
                <label class=LABEL for="host-proxy">"Proxy"</label>
                <select
                    id="host-proxy"
                    class=format!("mt-1 {FIELD}")
                    prop:value=move || draft.with(|d| d.proxy_kind.clone())
                    on:change=move |e| draft.update(|d| d.proxy_kind = event_target_value(&e))
                >
                    <option value="">"Herdar da pasta (ou nenhum)"</option>
                    <option value="socks5">"SOCKS5"</option>
                    <option value="http">"HTTP (CONNECT)"</option>
                </select>
            </div>
            {move || draft.with(|d| !d.proxy_kind.is_empty()).then(|| view! {
                <div class="grid grid-cols-[1fr_6rem] gap-2">
                    {text_field(draft, "host-proxy-host", "Endereço do proxy", "127.0.0.1", |d| d.proxy_host.clone(), |d, v| d.proxy_host = v)}
                    {text_field(draft, "host-proxy-port", "Porta", "1080", |d| d.proxy_port.clone(), |d, v| d.proxy_port = v)}
                </div>
            })}
            <p class="text-xs text-muted sm:col-span-2">"Proxies com senha ainda não são suportados."</p>
        </div>
    }
}

/// Criar (`None`) ou editar uma pasta: nome, pasta pai e o que os hosts dela herdam.
#[component]
fn GroupDialog(group: Edit<HostGroup>, open: RwSignal<Option<Edit<HostGroup>>>) -> impl IntoView {
    let data = expect_context::<Data>();
    let group = group.existing();
    let is_new = group.is_none();
    let group = group.unwrap_or_else(|| HostGroup {
        id: new_id(),
        parent_id: None,
        name: String::new(),
        defaults: HostDefaults {
            identity_id: None,
            jump_host_id: None,
            advanced: Value::Null,
        },
    });
    let id = group.id.clone();
    let name = RwSignal::new(group.name.clone());
    let parent = RwSignal::new(group.parent_id.clone().unwrap_or_default());
    let identity = RwSignal::new(group.defaults.identity_id.clone().unwrap_or_default());
    let jump = RwSignal::new(group.defaults.jump_host_id.clone().unwrap_or_default());
    let advanced = group.defaults.advanced.clone();
    let error = RwSignal::new(None::<String>);
    let close = Callback::new(move |()| open.set(None));
    let group_id = StoredValue::new(id.clone());

    let save = move |_| {
        if name.with_untracked(|n| n.trim().is_empty()) {
            error.set(Some("Dê um nome à pasta.".to_owned()));
            return;
        }
        let some = |s: String| (!s.is_empty()).then_some(s);
        let saved = HostGroup {
            id: group_id.get_value(),
            parent_id: some(parent.get_untracked()),
            name: name.get_untracked().trim().to_owned(),
            defaults: HostDefaults {
                identity_id: some(identity.get_untracked()),
                jump_host_id: some(jump.get_untracked()),
                advanced: advanced.clone(),
            },
        };
        spawn_local(async move {
            match ui::call(METHOD_GROUP_SAVE, &saved).await {
                Ok(_) => {
                    data.reload();
                    open.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete = move |_| {
        if !window()
            .confirm_with_message(
                "Apagar a pasta? Os hosts e as subpastas sobem para a pasta de cima.",
            )
            .unwrap_or(false)
        {
            return;
        }
        let id = group_id.get_value();
        spawn_local(async move {
            match ui::call(METHOD_GROUP_DELETE, &ConnectionId { id }).await {
                Ok(_) => {
                    data.reload();
                    open.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <Dialog title=if is_new { "Nova pasta".to_owned() } else { "Editar pasta".to_owned() } on_close=close>
            <div class="space-y-4 overflow-auto p-5">
                <div>
                    <label class=LABEL for="group-name">"Nome"</label>
                    <input
                        id="group-name"
                        class=format!("mt-1 {FIELD}")
                        placeholder="Clientes"
                        prop:value=name
                        on:input=move |e| name.set(event_target_value(&e))
                    />
                </div>
                <div>
                    <label class=LABEL for="group-parent">"Dentro de"</label>
                    <select
                        id="group-parent"
                        class=format!("mt-1 {FIELD}")
                        prop:value=parent
                        on:change=move |e| parent.set(event_target_value(&e))
                    >
                        <option value="">"Nenhuma (raiz)"</option>
                        {
                            let me = id.clone();
                            tree(&data.get_untracked().groups)
                                .into_iter()
                                .filter(|(g, _)| g.id != me)
                                .map(|(g, depth)| view! {
                                    <option value=g.id.clone()>{format!("{}{}", "  ".repeat(depth), g.name)}</option>
                                })
                                .collect_view()
                        }
                    </select>
                </div>
                <p class="text-xs text-muted">"Os hosts da pasta herdam o que não definirem:"</p>
                <div>
                    <label class=LABEL for="group-identity">"Identidade"</label>
                    <select
                        id="group-identity"
                        class=format!("mt-1 {FIELD}")
                        prop:value=identity
                        on:change=move |e| identity.set(event_target_value(&e))
                    >
                        <option value="">"Nenhuma"</option>
                        {data
                            .get_untracked()
                            .identities
                            .into_iter()
                            .map(|i| view! { <option value=i.id.clone()>{format!("{} ({})", i.name, i.username)}</option> })
                            .collect_view()}
                    </select>
                </div>
                <div>
                    <label class=LABEL for="group-jump">"Jump host"</label>
                    <select
                        id="group-jump"
                        class=format!("mt-1 {FIELD}")
                        prop:value=jump
                        on:change=move |e| jump.set(event_target_value(&e))
                    >
                        <option value="">"Nenhum"</option>
                        {data
                            .get_untracked()
                            .hosts
                            .into_iter()
                            .map(|h| view! { <option value=h.id.clone()>{format!("{} ({})", h.name, h.address)}</option> })
                            .collect_view()}
                    </select>
                </div>
                <FormError error=error />
            </div>
            <footer class="flex items-center gap-2 border-t border-line px-5 py-3">
                {(!is_new).then(|| view! {
                    <button type="button" class=DANGER on:click=delete>
                        <Svg icon=Icon::Trash size=16 />
                        "Apagar pasta"
                    </button>
                })}
                <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| close.run(())>"Cancelar"</button>
                <button type="button" class=PRIMARY on:click=save>"Salvar"</button>
            </footer>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(extra: &Value) -> Host {
        let mut v = json!({"id": "h1", "name": "web", "address": "web.lan"});
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn rascunho_ida_e_volta_preserva_o_advanced() {
        let original = host(&json!({
            "port": 2222,
            "tags": ["prod", "web"],
            "advanced": {"sshConfig": {"forwardagent": "yes"}, "proxy": {"type": "socks5", "host": "10.0.0.1", "port": 1080}}
        }));
        let draft = Draft::from_host(&original);
        assert_eq!(draft.tags, "prod, web");
        assert_eq!(
            (draft.proxy_kind.as_str(), draft.proxy_port.as_str()),
            ("socks5", "1080")
        );
        assert_eq!(draft.to_host().unwrap(), original);

        // Sem proxy, o resto do advanced fica.
        let mut draft = draft;
        draft.proxy_kind.clear();
        let saved = draft.to_host().unwrap();
        assert_eq!(
            saved.advanced,
            json!({"sshConfig": {"forwardagent": "yes"}})
        );
    }

    #[test]
    fn validacao_do_rascunho() {
        let mut draft = Draft::new("h1".into());
        assert!(draft.to_host().is_err());
        draft.name = "web".into();
        draft.address = "web.lan".into();
        assert_eq!(draft.to_host().unwrap().port, 22);
        draft.port = "70000".into();
        assert!(draft.to_host().is_err());
        draft.port = "22".into();
        draft.proxy_kind = "http".into();
        assert!(draft.to_host().is_err());
        draft.proxy_host = "proxy".into();
        draft.proxy_port = "3128".into();
        assert_eq!(draft.to_host().unwrap().advanced["proxy"]["port"], 3128);
    }

    #[test]
    fn busca_e_arvore() {
        let h = host(&json!({"tags": ["Produção"]}));
        assert!(matches(&h, ""));
        assert!(matches(&h, "WEB"));
        assert!(matches(&h, "produ"));
        assert!(!matches(&h, "db"));

        let g = |id: &str, parent: Option<&str>, name: &str| HostGroup {
            id: id.into(),
            parent_id: parent.map(Into::into),
            name: name.into(),
            defaults: HostDefaults {
                identity_id: None,
                jump_host_id: None,
                advanced: Value::Null,
            },
        };
        let groups = [
            g("b", None, "B"),
            g("a1", Some("a"), "Filha"),
            g("a", None, "A"),
            g("o", Some("sumiu"), "Órfã"),
        ];
        let order: Vec<(String, usize)> =
            tree(&groups).into_iter().map(|(g, d)| (g.id, d)).collect();
        assert_eq!(
            order,
            [
                ("a".into(), 0),
                ("a1".into(), 1),
                ("b".into(), 0),
                ("o".into(), 0)
            ]
        );
    }
}
