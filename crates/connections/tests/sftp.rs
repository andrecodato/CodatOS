//! `sftp.*` contra um servidor `russh` + `russh-sftp` no próprio teste, que serve uma pasta
//! temporária: listar, criar, renomear, apagar, chmod, enviar, baixar, retomar, pausar e
//! cancelar.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use codatos_connections::{keys, sftp_methods};
use codatos_core::{RunningServer, ServerConfig, start};
use codatos_protocol::channel::{Frame, decode, encode};
use codatos_protocol::connections::KeyAlgorithm;
use codatos_protocol::sftp::{PARTIAL_SUFFIX, TransferEvent, TransferState, errors};
use codatos_protocol::vault::VaultProtection;
use codatos_store::{Keychain, KeychainError, Store, Vault, connection_methods};
use futures_util::{SinkExt, StreamExt};
use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};
use serde_json::{Value, json};
use std::sync::Mutex;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use zeroize::Zeroizing;

// ---------- servidor SFTP de teste ----------

/// Aceita `andre`/`segredo` e abre o subsistema `sftp`.
#[derive(Default)]
struct SshServer {
    root: PathBuf,
    /// Recusa o subsistema, como um servidor com o SFTP desligado.
    no_sftp: bool,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

#[allow(clippy::unused_async_trait_impl)]
impl russh::server::Handler for SshServer {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == "andre" && password == "segredo" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        match (name, self.channels.remove(&id)) {
            ("sftp", Some(channel)) if !self.no_sftp => {
                session.channel_success(id)?;
                russh_sftp::server::run(
                    channel.into_stream(),
                    SftpServer {
                        root: self.root.clone(),
                        ..SftpServer::default()
                    },
                )
                .await;
            }
            _ => session.channel_failure(id)?,
        }
        Ok(())
    }
}

/// SFTP sobre `root`: o caminho `/a/b` do cliente é `root/a/b`.
#[derive(Default)]
struct SftpServer {
    root: PathBuf,
    files: HashMap<String, fs::File>,
    dirs: HashMap<String, Option<Vec<File>>>,
    next: u32,
}

impl SftpServer {
    fn local(&self, path: &str) -> PathBuf {
        let relative = path.trim_start_matches('/');
        if relative.is_empty() || relative == "." {
            self.root.clone()
        } else {
            self.root.join(relative)
        }
    }

    fn handle(&mut self) -> String {
        self.next += 1;
        format!("h{}", self.next)
    }
}

fn ok(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: "Ok".into(),
        language_tag: "en-US".into(),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn io_status(error: std::io::Error) -> StatusCode {
    match error.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        _ => StatusCode::Failure,
    }
}

#[allow(clippy::unused_async_trait_impl)]
impl russh_sftp::server::Handler for SftpServer {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(&mut self, _: u32, _: HashMap<String, String>) -> Result<Version, Self::Error> {
        Ok(Version::new())
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other),
            }
        }
        Ok(Name {
            id,
            files: vec![File::dummy(format!("/{}", parts.join("/")))],
        })
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        flags: OpenFlags,
        _: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let file = fs::OpenOptions::from(flags)
            .open(self.local(&filename))
            .map_err(io_status)?;
        let handle = self.handle();
        self.files.insert(handle.clone(), file);
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.files.remove(&handle);
        self.dirs.remove(&handle);
        Ok(ok(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        use std::io::{Read, Seek};
        let file = self.files.get_mut(&handle).ok_or(StatusCode::Failure)?;
        file.seek(std::io::SeekFrom::Start(offset))
            .map_err(io_status)?;
        let mut data = vec![0; len as usize];
        let read = file.read(&mut data).map_err(io_status)?;
        if read == 0 {
            return Err(StatusCode::Eof);
        }
        data.truncate(read);
        Ok(Data { id, data })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        use std::io::{Seek, Write};
        let file = self.files.get_mut(&handle).ok_or(StatusCode::Failure)?;
        file.seek(std::io::SeekFrom::Start(offset))
            .map_err(io_status)?;
        file.write_all(&data).map_err(io_status)?;
        Ok(ok(id))
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let meta = fs::metadata(self.local(&path)).map_err(io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&meta),
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        let meta = fs::symlink_metadata(self.local(&path)).map_err(io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&meta),
        })
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        let file = self.files.get(&handle).ok_or(StatusCode::Failure)?;
        let meta = file.metadata().map_err(io_status)?;
        Ok(Attrs {
            id,
            attrs: FileAttributes::from(&meta),
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        #[cfg(unix)]
        if let Some(mode) = attrs.permissions {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(self.local(&path), fs::Permissions::from_mode(mode))
                .map_err(io_status)?;
        }
        #[cfg(not(unix))]
        let _ = (path, attrs);
        Ok(ok(id))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        let file = self.files.get(&handle).ok_or(StatusCode::Failure)?;
        if let Some(size) = attrs.size {
            file.set_len(size).map_err(io_status)?;
        }
        Ok(ok(id))
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let mut files = Vec::new();
        for entry in fs::read_dir(self.local(&path)).map_err(io_status)? {
            let entry = entry.map_err(io_status)?;
            let meta = entry.metadata().map_err(io_status)?;
            files.push(File::new(
                entry.file_name().to_string_lossy(),
                FileAttributes::from(&meta),
            ));
        }
        let handle = self.handle();
        self.dirs.insert(handle.clone(), Some(files));
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        match self.dirs.get_mut(&handle).and_then(Option::take) {
            Some(files) => Ok(Name { id, files }),
            None => Err(StatusCode::Eof),
        }
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _: FileAttributes,
    ) -> Result<Status, Self::Error> {
        fs::create_dir(self.local(&path)).map_err(io_status)?;
        Ok(ok(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        fs::remove_dir(self.local(&path)).map_err(io_status)?;
        Ok(ok(id))
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        fs::remove_file(self.local(&filename)).map_err(io_status)?;
        Ok(ok(id))
    }

    async fn rename(&mut self, id: u32, from: String, to: String) -> Result<Status, Self::Error> {
        // Como o SFTP v3: não substitui um destino existente.
        if self.local(&to).exists() {
            return Err(StatusCode::Failure);
        }
        fs::rename(self.local(&from), self.local(&to)).map_err(io_status)?;
        Ok(ok(id))
    }
}

async fn ssh_server(root: PathBuf) -> u16 {
    ssh_server_with(root, false).await
}

async fn ssh_server_with(root: PathBuf, no_sftp: bool) -> u16 {
    let host_key = keys::generate(KeyAlgorithm::Ed25519, "servidor").unwrap();
    let config = Arc::new(russh::server::Config {
        keys: vec![host_key],
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..russh::server::Config::default()
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let config = Arc::clone(&config);
            let handler = SshServer {
                root: root.clone(),
                no_sftp,
                ..SshServer::default()
            };
            tokio::spawn(async move {
                if let Ok(session) = russh::server::run_stream(config, stream, handler).await {
                    let _ = session.await;
                }
            });
        }
    });
    port
}

// ---------- núcleo com banco e cofre ----------

#[derive(Debug, Default)]
struct MemoryKeychain(Mutex<Option<String>>);

impl Keychain for MemoryKeychain {
    fn available(&self) -> bool {
        true
    }
    fn get(&self) -> Result<Option<Zeroizing<String>>, KeychainError> {
        Ok(self.0.lock().unwrap().clone().map(Zeroizing::new))
    }
    fn set(&self, secret: &str) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = Some(secret.to_owned());
        Ok(())
    }
    fn delete(&self) -> Result<(), KeychainError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

async fn core() -> RunningServer {
    let store = Store::in_memory().await.unwrap();
    let vault = Vault::open(store.clone(), Arc::new(MemoryKeychain::default()))
        .await
        .unwrap();
    vault
        .create(VaultProtection::OsSession, None)
        .await
        .unwrap();
    let mut methods = connection_methods(&store, &vault);
    let (open, sftp) = sftp_methods(&store, &vault);
    methods.extend(sftp);
    start(ServerConfig {
        port: 0,
        methods,
        channel_methods: vec![open],
        ..ServerConfig::default()
    })
    .await
    .expect("servidor sobe")
}

/// Cliente do WebSocket que guarda os eventos de transferência que chegam entre respostas.
struct Client {
    socket: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    buffer: Vec<u8>,
    events: VecDeque<TransferEvent>,
    closed: bool,
    next_id: u64,
}

impl Client {
    async fn connect(server: &RunningServer) -> Self {
        let mut request = format!("ws://127.0.0.1:{}/rpc", server.local_addr().port())
            .into_client_request()
            .unwrap();
        let protocol = format!("codatos.v1, codatos.token.{}", server.app_token());
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str(&protocol).unwrap(),
        );
        Self {
            socket: tokio_tungstenite::connect_async(request).await.unwrap().0,
            buffer: Vec::new(),
            events: VecDeque::new(),
            closed: false,
            next_id: 0,
        }
    }

    /// Lê uma mensagem; frames binários viram eventos. Devolve o texto, se for texto.
    async fn pump(&mut self) -> Option<Value> {
        let message = tokio::time::timeout(Duration::from_secs(20), self.socket.next())
            .await
            .expect("mensagem em até 20 s")
            .unwrap()
            .unwrap();
        match message {
            Message::Text(text) => Some(serde_json::from_str(text.as_str()).unwrap()),
            Message::Binary(bytes) => {
                let (channel, frame) = decode(&bytes).unwrap();
                match frame {
                    Frame::Data(data) => {
                        self.buffer.extend_from_slice(data);
                        let ack = encode(channel, Frame::Ack(u32::try_from(data.len()).unwrap()));
                        self.socket.send(Message::Binary(ack.into())).await.unwrap();
                        while let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
                            let line: Vec<u8> = self.buffer.drain(..=end).collect();
                            self.events
                                .push_back(serde_json::from_slice(&line).unwrap());
                        }
                    }
                    Frame::Close => self.closed = true,
                    other => panic!("frame inesperado: {other:?}"),
                }
                None
            }
            _ => None,
        }
    }

    async fn call(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.socket
            .send(Message::Text(request.to_string().into()))
            .await
            .unwrap();
        loop {
            if let Some(value) = self.pump().await
                && value["id"] == id
            {
                return value;
            }
        }
    }

    async fn ok(&mut self, method: &str, params: Value) -> Value {
        let r = self.call(method, params).await;
        assert!(r.get("error").is_none(), "{method}: {r}");
        r["result"].clone()
    }

    /// Eventos da transferência até ela chegar a um estado final (ou a `until`).
    async fn follow(&mut self, transfer: &str, until: TransferState) -> Vec<TransferEvent> {
        let mut seen = Vec::new();
        loop {
            while let Some(event) = self.events.pop_front() {
                if event.transfer != transfer {
                    continue;
                }
                let state = event.state;
                seen.push(event);
                if state == until
                    || matches!(
                        state,
                        TransferState::Done | TransferState::Failed | TransferState::Canceled
                    )
                {
                    return seen;
                }
            }
            self.pump().await;
        }
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "codatos-sftp-{name}-{}-{}",
        std::process::id(),
        rand_suffix()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 1_000_000_000,
    )
    .unwrap()
}

/// Conteúdo que não se repete a cada bloco, para pegar pedaço fora do lugar.
fn content(len: usize) -> Vec<u8> {
    let mut state = 0x2545_f491_u32;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect()
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

async fn open_session(client: &mut Client, port: u16) -> (String, u32) {
    client
        .ok(
            "connections.saveIdentity",
            json!({"id": "i1", "name": "André", "username": "andre", "password": {"action": "set", "value": "segredo"}}),
        )
        .await;
    client
        .ok(
            "connections.saveHost",
            json!({"id": "h1", "name": "Servidor", "address": "127.0.0.1", "port": port, "identityId": "i1"}),
        )
        .await;
    let r = client.call("sftp.open", json!({"hostId": "h1"})).await;
    assert_eq!(r["error"]["data"]["code"], "ssh_host_key_unknown", "{r}");
    let fingerprint = r["error"]["data"]["detail"].clone();
    let opened = client
        .ok(
            "sftp.open",
            json!({"hostId": "h1", "trustHostKey": fingerprint}),
        )
        .await;
    assert_eq!(opened["home"], "/");
    assert_eq!(opened["title"], "Servidor");
    (
        opened["session"].as_str().unwrap().to_owned(),
        u32::try_from(opened["channel"].as_u64().unwrap()).unwrap(),
    )
}

#[tokio::test]
async fn pastas_e_arquivos_no_servidor() {
    let remote = temp_dir("remoto");
    let port = ssh_server(remote.clone()).await;
    let server = core().await;
    let mut client = Client::connect(&server).await;
    let (session, channel) = open_session(&mut client, port).await;

    client
        .ok("sftp.mkdir", json!({"session": session, "path": "/pasta"}))
        .await;
    fs::write(remote.join("pasta/b.txt"), b"bb").unwrap();
    fs::write(remote.join("Z.txt"), b"z").unwrap();
    let listing = client
        .ok(
            "sftp.list",
            json!({"session": session, "path": "/pasta/.."}),
        )
        .await;
    assert_eq!(listing["path"], "/");
    let names: Vec<&str> = listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["pasta", "Z.txt"], "pastas primeiro");
    assert_eq!(listing["entries"][0]["kind"], "dir");
    assert_eq!(listing["entries"][1]["kind"], "file");
    assert_eq!(listing["entries"][1]["size"], 1);

    // Renomear não passa por cima de outro item.
    let r = client
        .call(
            "sftp.rename",
            json!({"session": session, "from": "/Z.txt", "to": "/pasta/b.txt"}),
        )
        .await;
    assert_eq!(r["error"]["data"]["code"], errors::EXISTS, "{r}");
    client
        .ok(
            "sftp.rename",
            json!({"session": session, "from": "/Z.txt", "to": "/pasta/z.txt"}),
        )
        .await;
    assert!(remote.join("pasta/z.txt").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        client
            .ok(
                "sftp.chmod",
                json!({"session": session, "path": "/pasta/z.txt", "mode": 0o600}),
            )
            .await;
        let mode = fs::metadata(remote.join("pasta/z.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let r = client
        .call("sftp.remove", json!({"session": session, "path": "/pasta"}))
        .await;
    assert_eq!(r["error"]["data"]["code"], errors::NOT_EMPTY, "{r}");
    fs::create_dir(remote.join("pasta/sub")).unwrap();
    fs::write(remote.join("pasta/sub/c.txt"), b"c").unwrap();
    client
        .ok(
            "sftp.remove",
            json!({"session": session, "path": "/pasta", "recursive": true}),
        )
        .await;
    assert!(!remote.join("pasta").exists());

    let r = client
        .call("sftp.list", json!({"session": session, "path": "/sumiu"}))
        .await;
    assert_eq!(r["error"]["data"]["code"], errors::NOT_FOUND, "{r}");

    // Fechar o canal encerra a sessão.
    client
        .socket
        .send(Message::Binary(encode(channel, Frame::Close).into()))
        .await
        .unwrap();
    let mut gone = false;
    for _ in 0..50 {
        let r = client
            .call("sftp.list", json!({"session": session, "path": "/"}))
            .await;
        if r["error"]["data"]["code"] == errors::SESSION_NOT_FOUND {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(gone, "a sessão continuou depois de fechar o canal");
    let _ = fs::remove_dir_all(remote);
}

#[tokio::test]
async fn envia_baixa_e_retoma() {
    let remote = temp_dir("remoto");
    let local = temp_dir("local");
    let port = ssh_server(remote.clone()).await;
    let server = core().await;
    let mut client = Client::connect(&server).await;
    let (session, _) = open_session(&mut client, port).await;
    let data = content(1_500_000);

    // Envio.
    let source = local.join("a.bin");
    fs::write(&source, &data).unwrap();
    let started = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "upload", "local": path_str(&source), "remote": "/a.bin"}),
        )
        .await;
    let transfer = started["transfer"].as_str().unwrap().to_owned();
    let events = client.follow(&transfer, TransferState::Done).await;
    assert_eq!(events[0].state, TransferState::Queued);
    let last = events.last().unwrap();
    assert_eq!(last.state, TransferState::Done, "{events:?}");
    assert_eq!((last.done, last.total), (1_500_000, 1_500_000));
    assert_eq!(fs::read(remote.join("a.bin")).unwrap(), data);
    assert!(!remote.join(format!("a.bin{PARTIAL_SUFFIX}")).exists());

    // Download; o destino existente só é substituído com `overwrite`.
    let target = local.join("b.bin");
    fs::write(&target, b"velho").unwrap();
    let request = json!({"session": session, "direction": "download", "local": path_str(&target), "remote": "/a.bin"});
    let r = client.call("sftp.transfer", request.clone()).await;
    assert_eq!(r["error"]["data"]["code"], errors::EXISTS, "{r}");
    let mut overwrite = request.clone();
    overwrite["overwrite"] = json!(true);
    let started = client.ok("sftp.transfer", overwrite).await;
    let events = client
        .follow(started["transfer"].as_str().unwrap(), TransferState::Done)
        .await;
    assert_eq!(
        events.last().unwrap().state,
        TransferState::Done,
        "{events:?}"
    );
    assert_eq!(fs::read(&target).unwrap(), data);

    // Download que caiu no meio: continua do parcial.
    let target = local.join("c.bin");
    fs::write(
        local.join(format!("c.bin{PARTIAL_SUFFIX}")),
        &data[..400_000],
    )
    .unwrap();
    let started = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "download", "local": path_str(&target), "remote": "/a.bin", "resume": true}),
        )
        .await;
    let events = client
        .follow(started["transfer"].as_str().unwrap(), TransferState::Done)
        .await;
    let first_running = events
        .iter()
        .find(|e| e.state == TransferState::Running)
        .unwrap();
    assert_eq!(first_running.done, 400_000, "{events:?}");
    assert_eq!(events.last().unwrap().state, TransferState::Done);
    assert_eq!(fs::read(&target).unwrap(), data);

    // Envio que caiu no meio.
    fs::write(
        remote.join(format!("d.bin{PARTIAL_SUFFIX}")),
        &data[..300_000],
    )
    .unwrap();
    let started = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "upload", "local": path_str(&source), "remote": "/d.bin", "resume": true}),
        )
        .await;
    let events = client
        .follow(started["transfer"].as_str().unwrap(), TransferState::Done)
        .await;
    assert_eq!(
        events.last().unwrap().state,
        TransferState::Done,
        "{events:?}"
    );
    assert_eq!(fs::read(remote.join("d.bin")).unwrap(), data);

    let _ = fs::remove_dir_all(remote);
    let _ = fs::remove_dir_all(local);
}

#[tokio::test]
async fn pausa_retoma_e_cancela() {
    let remote = temp_dir("remoto");
    let local = temp_dir("local");
    let port = ssh_server(remote.clone()).await;
    let server = core().await;
    let mut client = Client::connect(&server).await;
    let (session, _) = open_session(&mut client, port).await;
    let data = content(3_000_000);
    let source = local.join("grande.bin");
    fs::write(&source, &data).unwrap();

    // A segunda espera a primeira na fila; pausada ali, para logo ao começar.
    let first = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "upload", "local": path_str(&source), "remote": "/um.bin"}),
        )
        .await["transfer"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "upload", "local": path_str(&source), "remote": "/dois.bin"}),
        )
        .await["transfer"]
        .as_str()
        .unwrap()
        .to_owned();
    client
        .ok(
            "sftp.transferControl",
            json!({"session": session, "transfer": second, "action": "pause"}),
        )
        .await;
    let events = client.follow(&first, TransferState::Done).await;
    assert_eq!(events.last().unwrap().state, TransferState::Done);
    let events = client.follow(&second, TransferState::Paused).await;
    assert_eq!(
        events.last().unwrap().state,
        TransferState::Paused,
        "{events:?}"
    );

    client
        .ok(
            "sftp.transferControl",
            json!({"session": session, "transfer": second, "action": "resume"}),
        )
        .await;
    let events = client.follow(&second, TransferState::Done).await;
    assert_eq!(
        events.last().unwrap().state,
        TransferState::Done,
        "{events:?}"
    );
    assert_eq!(fs::read(remote.join("dois.bin")).unwrap(), data);

    // Cancelada: some o parcial. Também espera na fila, atrás de outra.
    let blocker = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "upload", "local": path_str(&source), "remote": "/tres.bin"}),
        )
        .await["transfer"]
        .as_str()
        .unwrap()
        .to_owned();
    let third = client
        .ok(
            "sftp.transfer",
            json!({"session": session, "direction": "download", "local": path_str(&local.join("tres.bin")), "remote": "/dois.bin"}),
        )
        .await["transfer"]
        .as_str()
        .unwrap()
        .to_owned();
    client
        .ok(
            "sftp.transferControl",
            json!({"session": session, "transfer": third, "action": "pause"}),
        )
        .await;
    client.follow(&blocker, TransferState::Done).await;
    client.follow(&third, TransferState::Paused).await;
    assert!(local.join(format!("tres.bin{PARTIAL_SUFFIX}")).exists());
    client
        .ok(
            "sftp.transferControl",
            json!({"session": session, "transfer": third, "action": "cancel"}),
        )
        .await;
    let events = client.follow(&third, TransferState::Canceled).await;
    assert_eq!(events.last().unwrap().state, TransferState::Canceled);
    assert!(!local.join(format!("tres.bin{PARTIAL_SUFFIX}")).exists());
    assert!(!local.join("tres.bin").exists());
    let r = client
        .call(
            "sftp.transferControl",
            json!({"session": session, "transfer": third, "action": "resume"}),
        )
        .await;
    assert_eq!(r["error"]["data"]["code"], errors::TRANSFER_NOT_FOUND);

    let _ = fs::remove_dir_all(remote);
    let _ = fs::remove_dir_all(local);
}

#[tokio::test]
async fn servidor_sem_sftp_falha_sem_travar() {
    let port = ssh_server_with(temp_dir("sem-sftp"), true).await;
    let server = core().await;
    let mut client = Client::connect(&server).await;
    client
        .ok(
            "connections.saveIdentity",
            json!({"id": "i1", "name": "André", "username": "andre", "password": {"action": "set", "value": "segredo"}}),
        )
        .await;
    client
        .ok(
            "connections.saveHost",
            json!({"id": "h1", "name": "Servidor", "address": "127.0.0.1", "port": port, "identityId": "i1"}),
        )
        .await;
    let r = client.call("sftp.open", json!({"hostId": "h1"})).await;
    let fingerprint = r["error"]["data"]["detail"].clone();
    let r = tokio::time::timeout(
        Duration::from_secs(5),
        client.call(
            "sftp.open",
            json!({"hostId": "h1", "trustHostKey": fingerprint}),
        ),
    )
    .await
    .expect("sftp.open travou com o subsistema recusado");
    assert_eq!(r["error"]["data"]["code"], errors::FAILED, "{r}");
}
