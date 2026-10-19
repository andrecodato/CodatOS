//! Cofre local do CodatOS (`docs/04-seguranca.md`): de onde vêm as chaves e como os segredos
//! são cifrados.
//!
//! - A **chave do cofre** ([`Key`]) é aleatória e cifra os itens (senhas, chaves SSH,
//!   passphrases) em [`Envelope`]s XChaCha20-Poly1305 no formato v1 do §4.
//! - O [`Header`] guarda a chave do cofre embrulhada: pela senha do cofre (Argon2id → HKDF
//!   "wrap") ou, no modo sessão do SO, por nada: a chave fica no keychain e o núcleo a lê de lá.
//! - Nada aqui toca disco, keychain ou rede; quem guarda o header e os envelopes é o núcleo.
//!
//! Só primitivas do projeto `RustCrypto`, sem criptografia caseira (§1.3).

mod envelope;
mod header;
mod kdf;
mod key;

pub use envelope::{Aad, Envelope, MAX_ITEM};
pub use header::{Header, Protection};
pub use kdf::KdfParams;
pub use key::Key;

/// Falhas do cofre. Nenhuma carrega o segredo nem o texto cifrado.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Senha errada ou header adulterado (a tag não confere).
    #[error("senha do cofre incorreta")]
    WrongPassword,
    /// Envelope adulterado, movido para outro item/cofre ou cifrado com outra chave.
    #[error("não foi possível decifrar o item")]
    Decrypt,
    /// Formato, versão ou algoritmo que esta versão não entende.
    #[error("formato do cofre não suportado: {0}")]
    Unsupported(String),
    /// Item acima de [`MAX_ITEM`] (R-01).
    #[error("item grande demais para o cofre ({0} bytes)")]
    TooLarge(usize),
    /// Parâmetros do Argon2id fora do aceito pela biblioteca.
    #[error("parâmetros de derivação inválidos")]
    InvalidKdf,
    /// O header não é protegido por senha (modo sessão do SO).
    #[error("o cofre não usa senha")]
    NoPassword,
}

/// `n` bytes aleatórios do SO.
fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("gerador aleatório do SO indisponível");
    bytes
}
