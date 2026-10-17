//! Banco local do CodatOS: um arquivo SQLite por perfil (`docs/05-modelo-de-dados.md`).
//!
//! Guarda preferências, dashboards, conexões e o header do [`Vault`]. O arquivo não é
//! cifrado (Q-37): os segredos vão cifrados pelo cofre. [`methods`], [`vault_methods`] e
//! [`connection_methods`] expõem o banco à UI como métodos JSON-RPC.

mod connections;
mod connections_rpc;
mod dashboards;
mod keychain;
mod methods;
mod settings;
mod vault;
mod vault_rpc;

use std::path::Path;
use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

pub use connections::EffectiveHost;
pub use connections_rpc::connection_methods;
pub use keychain::{Keychain, KeychainError, os_keychain};
pub use methods::methods;
pub use vault::{Vault, VaultError};
pub use vault_rpc::{VaultCollector, vault_methods};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("não foi possível abrir o banco local")]
    Open(#[source] sqlx::Error),
    #[error("não foi possível atualizar o banco local")]
    Migrate(#[source] sqlx::migrate::MigrateError),
    #[error("erro no banco local")]
    Query(#[from] sqlx::Error),
    #[error("valor guardado inválido")]
    Json(#[from] serde_json::Error),
}

/// Banco aberto. Clonar é barato (o pool é compartilhado).
#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Abre (ou cria) o banco em `path` e aplica as migrações pendentes.
    pub async fn open(path: &Path) -> Result<Self, Error> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        Self::connect(options, 4).await
    }

    /// Banco em memória, para testes.
    pub async fn in_memory() -> Result<Self, Error> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(Error::Open)?
            .foreign_keys(true);
        // Uma conexão só: cada conexão em memória seria um banco diferente.
        Self::connect(options, 1).await
    }

    async fn connect(options: SqliteConnectOptions, max: u32) -> Result<Self, Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max)
            .connect_with(options)
            .await
            .map_err(Error::Open)?;
        MIGRATOR.run(&pool).await.map_err(Error::Migrate)?;
        Ok(Self { pool })
    }
}
