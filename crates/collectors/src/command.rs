//! Comandos externos (`wsl.exe`, `docker`) sem janela de console e com prazo.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

/// Prazo padrão de um comando.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(5);

/// Sem isso, cada chamada no Windows piscaria uma janela de console.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Roda `program` e devolve a saída padrão se ele terminar com sucesso dentro do prazo.
/// A saída do `wsl.exe` (UTF-16LE por padrão) é decodificada também.
pub(crate) async fn output(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        // `wsl.exe` passa a responder em UTF-8.
        .env("WSL_UTF8", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = tokio::time::timeout(timeout, command.output())
        .await
        .ok()?
        .ok()?;
    output.status.success().then(|| decode(&output.stdout))
}

/// UTF-16LE (o `wsl.exe` sem `WSL_UTF8`) ou UTF-8. Um byte nulo denuncia o UTF-16.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_utf16_e_utf8() {
        let utf16: Vec<u8> = "\u{feff}Ubuntu\r\nDebian\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(decode(&utf16), "Ubuntu\r\nDebian\r\n");
        assert_eq!(decode(b"Ubuntu\n"), "Ubuntu\n");
    }

    #[tokio::test]
    async fn comando_inexistente_ou_com_erro_vira_none() {
        assert_eq!(output("comando-que-nao-existe", &[], TIMEOUT).await, None);
    }
}
