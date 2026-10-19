//! Chave simétrica de 256 bits, apagada da memória ao sair de escopo.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Chave de 32 bytes. Não implementa `Clone` nem mostra o conteúdo no `Debug`.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Key([u8; 32]);

impl Key {
    pub const LEN: usize = 32;

    /// Chave nova do gerador do SO.
    pub fn generate() -> Self {
        Self(crate::random())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Base64 para guardar no keychain do SO (modo sessão).
    pub fn to_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(STANDARD.encode(self.0))
    }

    /// Lê de volta o que [`Key::to_base64`] gerou.
    pub fn from_base64(text: &str) -> Option<Self> {
        let bytes = Zeroizing::new(STANDARD.decode(text.trim()).ok()?);
        let array: [u8; 32] = bytes.as_slice().try_into().ok()?;
        Some(Self(array))
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(…)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_nao_mostra_a_chave() {
        let key = Key::from_bytes([7; 32]);
        assert_eq!(format!("{key:?}"), "Key(…)");
    }

    #[test]
    fn base64_ida_e_volta() {
        let key = Key::generate();
        let back = Key::from_base64(&key.to_base64()).unwrap();
        assert_eq!(key.as_bytes(), back.as_bytes());
        assert!(Key::from_base64("curta").is_none());
    }

    #[test]
    fn chaves_novas_sao_diferentes() {
        assert_ne!(Key::generate().as_bytes(), Key::generate().as_bytes());
    }
}
