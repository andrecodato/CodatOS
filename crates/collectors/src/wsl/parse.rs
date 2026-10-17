//! Leitura das saídas do `wsl.exe` e do `df`, testável em qualquer SO.

/// Texto de um comando do `wsl.exe`: UTF-16LE por padrão, UTF-8 com `WSL_UTF8=1` ou quando
/// vem de dentro da distro. Um byte nulo denuncia o UTF-16.
pub(crate) fn decode(bytes: &[u8]) -> String {
    if bytes.contains(&0) {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
    .trim_start_matches('\u{feff}')
    .to_owned()
}

/// Nomes de `wsl.exe --list --running --quiet`, um por linha.
pub(crate) fn running_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Usado e total em bytes de `df -B1 --output=used,size /`.
pub(crate) fn df_used_total(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().nth(1)?;
    let mut fields = line.split_whitespace().map(str::parse::<u64>);
    Some((fields.next()?.ok()?, fields.next()?.ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn decodifica_utf16_e_utf8() {
        assert_eq!(
            decode(&utf16("\u{feff}Ubuntu\r\nDebian\r\n")),
            "Ubuntu\r\nDebian\r\n"
        );
        assert_eq!(decode(b"Ubuntu\n"), "Ubuntu\n");
    }

    #[test]
    fn nomes_das_distros_ligadas() {
        let text = decode(&utf16("Ubuntu-24.04\r\ndocker-desktop\r\n\r\n"));
        assert_eq!(running_names(&text), ["Ubuntu-24.04", "docker-desktop"]);
        assert_eq!(running_names(""), Vec::<String>::new());
    }

    #[test]
    fn saida_do_df() {
        let text = "    Used        1B-blocks\n 12345678 1081101176832\n";
        assert_eq!(df_used_total(text), Some((12_345_678, 1_081_101_176_832)));
        assert_eq!(df_used_total("df: unrecognized option\n"), None);
    }
}
