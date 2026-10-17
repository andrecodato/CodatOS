//! Temperatura e potência da CPU lidas do LibreHardwareMonitor (ver [`crate::lhm`]).

use std::net::SocketAddr;

use codatos_core::CollectError;
use codatos_protocol::system::{CpuSensors, SensorSource};
use serde_json::Value;

use crate::lhm::{self, Kind, parse_value};

#[cfg_attr(not(windows), allow(dead_code))]
pub(super) async fn read(addr: SocketAddr) -> Result<CpuSensors, CollectError> {
    let tree = lhm::tree(
        addr,
        "Temperatura indisponível: abra o LibreHardwareMonitor com o servidor web ligado",
    )
    .await?;
    let sensors = cpu_sensors(&tree);
    if sensors.temperature_c.is_none() && sensors.power_w.is_none() {
        return Err(CollectError::unavailable(
            "O LibreHardwareMonitor não mostra sensores da CPU",
        ));
    }
    Ok(sensors)
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
    let group = Kind::from_group(text).unwrap_or(group);

    let children = node.get("Children").and_then(Value::as_array);
    if children.is_none_or(Vec::is_empty) {
        let (cpu_sensor, kind) = match sensor_id {
            // `/intelcpu/0/temperature/0`: hardware e tipo vêm no próprio id.
            Some(id) => (
                id.split('/').nth(1).is_some_and(|h| h.ends_with("cpu")),
                Kind::from_sensor_id(id),
            ),
            None => (in_cpu, group),
        };
        if cpu_sensor && matches!(kind, Kind::Temperature | Kind::Power) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lhm::tests::{sensor, serve_once};
    use serde_json::json;

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

    #[tokio::test]
    async fn le_de_um_servidor_de_verdade() {
        let addr = serve_once(tree_without_ids().to_string()).await;
        let s = read(addr).await.unwrap();
        assert_eq!(s.temperature_c, Some(61.3));
    }
}
