//! Canais binários do WebSocket `/rpc` (terminal e SFTP), `docs/06-api-e-protocolos.md`.
//!
//! Cada frame binário começa com o número do canal (`u32` big-endian) e o tipo (`u8`).
//! Quem abre um canal é um método JSON-RPC, que devolve `{"channel": n, …}`.
//!
//! Controle de fluxo: o núcleo só manda [`INITIAL_WINDOW`] bytes de dados sem confirmação;
//! a UI devolve [`Frame::Ack`] com quantos bytes já consumiu, e a janela reabre.

/// Bytes antes do conteúdo: canal (4) e tipo (1).
pub const HEADER_LEN: usize = 5;
/// Dados por frame; mensagens maiores são cortadas.
pub const MAX_PAYLOAD: usize = 64 * 1024;
/// Bytes de dados que o núcleo manda por canal sem confirmação da UI.
pub const INITIAL_WINDOW: u32 = 256 * 1024;

const DATA: u8 = 0;
const RESIZE: u8 = 1;
const CLOSE: u8 = 2;
const ACK: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame<'a> {
    Data(&'a [u8]),
    /// Novo tamanho do terminal (só da UI para o núcleo).
    Resize {
        cols: u16,
        rows: u16,
    },
    /// Fim do canal, de qualquer lado.
    Close,
    /// Bytes de dados que a UI já consumiu (só da UI para o núcleo).
    Ack(u32),
}

#[must_use]
pub fn encode(channel: u32, frame: Frame<'_>) -> Vec<u8> {
    let (kind, body): (u8, Vec<u8>) = match frame {
        Frame::Data(data) => (DATA, data.to_vec()),
        Frame::Resize { cols, rows } => (RESIZE, [cols.to_be_bytes(), rows.to_be_bytes()].concat()),
        Frame::Close => (CLOSE, Vec::new()),
        Frame::Ack(bytes) => (ACK, bytes.to_be_bytes().to_vec()),
    };
    let mut out = Vec::with_capacity(HEADER_LEN + body.len());
    out.extend_from_slice(&channel.to_be_bytes());
    out.push(kind);
    out.extend_from_slice(&body);
    out
}

/// `None` para frames curtos demais ou de tipo desconhecido.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(u32, Frame<'_>)> {
    let (header, body) = bytes.split_at_checked(HEADER_LEN)?;
    let channel = u32::from_be_bytes(header[..4].try_into().ok()?);
    let frame = match header[4] {
        DATA => Frame::Data(body),
        RESIZE => {
            let [c0, c1, r0, r1] = body.try_into().ok()?;
            Frame::Resize {
                cols: u16::from_be_bytes([c0, c1]),
                rows: u16::from_be_bytes([r0, r1]),
            }
        }
        CLOSE if body.is_empty() => Frame::Close,
        ACK => Frame::Ack(u32::from_be_bytes(body.try_into().ok()?)),
        _ => return None,
    };
    Some((channel, frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta() {
        for frame in [
            Frame::Data(b"ls -la\r"),
            Frame::Data(b""),
            Frame::Resize {
                cols: 120,
                rows: 40,
            },
            Frame::Close,
            Frame::Ack(65536),
        ] {
            let bytes = encode(7, frame);
            assert_eq!(decode(&bytes), Some((7, frame)));
        }
        assert_eq!(encode(1, Frame::Close), [0, 0, 0, 1, 2]);
    }

    #[test]
    fn recusa_frames_quebrados() {
        assert_eq!(decode(&[0, 0, 0]), None);
        assert_eq!(decode(&[0, 0, 0, 1, 9]), None);
        assert_eq!(decode(&[0, 0, 0, 1, 1, 0, 80]), None);
        assert_eq!(decode(&[0, 0, 0, 1, 2, 0]), None);
    }
}
