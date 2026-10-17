//! Dashboard (`docs/modulos/dashboard.md`): grade de 12 colunas, modo edição e catálogo.
//!
//! O layout vem do núcleo (`dashboards.*`) e só é gravado ao sair do modo edição
//! (RF-DASH-002). Abaixo de 768 px a grade vira uma coluna na ordem de leitura (RF-DASH-006).

mod catalog;
mod grid;
mod onboarding;
mod state;
mod widgets;

use std::fmt::Write as _;

use codatos_protocol::dashboard::{
    Dashboard as Saved, DashboardId, DashboardSummary, METHOD_DASHBOARDS_GET,
    METHOD_DASHBOARDS_LIST, METHOD_DASHBOARDS_SAVE, Widget,
};
use codatos_protocol::rpc::RpcError;
use leptos::{ev, html, prelude::*, task::spawn_local};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wasm_bindgen::JsCast;

use self::catalog::Kind;
use self::grid::{COLUMNS, Cell};
use self::onboarding::Onboarding;
use self::state::{ErrorState, Retry, Skeleton};
use crate::api::{self, CoreStatus};
use crate::components::icon::{Icon, Svg};

/// Altura de uma linha da grade e espaço entre células (tem de bater com `.dash-grid`).
const ROW_PX: f64 = 72.0;
const GAP_PX: f64 = 16.0;
/// Passos guardados para desfazer.
const HISTORY: usize = 50;
/// JSON-RPC "method not found": núcleo sem banco local.
const METHOD_NOT_FOUND: i32 = -32601;

#[derive(Debug, Clone, PartialEq)]
struct Item {
    id: String,
    kind: String,
    cell: Cell,
    config: Value,
}

#[derive(Debug, Clone, Default)]
struct History {
    undo: Vec<Vec<Item>>,
    redo: Vec<Vec<Item>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Move,
    Resize,
}

/// Arraste em andamento: tudo é calculado a partir do layout de quando ele começou.
#[derive(Debug, Clone)]
struct Drag {
    index: usize,
    mode: Mode,
    start: (f64, f64),
    origin: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
enum Load {
    Waiting,
    Loading,
    Ready,
    /// Primeira execução: o onboarding monta o dashboard padrão.
    Onboarding,
    /// Núcleo sem banco: o layout funciona, mas não é gravado.
    NoStore,
    Failed(String),
}

/// Estado compartilhado pela grade e pelos widgets dela.
#[derive(Clone, Copy)]
struct Board {
    meta: StoredValue<Option<(String, String, i64)>>,
    items: RwSignal<Vec<Item>>,
    editing: RwSignal<bool>,
    history: RwSignal<History>,
    drag: StoredValue<Option<Drag>>,
    dragging: RwSignal<Option<usize>>,
    /// Widget com o diálogo de configuração aberto.
    configuring: RwSignal<Option<String>>,
    grid: NodeRef<html::Div>,
}

impl Board {
    fn new() -> Self {
        Self {
            meta: StoredValue::new(None),
            items: RwSignal::new(Vec::new()),
            editing: RwSignal::new(false),
            history: RwSignal::new(History::default()),
            drag: StoredValue::new(None),
            dragging: RwSignal::new(None),
            configuring: RwSignal::new(None),
            grid: NodeRef::new(),
        }
    }

    fn commit(self, before: Vec<Item>) {
        self.history.update(|h| {
            h.undo.push(before);
            if h.undo.len() > HISTORY {
                h.undo.remove(0);
            }
            h.redo.clear();
        });
    }

    /// Aplica `f` ao layout e guarda o anterior para desfazer.
    fn change(self, f: impl FnOnce(&mut Vec<Item>)) {
        let before = self.items.get_untracked();
        let mut next = before.clone();
        f(&mut next);
        if next != before {
            self.items.set(next);
            self.commit(before);
        }
    }

    fn undo(self) {
        let Some(previous) = self.history.try_update(|h| h.undo.pop()).flatten() else {
            return;
        };
        let current = self.items.get_untracked();
        self.history.update(|h| h.redo.push(current));
        self.items.set(previous);
    }

    fn redo(self) {
        let Some(next) = self.history.try_update(|h| h.redo.pop()).flatten() else {
            return;
        };
        let current = self.items.get_untracked();
        self.history.update(|h| h.undo.push(current));
        self.items.set(next);
    }

    fn add(self, kind: &'static Kind) {
        self.change(|items| {
            let cells: Vec<Cell> = items.iter().map(|i| i.cell).collect();
            items.push(Item {
                id: new_id(),
                kind: kind.id.to_owned(),
                cell: grid::free_spot(&cells, kind.size.0, kind.size.1),
                config: json!({}),
            });
        });
    }

    fn set_config(self, id: &str, config: Value) {
        self.change(|items| {
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                item.config = config;
            }
        });
    }

    fn remove(self, id: &str) {
        self.change(|items| {
            items.retain(|i| i.id != id);
            place(items, None);
        });
    }

    /// Teclado: setas movem, Shift + setas redimensionam.
    fn nudge(self, id: &str, mode: Mode, dx: i32, dy: i32) {
        self.change(|items| {
            if let Some(index) = items.iter().position(|i| i.id == id) {
                items[index].cell = moved(&items[index], mode, dx, dy);
                place(items, Some(index));
            }
        });
    }

    fn begin(self, id: &str, mode: Mode, ev: &ev::PointerEvent) {
        if ev.button() != 0 {
            return;
        }
        let origin = self.items.get_untracked();
        let Some(index) = origin.iter().position(|i| i.id == id) else {
            return;
        };
        ev.prevent_default();
        // Sem captura, o arraste se perde quando o ponteiro sai do puxador.
        if let Some(el) = ev
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        self.drag.set_value(Some(Drag {
            index,
            mode,
            start: (f64::from(ev.client_x()), f64::from(ev.client_y())),
            origin,
        }));
        self.dragging.set(Some(index));
    }

    fn drag_to(self, ev: &ev::PointerEvent) {
        let Some(drag) = self.drag.get_value() else {
            return;
        };
        let Some(grid) = self.grid.get_untracked() else {
            return;
        };
        let column = (grid.get_bounding_client_rect().width() + GAP_PX) / f64::from(COLUMNS);
        let dx = steps(f64::from(ev.client_x()) - drag.start.0, column);
        let dy = steps(f64::from(ev.client_y()) - drag.start.1, ROW_PX + GAP_PX);
        let mut next = drag.origin;
        next[drag.index].cell = moved(&next[drag.index], drag.mode, dx, dy);
        place(&mut next, Some(drag.index));
        if next != self.items.get_untracked() {
            self.items.set(next);
        }
    }

    fn end(self) {
        let Some(drag) = self.drag.get_value() else {
            return;
        };
        self.drag.set_value(None);
        self.dragging.set(None);
        if self.items.get_untracked() != drag.origin {
            self.commit(drag.origin);
        }
    }

    fn to_saved(self) -> Option<Saved> {
        let (id, name, position) = self.meta.get_value()?;
        let widgets = self
            .items
            .get_untracked()
            .into_iter()
            .map(|i| Widget {
                id: i.id,
                kind: i.kind,
                layout: json!({ "lg": i.cell }),
                config: i.config,
            })
            .collect();
        Some(Saved {
            id,
            name,
            position,
            widgets,
        })
    }
}

/// Célula do item depois de arrastar `dx × dy` células, no tamanho mínimo do tipo dele.
fn moved(item: &Item, mode: Mode, dx: i32, dy: i32) -> Cell {
    let (min_w, min_h) = catalog::find(&item.kind).map_or((1, 1), |k| k.min);
    match mode {
        Mode::Move => item.cell.shifted(dx, dy, min_w, min_h),
        Mode::Resize => item.cell.resized(dx, dy, min_w, min_h),
    }
}

/// Compacta a grade; `priority` é o widget que acabou de mudar.
fn place(items: &mut [Item], priority: Option<usize>) {
    let mut cells: Vec<Cell> = items.iter().map(|i| i.cell).collect();
    grid::compact(&mut cells, priority);
    for (item, cell) in items.iter_mut().zip(cells) {
        item.cell = cell;
    }
}

#[allow(clippy::cast_possible_truncation)] // deslocamentos de tela cabem num i32
fn steps(px: f64, step: f64) -> i32 {
    (px / step).round() as i32
}

/// UUID v7 (tempo + aleatório), como pede o modelo de dados para ids gerados no cliente.
fn new_id() -> String {
    let mut random = [0u8; 10];
    if let Ok(crypto) = window().crypto() {
        let _ = crypto.get_random_values_with_u8_array(&mut random);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // ms desde 1970
    let ms = js_sys::Date::now() as u64;
    uuid_v7(ms, random)
}

fn uuid_v7(ms: u64, random: [u8; 10]) -> String {
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[6..].copy_from_slice(&random);
    b[6] = 0x70 | (b[6] & 0x0f);
    b[8] = 0x80 | (b[8] & 0x3f);
    let mut out = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn item_from(widget: Widget) -> Item {
    let cell = serde_json::from_value(widget.layout["lg"].clone()).unwrap_or(Cell {
        x: 0,
        y: u16::MAX / 2,
        w: 3,
        h: 2,
    });
    let (min_w, min_h) = catalog::find(&widget.kind).map_or((1, 1), |k| k.min);
    Item {
        id: widget.id,
        kind: widget.kind,
        cell: cell.clamped(min_w, min_h),
        config: widget.config,
    }
}

/// Dashboard inicial enquanto o onboarding não existe.
/// Widgets que entram sempre no dashboard padrão, já arrumados na grade.
const DEFAULT_WIDGETS: &[(&str, Cell)] = &[
    (
        "system.usage",
        Cell {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        },
    ),
    (
        "clock",
        Cell {
            x: 4,
            y: 0,
            w: 4,
            h: 2,
        },
    ),
    (
        "projects",
        Cell {
            x: 8,
            y: 0,
            w: 4,
            h: 4,
        },
    ),
    (
        "system.history",
        Cell {
            x: 0,
            y: 2,
            w: 8,
            h: 3,
        },
    ),
    (
        "disks",
        Cell {
            x: 8,
            y: 4,
            w: 4,
            h: 3,
        },
    ),
    (
        "network",
        Cell {
            x: 0,
            y: 5,
            w: 4,
            h: 2,
        },
    ),
];

/// Posições do dashboard padrão (RF-DASH-001): os de sempre mais `extra`, os que têm fonte
/// de dados neste PC (WSL, Docker, GPU, clima com cidade escolhida), nos lugares livres.
fn default_layout(extra: &[&str]) -> Vec<(&'static str, Cell)> {
    let mut out: Vec<(&'static str, Cell)> = DEFAULT_WIDGETS.to_vec();
    for kind in extra {
        let Some(k) = catalog::find(kind) else {
            continue;
        };
        let cells: Vec<Cell> = out.iter().map(|(_, c)| *c).collect();
        out.push((k.id, grid::free_spot(&cells, k.size.0, k.size.1)));
    }
    out
}

fn default_dashboard(extra: &[&str]) -> Saved {
    let widgets = default_layout(extra)
        .into_iter()
        .map(|(kind, cell)| Widget {
            id: new_id(),
            kind: kind.to_owned(),
            layout: json!({ "lg": cell }),
            config: json!({}),
        })
        .collect();
    Saved {
        id: new_id(),
        name: "Dia a dia".to_owned(),
        position: 0,
        widgets,
    }
}

/// O que o núcleo tem para a tela inicial.
enum Fetched {
    Saved(Saved),
    /// Nenhum dashboard ainda: primeira execução, vai para o onboarding.
    Empty,
    /// Núcleo sem banco local.
    NoStore,
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T, RpcError> {
    serde_json::from_value(value)
        .map_err(|e| RpcError::new(-32000, "bad_response", format!("Resposta inesperada: {e}")))
}

/// Primeiro dashboard salvo.
async fn fetch() -> Result<Fetched, RpcError> {
    let list = match api::call(METHOD_DASHBOARDS_LIST, json!({})).await {
        Ok(v) => decode::<Vec<DashboardSummary>>(v)?,
        Err(e) if e.code == METHOD_NOT_FOUND => return Ok(Fetched::NoStore),
        Err(e) => return Err(e),
    };
    if let Some(first) = list.first() {
        let params = serde_json::to_value(DashboardId {
            id: first.id.clone(),
        })
        .unwrap_or_default();
        if let Some(saved) =
            decode::<Option<Saved>>(api::call(METHOD_DASHBOARDS_GET, params).await?)?
        {
            return Ok(Fetched::Saved(saved));
        }
    }
    Ok(Fetched::Empty)
}

#[component]
pub fn Dashboard(status: ReadSignal<CoreStatus>) -> impl IntoView {
    let board = Board::new();
    provide_context(board);
    let load = RwSignal::new(Load::Waiting);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let catalog_open = RwSignal::new(false);

    let apply = move |saved: Saved, state: Load| {
        let mut items: Vec<Item> = saved.widgets.into_iter().map(item_from).collect();
        place(&mut items, None);
        board
            .meta
            .set_value(Some((saved.id, saved.name, saved.position)));
        board.items.set(items);
        load.set(state);
    };

    // Carrega quando o núcleo conecta (e de novo ao pedir "tentar de novo").
    Effect::new(move |_| {
        if status.get() != CoreStatus::Connected || load.get() != Load::Waiting {
            return;
        }
        load.set(Load::Loading);
        spawn_local(async move {
            match fetch().await {
                Ok(Fetched::Saved(saved)) => apply(saved, Load::Ready),
                Ok(Fetched::Empty) => load.set(Load::Onboarding),
                Ok(Fetched::NoStore) => apply(default_dashboard(&[]), Load::NoStore),
                Err(e) if e.code == -32000 && e.message == "Núcleo offline" => {
                    load.set(Load::Waiting);
                }
                Err(e) => load.set(Load::Failed(e.message)),
            }
        });
    });

    let finish = move || {
        if load.get_untracked() != Load::Ready {
            leave_edit(board, catalog_open);
            return;
        }
        let Some(saved) = board.to_saved() else {
            return;
        };
        saving.set(true);
        save_error.set(None);
        spawn_local(async move {
            let result = api::call(
                METHOD_DASHBOARDS_SAVE,
                serde_json::to_value(&saved).unwrap_or_default(),
            )
            .await;
            saving.set(false);
            match result {
                Ok(_) => {
                    leave_edit(board, catalog_open);
                    board.history.set(History::default());
                }
                Err(e) => save_error.set(Some(e.message)),
            }
        });
    };

    undo_shortcuts(board);

    let name = move || {
        load.track();
        board
            .meta
            .with_value(|m| m.as_ref().map(|(_, name, _)| name.clone()))
    };

    view! {
        <section class="mt-6" aria-label="Dashboard">
            <div class="mb-4 flex min-h-9 items-center gap-2">
                <h2 class="text-sm font-semibold">{name}</h2>
                {move || (load.get() == Load::NoStore).then(|| view! {
                    <span class="text-xs text-warning">"Sem banco local: o layout não será salvo."</span>
                })}
                <Toolbar board=board load=load saving=saving catalog_open=catalog_open finish=finish />
            </div>
            {move || save_error.get().map(|e| view! {
                <p role="alert" class="mb-3 text-sm text-danger">{format!("Não foi possível salvar: {e}")}</p>
            })}
            {move || match load.get() {
                Load::Waiting | Load::Loading => view! { <BoardSkeleton /> }.into_any(),
                Load::Failed(message) => view! { <LoadFailed message=message load=load /> }.into_any(),
                Load::Ready | Load::NoStore => view! { <Grid catalog_open=catalog_open /> }.into_any(),
                Load::Onboarding => {
                    let done = Callback::new(move |saved: Saved| apply(saved, Load::Ready));
                    view! { <Onboarding on_done=done /> }.into_any()
                }
            }}
            <Show when=move || catalog_open.get()>
                <Catalog open=catalog_open />
            </Show>
            {move || board.configuring.get().map(|id| view! { <ConfigDialog id=id /> })}
        </section>
    }
}

fn leave_edit(board: Board, catalog_open: RwSignal<bool>) {
    board.editing.set(false);
    board.configuring.set(None);
    catalog_open.set(false);
}

/// Ctrl + Z / Ctrl + Shift + Z (ou Ctrl + Y) no modo edição.
fn undo_shortcuts(board: Board) {
    let keys = window_event_listener(ev::keydown, move |e| {
        if !board.editing.get_untracked() || !(e.ctrl_key() || e.meta_key()) {
            return;
        }
        match (e.key().to_lowercase().as_str(), e.shift_key()) {
            ("z", false) => board.undo(),
            ("z", true) | ("y", _) => board.redo(),
            _ => return,
        }
        e.prevent_default();
    });
    on_cleanup(move || keys.remove());
}

#[component]
fn LoadFailed(message: String, load: RwSignal<Load>) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-line bg-card p-5 shadow-card">
            <ErrorState title="O dashboard não carregou".to_owned() detail=message />
            <button
                type="button"
                on:click=move |_| load.set(Load::Waiting)
                class="mt-2 rounded-md border border-line px-2 py-1 text-xs hover:bg-card-hover"
            >
                "Tentar de novo"
            </button>
        </div>
    }
}

#[component]
fn Toolbar<F>(
    board: Board,
    load: RwSignal<Load>,
    saving: RwSignal<bool>,
    catalog_open: RwSignal<bool>,
    finish: F,
) -> impl IntoView
where
    F: Fn() + Copy + Send + Sync + 'static,
{
    let ready = move || matches!(load.get(), Load::Ready | Load::NoStore);
    let button = "items-center gap-1.5 rounded-md px-2.5 py-1.5 text-sm text-muted hover:bg-card-hover hover:text-fg disabled:opacity-40 disabled:hover:bg-transparent";
    view! {
        <div class="ml-auto flex items-center gap-1">
            <Show
                when=move || board.editing.get()
                fallback=move || view! {
                    // Editar só no desktop; no celular a grade se reorganiza sozinha.
                    <button
                        type="button"
                        class=format!("hidden md:inline-flex {button}")
                        disabled=move || !ready()
                        on:click=move |_| board.editing.set(true)
                    >
                        <Svg icon=Icon::Pencil size=16 />
                        "Editar"
                    </button>
                }
            >
                <button
                    type="button"
                    class=format!("inline-flex {button}")
                    aria-label="Desfazer"
                    title="Desfazer (Ctrl + Z)"
                    disabled=move || board.history.with(|h| h.undo.is_empty())
                    on:click=move |_| board.undo()
                >
                    <Svg icon=Icon::Undo size=16 />
                </button>
                <button
                    type="button"
                    class=format!("inline-flex {button}")
                    aria-label="Refazer"
                    title="Refazer (Ctrl + Shift + Z)"
                    disabled=move || board.history.with(|h| h.redo.is_empty())
                    on:click=move |_| board.redo()
                >
                    <Svg icon=Icon::Redo size=16 />
                </button>
                <button type="button" class=format!("inline-flex {button}") on:click=move |_| catalog_open.set(true)>
                    <Svg icon=Icon::Plus size=16 />
                    "Adicionar widget"
                </button>
                <button
                    type="button"
                    class="ml-1 inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-hover disabled:opacity-60"
                    disabled=move || saving.get()
                    on:click=move |_| finish()
                >
                    <Svg icon=Icon::Check size=16 />
                    {move || if saving.get() { "Salvando…" } else { "Concluir" }}
                </button>
            </Show>
        </div>
    }
}

#[component]
fn Grid(catalog_open: RwSignal<bool>) -> impl IntoView {
    let board = expect_context::<Board>();
    let empty = move || board.items.with(Vec::is_empty);
    view! {
        <Show
            when=move || !empty() || board.editing.get()
            fallback=move || view! {
                <div class="flex flex-col items-center gap-3 rounded-lg border border-dashed border-line-strong p-10 text-center">
                    <p class="text-sm text-muted">"Nenhum widget por aqui ainda."</p>
                    <button
                        type="button"
                        class="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-hover"
                        on:click=move |_| {
                            board.editing.set(true);
                            catalog_open.set(true);
                        }
                    >
                        <Svg icon=Icon::Plus size=16 />
                        "Adicionar widget"
                    </button>
                </div>
            }
        >
            <div
                node_ref=board.grid
                class="dash-grid"
                class:dash-editing=move || board.editing.get()
                style=move || {
                    // No modo edição sobram duas linhas livres para soltar widgets embaixo.
                    let rows = board.items.with(|v| grid::rows(&v.iter().map(|i| i.cell).collect::<Vec<_>>()));
                    let extra = if board.editing.get() { 2 } else { 0 };
                    format!("--rows: {}", rows + extra)
                }
            >
                <For each=move || board.items.get() key=|item| item.id.clone() let:item>
                    <GridItem id=item.id kind=item.kind />
                </For>
            </div>
        </Show>
    }
}

#[component]
fn GridItem(id: String, kind: String) -> impl IntoView {
    let board = expect_context::<Board>();
    let kind_ref = catalog::find(&kind);
    let id = StoredValue::new(id);
    let index = move || {
        board
            .items
            .with(|v| id.with_value(|id| v.iter().position(|i| &i.id == id)))
    };

    let style = move || {
        let cell = board
            .items
            .with(|v| id.with_value(|id| v.iter().find(|i| &i.id == id).map(|i| i.cell)));
        let Some(c) = cell else { return String::new() };
        let min_h = f64::from(c.h) * ROW_PX + f64::from(c.h.saturating_sub(1)) * GAP_PX;
        format!(
            "--col: {} / span {}; --row: {} / span {}; --order: {}; --min-h: {min_h}px",
            c.x + 1,
            c.w,
            c.y + 1,
            c.h,
            u32::from(c.y) * u32::from(COLUMNS) + u32::from(c.x),
        )
    };
    let class = move || {
        let dragged = board.dragging.get().is_some() && board.dragging.get() == index();
        let mut class =
            "dash-item flex flex-col overflow-hidden rounded-lg border bg-card shadow-card"
                .to_owned();
        class.push_str(if dragged {
            " z-10 border-primary shadow-glow"
        } else if board.editing.get() {
            " border-dashed border-line-strong"
        } else {
            " border-line"
        });
        class
    };

    let retry = Retry(RwSignal::new(0));
    provide_context(retry);
    let title = kind_ref.map_or("Widget desconhecido", |k| k.name);
    let icon = kind_ref.map_or(Icon::Alert, |k| k.icon);
    // Mudar a configuração remonta o conteúdo do widget com ela.
    let config = Memo::new(move |_| {
        board
            .items
            .with(|v| id.with_value(|id| v.iter().find(|i| &i.id == id).map(|i| i.config.clone())))
    });
    let body = move || {
        retry.0.track();
        match kind_ref {
            Some(k) => config.with(|c| (k.render)(c.as_ref().unwrap_or(&Value::Null))),
            None => view! {
                <ErrorState title="Este widget não existe nesta versão".to_owned() detail=kind.clone() />
            }
            .into_any(),
        }
    };

    let on_key = move |mode: Mode| {
        move |e: ev::KeyboardEvent| {
            let (dx, dy) = match e.key().as_str() {
                "ArrowLeft" => (-1, 0),
                "ArrowRight" => (1, 0),
                "ArrowUp" => (0, -1),
                "ArrowDown" => (0, 1),
                _ => return,
            };
            e.prevent_default();
            let mode = if e.shift_key() { Mode::Resize } else { mode };
            id.with_value(|id| board.nudge(id, mode, dx, dy));
        }
    };

    view! {
        <article class=class style=style aria-label=title>
            <header class="flex h-10 shrink-0 items-center gap-2 px-4 text-sm font-medium text-muted">
                <Svg icon=icon size=16 />
                <h3 class="truncate">{title}</h3>
                <Show when=move || board.editing.get()>
                    <div class="ml-auto flex items-center gap-0.5">
                        <button
                            type="button"
                            class="cursor-grab touch-none rounded p-1 hover:bg-card-hover hover:text-fg active:cursor-grabbing"
                            aria-label=format!("Mover {title} (setas; Shift + setas redimensiona)")
                            on:pointerdown=move |e| id.with_value(|id| board.begin(id, Mode::Move, &e))
                            on:pointermove=move |e| board.drag_to(&e)
                            on:pointerup=move |_| board.end()
                            on:pointercancel=move |_| board.end()
                            on:keydown=on_key(Mode::Move)
                        >
                            <Svg icon=Icon::Grip size=16 />
                        </button>
                        {kind_ref.and_then(|k| k.editor).map(|_| view! {
                            <button
                                type="button"
                                class="rounded p-1 hover:bg-card-hover hover:text-fg"
                                aria-label=format!("Configurar {title}")
                                on:click=move |_| board.configuring.set(Some(id.get_value()))
                            >
                                <Svg icon=Icon::Settings size=16 />
                            </button>
                        })}
                        <button
                            type="button"
                            class="rounded p-1 hover:bg-card-hover hover:text-danger"
                            aria-label=format!("Remover {title}")
                            on:click=move |_| id.with_value(|id| board.remove(id))
                        >
                            <Svg icon=Icon::Trash size=16 />
                        </button>
                    </div>
                </Show>
            </header>
            <div class="relative min-h-0 flex-1 px-4 pb-4">
                {body}
                <Show when=move || board.editing.get()>
                    <button
                        type="button"
                        class="absolute right-1 bottom-1 cursor-nwse-resize touch-none rounded p-0.5 text-subtle hover:text-fg"
                        aria-label=format!("Redimensionar {title}")
                        on:pointerdown=move |e| id.with_value(|id| board.begin(id, Mode::Resize, &e))
                        on:pointermove=move |e| board.drag_to(&e)
                        on:pointerup=move |_| board.end()
                        on:pointercancel=move |_| board.end()
                        on:keydown=on_key(Mode::Resize)
                    >
                        <Svg icon=Icon::Resize size=14 />
                    </button>
                </Show>
            </div>
        </article>
    }
}

#[component]
fn BoardSkeleton() -> impl IntoView {
    view! {
        <div class="dash-grid" style="--rows: 2">
            {[(1, 4), (5, 3), (8, 5)]
                .map(|(col, span)| view! {
                    <div
                        class="dash-item rounded-lg border border-line bg-card p-4 shadow-card"
                        style=format!("--col: {col} / span {span}; --row: 1 / span 2; --min-h: 160px")
                    >
                        <Skeleton />
                    </div>
                })}
        </div>
    }
}

/// Configuração de uma instância (RF-DASH-004). Aplicar entra no desfazer e é gravado
/// junto com o layout ao concluir.
#[component]
fn ConfigDialog(id: String) -> impl IntoView {
    let board = expect_context::<Board>();
    let item = board
        .items
        .with_untracked(|v| v.iter().find(|i| i.id == id).cloned());
    let Some(item) = item else {
        return ().into_any();
    };
    let Some(kind) = catalog::find(&item.kind) else {
        return ().into_any();
    };
    let Some(editor) = kind.editor else {
        return ().into_any();
    };
    let draft = RwSignal::new(item.config);
    let close = move || board.configuring.set(None);
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            close();
        }
    });
    on_cleanup(move || keys.remove());
    let id = StoredValue::new(id);

    view! {
        <div class="fixed inset-0 z-40 bg-app/60" on:click=move |_| close()></div>
        <div
            role="dialog"
            aria-modal="true"
            aria-label=format!("Configurar {}", kind.name)
            class="fixed top-1/2 left-1/2 z-50 flex max-h-[80vh] w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl border border-line bg-sidebar shadow-card"
        >
            <header class="flex items-center gap-2 border-b border-line px-5 py-4">
                <Svg icon=kind.icon size=16 />
                <h2 class="text-sm font-semibold">{format!("Configurar {}", kind.name)}</h2>
            </header>
            <div class="flex-1 overflow-auto p-5">{editor(draft)}</div>
            <footer class="flex justify-end gap-2 border-t border-line px-5 py-3">
                <button
                    type="button"
                    class="rounded-md px-3 py-1.5 text-sm text-muted hover:bg-card-hover hover:text-fg"
                    on:click=move |_| close()
                >
                    "Cancelar"
                </button>
                <button
                    type="button"
                    class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-hover"
                    on:click=move |_| {
                        id.with_value(|id| board.set_config(id, draft.get_untracked()));
                        close();
                    }
                >
                    "Aplicar"
                </button>
            </footer>
        </div>
    }
    .into_any()
}

/// Painel lateral com busca para adicionar widgets.
#[component]
fn Catalog(open: RwSignal<bool>) -> impl IntoView {
    let board = expect_context::<Board>();
    let (query, set_query) = signal(String::new());
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = input.get() {
            let _ = el.focus();
        }
    });
    let close = move || open.set(false);
    let keys = window_event_listener(ev::keydown, move |e| {
        if e.key() == "Escape" {
            close();
        }
    });
    on_cleanup(move || keys.remove());

    view! {
        <div class="fixed inset-0 z-40 bg-app/60" on:click=move |_| close()></div>
        <aside
            role="dialog"
            aria-modal="true"
            aria-label="Adicionar widget"
            class="fixed inset-y-0 right-0 z-50 flex w-full max-w-sm flex-col border-l border-line bg-sidebar shadow-card"
        >
            <header class="flex items-center gap-2 border-b border-line px-5 py-4">
                <h2 class="text-sm font-semibold">"Adicionar widget"</h2>
                <button
                    type="button"
                    class="ml-auto rounded p-1 text-muted hover:bg-card-hover hover:text-fg"
                    aria-label="Fechar"
                    on:click=move |_| close()
                >
                    <Svg icon=Icon::X size=16 />
                </button>
            </header>
            <div class="px-5 pt-4">
                <label class="flex items-center gap-2 rounded-md border border-line bg-input px-3 py-1.5 text-sm text-subtle">
                    <Svg icon=Icon::Search size=16 />
                    <input
                        node_ref=input
                        type="search"
                        placeholder="Buscar widgets"
                        class="w-full bg-transparent text-fg placeholder:text-subtle focus:outline-none"
                        prop:value=query
                        on:input=move |e| set_query.set(event_target_value(&e))
                    />
                </label>
            </div>
            <ul class="flex-1 space-y-2 overflow-auto p-5">
                {move || {
                    let found = catalog::search(&query.get());
                    if found.is_empty() {
                        return view! { <li class="text-sm text-muted">"Nenhum widget encontrado."</li> }.into_any();
                    }
                    found
                        .into_iter()
                        .map(|k| view! {
                            <li class="flex items-center gap-3 rounded-lg border border-line bg-card p-3">
                                <span class="rounded-md bg-primary-soft p-2 text-primary">
                                    <Svg icon=k.icon size=18 />
                                </span>
                                <div class="min-w-0 flex-1">
                                    <p class="text-sm font-medium">{k.name}</p>
                                    <p class="text-xs text-muted">{k.description}</p>
                                    <p class="mt-0.5 text-[11px] text-subtle">{format!("{} × {} células", k.size.0, k.size.1)}</p>
                                </div>
                                <button
                                    type="button"
                                    class="rounded-md border border-line px-2 py-1 text-xs hover:bg-card-hover"
                                    aria-label=format!("Adicionar {}", k.name)
                                    on:click=move |_| {
                                        board.add(k);
                                        open.set(false);
                                    }
                                >
                                    "Adicionar"
                                </button>
                            </li>
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
        </aside>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v7_tem_versao_variante_e_tempo() {
        let id = uuid_v7(0x0189_abcd_ef01, [0xff; 10]);
        assert_eq!(id, "0189abcd-ef01-7fff-bfff-ffffffffffff");
        assert_eq!(id.len(), 36);
    }

    #[test]
    fn padrao_sem_sobreposicao_e_com_extras() {
        assert!(
            DEFAULT_WIDGETS
                .iter()
                .all(|(k, _)| catalog::find(k).is_some())
        );
        let layout = default_layout(&["wsl", "nao.existe"]);
        assert_eq!(layout.len(), DEFAULT_WIDGETS.len() + 1);
        assert_eq!(layout.last().map(|(k, _)| *k), Some("wsl"));
        let cells: Vec<Cell> = layout.iter().map(|(_, c)| *c).collect();
        let mut compacted = cells.clone();
        grid::compact(&mut compacted, None);
        assert_eq!(cells, compacted, "já nasce compactado e sem sobreposição");
    }

    #[test]
    fn widget_sem_layout_vai_para_o_fim_e_compacta() {
        let mut items = vec![
            item_from(Widget {
                id: "a".into(),
                kind: "clock".into(),
                layout: json!({}),
                config: json!({}),
            }),
            item_from(Widget {
                id: "b".into(),
                kind: "system.usage".into(),
                layout: json!({ "lg": { "x": 0, "y": 0, "w": 1, "h": 1 } }),
                config: json!({}),
            }),
        ];
        place(&mut items, None);
        // O mínimo do tipo vale mesmo com layout salvo menor.
        assert_eq!(
            items[1].cell,
            Cell {
                x: 0,
                y: 0,
                w: 3,
                h: 2
            }
        );
        assert_eq!(
            items[0].cell,
            Cell {
                x: 0,
                y: 2,
                w: 3,
                h: 2
            }
        );
    }
}
