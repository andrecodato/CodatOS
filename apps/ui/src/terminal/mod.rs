//! Terminal em abas (`docs/modulos/conexoes.md#terminal`, RF-CON-020): shells deste PC,
//! distros WSL, containers e sessões SSH, cada um num canal binário do WebSocket `/rpc`.
//!
//! As sessões vivem fora dos componentes: trocar de tela ou de aba não derruba nenhum
//! terminal. O elemento de cada xterm fica num estacionamento escondido no `<body>` e passa
//! para o painel quando a aba dele está à vista.

mod panel;
mod snippets;
pub mod xterm;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use codatos_protocol::terminal::{METHOD_TERMINAL_OPEN, OpenTerminal, TerminalTarget, errors};
use codatos_protocol::vault::errors as vault_errors;
use js_sys::{Function, Reflect, Uint8Array};
use leptos::{prelude::*, task::spawn_local};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use web_sys::Element;

use crate::api::{self, ChannelEvent};
use crate::vault::Vault;
use xterm::{FitAddon, Terminal};

pub(crate) use panel::AskForm;
pub use panel::TerminalPanel;

/// Uma aba, do jeito que a tela precisa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    pub phase: Phase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    Open,
    /// A sessão SSH precisa de uma resposta do usuário para seguir.
    Ask(Ask),
    Failed(String),
    /// O processo saiu ou a conexão com o núcleo caiu.
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    /// Fingerprint (`SHA256:…`) do servidor para o usuário conferir.
    HostKey {
        fingerprint: String,
        changed: bool,
    },
    /// `rejected`: o servidor recusou a senha (ou a chave e a senha guardadas).
    Password {
        rejected: bool,
    },
    Username,
}

/// O lado do xterm e do canal de uma aba. Fica fora dos sinais (não é `Send`).
struct Session {
    target: TerminalTarget,
    element: Element,
    term: Option<(Terminal, FitAddon)>,
    /// Canal aberto; `0` sem canal.
    channel: Rc<Cell<u32>>,
    /// O usuário renomeou a aba: o título do núcleo não substitui.
    renamed: bool,
    /// Callbacks do xterm, vivos enquanto a sessão existir.
    listeners: Vec<Closure<dyn FnMut(JsValue)>>,
}

#[derive(Default)]
struct Store {
    sessions: HashMap<u64, Session>,
    next: u64,
    parking: Option<Element>,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store::default());
    static CURRENT: Cell<Option<Terminals>> = const { Cell::new(None) };
}

/// Abas abertas e a ativa. Criado uma vez no topo do app ([`Terminals::provide`]).
#[derive(Debug, Clone, Copy)]
pub struct Terminals {
    pub tabs: RwSignal<Vec<Tab>>,
    pub active: RwSignal<Option<u64>>,
}

impl Terminals {
    pub fn provide() {
        let terminals = Self {
            tabs: RwSignal::new(Vec::new()),
            active: RwSignal::new(None),
        };
        CURRENT.with(|c| c.set(Some(terminals)));
        provide_context(terminals);
    }

    /// O mesmo do contexto, também fora do dono reativo (depois de um `await`).
    pub fn current() -> Self {
        CURRENT
            .with(Cell::get)
            .expect("Terminals::provide no topo do app")
    }

    /// Abre uma aba nova e conecta.
    pub fn open(self, target: TerminalTarget, title: String) {
        let element = leptos::prelude::document()
            .create_element("div")
            .expect("div");
        element.set_class_name("h-full w-full");
        let _ = parking().append_child(&element);
        let id = STORE.with(|s| {
            let mut s = s.borrow_mut();
            s.next += 1;
            let id = s.next;
            s.sessions.insert(
                id,
                Session {
                    target,
                    element,
                    term: None,
                    channel: Rc::new(Cell::new(0)),
                    renamed: false,
                    listeners: Vec::new(),
                },
            );
            id
        });
        self.tabs.update(|tabs| {
            tabs.push(Tab {
                id,
                title,
                phase: Phase::Connecting,
            });
        });
        self.active.set(Some(id));
        self.start(id);
    }

    /// Abre outra aba com o mesmo alvo.
    pub fn duplicate(self, id: u64) {
        let Some(target) = STORE.with(|s| s.borrow().sessions.get(&id).map(|s| s.target.clone()))
        else {
            return;
        };
        let title = self
            .tabs
            .with_untracked(|tabs| tabs.iter().find(|t| t.id == id).map(|t| t.title.clone()))
            .unwrap_or_default();
        self.open(target, title);
    }

    /// Fecha a aba e encerra o processo do outro lado.
    pub fn close(self, id: u64) {
        let session = STORE.with(|s| s.borrow_mut().sessions.remove(&id));
        if let Some(session) = session {
            let channel = session.channel.replace(0);
            if channel != 0 {
                api::channel_close(channel);
            }
            if let Some((term, _)) = &session.term {
                term.dispose();
            }
            session.element.remove();
        }
        let mut next = None;
        self.tabs.update(|tabs| {
            if let Some(index) = tabs.iter().position(|t| t.id == id) {
                tabs.remove(index);
                next = tabs.get(index).or_else(|| tabs.last()).map(|t| t.id);
            }
        });
        if self.active.get_untracked() == Some(id) {
            self.active.set(next);
        }
    }

    pub fn rename(self, id: u64, title: &str) {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return;
        }
        STORE.with(|s| {
            if let Some(session) = s.borrow_mut().sessions.get_mut(&id) {
                session.renamed = true;
            }
        });
        self.update_tab(id, |tab| tab.title = title);
    }

    /// Conecta de novo uma aba que falhou ou terminou.
    pub fn reconnect(self, id: u64) {
        self.start(id);
    }

    /// Resposta a um [`Ask`]: tenta de novo com ela.
    /// Abas com a sessão aberta, a ativa primeiro.
    pub fn open_ids(self) -> Vec<u64> {
        let active = self.active.get_untracked();
        let mut ids: Vec<u64> = self.tabs.with_untracked(|tabs| {
            tabs.iter()
                .filter(|t| t.phase == Phase::Open)
                .map(|t| t.id)
                .collect()
        });
        ids.sort_by_key(|id| Some(*id) != active);
        ids
    }

    /// Manda texto como se fosse digitado. Devolve `false` se a aba não tem sessão aberta.
    pub fn type_text(self, id: u64, text: &str) -> bool {
        let open = self
            .tabs
            .with_untracked(|tabs| tabs.iter().any(|t| t.id == id && t.phase == Phase::Open));
        if !open {
            return false;
        }
        let channel = STORE.with(|s| s.borrow().sessions.get(&id).map_or(0, |s| s.channel.get()));
        if channel == 0 {
            return false;
        }
        api::channel_send(channel, text.as_bytes());
        true
    }

    pub fn answer(self, id: u64, answer: Answer) {
        let known = STORE.with(|s| {
            let mut s = s.borrow_mut();
            let Some(session) = s.sessions.get_mut(&id) else {
                return false;
            };
            if let TerminalTarget::Ssh {
                username,
                password,
                trust_host_key,
                ..
            } = &mut session.target
            {
                match answer {
                    Answer::TrustHostKey(fingerprint) => *trust_host_key = Some(fingerprint),
                    Answer::Password(value) => *password = Some(value),
                    Answer::Username(value) => *username = Some(value),
                }
            }
            true
        });
        if known {
            self.start(id);
        }
    }

    fn phase(self, id: u64, phase: Phase) {
        self.update_tab(id, |tab| tab.phase = phase);
    }

    fn update_tab(self, id: u64, f: impl FnOnce(&mut Tab)) {
        self.tabs.update(|tabs| {
            if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                f(tab);
            }
        });
    }

    fn start(self, id: u64) {
        self.phase(id, Phase::Connecting);
        spawn_local(async move {
            let prepared = match xterm::load().await {
                Ok(()) => STORE.with(|s| {
                    let mut s = s.borrow_mut();
                    s.sessions
                        .get_mut(&id)
                        .map(|session| ensure_term(session).map(|term| (term, prepare(session))))
                }),
                Err(err) => Some(Err(err)),
            };
            let (term, (request, channel)) = match prepared {
                // A aba foi fechada enquanto o xterm carregava.
                None => return,
                Some(Ok(ready)) => ready,
                Some(Err(err)) => {
                    leptos::logging::warn!("xterm.js não carregou: {err:?}");
                    self.phase(
                        id,
                        Phase::Failed("Não consegui carregar o terminal.".into()),
                    );
                    return;
                }
            };
            let request = OpenTerminal {
                cols: term.cols(),
                rows: term.rows(),
                ..request
            };
            OWED.with(|o| o.borrow_mut().remove(&owed_key(&channel)));
            let params = serde_json::to_value(&request).unwrap_or(Value::Null);
            let result =
                api::open_channel(METHOD_TERMINAL_OPEN, params, receiver(id, &term, &channel))
                    .await;
            match result {
                Ok((number, value)) => {
                    // A aba pode ter sido fechada enquanto conectava.
                    let renamed = STORE.with(|s| s.borrow().sessions.get(&id).map(|s| s.renamed));
                    let Some(renamed) = renamed else {
                        api::channel_close(number);
                        return;
                    };
                    // O processo pode ter saído antes da resposta chegar.
                    let ended = self.tabs.with_untracked(|tabs| {
                        tabs.iter().any(|t| t.id == id && t.phase == Phase::Ended)
                    });
                    if !ended {
                        channel.set(number);
                        flush_ack(&channel);
                        self.phase(id, Phase::Open);
                    }
                    if let Some(title) = value.get("title").and_then(Value::as_str)
                        && !renamed
                    {
                        let title = title.to_owned();
                        self.update_tab(id, |tab| tab.title = title);
                    }
                    if self.active.get_untracked() == Some(id) {
                        term.focus();
                    }
                }
                Err(error) => self.phase(id, failure(&error)),
            }
        });
    }
}

/// O que o usuário respondeu a um [`Ask`].
#[derive(Debug, Clone)]
pub enum Answer {
    TrustHostKey(String),
    Password(String),
    Username(String),
}

// Bytes recebidos e ainda não confirmados, antes de o número do canal chegar.
thread_local! {
    static OWED: RefCell<HashMap<usize, u32>> = RefCell::new(HashMap::new());
}

fn owed_key(channel: &Rc<Cell<u32>>) -> usize {
    Rc::as_ptr(channel) as usize
}

fn flush_ack(channel: &Rc<Cell<u32>>) {
    let owed = OWED.with(|o| o.borrow_mut().remove(&owed_key(channel)));
    if let Some(bytes) = owed {
        api::channel_ack(channel.get(), bytes);
    }
}

/// Recebe o canal: escreve no xterm e confirma ao núcleo quando o xterm terminou de processar.
fn receiver(
    id: u64,
    term: &Terminal,
    channel: &Rc<Cell<u32>>,
) -> impl FnMut(ChannelEvent) + 'static {
    let term = term.clone();
    let channel = Rc::clone(channel);
    move |event| match event {
        ChannelEvent::Data(bytes) => {
            #[allow(clippy::cast_possible_truncation)] // frames de até 64 KiB
            let len = bytes.len() as u32;
            let channel = Rc::clone(&channel);
            let done = Closure::once_into_js(move || match channel.get() {
                0 => OWED.with(|o| *o.borrow_mut().entry(owed_key(&channel)).or_default() += len),
                number => api::channel_ack(number, len),
            });
            term.write(&Uint8Array::from(bytes.as_slice()), done.unchecked_ref());
        }
        ChannelEvent::Closed => {
            channel.set(0);
            OWED.with(|o| o.borrow_mut().remove(&owed_key(&channel)));
            term.write_str("\r\n\x1b[90m[sessão encerrada]\x1b[0m\r\n");
            let terminals = Terminals::current();
            let open = terminals.tabs.with_untracked(|tabs| {
                tabs.iter()
                    .any(|t| t.id == id && matches!(t.phase, Phase::Open | Phase::Connecting))
            });
            if open {
                terminals.phase(id, Phase::Ended);
            }
        }
    }
}

/// Erro do `terminal.open` virando o que a aba mostra.
pub(crate) fn failure(error: &codatos_protocol::rpc::RpcError) -> Phase {
    let code = error.data.as_ref().map_or("", |p| p.code.as_str());
    let detail = error
        .data
        .as_ref()
        .and_then(|p| p.detail.clone())
        .unwrap_or_default();
    match code {
        errors::HOST_KEY_UNKNOWN => Phase::Ask(Ask::HostKey {
            fingerprint: detail,
            changed: false,
        }),
        errors::HOST_KEY_CHANGED => Phase::Ask(Ask::HostKey {
            fingerprint: detail,
            changed: true,
        }),
        errors::PASSWORD_REQUIRED => Phase::Ask(Ask::Password { rejected: false }),
        errors::AUTH_FAILED => Phase::Ask(Ask::Password { rejected: true }),
        errors::USERNAME_REQUIRED => Phase::Ask(Ask::Username),
        vault_errors::LOCKED => {
            if let Some(vault) = Vault::current() {
                vault.ask_unlock();
            }
            Phase::Failed("O cofre está trancado. Desbloqueie e tente de novo.".into())
        }
        _ => Phase::Failed(error.message.clone()),
    }
}

/// Pedido do `terminal.open` (o tamanho entra depois) e o canal da sessão. Senha digitada e
/// fingerprint aceita valem só para esta tentativa.
fn prepare(session: &mut Session) -> (OpenTerminal, Rc<Cell<u32>>) {
    let request = OpenTerminal {
        target: session.target.clone(),
        cols: 80,
        rows: 24,
    };
    if let TerminalTarget::Ssh {
        password,
        trust_host_key,
        ..
    } = &mut session.target
    {
        *password = None;
        *trust_host_key = None;
    }
    (request, Rc::clone(&session.channel))
}

/// Cria o xterm da sessão na primeira vez, ligado ao canal dela.
fn ensure_term(session: &mut Session) -> Result<Terminal, JsValue> {
    if let Some((term, _)) = &session.term {
        return Ok(term.clone());
    }
    let term = Terminal::new(&xterm::options());
    let fit = FitAddon::create()?;
    term.load_addon(&fit);
    term.open(&session.element);
    fit.fit();

    let typed = {
        let channel = Rc::clone(&session.channel);
        Closure::<dyn FnMut(JsValue)>::new(move |data: JsValue| {
            if let (Some(text), number) = (data.as_string(), channel.get())
                && number != 0
            {
                api::channel_send(number, text.as_bytes());
            }
        })
    };
    let binary = {
        let channel = Rc::clone(&session.channel);
        Closure::<dyn FnMut(JsValue)>::new(move |data: JsValue| {
            if let (Some(text), number) = (data.as_string(), channel.get())
                && number != 0
            {
                #[allow(clippy::cast_possible_truncation)] // o xterm manda um byte por caractere
                let bytes: Vec<u8> = text.chars().map(|c| c as u8).collect();
                api::channel_send(number, &bytes);
            }
        })
    };
    let resized = {
        let channel = Rc::clone(&session.channel);
        Closure::<dyn FnMut(JsValue)>::new(move |size: JsValue| {
            let number = channel.get();
            let get = |key: &str| {
                Reflect::get(&size, &key.into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .and_then(num_u16)
            };
            if let (Some(cols), Some(rows)) = (get("cols"), get("rows"))
                && number != 0
            {
                api::channel_resize(number, cols, rows);
            }
        })
    };
    term.on_data(typed.as_ref().unchecked_ref::<Function>());
    term.on_binary(binary.as_ref().unchecked_ref::<Function>());
    term.on_resize(resized.as_ref().unchecked_ref::<Function>());
    session.listeners = vec![typed, binary, resized];
    session.term = Some((term.clone(), fit));
    Ok(term)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn num_u16(value: f64) -> Option<u16> {
    (value.is_finite() && (1.0..=f64::from(u16::MAX)).contains(&value)).then_some(value as u16)
}

/// Põe o xterm da aba `id` dentro de `parent` (o painel) e todas as outras no estacionamento.
fn attach(active: Option<u64>, parent: Option<&Element>) {
    let parking = parking();
    STORE.with(|s| {
        for (id, session) in &s.borrow().sessions {
            let target = match parent {
                Some(parent) if Some(*id) == active => parent,
                _ => &parking,
            };
            let _ = target.append_child(&session.element);
        }
    });
    let open = active.is_some_and(|id| {
        Terminals::current()
            .tabs
            .with_untracked(|tabs| tabs.iter().any(|t| t.id == id && t.phase == Phase::Open))
    });
    fit(active, open);
}

/// Ajusta colunas e linhas da aba ao tamanho atual e, com `focus`, dá o foco a ela.
fn fit(id: Option<u64>, focus: bool) {
    let Some(id) = id else { return };
    let term = STORE.with(|s| s.borrow().sessions.get(&id).and_then(|s| s.term.clone()));
    if let Some((term, fit)) = term {
        fit.fit();
        if focus {
            term.focus();
        }
    }
}

/// Lugar escondido, com tamanho, onde os xterms fora de vista continuam recebendo saída.
fn parking() -> Element {
    STORE.with(|s| {
        s.borrow_mut()
            .parking
            .get_or_insert_with(|| {
                let document = leptos::prelude::document();
                let div = document.create_element("div").expect("div");
                let _ = div.set_attribute(
                    "style",
                    "position:fixed;left:-10000px;top:0;width:960px;height:540px;visibility:hidden;overflow:hidden",
                );
                let _ = div.set_attribute("aria-hidden", "true");
                if let Ok(Some(body)) = document.query_selector("body") {
                    let _ = body.append_child(&div);
                }
                div
            })
            .clone()
    })
}
