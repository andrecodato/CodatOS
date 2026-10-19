//! Ids gerados no cliente (`docs/05-modelo-de-dados.md`).

use std::fmt::Write as _;

use leptos::prelude::window;

/// UUID v7 (tempo + aleatório), como pede o modelo de dados para ids gerados no cliente.
pub fn new_id() -> String {
    let mut random = [0u8; 10];
    if let Ok(crypto) = window().crypto() {
        let _ = crypto.get_random_values_with_u8_array(&mut random);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // ms desde 1970
    let ms = js_sys::Date::now() as u64;
    uuid_v7(ms, random)
}

fn uuid_v7(ms: u64, random: [u8; 10]) -> String {
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[6..].copy_from_slice(&random);
    b[6] = 0x70 | (b[6] & 0x0f);
    b[8] = 0x80 | (b[8] & 0x3f);
    let mut out = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v7_tem_versao_variante_e_tempo() {
        let id = uuid_v7(0x0189_abcd_ef01, [0xff; 10]);
        assert_eq!(id, "0189abcd-ef01-7fff-bfff-ffffffffffff");
        assert_eq!(id.len(), 36);
    }
}
