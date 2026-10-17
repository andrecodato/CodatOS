//! Leitura das saídas do `wsl.exe` e do `df`, testável em qualquer SO.

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

    #[test]
    fn nomes_das_distros_ligadas() {
        let text = "Ubuntu-24.04\r\ndocker-desktop\r\n\r\n";
        assert_eq!(running_names(text), ["Ubuntu-24.04", "docker-desktop"]);
        assert_eq!(running_names(""), Vec::<String>::new());
    }

    #[test]
    fn saida_do_df() {
        let text = "    Used        1B-blocks\n 12345678 1081101176832\n";
        assert_eq!(df_used_total(text), Some((12_345_678, 1_081_101_176_832)));
        assert_eq!(df_used_total("df: unrecognized option\n"), None);
    }
}
