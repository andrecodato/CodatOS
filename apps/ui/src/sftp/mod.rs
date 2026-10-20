//! SFTP em painel duplo (`docs/modulos/conexoes.md#sftp`): à esquerda este PC, à direita o
//! servidor. A sessão é um canal binário de `sftp.open`; por ele o núcleo manda o andamento
//! das transferências. Como os terminais, a sessão vive fora dos componentes: trocar de tela
//! não derruba a conexão nem a fila.

mod panel;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use codatos_protocol::files::{
    METHOD_FILES_LIST, METHOD_FILES_MKDIR, METHOD_FILES_REMOVE, METHOD_FILES_RENAME,
};
use codatos_protocol::sftp::{
    ControlTransfer, Direction, Entry, Listing, METHOD_SFTP_CHMOD, METHOD_SFTP_LIST,
    METHOD_SFTP_MKDIR, METHOD_SFTP_OPEN, METHOD_SFTP_REMOVE, METHOD_SFTP_RENAME,
    METHOD_SFTP_TRANSFER, METHOD_SFTP_TRANSFER_CONTROL, OpenSftp, Opened, StartTransfer,
    TransferAction, TransferEvent, TransferStarted, TransferState, errors,
};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};

use crate::api::{self, ChannelEvent};
use crate::connections::ui;
use crate::terminal::{self, Answer, Ask};

pub use panel::SftpPanel;

/// Um dos dois painéis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Local,
    Remote,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Self::Local => Self::Remote,
            Self::Remote => Self::Local,
        }
    }

    /// Separador de caminho do lado. Este PC usa `\` quando a pasta é do Windows.
    fn separator(self, dir: &str) -> char {
        if self == Self::Local && dir.contains('\\') {
            '\\'
        } else {
            '/'
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Nenhum host escolhido.
    Idle,
    Connecting,
    Ask(Ask),
    Failed(String),
    Open,
    /// A conexão caiu ou o núcleo saiu do ar.
    Ended,
}

/// Uma pasta à vista num painel.
#[derive(Debug, Clone, Copy)]
pub struct Pane {
    /// Caminho canônico da listagem à vista (vazio antes da primeira).
    pub path: RwSignal<String>,
    /// `None` enquanto carrega.
    pub listing: RwSignal<Option<Result<Vec<Entry>, String>>>,
    pub selected: RwSignal<HashSet<String>>,
    /// Descarta respostas de listagens que chegaram fora de ordem.
    seq: StoredValue<u64>,
}

impl Pane {
    fn new() -> Self {
        Self {
            path: RwSignal::new(String::new()),
            listing: RwSignal::new(None),
            selected: RwSignal::new(HashSet::new()),
            seq: StoredValue::new(0),
        }
    }

    /// O item com esse nome na listagem à vista.
    pub fn entry(self, name: &str) -> Option<Entry> {
        self.listing.with_untracked(|l| {
            l.as_ref()
                .and_then(|r| r.as_ref().ok())
                .and_then(|entries| entries.iter().find(|e| e.name == name).cloned())
        })
    }
}

/// Uma transferência na fila, do jeito que a tela mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub direction: Direction,
    pub name: String,
    pub local: String,
    pub remote: String,
    pub state: TransferState,
    pub done: u64,
    pub total: u64,
    pub error: Option<String>,
}

impl Row {
    pub fn finished(&self) -> bool {
        matches!(self.state, TransferState::Done | TransferState::Canceled)
    }
}

/// Transferência que esbarrou num destino que já existe, esperando o usuário decidir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub name: String,
    pub request: StartTransfer,
}

/// A conexão atual, fora dos sinais.
struct Conn {
    login: OpenSftp,
    session: Option<String>,
    channel: u32,
    /// Muda a cada tentativa: respostas e eventos de uma tentativa velha são ignorados.
    attempt: u64,
}

thread_local! {
    static CONN: RefCell<Option<Conn>> = const { RefCell::new(None) };
    static CURRENT: Cell<Option<Sftp>> = const { Cell::new(None) };
}

/// Estado da tela do SFTP. Criado uma vez no topo do app ([`Sftp::provide`]).
#[derive(Debug, Clone, Copy)]
pub struct Sftp {
    pub phase: RwSignal<Phase>,
    /// Nome do host conectado (ou tentando).
    pub title: RwSignal<String>,
    pub local: Pane,
    pub remote: Pane,
    pub transfers: RwSignal<Vec<Row>>,
    pub conflicts: RwSignal<Vec<Conflict>>,
    /// Aviso de uma ação que falhou, até o usuário dispensar.
    pub notice: RwSignal<Option<String>>,
    /// De que painel vêm os itens sendo arrastados.
    pub dragging: RwSignal<Option<Side>>,
}

impl Sftp {
    pub fn provide() {
        let sftp = Self {
            phase: RwSignal::new(Phase::Idle),
            title: RwSignal::new(String::new()),
            local: Pane::new(),
            remote: Pane::new(),
            transfers: RwSignal::new(Vec::new()),
            conflicts: RwSignal::new(Vec::new()),
            notice: RwSignal::new(None),
            dragging: RwSignal::new(None),
        };
        CURRENT.with(|c| c.set(Some(sftp)));
        provide_context(sftp);
    }

    /// O mesmo do contexto, também fora do dono reativo (depois de um `await`).
    pub fn current() -> Self {
        CURRENT
            .with(Cell::get)
            .expect("Sftp::provide no topo do app")
    }

    pub fn pane(self, side: Side) -> Pane {
        match side {
            Side::Local => self.local,
            Side::Remote => self.remote,
        }
    }

    /// Conecta a um host cadastrado, encerrando a sessão anterior.
    pub fn open(self, host_id: String, title: String) {
        self.close();
        CONN.with(|c| {
            *c.borrow_mut() = Some(Conn {
                login: OpenSftp {
                    host_id,
                    username: None,
                    password: None,
                    trust_host_key: None,
                },
                session: None,
                channel: 0,
                attempt: 0,
            });
        });
        self.title.set(title);
        self.connect();
        if self.local.path.get_untracked().is_empty() {
            self.list(Side::Local, "");
        }
    }

    /// Encerra a sessão e volta à escolha de host. A fila do servidor some junto.
    pub fn close(self) {
        if let Some(conn) = CONN.with(|c| c.borrow_mut().take())
            && conn.channel != 0
        {
            api::channel_close(conn.channel);
        }
        self.phase.set(Phase::Idle);
        self.transfers.set(Vec::new());
        self.conflicts.set(Vec::new());
        self.remote.path.set(String::new());
        self.remote.listing.set(None);
        self.remote.selected.set(HashSet::new());
    }

    /// Tenta de novo com o mesmo host (depois de uma queda ou falha).
    pub fn reconnect(self) {
        self.connect();
    }

    /// Resposta a uma pergunta da conexão; vale só para esta tentativa.
    pub fn answer(self, answer: Answer) {
        CONN.with(|c| {
            if let Some(conn) = c.borrow_mut().as_mut() {
                match answer {
                    Answer::TrustHostKey(fingerprint) => {
                        conn.login.trust_host_key = Some(fingerprint);
                    }
                    Answer::Password(password) => conn.login.password = Some(password),
                    Answer::Username(username) => conn.login.username = Some(username),
                }
            }
        });
        self.connect();
    }

    fn connect(self) {
        let Some((login, attempt)) = CONN.with(|c| {
            c.borrow_mut().as_mut().map(|conn| {
                if conn.channel != 0 {
                    api::channel_close(conn.channel);
                }
                conn.channel = 0;
                conn.session = None;
                conn.attempt += 1;
                (conn.login.clone(), conn.attempt)
            })
        }) else {
            return;
        };
        self.phase.set(Phase::Connecting);
        let current = move || {
            CONN.with(|c| {
                c.borrow()
                    .as_ref()
                    .is_some_and(|conn| conn.attempt == attempt)
            })
        };
        // Bytes recebidos antes de o número do canal chegar, para confirmar depois.
        let channel = Rc::new(Cell::new(0u32));
        let owed = Rc::new(Cell::new(0u32));
        let mut line = Vec::new();
        let handler = {
            let (channel, owed) = (Rc::clone(&channel), Rc::clone(&owed));
            move |event| match event {
                ChannelEvent::Data(bytes) => {
                    let n = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                    match channel.get() {
                        0 => owed.set(owed.get().saturating_add(n)),
                        id => api::channel_ack(id, n),
                    }
                    line.extend_from_slice(&bytes);
                    while let Some(end) = line.iter().position(|&b| b == b'\n') {
                        let text: Vec<u8> = line.drain(..=end).collect();
                        if let Ok(event) = serde_json::from_slice::<TransferEvent>(&text[..end]) {
                            self.on_event(event);
                        }
                    }
                }
                ChannelEvent::Closed => {
                    if current() {
                        self.on_closed();
                    }
                }
            }
        };
        spawn_local(async move {
            let params = serde_json::to_value(&login).unwrap_or(Value::Null);
            let result = api::open_channel(METHOD_SFTP_OPEN, params, handler).await;
            if !current() {
                if let Ok((id, _)) = result {
                    api::channel_close(id);
                }
                return;
            }
            match result {
                Ok((id, value)) => {
                    channel.set(id);
                    if owed.get() > 0 {
                        api::channel_ack(id, owed.replace(0));
                    }
                    let Ok(opened) = serde_json::from_value::<Opened>(value) else {
                        api::channel_close(id);
                        self.phase
                            .set(Phase::Failed("Resposta inesperada do núcleo".into()));
                        return;
                    };
                    CONN.with(|c| {
                        if let Some(conn) = c.borrow_mut().as_mut() {
                            conn.channel = id;
                            conn.session = Some(opened.session);
                            // Senha digitada e fingerprint aceita valem só para esta tentativa.
                            conn.login.password = None;
                            conn.login.trust_host_key = None;
                        }
                    });
                    self.phase.set(Phase::Open);
                    // Numa reconexão, volta para a mesma pasta.
                    let path = self.remote.path.get_untracked();
                    self.list(
                        Side::Remote,
                        if path.is_empty() { &opened.home } else { &path },
                    );
                }
                Err(error) => {
                    self.phase.set(match terminal::failure(&error) {
                        terminal::Phase::Ask(ask) => Phase::Ask(ask),
                        terminal::Phase::Failed(message) => Phase::Failed(message),
                        _ => Phase::Failed(error.message),
                    });
                }
            }
        });
    }

    fn on_closed(self) {
        CONN.with(|c| {
            if let Some(conn) = c.borrow_mut().as_mut() {
                conn.channel = 0;
                conn.session = None;
            }
        });
        self.phase.set(Phase::Ended);
        self.transfers.update(|rows| {
            for row in rows.iter_mut().filter(|r| {
                matches!(
                    r.state,
                    TransferState::Queued | TransferState::Running | TransferState::Paused
                )
            }) {
                row.state = TransferState::Failed;
                row.error = Some("A conexão caiu".into());
            }
        });
    }

    fn on_event(self, event: TransferEvent) {
        let done = event.state == TransferState::Done;
        let mut direction = None;
        self.upsert(&event.transfer, |row| {
            row.state = event.state;
            row.done = event.done;
            row.total = event.total;
            row.error = event.error;
            direction = Some(row.direction);
        });
        // O arquivo novo aparece no painel de destino.
        if done {
            match direction {
                Some(Direction::Upload) => self.refresh(Side::Remote),
                Some(Direction::Download) => self.refresh(Side::Local),
                None => {}
            }
        }
    }

    /// Atualiza a linha da transferência, criando se o evento chegou antes da resposta.
    fn upsert(self, id: &str, change: impl FnOnce(&mut Row)) {
        self.transfers.update(|rows| {
            let index = rows.iter().position(|r| r.id == id).unwrap_or_else(|| {
                rows.push(Row {
                    id: id.to_owned(),
                    direction: Direction::Upload,
                    name: String::new(),
                    local: String::new(),
                    remote: String::new(),
                    state: TransferState::Queued,
                    done: 0,
                    total: 0,
                    error: None,
                });
                rows.len() - 1
            });
            change(&mut rows[index]);
        });
    }

    fn session() -> Option<String> {
        CONN.with(|c| c.borrow().as_ref().and_then(|conn| conn.session.clone()))
    }

    /// Lista uma pasta. Caminho vazio, deste PC, é a pasta do usuário.
    pub fn list(self, side: Side, path: &str) {
        let pane = self.pane(side);
        let (method, params) = match side {
            Side::Local => (METHOD_FILES_LIST, json!({ "path": path })),
            Side::Remote => {
                let Some(session) = Self::session() else {
                    return;
                };
                (
                    METHOD_SFTP_LIST,
                    json!({ "session": session, "path": path }),
                )
            }
        };
        pane.seq.update_value(|n| *n += 1);
        let seq = pane.seq.get_value();
        pane.listing.set(None);
        spawn_local(async move {
            let result = ui::call(method, &params)
                .await
                .and_then(|v| serde_json::from_value::<Listing>(v).map_err(|e| e.to_string()));
            if pane.seq.try_get_value() != Some(seq) {
                return;
            }
            match result {
                Ok(listing) => {
                    pane.path.set(listing.path);
                    pane.selected.set(HashSet::new());
                    pane.listing.set(Some(Ok(listing.entries)));
                }
                Err(message) => pane.listing.set(Some(Err(message))),
            }
        });
    }

    pub fn refresh(self, side: Side) {
        let path = self.pane(side).path.get_untracked();
        if side == Side::Local || !path.is_empty() {
            self.list(side, &path);
        }
    }

    /// Caminho completo de um item da pasta à vista.
    pub fn child(self, side: Side, name: &str) -> String {
        join(side, &self.pane(side).path.get_untracked(), name)
    }

    pub fn up(self, side: Side) {
        let path = self.pane(side).path.get_untracked();
        let parent = parent(side, &path);
        if parent != path {
            self.list(side, &parent);
        }
    }

    pub async fn mkdir(self, side: Side, name: &str) -> Result<(), String> {
        let path = self.child(side, name);
        self.run(
            side,
            METHOD_FILES_MKDIR,
            METHOD_SFTP_MKDIR,
            json!({ "path": path }),
        )
        .await
    }

    pub async fn rename(self, side: Side, from: &str, to: &str) -> Result<(), String> {
        let params = json!({ "from": self.child(side, from), "to": self.child(side, to) });
        self.run(side, METHOD_FILES_RENAME, METHOD_SFTP_RENAME, params)
            .await
    }

    /// Apaga os itens; pastas vão com tudo dentro.
    pub async fn remove(self, side: Side, names: &[String]) -> Result<(), String> {
        for name in names {
            let params = json!({ "path": self.child(side, name), "recursive": true });
            if let Err(message) = self
                .run(side, METHOD_FILES_REMOVE, METHOD_SFTP_REMOVE, params)
                .await
            {
                self.refresh(side);
                return Err(message);
            }
        }
        Ok(())
    }

    pub async fn chmod(self, name: &str, mode: u32) -> Result<(), String> {
        let params = json!({ "path": self.child(Side::Remote, name), "mode": mode });
        self.run(Side::Remote, "", METHOD_SFTP_CHMOD, params).await
    }

    /// Chama o método do lado e atualiza a pasta. No servidor, junta a sessão aos parâmetros.
    async fn run(
        self,
        side: Side,
        local: &str,
        remote: &str,
        mut params: Value,
    ) -> Result<(), String> {
        let method = match side {
            Side::Local => local,
            Side::Remote => {
                let session =
                    Self::session().ok_or_else(|| "A sessão SFTP foi encerrada".to_owned())?;
                params["session"] = json!(session);
                remote
            }
        };
        ui::call(method, &params).await?;
        self.refresh(side);
        Ok(())
    }

    /// Manda os itens de um painel para a pasta à vista no outro.
    pub fn send(self, from: Side, names: Vec<String>) {
        let Some(session) = Self::session() else {
            return;
        };
        let mut folders = 0;
        for name in names {
            match self.pane(from).entry(&name) {
                Some(entry) if entry.kind == codatos_protocol::sftp::EntryKind::Dir => {
                    folders += 1;
                }
                Some(_) => {
                    let (direction, local, remote) = match from {
                        Side::Local => (
                            Direction::Upload,
                            self.child(Side::Local, &name),
                            self.child(Side::Remote, &name),
                        ),
                        Side::Remote => (
                            Direction::Download,
                            self.child(Side::Local, &name),
                            self.child(Side::Remote, &name),
                        ),
                    };
                    self.start(
                        name,
                        StartTransfer {
                            session: session.clone(),
                            direction,
                            local,
                            remote,
                            resume: false,
                            overwrite: false,
                        },
                    );
                }
                None => {}
            }
        }
        if folders > 0 {
            self.notice.set(Some(
                "Pastas inteiras ainda não são transferidas: abra a pasta e selecione os arquivos."
                    .into(),
            ));
        }
    }

    /// Põe na fila. Destino existente vira um [`Conflict`] para o usuário decidir.
    pub fn start(self, name: String, request: StartTransfer) {
        spawn_local(async move {
            match ui::call_coded(METHOD_SFTP_TRANSFER, &request).await {
                Ok(value) => {
                    let Ok(started) = serde_json::from_value::<TransferStarted>(value) else {
                        return;
                    };
                    self.upsert(&started.transfer, |row| {
                        row.direction = request.direction;
                        row.name = name;
                        row.local = request.local;
                        row.remote = request.remote;
                    });
                }
                Err((code, _)) if code == errors::EXISTS => {
                    self.conflicts
                        .update(|c| c.push(Conflict { name, request }));
                }
                Err((_, message)) => self.notice.set(Some(format!("{name}: {message}"))),
            }
        });
    }

    pub fn control(self, transfer: String, action: TransferAction) {
        let Some(session) = Self::session() else {
            return;
        };
        spawn_local(async move {
            let request = ControlTransfer {
                session,
                transfer,
                action,
            };
            if let Err(message) = ui::call(METHOD_SFTP_TRANSFER_CONTROL, &request).await {
                self.notice.set(Some(message));
            }
        });
    }

    /// Recomeça uma transferência que falhou, do ponto em que parou.
    pub fn retry(self, id: &str) {
        let Some(session) = Self::session() else {
            return;
        };
        let Some(row) = self.transfers.with_untracked(|rows| {
            rows.iter()
                .find(|r| r.id == id && r.state == TransferState::Failed)
                .cloned()
        }) else {
            return;
        };
        self.transfers.update(|rows| rows.retain(|r| r.id != id));
        self.start(
            row.name,
            StartTransfer {
                session,
                direction: row.direction,
                local: row.local,
                remote: row.remote,
                resume: true,
                overwrite: false,
            },
        );
    }

    pub fn clear_finished(self) {
        self.transfers.update(|rows| rows.retain(|r| !r.finished()));
    }
}

fn join(side: Side, dir: &str, name: &str) -> String {
    let sep = side.separator(dir);
    if dir.ends_with(sep) {
        format!("{dir}{name}")
    } else {
        format!("{dir}{sep}{name}")
    }
}

/// Pasta de cima; a raiz (`/`, `C:\`) fica como está.
fn parent(side: Side, path: &str) -> String {
    let sep = side.separator(path);
    let trimmed = path.trim_end_matches(sep);
    match trimmed.rsplit_once(sep) {
        Some((head, _)) if head.is_empty() || head.ends_with(':') => format!("{head}{sep}"),
        Some((head, _)) => head.to_owned(),
        None => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminhos() {
        assert_eq!(join(Side::Remote, "/", "a"), "/a");
        assert_eq!(join(Side::Remote, "/home/x", "a"), "/home/x/a");
        assert_eq!(join(Side::Local, r"C:\", "a"), r"C:\a");
        assert_eq!(join(Side::Local, r"C:\Users", "a"), r"C:\Users\a");
        assert_eq!(join(Side::Local, "/home/x", "a"), "/home/x/a");
        assert_eq!(parent(Side::Remote, "/home/x"), "/home");
        assert_eq!(parent(Side::Remote, "/home"), "/");
        assert_eq!(parent(Side::Remote, "/"), "/");
        assert_eq!(parent(Side::Local, r"C:\Users\andre"), r"C:\Users");
        assert_eq!(parent(Side::Local, r"C:\Users"), r"C:\");
        assert_eq!(parent(Side::Local, r"C:\"), r"C:\");
    }
}
