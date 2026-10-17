//! Conexões do CodatOS (`docs/modulos/conexoes.md`): chaves SSH por enquanto; sessões SSH,
//! SFTP, túneis e terminais locais chegam nas próximas PRs do M2.
//!
//! Segredos (chaves privadas, senhas) só são decifrados aqui, no núcleo, e nunca vão para
//! a UI (RNF-SEG-09).

pub mod keys;
mod keys_rpc;

pub use keys_rpc::key_methods;
