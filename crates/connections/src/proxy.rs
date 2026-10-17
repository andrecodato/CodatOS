//! Proxy SOCKS5 ou HTTP (`CONNECT`) antes da conexão SSH (RF-CON-041), configurado em
//! `advanced.proxy` do host ou do grupo: `{"type": "socks5" | "http", "host": …, "port": …}`.
//!
//! Só proxies sem autenticação por enquanto: a senha do proxy precisaria ir para o cofre,
//! e `advanced` fica em texto puro.

use std::io;
use std::time::Duration;

use codatos_core::MethodError;
use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Resposta do proxy HTTP até o fim dos cabeçalhos.
const MAX_HTTP_REPLY: usize = 8 * 1024;
/// Para o caso de o proxy aceitar a conexão e não responder.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    Socks5,
    Http,
}

/// Proxy de um host.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Proxy {
    #[serde(rename = "type")]
    kind: Kind,
    host: String,
    port: u16,
}

impl Proxy {
    /// Lê `advanced.proxy`. Ausente ou `null`: conexão direta.
    pub(crate) fn from_advanced(advanced: &Value) -> Result<Option<Self>, MethodError> {
        match advanced.get("proxy") {
            None | Some(Value::Null) => Ok(None),
            Some(value) => serde_json::from_value::<Self>(value.clone())
                .ok()
                .filter(|p| !p.host.trim().is_empty() && p.port != 0)
                .map(Some)
                .ok_or_else(|| {
                    MethodError::invalid_params(
                        "Proxy inválido: use {\"type\": \"socks5\" ou \"http\", \"host\", \"port\"}",
                    )
                }),
        }
    }

    pub(crate) fn describe(&self) -> String {
        let kind = match self.kind {
            Kind::Socks5 => "SOCKS5",
            Kind::Http => "HTTP",
        };
        format!("proxy {kind} {}:{}", self.host.trim(), self.port)
    }

    /// Conexão TCP com `host:port` passando pelo proxy.
    pub(crate) async fn connect(&self, host: &str, port: u16) -> io::Result<TcpStream> {
        let mut stream = TcpStream::connect((self.host.trim(), self.port)).await?;
        stream.set_nodelay(true)?;
        let handshake = async {
            match self.kind {
                Kind::Socks5 => socks5(&mut stream, host, port).await,
                Kind::Http => http_connect(&mut stream, host, port).await,
            }
        };
        tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "o proxy não respondeu"))??;
        Ok(stream)
    }
}

fn refused(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, message.to_owned())
}

/// RFC 1928, sem autenticação, com o nome resolvido pelo proxy.
async fn socks5(stream: &mut TcpStream, host: &str, port: u16) -> io::Result<()> {
    stream.write_all(&[5, 1, 0]).await?;
    let mut choice = [0_u8; 2];
    stream.read_exact(&mut choice).await?;
    if choice != [5, 0] {
        return Err(refused("o proxy SOCKS5 exige autenticação"));
    }
    let name = host.as_bytes();
    let len = u8::try_from(name.len()).map_err(|_| refused("nome de host longo demais"))?;
    let mut request = vec![5, 1, 0, 3, len];
    request.extend_from_slice(name);
    request.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&request).await?;

    let mut head = [0_u8; 4];
    stream.read_exact(&mut head).await?;
    if head[0] != 5 {
        return Err(refused("resposta inválida do proxy SOCKS5"));
    }
    if head[1] != 0 {
        return Err(refused(match head[1] {
            2 => "o proxy SOCKS5 não permite este destino",
            3 => "rede inalcançável pelo proxy SOCKS5",
            4 => "host inalcançável pelo proxy SOCKS5",
            5 => "o destino recusou a conexão do proxy SOCKS5",
            _ => "o proxy SOCKS5 recusou a conexão",
        }));
    }
    // Endereço de saída do proxy, que não usamos.
    let address = match head[3] {
        1 => 4,
        4 => 16,
        3 => usize::from(stream.read_u8().await?),
        _ => return Err(refused("resposta inválida do proxy SOCKS5")),
    };
    let mut rest = vec![0_u8; address + 2];
    stream.read_exact(&mut rest).await?;
    Ok(())
}

/// `CONNECT host:porta`. Lê byte a byte até o fim dos cabeçalhos para não consumir o
/// começo da conversa SSH.
async fn http_connect(stream: &mut TcpStream, host: &str, port: u16) -> io::Result<()> {
    let authority = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    stream
        .write_all(format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes())
        .await?;
    let mut reply = Vec::new();
    while !reply.ends_with(b"\r\n\r\n") {
        if reply.len() >= MAX_HTTP_REPLY {
            return Err(refused("resposta longa demais do proxy HTTP"));
        }
        reply.push(stream.read_u8().await?);
    }
    let status = reply
        .split(|&b| b == b' ')
        .nth(1)
        .and_then(|code| std::str::from_utf8(code).ok());
    match status {
        Some("200") => Ok(()),
        Some("407") => Err(refused("o proxy HTTP exige autenticação")),
        _ => Err(refused("o proxy HTTP recusou a conexão")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::net::TcpListener;

    #[test]
    fn le_do_advanced() {
        assert_eq!(Proxy::from_advanced(&json!({})).unwrap(), None);
        assert_eq!(Proxy::from_advanced(&json!({"proxy": null})).unwrap(), None);
        let proxy = Proxy::from_advanced(
            &json!({"proxy": {"type": "socks5", "host": "10.0.0.1", "port": 1080}}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(proxy.describe(), "proxy SOCKS5 10.0.0.1:1080");
        for bad in [
            json!({"proxy": {"type": "ftp", "host": "x", "port": 1}}),
            json!({"proxy": {"type": "http", "host": "", "port": 3128}}),
            json!({"proxy": {"type": "http", "host": "x", "port": 0}}),
            json!({"proxy": "socks5://x"}),
        ] {
            assert!(Proxy::from_advanced(&bad).is_err(), "{bad}");
        }
    }

    /// Proxy de mentira: a cada passo confere o que chegou e responde; no fim manda "ola"
    /// como se fosse o destino.
    async fn fake_proxy(steps: &'static [(&'static [u8], &'static [u8])]) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            for (expect, reply) in steps {
                let mut got = vec![0_u8; expect.len()];
                socket.read_exact(&mut got).await.unwrap();
                assert_eq!(got, *expect);
                socket.write_all(reply).await.unwrap();
            }
            socket.write_all(b"ola").await.unwrap();
        });
        port
    }

    async fn through(kind: &str, port: u16, host: &str) -> io::Result<String> {
        let proxy = Proxy::from_advanced(
            &json!({"proxy": {"type": kind, "host": "127.0.0.1", "port": port}}),
        )
        .unwrap()
        .unwrap();
        let mut stream = proxy.connect(host, 22).await?;
        let mut first = [0_u8; 3];
        stream.read_exact(&mut first).await?;
        Ok(String::from_utf8_lossy(&first).into_owned())
    }

    #[tokio::test]
    async fn socks5_sem_autenticacao() {
        let port = fake_proxy(&[
            (b"\x05\x01\x00", b"\x05\x00"),
            (
                b"\x05\x01\x00\x03\x03srv\x00\x16",
                b"\x05\x00\x00\x01\x7f\x00\x00\x01\x04\x38",
            ),
        ])
        .await;
        assert_eq!(through("socks5", port, "srv").await.unwrap(), "ola");

        let port = fake_proxy(&[(b"\x05\x01\x00", b"\x05\xff")]).await;
        let err = through("socks5", port, "srv").await.unwrap_err();
        assert!(err.to_string().contains("autenticação"), "{err}");

        let port = fake_proxy(&[
            (b"\x05\x01\x00", b"\x05\x00"),
            (b"\x05\x01\x00\x03\x03srv\x00\x16", b"\x05\x04\x00\x01"),
        ])
        .await;
        let err = through("socks5", port, "srv").await.unwrap_err();
        assert!(err.to_string().contains("inalcançável"), "{err}");
    }

    #[tokio::test]
    async fn http_connect_200_e_407() {
        let port = fake_proxy(&[(
            b"CONNECT srv:22 HTTP/1.1\r\nHost: srv:22\r\n\r\n",
            b"HTTP/1.1 200 Connection established\r\nVia: teste\r\n\r\n",
        )])
        .await;
        // O "ola" depois dos cabeçalhos fica para a conexão SSH.
        assert_eq!(through("http", port, "srv").await.unwrap(), "ola");

        let port = fake_proxy(&[(
            b"CONNECT [::1]:22 HTTP/1.1\r\nHost: [::1]:22\r\n\r\n",
            b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n",
        )])
        .await;
        let err = through("http", port, "::1").await.unwrap_err();
        assert!(err.to_string().contains("autenticação"), "{err}");
    }
}
