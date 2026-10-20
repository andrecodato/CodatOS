//! SFTP (`russh-sftp`) com um host cadastrado: `sftp.open` conecta como o terminal SSH
//! (jump hosts, proxy, known hosts) e abre o subsistema `sftp`. A sessão fica num registro
//! enquanto o canal binário que a abriu estiver aberto; os outros métodos a acham pelo id.
//!
//! Transferências entram numa fila por sessão e rodam uma por vez, gravando num arquivo
//! parcial (`…codatos-part`) que só ganha o nome final no fim; com `resume`, continuam do
//! tamanho dele. O andamento sai pelo canal, um [`TransferEvent`] em JSON por linha.
//! O conteúdo dos arquivos nunca vai para log (R-06).

use std::collections::HashMap;
use std::fmt;
use std::io::SeekFrom;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use codatos_core::{
    Channel, ChannelMethod, ChannelSender, Method, MethodError, MethodFuture, params,
};
use codatos_protocol::sftp::{
    Chmod, ControlTransfer, Direction, Entry, EntryKind, Listing, METHOD_SFTP_CHMOD,
    METHOD_SFTP_LIST, METHOD_SFTP_MKDIR, METHOD_SFTP_OPEN, METHOD_SFTP_REMOVE, METHOD_SFTP_RENAME,
    METHOD_SFTP_TRANSFER, METHOD_SFTP_TRANSFER_CONTROL, OpenSftp, Opened, PARTIAL_SUFFIX, Remove,
    Rename, SessionPath, StartTransfer, TransferAction, TransferEvent, TransferStarted,
    TransferState, errors,
};
use codatos_store::{Store, Vault};
use russh_sftp::client::SftpSession;
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::protocol::{FileAttributes, FileType, OpenFlags, StatusCode};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{mpsc, watch};
use zeroize::Zeroizing;

use crate::ids::new_id;
use crate::ssh::{self, Connected, Login, Ssh};

/// Bytes lidos e gravados por vez numa transferência.
const CHUNK: usize = 256 * 1024;
/// Intervalo mínimo entre dois avisos de andamento da mesma transferência.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// De quanto em quanto tempo a sessão confere se a conexão SSH caiu.
const LIVENESS: Duration = Duration::from_secs(5);
/// Itens de uma pasta numa listagem e itens apagados por um `remove` recursivo.
const MAX_ITEMS: usize = 10_000;

/// Os métodos do SFTP, que compartilham o registro de sessões: `sftp.open` (canal) e os
/// demais.
#[must_use]
pub fn sftp_methods(
    store: &Store,
    vault: &Vault,
) -> (Arc<dyn ChannelMethod>, Vec<Arc<dyn Method>>) {
    let sessions = Sessions::default();
    let open = Arc::new(OpenSftpMethod {
        ssh: Ssh {
            store: store.clone(),
            vault: vault.clone(),
        },
        sessions: sessions.clone(),
    });
    let methods = [
        METHOD_SFTP_LIST,
        METHOD_SFTP_MKDIR,
        METHOD_SFTP_RENAME,
        METHOD_SFTP_REMOVE,
        METHOD_SFTP_CHMOD,
        METHOD_SFTP_TRANSFER,
        METHOD_SFTP_TRANSFER_CONTROL,
    ]
    .into_iter()
    .map(|name| {
        Arc::new(SftpMethod {
            name,
            sessions: sessions.clone(),
        }) as Arc<dyn Method>
    })
    .collect();
    (open, methods)
}

/// Sessões abertas, pelo id devolvido no `sftp.open`.
#[derive(Debug, Clone, Default)]
struct Sessions(Arc<Mutex<HashMap<String, Arc<Session>>>>);

impl Sessions {
    fn get(&self, id: &str) -> Result<Arc<Session>, MethodError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .cloned()
            .ok_or_else(|| {
                MethodError::new(errors::SESSION_NOT_FOUND, "A sessão SFTP foi encerrada")
            })
    }

    fn insert(&self, id: String, session: Arc<Session>) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, session);
    }

    fn remove(&self, id: &str) -> Option<Arc<Session>> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(id)
    }
}

struct Session {
    sftp: SftpSession,
    /// Controle de cada transferência na fila ou rodando.
    transfers: Mutex<HashMap<String, watch::Sender<Control>>>,
    queue: mpsc::UnboundedSender<Job>,
    events: ChannelSender,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl Session {
    fn control(&self, transfer: &str, control: Control) -> bool {
        let transfers = self
            .transfers
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        transfers.get(transfer).is_some_and(|tx| {
            tx.send_replace(control);
            true
        })
    }

    fn cancel_all(&self) {
        for tx in self
            .transfers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
        {
            tx.send_replace(Control::Cancel);
        }
    }

    async fn report(&self, event: &TransferEvent) {
        let mut line = serde_json::to_vec(event).unwrap_or_default();
        line.push(b'\n');
        let _ = self.events.send(&line).await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Run,
    Pause,
    Cancel,
}

struct Job {
    id: String,
    request: StartTransfer,
    control: watch::Receiver<Control>,
}

/// `sftp.open`.
#[derive(Debug)]
struct OpenSftpMethod {
    ssh: Ssh,
    sessions: Sessions,
}

impl ChannelMethod for OpenSftpMethod {
    fn name(&self) -> &'static str {
        METHOD_SFTP_OPEN
    }

    fn open(&self, value: Value, channel: Channel) -> MethodFuture<'_> {
        Box::pin(async move {
            let p: OpenSftp = params(value)?;
            let connected = self
                .ssh
                .connect_host(Login {
                    host_id: p.host_id,
                    username: p.username,
                    password: p.password.map(Zeroizing::new),
                    trust_host_key: p.trust_host_key,
                })
                .await?;
            let sftp = match start_subsystem(&connected).await {
                Ok(sftp) => sftp,
                Err(error) => {
                    connected.disconnect().await;
                    return Err(error);
                }
            };
            let home = sftp
                .canonicalize(".")
                .await
                .unwrap_or_else(|_| "/".to_owned());
            let id = new_id();
            let (queue, jobs) = mpsc::unbounded_channel();
            let session = Arc::new(Session {
                sftp,
                transfers: Mutex::new(HashMap::new()),
                queue,
                events: channel.sender.clone(),
            });
            self.sessions.insert(id.clone(), Arc::clone(&session));
            let (stop, stopped) = watch::channel(false);
            tokio::spawn(worker(Arc::clone(&session), jobs, stopped));
            let title = connected.host.host.name.clone();
            tokio::spawn(lifetime(
                self.sessions.clone(),
                id.clone(),
                connected,
                channel,
                stop,
            ));
            Ok(json!(Opened {
                session: id,
                home,
                title
            }))
        })
    }
}

async fn start_subsystem(connected: &Connected) -> Result<SftpSession, MethodError> {
    let channel = connected
        .session
        .channel_open_session()
        .await
        .map_err(ssh::connect_failed)?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(ssh::connect_failed)?;
    SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| failed(&e))
}

/// Mantém a sessão até a UI fechar o canal ou a conexão SSH cair.
async fn lifetime(
    sessions: Sessions,
    id: String,
    connected: Connected,
    mut channel: Channel,
    stop: watch::Sender<bool>,
) {
    let mut check = tokio::time::interval(LIVENESS);
    loop {
        tokio::select! {
            event = channel.incoming.recv() => {
                // A UI não manda nada por este canal; só o fim importa.
                if event.is_none() {
                    break;
                }
            }
            _ = check.tick() => {
                if connected.session.is_closed() {
                    break;
                }
            }
        }
    }
    if let Some(session) = sessions.remove(&id) {
        session.cancel_all();
        let _ = stop.send(true);
        let _ = session.sftp.close().await;
    }
    connected.disconnect().await;
    channel.sender.close().await;
}

/// Roda as transferências da fila, uma por vez.
async fn worker(
    session: Arc<Session>,
    mut jobs: mpsc::UnboundedReceiver<Job>,
    mut stopped: watch::Receiver<bool>,
) {
    loop {
        let job = tokio::select! {
            job = jobs.recv() => match job {
                Some(job) => job,
                None => return,
            },
            _ = stopped.changed() => return,
        };
        let mut progress = Progress {
            session: &session,
            transfer: job.id.clone(),
            done: 0,
            total: 0,
            last: None,
        };
        let mut control = job.control;
        let result = match job.request.direction {
            Direction::Download => {
                download(&session.sftp, &job.request, &mut control, &mut progress).await
            }
            Direction::Upload => {
                upload(&session.sftp, &job.request, &mut control, &mut progress).await
            }
        };
        let (state, error) = match result {
            Ok(()) => (TransferState::Done, None),
            Err(Stop::Canceled) => {
                remove_partial(&session.sftp, &job.request).await;
                (TransferState::Canceled, None)
            }
            Err(Stop::Failed(message)) => (TransferState::Failed, Some(message)),
        };
        progress.send(state, error).await;
        session
            .transfers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&job.id);
    }
}

/// Por que uma transferência parou antes do fim.
enum Stop {
    Canceled,
    Failed(String),
}

impl From<SftpError> for Stop {
    fn from(error: SftpError) -> Self {
        Self::Failed(failed(&error).to_string())
    }
}

impl From<std::io::Error> for Stop {
    fn from(error: std::io::Error) -> Self {
        Self::Failed(format!("Erro no disco deste PC: {error}"))
    }
}

struct Progress<'a> {
    session: &'a Session,
    transfer: String,
    done: u64,
    total: u64,
    last: Option<Instant>,
}

impl Progress<'_> {
    async fn send(&mut self, state: TransferState, error: Option<String>) {
        self.last = Some(Instant::now());
        self.session
            .report(&TransferEvent {
                transfer: self.transfer.clone(),
                state,
                done: self.done,
                total: self.total,
                error,
            })
            .await;
    }

    async fn advance(&mut self, bytes: usize) {
        self.done += bytes as u64;
        if self.last.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY) {
            self.send(TransferState::Running, None).await;
        }
    }

    /// Espera enquanto a transferência estiver pausada; para se ela for cancelada.
    async fn gate(&mut self, control: &mut watch::Receiver<Control>) -> Result<(), Stop> {
        let mut paused = false;
        loop {
            let state = *control.borrow_and_update();
            match state {
                Control::Run => {
                    if paused {
                        self.send(TransferState::Running, None).await;
                    }
                    return Ok(());
                }
                Control::Cancel => return Err(Stop::Canceled),
                Control::Pause => {
                    if !paused {
                        paused = true;
                        self.send(TransferState::Paused, None).await;
                    }
                    if control.changed().await.is_err() {
                        return Err(Stop::Canceled);
                    }
                }
            }
        }
    }
}

fn partial(path: &str) -> String {
    format!("{path}{PARTIAL_SUFFIX}")
}

async fn download(
    sftp: &SftpSession,
    request: &StartTransfer,
    control: &mut watch::Receiver<Control>,
    progress: &mut Progress<'_>,
) -> Result<(), Stop> {
    let total = sftp.metadata(request.remote.as_str()).await?.len();
    let part = partial(&request.local);
    let start = if request.resume {
        tokio::fs::metadata(&part)
            .await
            .map_or(0, |m| m.len())
            .min(total)
    } else {
        0
    };
    progress.total = total;
    progress.done = start;
    progress.send(TransferState::Running, None).await;

    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&part)
        .await?;
    output.set_len(start).await?;
    output.seek(SeekFrom::Start(start)).await?;
    let mut input = sftp.open(request.remote.as_str()).await?;
    input.seek(SeekFrom::Start(start)).await?;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        progress.gate(control).await?;
        let read = input.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).await?;
        progress.advance(read).await;
    }
    output.flush().await?;
    output.sync_all().await?;
    drop(output);
    // `rename` substitui o destino nos dois SOs; quem não quer isso foi barrado antes.
    tokio::fs::rename(&part, &request.local).await?;
    Ok(())
}

async fn upload(
    sftp: &SftpSession,
    request: &StartTransfer,
    control: &mut watch::Receiver<Control>,
    progress: &mut Progress<'_>,
) -> Result<(), Stop> {
    let total = tokio::fs::metadata(&request.local).await?.len();
    let part = partial(&request.remote);
    let start = if request.resume {
        sftp.metadata(part.as_str())
            .await
            .map_or(0, |m| m.len())
            .min(total)
    } else {
        0
    };
    progress.total = total;
    progress.done = start;
    progress.send(TransferState::Running, None).await;

    let mut flags = OpenFlags::WRITE | OpenFlags::CREATE;
    if start == 0 {
        flags |= OpenFlags::TRUNCATE;
    }
    let mut output = sftp.open_with_flags(part.as_str(), flags).await?;
    if start > 0 {
        // Parcial maior que o arquivo (era de outro com o mesmo nome): corta no tamanho.
        output
            .set_metadata(FileAttributes {
                size: Some(start),
                ..FileAttributes::empty()
            })
            .await?;
        output.seek(SeekFrom::Start(start)).await?;
    }
    let mut input = tokio::fs::File::open(&request.local).await?;
    input.seek(SeekFrom::Start(start)).await?;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        progress.gate(control).await?;
        let read = input.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).await?;
        progress.advance(read).await;
    }
    output.flush().await?;
    output.shutdown().await?;
    drop(output);
    // O `rename` do SFTP v3 falha se o destino existe.
    if request.overwrite && sftp.try_exists(request.remote.as_str()).await? {
        sftp.remove_file(request.remote.as_str()).await?;
    }
    sftp.rename(part, request.remote.as_str()).await?;
    Ok(())
}

/// Cancelada: o parcial não serve para retomar.
async fn remove_partial(sftp: &SftpSession, request: &StartTransfer) {
    match request.direction {
        Direction::Download => {
            let _ = tokio::fs::remove_file(partial(&request.local)).await;
        }
        Direction::Upload => {
            let _ = sftp.remove_file(partial(&request.remote)).await;
        }
    }
}

/// Os outros métodos, que agem numa sessão aberta.
#[derive(Debug)]
struct SftpMethod {
    name: &'static str,
    sessions: Sessions,
}

impl Method for SftpMethod {
    fn name(&self) -> &'static str {
        self.name
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            match self.name {
                METHOD_SFTP_LIST => {
                    let p: SessionPath = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    Ok(json!(list(&session.sftp, &p.path).await?))
                }
                METHOD_SFTP_MKDIR => {
                    let p: SessionPath = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    session
                        .sftp
                        .create_dir(p.path)
                        .await
                        .map_err(|e| failed(&e))?;
                    Ok(Value::Null)
                }
                METHOD_SFTP_RENAME => {
                    let p: Rename = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    if session
                        .sftp
                        .try_exists(p.to.as_str())
                        .await
                        .map_err(|e| failed(&e))?
                    {
                        return Err(MethodError::new(
                            errors::EXISTS,
                            "Já existe um item com esse nome",
                        ));
                    }
                    session
                        .sftp
                        .rename(p.from, p.to)
                        .await
                        .map_err(|e| failed(&e))?;
                    Ok(Value::Null)
                }
                METHOD_SFTP_REMOVE => {
                    let p: Remove = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    remove(&session.sftp, &p.path, p.recursive).await?;
                    Ok(Value::Null)
                }
                METHOD_SFTP_CHMOD => {
                    let p: Chmod = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    session
                        .sftp
                        .set_metadata(
                            p.path,
                            FileAttributes {
                                permissions: Some(p.mode & 0o7777),
                                ..FileAttributes::empty()
                            },
                        )
                        .await
                        .map_err(|e| failed(&e))?;
                    Ok(Value::Null)
                }
                METHOD_SFTP_TRANSFER => {
                    let p: StartTransfer = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    Ok(json!(enqueue(&session, p).await?))
                }
                METHOD_SFTP_TRANSFER_CONTROL => {
                    let p: ControlTransfer = params(value)?;
                    let session = self.sessions.get(&p.session)?;
                    let control = match p.action {
                        TransferAction::Pause => Control::Pause,
                        TransferAction::Resume => Control::Run,
                        TransferAction::Cancel => Control::Cancel,
                    };
                    if !session.control(&p.transfer, control) {
                        return Err(MethodError::new(
                            errors::TRANSFER_NOT_FOUND,
                            "A transferência já terminou",
                        ));
                    }
                    Ok(Value::Null)
                }
                _ => unreachable!("método SFTP registrado sem implementação"),
            }
        })
    }
}

async fn list(sftp: &SftpSession, path: &str) -> Result<Listing, MethodError> {
    let path = sftp.canonicalize(path).await.map_err(|e| failed(&e))?;
    let mut entries: Vec<Entry> = sftp
        .read_dir(path.as_str())
        .await
        .map_err(|e| failed(&e))?
        .filter(|e| !matches!(e.file_name().as_str(), "." | ".."))
        .take(MAX_ITEMS)
        .map(|e| {
            let meta = e.metadata();
            Entry {
                name: e.file_name(),
                kind: match e.file_type() {
                    FileType::Dir => EntryKind::Dir,
                    FileType::File => EntryKind::File,
                    FileType::Symlink => EntryKind::Symlink,
                    FileType::Other => EntryKind::Other,
                },
                size: meta.size.unwrap_or(0),
                modified: meta.mtime.map(i64::from),
                mode: meta.permissions.map(|m| m & 0o7777),
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        (a.kind != EntryKind::Dir)
            .cmp(&(b.kind != EntryKind::Dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(Listing { path, entries })
}

/// Apaga um arquivo, uma pasta vazia ou, com `recursive`, a pasta e tudo dentro (sem seguir
/// links).
async fn remove(sftp: &SftpSession, path: &str, recursive: bool) -> Result<(), MethodError> {
    let meta = sftp.symlink_metadata(path).await.map_err(|e| failed(&e))?;
    if !meta.is_dir() {
        return sftp.remove_file(path).await.map_err(|e| failed(&e));
    }
    if !recursive {
        return sftp.remove_dir(path).await.map_err(|e| match e {
            SftpError::Status(ref s) if s.status_code == StatusCode::Failure => {
                MethodError::new(errors::NOT_EMPTY, "A pasta não está vazia")
            }
            e => failed(&e),
        });
    }
    // Em profundidade, sem recursão: pastas entram na pilha e saem depois do conteúdo.
    let mut pending = vec![(path.to_owned(), false)];
    let mut removed = 0_usize;
    while let Some((dir, expanded)) = pending.pop() {
        if expanded {
            sftp.remove_dir(dir.as_str())
                .await
                .map_err(|e| failed(&e))?;
            continue;
        }
        pending.push((dir.clone(), true));
        for entry in sftp.read_dir(dir.as_str()).await.map_err(|e| failed(&e))? {
            let name = entry.file_name();
            if matches!(name.as_str(), "." | "..") {
                continue;
            }
            removed += 1;
            if removed > MAX_ITEMS {
                return Err(MethodError::new(
                    errors::FAILED,
                    format!("Pasta grande demais para apagar daqui (mais de {MAX_ITEMS} itens)"),
                ));
            }
            let child = format!("{}/{name}", dir.trim_end_matches('/'));
            if entry.file_type() == FileType::Dir {
                pending.push((child, false));
            } else {
                sftp.remove_file(child).await.map_err(|e| failed(&e))?;
            }
        }
    }
    Ok(())
}

/// Confere o pedido e põe a transferência na fila da sessão.
async fn enqueue(
    session: &Session,
    request: StartTransfer,
) -> Result<TransferStarted, MethodError> {
    let exists = match request.direction {
        Direction::Download => {
            let meta = session
                .sftp
                .metadata(request.remote.as_str())
                .await
                .map_err(|e| failed(&e))?;
            if meta.is_dir() {
                return Err(MethodError::invalid_params(
                    "Pastas ainda não são transferidas; escolha arquivos",
                ));
            }
            tokio::fs::try_exists(&request.local).await.unwrap_or(false)
        }
        Direction::Upload => {
            let meta = tokio::fs::metadata(&request.local)
                .await
                .map_err(local_failed)?;
            if !meta.is_file() {
                return Err(MethodError::invalid_params(
                    "Pastas ainda não são transferidas; escolha arquivos",
                ));
            }
            session
                .sftp
                .try_exists(request.remote.as_str())
                .await
                .map_err(|e| failed(&e))?
        }
    };
    if exists && !request.overwrite {
        return Err(MethodError::new(errors::EXISTS, "O destino já existe"));
    }
    let id = new_id();
    let (tx, control) = watch::channel(Control::Run);
    session
        .transfers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(id.clone(), tx);
    session
        .report(&TransferEvent {
            transfer: id.clone(),
            state: TransferState::Queued,
            done: 0,
            total: 0,
            error: None,
        })
        .await;
    session
        .queue
        .send(Job {
            id: id.clone(),
            request,
            control,
        })
        .map_err(|_| MethodError::new(errors::SESSION_NOT_FOUND, "A sessão SFTP foi encerrada"))?;
    Ok(TransferStarted { transfer: id })
}

/// Erro do servidor SFTP como resposta ao usuário.
fn failed(error: &SftpError) -> MethodError {
    match error {
        SftpError::Status(status) => match status.status_code {
            StatusCode::NoSuchFile => {
                MethodError::new(errors::NOT_FOUND, "Arquivo ou pasta não encontrado")
            }
            StatusCode::PermissionDenied => {
                MethodError::new(errors::PERMISSION_DENIED, "Sem permissão no servidor")
            }
            _ => MethodError::new(
                errors::FAILED,
                format!("O servidor recusou: {}", status.error_message),
            ),
        },
        other => MethodError::new(errors::FAILED, format!("Falha no SFTP: {other}")),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn local_failed(error: std::io::Error) -> MethodError {
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            MethodError::new(errors::NOT_FOUND, "Arquivo não encontrado neste PC")
        }
        std::io::ErrorKind::PermissionDenied => {
            MethodError::new(errors::PERMISSION_DENIED, "Sem permissão neste PC")
        }
        _ => MethodError::new(errors::FAILED, format!("Erro no disco deste PC: {error}")),
    }
}
