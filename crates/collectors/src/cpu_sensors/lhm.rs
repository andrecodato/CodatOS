//! Leitura do servidor web do LibreHardwareMonitor (`http://127.0.0.1:8085/data.json`).
//!
//! O JSON é uma árvore: máquina → hardware (CPU, GPU…) → grupo (Temperatures, Powers…) →
//! sensor. Versões novas trazem `SensorId` (`/amdcpu/0/temperature/2`) e as antigas só o
//! texto do grupo; os valores vêm como texto com unidade e, no Windows em pt-BR, com
//! vírgula decimal (`"45,0 °C"`).

use std::net::SocketAddr;
use std::time::Duration;

use codatos_core::CollectError;
use codatos_protocol::system::{CpuSensors, SensorSource};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[cfg_attr(not(windows), allow(dead_code))]
pub(super) const DEFAULT_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8085);

/// Tamanho máximo da resposta (a árvore completa costuma ter 100–300 KB).
const MAX_RESPONSE: u64 = 8 << 20;

#[cfg_attr(not(windows), allow(dead_code))]
pub(super) async fn read(addr: SocketAddr) -> Result<CpuSensors, CollectError> {
    let body = fetch(addr).await?;
    let tree: Value = serde_json::from_slice(&body).map_err(|_| {
        CollectError::new("lhm_invalid", "Resposta inválida do LibreHardwareMonitor")
    })?;
    let sensors = cpu_sensors(&tree);
    if sensors.temperature_c.is_none() && sensors.power_w.is_none() {
        return Err(CollectError::unavailable(
            "O LibreHardwareMonitor não mostra sensores da CPU",
        ));
    }
    Ok(sensors)
}

/// GET HTTP/1.0 simples: é só `127.0.0.1`, sem TLS nem redirecionamento.
async fn fetch(addr: SocketAddr) -> Result<Vec<u8>, CollectError> {
    let mut stream = tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(addr))
        .await
        .map_err(|_| not_running())?
        .map_err(|_| not_running())?;
    let request =
        format!("GET /data.json HTTP/1.0\r\nHost: {addr}\r\nAccept: application/json\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|_| not_running())?;
    let mut raw = Vec::new();
    stream
        .take(MAX_RESPONSE)
        .read_to_end(&mut raw)
        .await
        .map_err(|_| not_running())?;
    split_response(&raw).map(<[u8]>::to_vec).ok_or_else(|| {
        CollectError::new("lhm_invalid", "Resposta inválida do LibreHardwareMonitor")
    })
}

fn not_running() -> CollectError {
    CollectError::new(
        "lhm_not_running",
        "Temperatura indisponível: abra o LibreHardwareMonitor com o servidor web ligado",
    )
}

/// Corpo de uma resposta `200`.
fn split_response(raw: &[u8]) -> Option<&[u8]> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&raw[..end]).ok()?;
    let status = head.split_whitespace().nth(1)?;
    (status == "200").then_some(&raw[end + 4..])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Temperature,
    Power,
    Other,
}

/// Sensores da primeira CPU da árvore.
fn cpu_sensors(tree: &Value) -> CpuSensors {
    let mut found = Vec::new();
    walk(tree, false, Kind::Other, &mut found);

    let pick = |kind: Kind, preferred: &[&str]| -> Option<f32> {
        let of_kind: Vec<&(Kind, String, f32)> =
            found.iter().filter(|(k, _, _)| *k == kind).collect();
        for name in preferred {
            if let Some((_, _, v)) = of_kind
                .iter()
                .find(|(_, text, _)| text.eq_ignore_ascii_case(name))
            {
                return Some(*v);
            }
        }
        match kind {
            // Sem sensor de pacote: o núcleo mais quente.
            Kind::Temperature => of_kind
                .iter()
                .filter(|(_, text, _)| text.to_ascii_lowercase().starts_with("core"))
                .map(|(_, _, v)| *v)
                .reduce(f32::max),
            _ => None,
        }
    };

    CpuSensors {
        temperature_c: pick(
            Kind::Temperature,
            &[
                "CPU Package",
                "Core (Tctl/Tdie)",
                "Core (Tctl)",
                "Tctl",
                "Tdie",
                "Core Average",
            ],
        ),
        power_w: pick(Kind::Power, &["CPU Package", "Package"]),
        source: SensorSource::LibreHardwareMonitor,
    }
}

fn walk(node: &Value, in_cpu: bool, group: Kind, found: &mut Vec<(Kind, String, f32)>) {
    let text = node.get("Text").and_then(Value::as_str).unwrap_or_default();
    let image = node
        .get("ImageURL")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let sensor_id = node.get("SensorId").and_then(Value::as_str);

    let in_cpu = in_cpu || image.contains("cpu");
    let group = match text.to_ascii_lowercase().as_str() {
        "temperatures" => Kind::Temperature,
        "powers" => Kind::Power,
        _ => group,
    };

    let children = node.get("Children").and_then(Value::as_array);
    if children.is_none_or(Vec::is_empty) {
        let (cpu_sensor, kind) = match sensor_id {
            // `/intelcpu/0/temperature/0`: hardware e tipo vêm no próprio id.
            Some(id) => {
                let parts: Vec<&str> = id.split('/').collect();
                let kind = match parts.get(3).copied() {
                    Some("temperature") => Kind::Temperature,
                    Some("power") => Kind::Power,
                    _ => Kind::Other,
                };
                (parts.get(1).is_some_and(|h| h.ends_with("cpu")), kind)
            }
            None => (in_cpu, group),
        };
        if cpu_sensor && kind != Kind::Other {
            let value = node
                .get("Value")
                .and_then(Value::as_str)
                .and_then(parse_value);
            if let Some(value) = value {
                found.push((kind, text.to_owned(), value));
            }
        }
        return;
    }
    for child in children.into_iter().flatten() {
        walk(child, in_cpu, group, found);
    }
}

/// `"45,0 °C"`, `"12.3 W"` → número. Traço ou texto vazio (sensor sem leitura) → `None`.
fn parse_value(text: &str) -> Option<f32> {
    let number: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '-'))
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    number.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sensor(text: &str, value: &str, id: Option<&str>) -> Value {
        let mut v = json!({"Text": text, "Value": value, "Children": []});
        if let Some(id) = id {
            v["SensorId"] = json!(id);
        }
        v
    }

    /// Formato antigo: sem `SensorId`, o tipo vem do grupo e a CPU do ícone.
    fn tree_without_ids() -> Value {
        json!({"Text": "Sensor", "Children": [{"Text": "DESKTOP", "Children": [
            {"Text": "AMD Ryzen 7 5800X", "ImageURL": "images_icon/cpu.png", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("Core (Tctl/Tdie)", "61,3 °C", None),
                    sensor("CCD1 (Tdie)", "58,0 °C", None),
                ]},
                {"Text": "Powers", "Children": [sensor("Package", "45,2 W", None)]},
            ]},
            {"Text": "NVIDIA GeForce RTX 3070", "ImageURL": "images_icon/nvidia.png", "Children": [
                {"Text": "Temperatures", "Children": [sensor("GPU Core", "50,0 °C", None)]},
            ]},
        ]}]})
    }

    #[test]
    fn le_formato_antigo_com_virgula() {
        let s = cpu_sensors(&tree_without_ids());
        assert_eq!(s.temperature_c, Some(61.3));
        assert_eq!(s.power_w, Some(45.2));
        assert_eq!(s.source, SensorSource::LibreHardwareMonitor);
    }

    #[test]
    fn le_formato_novo_com_sensor_id() {
        let tree = json!({"Text": "Sensor", "Children": [{"Text": "PC", "Children": [
            {"Text": "Intel Core i7-12700K", "ImageURL": "images_icon/cpu.png", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("Core #1", "40.0 °C", Some("/intelcpu/0/temperature/0")),
                    sensor("CPU Package", "55.0 °C", Some("/intelcpu/0/temperature/12")),
                ]},
                {"Text": "Powers", "Children": [
                    sensor("CPU Package", "30.5 W", Some("/intelcpu/0/power/0")),
                ]},
            ]},
            {"Text": "GPU", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("GPU Core", "70.0 °C", Some("/gpu-nvidia/0/temperature/0")),
                ]},
            ]},
        ]}]});
        let s = cpu_sensors(&tree);
        assert_eq!(s.temperature_c, Some(55.0));
        assert_eq!(s.power_w, Some(30.5));
    }

    #[test]
    fn sem_pacote_usa_o_nucleo_mais_quente() {
        let tree = json!({"Children": [{"ImageURL": "cpu.png", "Children": [
            {"Text": "Temperatures", "Children": [
                sensor("Core #1", "40 °C", None),
                sensor("Core #2", "47 °C", None),
            ]},
        ]}]});
        assert_eq!(cpu_sensors(&tree).temperature_c, Some(47.0));
    }

    #[test]
    fn valores() {
        assert_eq!(parse_value("45,0 °C"), Some(45.0));
        assert_eq!(parse_value("12.3 W"), Some(12.3));
        assert_eq!(parse_value("-"), None);
        assert_eq!(parse_value(""), None);
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
        let error = read(addr).await.unwrap_err();
        assert_eq!(error.code(), "lhm_not_running");
    }

    #[tokio::test]
    async fn le_de_um_servidor_de_verdade() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = tree_without_ids().to_string();
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
        let s = read(addr).await.unwrap();
        assert_eq!(s.temperature_c, Some(61.3));
    }
}
