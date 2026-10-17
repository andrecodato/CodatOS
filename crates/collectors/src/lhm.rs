//! Servidor web do LibreHardwareMonitor (`http://127.0.0.1:8085/data.json`), fonte dos sensores
//! no Windows (Q-13): temperatura da CPU e GPUs que não são NVIDIA.
//!
//! O JSON é uma árvore: raiz → máquina → hardware (CPU, GPU…) → grupo (Temperatures, Powers…)
//! → sensor. Versões novas trazem `SensorId` (`/amdcpu/0/temperature/2`) e as antigas só o
//! texto do grupo; os valores vêm como texto com unidade e, no Windows em pt-BR, com vírgula
//! decimal (`"45,0 °C"`).

use std::net::SocketAddr;
use std::time::Duration;

use codatos_core::CollectError;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const DEFAULT_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8085);

/// Tamanho máximo da resposta (a árvore completa costuma ter 100–300 KB).
const MAX_RESPONSE: u64 = 8 << 20;

/// Código do erro quando o LHM não está rodando (ou está sem o servidor web).
pub(crate) const NOT_RUNNING: &str = "lhm_not_running";

/// Baixa e decodifica a árvore. `not_running` é o texto mostrado no widget quando o LHM
/// não responde.
pub(crate) async fn tree(addr: SocketAddr, not_running: &str) -> Result<Value, CollectError> {
    let gone = || CollectError::new(NOT_RUNNING, not_running);
    let mut stream = tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(addr))
        .await
        .map_err(|_| gone())?
        .map_err(|_| gone())?;
    // GET HTTP/1.0 simples: é só `127.0.0.1`, sem TLS nem redirecionamento.
    let request =
        format!("GET /data.json HTTP/1.0\r\nHost: {addr}\r\nAccept: application/json\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|_| gone())?;
    let mut raw = Vec::new();
    stream
        .take(MAX_RESPONSE)
        .read_to_end(&mut raw)
        .await
        .map_err(|_| gone())?;
    split_response(&raw)
        .and_then(|body| serde_json::from_slice(body).ok())
        .ok_or_else(|| {
            CollectError::new("lhm_invalid", "Resposta inválida do LibreHardwareMonitor")
        })
}

/// Corpo de uma resposta `200`.
fn split_response(raw: &[u8]) -> Option<&[u8]> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..end]).ok()?;
    let status = head.split_whitespace().nth(1)?;
    (status == "200").then_some(&raw[end + 4..])
}

/// Tipo do sensor, pelo `SensorId` (`/gpu-amd/0/load/0`) ou pelo grupo (`Load`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Temperature,
    Power,
    Load,
    /// `Data` (GB) e `SmallData` (MB): memória.
    Data,
    Other,
}

impl Kind {
    /// Pelo texto do grupo no formato antigo.
    pub(crate) fn from_group(text: &str) -> Option<Self> {
        Some(match text.to_ascii_lowercase().as_str() {
            "temperatures" => Self::Temperature,
            "powers" => Self::Power,
            "load" => Self::Load,
            "data" | "smalldata" => Self::Data,
            _ => return None,
        })
    }

    /// Pelo terceiro segmento do `SensorId`.
    pub(crate) fn from_sensor_id(id: &str) -> Self {
        match id.split('/').nth(3) {
            Some("temperature") => Self::Temperature,
            Some("power") => Self::Power,
            Some("load") => Self::Load,
            Some("data" | "smalldata") => Self::Data,
            _ => Self::Other,
        }
    }
}

/// `"45,0 °C"`, `"12.3 W"` → número. Traço ou texto vazio (sensor sem leitura) → `None`.
pub(crate) fn parse_value(text: &str) -> Option<f32> {
    let number: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '-'))
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    number.parse().ok()
}

/// `"10240 MB"`, `"1,5 GB"` → bytes. Sem unidade, o LHM usa MB (`SmallData`).
pub(crate) fn parse_bytes(text: &str) -> Option<u64> {
    let value = f64::from(parse_value(text)?);
    let unit = text.trim().to_ascii_uppercase();
    let factor = if unit.ends_with("GB") {
        1024.0 * 1024.0 * 1024.0
    } else if unit.ends_with("KB") {
        1024.0
    } else {
        1024.0 * 1024.0
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (value >= 0.0).then(|| (value * factor).round() as u64)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// Sensor folha, com ou sem `SensorId`.
    pub(crate) fn sensor(text: &str, value: &str, id: Option<&str>) -> Value {
        let mut v = json!({"Text": text, "Value": value, "Children": []});
        if let Some(id) = id {
            v["SensorId"] = json!(id);
        }
        v
    }

    /// Serve `body` uma vez em `127.0.0.1` e devolve o endereço.
    pub(crate) async fn serve_once(body: String) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        addr
    }

    #[test]
    fn valores() {
        assert_eq!(parse_value("45,0 °C"), Some(45.0));
        assert_eq!(parse_value("12.3 W"), Some(12.3));
        assert_eq!(parse_value("-"), None);
        assert_eq!(parse_value(""), None);
    }

    #[test]
    fn bytes() {
        assert_eq!(parse_bytes("12272 MB"), Some(12272 * 1024 * 1024));
        assert_eq!(parse_bytes("1,5 GB"), Some(1_610_612_736));
        assert_eq!(parse_bytes("512"), Some(512 * 1024 * 1024));
        assert_eq!(parse_bytes("-"), None);
    }

    #[test]
    fn tipos() {
        assert_eq!(Kind::from_sensor_id("/gpu-amd/0/smalldata/1"), Kind::Data);
        assert_eq!(Kind::from_sensor_id("/amdcpu/0/power/0"), Kind::Power);
        assert_eq!(Kind::from_sensor_id("/gpu-amd/0/clock/0"), Kind::Other);
        assert_eq!(Kind::from_group("Load"), Some(Kind::Load));
        assert_eq!(Kind::from_group("Clocks"), None);
    }

    #[test]
    fn resposta_http() {
        assert_eq!(
            split_response(b"HTTP/1.1 200 OK\r\nContent-Type: x\r\n\r\n{}"),
            Some(&b"{}"[..])
        );
        assert_eq!(split_response(b"HTTP/1.1 404 Not Found\r\n\r\nx"), None);
    }

    #[tokio::test]
    async fn servidor_desligado_vira_erro_claro() {
        // Porta livre e fechada.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let error = tree(addr, "abra o LHM").await.unwrap_err();
        assert_eq!(error.code(), NOT_RUNNING);
        assert_eq!(error.to_string(), "abra o LHM");
    }
}
