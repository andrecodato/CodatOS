//! Senha do cofre → master key (Argon2id) → chave de embrulho (HKDF-SHA256 "wrap"), §3.

use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::{Error, Key};

/// Parâmetros do Argon2id, guardados no header junto com o sal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfParams {
    /// Sempre `argon2id`.
    pub alg: String,
    /// Memória em KiB.
    pub m_kib: u32,
    /// Iterações.
    pub t: u32,
    /// Paralelismo.
    pub p: u32,
    /// Sal de 16 bytes, em base64.
    #[serde(with = "crate::envelope::b64")]
    pub salt: Vec<u8>,
}

impl KdfParams {
    /// Mínimo do §3: m = 64 MiB, t = 3, p = 4.
    pub const MIN_M_KIB: u32 = 64 * 1024;
    pub const MIN_T: u32 = 3;
    pub const MIN_P: u32 = 4;

    /// Parâmetros mínimos do §3 com sal novo.
    pub fn generate() -> Self {
        Self::with_cost(Self::MIN_M_KIB, Self::MIN_T, Self::MIN_P)
    }

    /// Custo explícito, com sal novo. Abaixo do mínimo só em testes.
    pub fn with_cost(m_kib: u32, t: u32, p: u32) -> Self {
        Self {
            alg: "argon2id".to_owned(),
            m_kib,
            t,
            p,
            salt: crate::random::<16>().to_vec(),
        }
    }

    /// `true` se atende ao mínimo do §3 (o núcleo pode pedir para refazer o header).
    pub fn meets_minimum(&self) -> bool {
        self.m_kib >= Self::MIN_M_KIB && self.t >= Self::MIN_T && self.p >= Self::MIN_P
    }

    /// Chave de embrulho derivada da senha. Lenta de propósito (≈ 0,5 s com o mínimo).
    pub(crate) fn wrap_key(&self, password: &[u8]) -> Result<Key, Error> {
        if self.alg != "argon2id" {
            return Err(Error::Unsupported(format!("kdf {}", self.alg)));
        }
        let params = Params::new(self.m_kib, self.t, self.p, Some(Key::LEN))
            .map_err(|_| Error::InvalidKdf)?;
        let mut master = Zeroizing::new([0u8; Key::LEN]);
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(password, &self.salt, master.as_mut())
            .map_err(|_| Error::InvalidKdf)?;
        let mut wrap = [0u8; Key::LEN];
        Hkdf::<Sha256>::new(None, master.as_ref())
            .expand(b"wrap", &mut wrap)
            .expect("32 bytes cabem no HKDF-SHA256");
        Ok(Key::from_bytes(wrap))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn cheap() -> KdfParams {
        KdfParams::with_cost(8, 1, 1)
    }

    #[test]
    fn mesma_senha_mesmo_sal_mesma_chave() {
        let params = cheap();
        let a = params.wrap_key(b"senha").unwrap();
        let b = params.wrap_key(b"senha").unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
        assert_ne!(a.as_bytes(), params.wrap_key(b"outra").unwrap().as_bytes());
    }

    #[test]
    fn sal_diferente_muda_a_chave() {
        let a = cheap().wrap_key(b"senha").unwrap();
        let b = cheap().wrap_key(b"senha").unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    /// Vetor fixo: trava o formato (Argon2id v19 + HKDF-SHA256 "wrap").
    #[test]
    fn vetor_fixo() {
        let params = KdfParams {
            alg: "argon2id".to_owned(),
            m_kib: 8,
            t: 1,
            p: 1,
            salt: b"codatos-salt-16b".to_vec(),
        };
        let key = params.wrap_key(b"correct horse").unwrap();
        assert_eq!(hex(key.as_bytes()), VECTOR_WRAP_KEY);
    }

    /// Conferido com uma implementação independente (argon2-cffi + HMAC-SHA256 em Python) ao
    /// criar o formato v1; não pode mudar.
    const VECTOR_WRAP_KEY: &str =
        "7e2370f321858c65ed40f17c7963529a9aa585a07bcf444914906a1b83321e81";

    pub(crate) fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    #[test]
    fn minimo_do_padrao() {
        assert!(KdfParams::generate().meets_minimum());
        assert!(!cheap().meets_minimum());
    }

    #[test]
    fn algoritmo_desconhecido() {
        let mut params = cheap();
        params.alg = "scrypt".to_owned();
        assert!(matches!(params.wrap_key(b"x"), Err(Error::Unsupported(_))));
    }
}
