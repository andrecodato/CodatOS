//! Arquivos deste PC, para o painel local do SFTP (`docs/modulos/conexoes.md#sftp`). As
//! respostas usam os mesmos tipos do SFTP ([`crate::sftp::Listing`]) e os mesmos códigos
//! de erro ([`crate::sftp::errors`]).

use serde::{Deserialize, Serialize};

/// Lista uma pasta ([`LocalPath`]; caminho vazio é a pasta do usuário); devolve
/// [`crate::sftp::Listing`].
pub const METHOD_FILES_LIST: &str = "files.list";
/// Cria uma pasta ([`LocalPath`]).
pub const METHOD_FILES_MKDIR: &str = "files.mkdir";
/// Renomeia ou move ([`LocalRename`]); não passa por cima de um item existente.
pub const METHOD_FILES_RENAME: &str = "files.rename";
/// Apaga um arquivo ou uma pasta ([`LocalRemove`]).
pub const METHOD_FILES_REMOVE: &str = "files.remove";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPath {
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRename {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRemove {
    pub path: String,
    /// Apaga a pasta com tudo dentro; sem isso, só pasta vazia.
    #[serde(default)]
    pub recursive: bool,
}
