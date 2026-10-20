//! SFTP (`docs/modulos/conexoes.md#sftp`): `sftp.open` abre uma sessão com um host
//! cadastrado num canal binário. A sessão vive enquanto o canal estiver aberto; os outros
//! métodos a citam por `session`. Pelo canal, o núcleo manda o andamento das
//! transferências, um [`TransferEvent`] em JSON por linha.

use serde::{Deserialize, Serialize};

/// Abre a sessão ([`OpenSftp`]); devolve [`Opened`] com `"channel": n`.
pub const METHOD_SFTP_OPEN: &str = "sftp.open";
/// Lista uma pasta ([`SessionPath`]); devolve [`Listing`].
pub const METHOD_SFTP_LIST: &str = "sftp.list";
/// Cria uma pasta ([`SessionPath`]).
pub const METHOD_SFTP_MKDIR: &str = "sftp.mkdir";
/// Renomeia ou move ([`Rename`]).
pub const METHOD_SFTP_RENAME: &str = "sftp.rename";
/// Apaga um arquivo ou uma pasta ([`Remove`]).
pub const METHOD_SFTP_REMOVE: &str = "sftp.remove";
/// Troca as permissões ([`Chmod`]).
pub const METHOD_SFTP_CHMOD: &str = "sftp.chmod";
/// Põe uma transferência na fila ([`StartTransfer`]); devolve [`TransferStarted`].
pub const METHOD_SFTP_TRANSFER: &str = "sftp.transfer";
/// Pausa, retoma ou cancela uma transferência ([`ControlTransfer`]).
pub const METHOD_SFTP_TRANSFER_CONTROL: &str = "sftp.transferControl";

pub mod errors {
    /// A sessão não existe mais (o canal fechou ou a conexão caiu).
    pub const SESSION_NOT_FOUND: &str = "sftp_session_not_found";
    pub const NOT_FOUND: &str = "sftp_not_found";
    pub const PERMISSION_DENIED: &str = "sftp_permission_denied";
    /// O destino já existe e o pedido não mandou sobrescrever.
    pub const EXISTS: &str = "sftp_exists";
    /// Pasta com conteúdo sem `recursive`.
    pub const NOT_EMPTY: &str = "sftp_not_empty";
    pub const TRANSFER_NOT_FOUND: &str = "sftp_transfer_not_found";
    /// Outra falha do servidor SFTP ou do disco local.
    pub const FAILED: &str = "sftp_failed";
}

/// Parâmetros de `sftp.open`: os mesmos da sessão SSH do terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSftp {
    pub host_id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub trust_host_key: Option<String>,
}

/// Resposta de `sftp.open` (o núcleo acrescenta `channel`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    pub session: String,
    /// Pasta inicial do usuário no servidor, já canônica.
    pub home: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPath {
    pub session: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rename {
    pub session: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Remove {
    pub session: String,
    pub path: String,
    /// Apaga a pasta com tudo dentro; sem isso, só pasta vazia.
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chmod {
    pub session: String,
    pub path: String,
    /// Bits de permissão (`0o755`); só os 12 de baixo valem.
    pub mode: u32,
}

/// Resposta de `sftp.list`: pastas primeiro, depois por nome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    /// O caminho pedido, canônico (sem `..`, links resolvidos).
    pub path: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    /// Segundos desde 1970 (UTC).
    pub modified: Option<i64>,
    /// Bits de permissão (`0o644`), quando o servidor informa.
    pub mode: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    /// Deste PC para o servidor.
    Upload,
    /// Do servidor para este PC.
    Download,
}

/// Parâmetros de `sftp.transfer`. Um arquivo por transferência.
///
/// O destino é gravado ao lado, com o sufixo [`PARTIAL_SUFFIX`], e só ganha o nome final
/// no fim. Com `resume`, continua do tamanho desse arquivo parcial (a transferência que
/// caiu no meio); sem ele, começa do zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTransfer {
    pub session: String,
    pub direction: Direction,
    /// Caminho neste PC.
    pub local: String,
    /// Caminho no servidor.
    pub remote: String,
    #[serde(default)]
    pub resume: bool,
    /// Substitui o destino se ele já existir.
    #[serde(default)]
    pub overwrite: bool,
}

/// Sufixo do arquivo parcial de uma transferência em andamento.
pub const PARTIAL_SUFFIX: &str = ".codatos-part";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferStarted {
    pub transfer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlTransfer {
    pub session: String,
    pub transfer: String,
    pub action: TransferAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferAction {
    Pause,
    Resume,
    /// Para e apaga o arquivo parcial.
    Cancel,
}

/// Andamento de uma transferência, mandado pelo canal da sessão (uma linha de JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferEvent {
    pub transfer: String,
    pub state: TransferState,
    /// Bytes no destino até agora (contando o que já estava, numa retomada).
    pub done: u64,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    /// Esperando a transferência da frente terminar.
    Queued,
    Running,
    Paused,
    Done,
    /// Parou com erro; o parcial fica para uma retomada.
    Failed,
    Canceled,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formatos() {
        let p: StartTransfer = serde_json::from_value(json!({
            "session": "s1", "direction": "download", "local": "/tmp/a", "remote": "/a"
        }))
        .unwrap();
        assert!(!p.resume && !p.overwrite);
        assert_eq!(p.direction, Direction::Download);
        let event = TransferEvent {
            transfer: "t1".into(),
            state: TransferState::Running,
            done: 10,
            total: 20,
            error: None,
        };
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            json!({"transfer": "t1", "state": "running", "done": 10, "total": 20})
        );
        let o: OpenSftp = serde_json::from_value(json!({"hostId": "h"})).unwrap();
        assert_eq!(o.username, None);
    }
}
