//! Ids novos criados pelo núcleo (importação): `UUIDv7`, como os da UI
//! (`docs/05-modelo-de-dados.md`).

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

/// `UUIDv7`: 48 bits de milissegundos desde 1970, versão 7, variante RFC 9562 e 74 bits
/// aleatórios.
#[must_use]
pub fn new_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes[6..]).expect("gerador aleatório do SO indisponível");
    #[allow(clippy::cast_possible_truncation)] // 48 bits de milissegundos
    let millis = (millis as u64).to_be_bytes();
    bytes[..6].copy_from_slice(&millis[2..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0F);
    bytes[8] = 0x80 | (bytes[8] & 0x3F);
    let mut out = String::with_capacity(36);
    for (i, b) in bytes.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_v7_e_ordem() {
        let a = new_id();
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "7");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(new_id() > a);
    }
}
