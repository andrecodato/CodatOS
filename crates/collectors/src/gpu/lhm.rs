//! GPUs AMD e Intel (e NVIDIA sem NVML) lidas do LibreHardwareMonitor (ver [`crate::lhm`]).
//!
//! O LHM lê as Radeon pelo ADLX/ADL do driver da AMD e mostra uso, temperatura, potência e
//! memória; o mesmo servidor web que já dá a temperatura da CPU (Q-13).

use std::net::SocketAddr;

use codatos_core::CollectError;
use codatos_protocol::system::Gpu;
use serde_json::Value;

use crate::lhm::{self, Kind, parse_bytes, parse_value};

#[cfg_attr(not(windows), allow(dead_code))]
pub(super) async fn read(addr: SocketAddr) -> Result<Vec<Gpu>, CollectError> {
    let tree = lhm::tree(
        addr,
        "Para ver GPUs AMD e Intel, abra o LibreHardwareMonitor com o servidor web ligado",
    )
    .await?;
    let gpus = gpus(&tree);
    if gpus.is_empty() {
        return Err(CollectError::unavailable(
            "O LibreHardwareMonitor não mostra nenhuma GPU",
        ));
    }
    Ok(gpus)
}

/// Um sensor de GPU: tipo, nome (`GPU Core`) e o texto do valor (`"45,0 °C"`).
type Reading = (Kind, String, String);

/// GPUs da árvore, na ordem em que aparecem.
fn gpus(tree: &Value) -> Vec<Gpu> {
    let mut list = Vec::new();
    // raiz → máquina → hardware.
    for machine in children(tree) {
        for hardware in children(machine) {
            let mut readings = Vec::new();
            let mut is_gpu = image(hardware).is_some_and(is_gpu_image);
            collect(hardware, Kind::Other, &mut readings, &mut is_gpu);
            if is_gpu && !readings.is_empty() {
                let name = hardware
                    .get("Text")
                    .and_then(Value::as_str)
                    .unwrap_or("GPU");
                list.push(gpu(name, &readings));
            }
        }
    }
    list
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node.get("Children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn image(node: &Value) -> Option<&str> {
    node.get("ImageURL").and_then(Value::as_str)
}

/// Ícones de GPU do LHM: `ati.png` (AMD), `nvidia.png`, `intel.png`.
fn is_gpu_image(image: &str) -> bool {
    let image = image.to_ascii_lowercase();
    ["ati.png", "amd.png", "nvidia.png", "intel.png", "gpu"]
        .iter()
        .any(|k| image.contains(k))
        && !image.contains("cpu")
}

fn collect(node: &Value, group: Kind, out: &mut Vec<Reading>, is_gpu: &mut bool) {
    let text = node.get("Text").and_then(Value::as_str).unwrap_or_default();
    let group = Kind::from_group(text).unwrap_or(group);
    let mut kids = children(node).peekable();
    if kids.peek().is_some() {
        for child in kids {
            collect(child, group, out, is_gpu);
        }
        return;
    }
    let kind = match node.get("SensorId").and_then(Value::as_str) {
        // `/gpu-amd/0/temperature/0`: hardware e tipo vêm no próprio id.
        Some(id) => {
            if id.split('/').nth(1).is_some_and(|h| h.starts_with("gpu")) {
                *is_gpu = true;
            }
            Kind::from_sensor_id(id)
        }
        None => group,
    };
    if kind == Kind::Other {
        return;
    }
    if let Some(value) = node.get("Value").and_then(Value::as_str) {
        out.push((kind, text.to_owned(), value.to_owned()));
    }
}

fn gpu(name: &str, readings: &[Reading]) -> Gpu {
    // Primeiro nome da lista que tiver leitura.
    let pick = |kind: Kind, names: &[&str]| -> Option<&str> {
        names.iter().find_map(|name| {
            readings
                .iter()
                .find(|(k, text, value)| {
                    *k == kind && text.eq_ignore_ascii_case(name) && parse_value(value).is_some()
                })
                .map(|(_, _, value)| value.as_str())
        })
    };
    let number = |kind, names| pick(kind, names).and_then(parse_value);
    let bytes = |names| pick(Kind::Data, names).and_then(parse_bytes);

    let memory_used = bytes(&["GPU Memory Used", "D3D Dedicated Memory Used"]);
    let memory_total = bytes(&["GPU Memory Total", "D3D Dedicated Memory Total"]).or_else(|| {
        // Algumas versões só mostram usada e livre.
        Some(memory_used? + bytes(&["GPU Memory Free", "D3D Dedicated Memory Free"])?)
    });

    Gpu {
        name: name.to_owned(),
        usage: number(Kind::Load, &["GPU Core", "D3D 3D"]).unwrap_or(0.0),
        memory_used: memory_used.unwrap_or(0),
        memory_total: memory_total.unwrap_or(0),
        temperature_c: number(Kind::Temperature, &["GPU Core", "GPU Hot Spot"]),
        power_w: number(
            Kind::Power,
            &["GPU Package", "GPU PPT", "GPU Core", "GPU Power"],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lhm::tests::{sensor, serve_once};
    use serde_json::json;

    const MB: u64 = 1024 * 1024;

    /// Árvore de um PC como o do autor: Ryzen 5 3600 + RX 6700 XT, formato novo, pt-BR.
    fn ryzen_radeon() -> Value {
        json!({"Text": "Sensor", "Children": [{"Text": "DESKTOP-AQ16PGV", "Children": [
            {"Text": "AMD Ryzen 5 3600", "ImageURL": "images_icon/cpu.png", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("Core (Tctl/Tdie)", "55,0 °C", Some("/amdcpu/0/temperature/2")),
                ]},
            ]},
            {"Text": "AMD Radeon RX 6700 XT", "ImageURL": "images_icon/ati.png", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("GPU Core", "48,0 °C", Some("/gpu-amd/0/temperature/0")),
                    sensor("GPU Hot Spot", "60,0 °C", Some("/gpu-amd/0/temperature/2")),
                ]},
                {"Text": "Load", "Children": [
                    sensor("GPU Core", "23,0 %", Some("/gpu-amd/0/load/0")),
                    sensor("D3D 3D", "20,0 %", Some("/gpu-amd/0/load/5")),
                ]},
                {"Text": "Powers", "Children": [
                    sensor("GPU Package", "41,0 W", Some("/gpu-amd/0/power/2")),
                ]},
                {"Text": "Clocks", "Children": [
                    sensor("GPU Core", "2450,0 MHz", Some("/gpu-amd/0/clock/0")),
                ]},
                {"Text": "Data", "Children": [
                    sensor("GPU Memory Used", "2048 MB", Some("/gpu-amd/0/smalldata/0")),
                    sensor("GPU Memory Total", "12272 MB", Some("/gpu-amd/0/smalldata/2")),
                ]},
            ]},
        ]}]})
    }

    #[test]
    fn le_a_radeon() {
        let list = gpus(&ryzen_radeon());
        assert_eq!(list.len(), 1, "a CPU não entra na lista");
        let g = &list[0];
        assert_eq!(g.name, "AMD Radeon RX 6700 XT");
        assert!((g.usage - 23.0).abs() < f32::EPSILON);
        assert_eq!(g.memory_used, 2048 * MB);
        assert_eq!(g.memory_total, 12272 * MB);
        assert_eq!(g.temperature_c, Some(48.0));
        assert_eq!(g.power_w, Some(41.0));
    }

    /// Formato antigo (sem `SensorId`), só com usada e livre, e sensores sem leitura.
    #[test]
    fn formato_antigo_soma_usada_e_livre() {
        let tree = json!({"Children": [{"Children": [
            {"Text": "Radeon RX 580", "ImageURL": "images_icon/ati.png", "Children": [
                {"Text": "Temperatures", "Children": [
                    sensor("GPU Core", "-", None),
                    sensor("GPU Hot Spot", "70.0 °C", None),
                ]},
                {"Text": "Load", "Children": [sensor("D3D 3D", "5.0 %", None)]},
                {"Text": "SmallData", "Children": [
                    sensor("D3D Dedicated Memory Used", "1024 MB", None),
                    sensor("D3D Dedicated Memory Free", "7168 MB", None),
                ]},
            ]},
        ]}]});
        let g = &gpus(&tree)[0];
        assert!((g.usage - 5.0).abs() < f32::EPSILON);
        assert_eq!(g.memory_used, 1024 * MB);
        assert_eq!(g.memory_total, 8192 * MB);
        assert_eq!(g.temperature_c, Some(70.0));
        assert_eq!(g.power_w, None);
    }

    #[test]
    fn varias_gpus_na_ordem() {
        let tree = json!({"Children": [{"Children": [
            {"Text": "Intel UHD Graphics 770", "Children": [
                {"Text": "Load", "Children": [sensor("D3D 3D", "1 %", Some("/gpu-intel-integrated/0/load/0"))]},
            ]},
            {"Text": "AMD Radeon RX 7900 XT", "Children": [
                {"Text": "Load", "Children": [sensor("GPU Core", "90 %", Some("/gpu-amd/0/load/0"))]},
            ]},
            {"Text": "Generic Memory", "ImageURL": "images_icon/ram.png", "Children": [
                {"Text": "Load", "Children": [sensor("Memory", "40 %", Some("/ram/load/0"))]},
            ]},
        ]}]});
        let names: Vec<String> = gpus(&tree).into_iter().map(|g| g.name).collect();
        assert_eq!(names, ["Intel UHD Graphics 770", "AMD Radeon RX 7900 XT"]);
    }

    #[tokio::test]
    async fn le_de_um_servidor_de_verdade() {
        let addr = serve_once(ryzen_radeon().to_string()).await;
        let list = read(addr).await.unwrap();
        assert_eq!(list[0].name, "AMD Radeon RX 6700 XT");
    }

    #[tokio::test]
    async fn sem_gpu_na_arvore_fica_indisponivel() {
        let tree = json!({"Children": [{"Children": [
            {"Text": "AMD Ryzen 5 3600", "ImageURL": "images_icon/cpu.png", "Children": [
                {"Text": "Temperatures", "Children": [sensor("Tctl", "50 °C", None)]},
            ]},
        ]}]});
        let addr = serve_once(tree.to_string()).await;
        assert_eq!(read(addr).await.unwrap_err().code(), "unavailable");
    }
}
