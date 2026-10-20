//! Leitura do pipe do serviço `codatos-sensors`, do lado do app.

use std::io;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ClientOptions;

use crate::{NOT_INSTALLED, PIPE_NAME, Reply};

/// `ERROR_PIPE_BUSY`: o serviço está entre uma conexão e outra.
const PIPE_BUSY: i32 = 231;
const MAX_LINE: u64 = 4096;

/// Uma leitura do serviço. Sem serviço, devolve [`NOT_INSTALLED`].
pub async fn read() -> Reply {
    match tokio::time::timeout(Duration::from_secs(2), connect_and_read()).await {
        Ok(Ok(reply)) => reply,
        Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => Reply::error(
            NOT_INSTALLED,
            "Temperatura da CPU desativada: ative no widget",
        ),
        Ok(Err(e)) => Reply::error(
            NOT_INSTALLED,
            format!("O serviço de sensores não respondeu: {e}"),
        ),
        Err(_) => Reply::error(NOT_INSTALLED, "O serviço de sensores não respondeu"),
    }
}

async fn connect_and_read() -> io::Result<Reply> {
    let mut attempts = 0;
    let pipe = loop {
        match ClientOptions::new().read(true).write(false).open(PIPE_NAME) {
            Ok(pipe) => break pipe,
            Err(e) if e.raw_os_error() == Some(PIPE_BUSY) && attempts < 10 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            Err(e) => return Err(e),
        }
    };
    let mut pipe = pipe.take(MAX_LINE);
    let mut line = Vec::new();
    let mut chunk = [0_u8; 512];
    while !line.contains(&b'\n') {
        match pipe.read(&mut chunk).await? {
            0 => break,
            n => line.extend_from_slice(&chunk[..n]),
        }
    }
    serde_json::from_slice(line.trim_ascii())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
