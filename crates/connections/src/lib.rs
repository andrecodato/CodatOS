//! Conexões do CodatOS (`docs/modulos/conexoes.md`): chaves SSH, `~/.ssh/config` e
//! terminais locais por enquanto; sessões SSH, SFTP e túneis chegam nas próximas PRs do M2.
//!
//! Segredos (chaves privadas, senhas) só são decifrados aqui, no núcleo, e nunca vão para
//! a UI (RNF-SEG-09).

mod ids;
pub mod keys;
mod keys_rpc;
pub mod ssh_config;
mod ssh_config_rpc;
mod terminal;

pub use keys_rpc::key_methods;
pub use ssh_config_rpc::ssh_config_methods;
pub use terminal::OpenTerminalMethod;
