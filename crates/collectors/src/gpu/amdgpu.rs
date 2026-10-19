//! GPUs AMD no Linux, pelo driver `amdgpu` no sysfs (`/sys/class/drm/card*/device`).
//!
//! Tudo é texto: `gpu_busy_percent`, `mem_info_vram_used`/`_total` em bytes e, em `hwmon`,
//! `temp1_input` em miligraus e `power1_average` em microwatts.

use std::fs;
use std::path::Path;

use codatos_protocol::system::Gpu;

/// Fabricante AMD no PCI.
const AMD_VENDOR: &str = "0x1002";

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) fn gpus() -> Vec<Gpu> {
    read(Path::new("/sys/class/drm"))
}

fn read(drm: &Path) -> Vec<Gpu> {
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<_> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        // `card0`, mas não `card0-DP-1` (conectores).
        .filter(|n| {
            n.strip_prefix("card")
                .is_some_and(|i| i.parse::<u32>().is_ok())
        })
        .collect();
    cards.sort();
    cards
        .into_iter()
        .filter_map(|card| gpu(&card, &drm.join(&card).join("device")))
        .collect()
}

fn gpu(card: &str, device: &Path) -> Option<Gpu> {
    let text = |file: &str| fs::read_to_string(device.join(file)).ok();
    let number = |file: &str| text(file)?.trim().parse::<u64>().ok();
    if text("vendor")?.trim() != AMD_VENDOR {
        return None;
    }
    let hwmon = fs::read_dir(device.join("hwmon"))
        .ok()?
        .flatten()
        .next()
        .map(|e| e.path());
    let hwmon_number = |file: &str| {
        let path = hwmon.as_ref()?.join(file);
        fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
    };
    #[allow(clippy::cast_precision_loss)]
    Some(Gpu {
        name: text("product_name")
            .map(|n| n.trim().to_owned())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("GPU AMD ({card})")),
        usage: number("gpu_busy_percent").map_or(0.0, |p| p as f32),
        memory_used: number("mem_info_vram_used").unwrap_or(0),
        memory_total: number("mem_info_vram_total").unwrap_or(0),
        temperature_c: hwmon_number("temp1_input").map(|m| m as f32 / 1000.0),
        power_w: hwmon_number("power1_average")
            .or_else(|| hwmon_number("power1_input"))
            .map(|uw| uw as f32 / 1_000_000.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn le_o_sysfs() {
        let root = std::env::temp_dir().join(format!("codatos-amdgpu-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let card = root.join("card1/device");
        write(&card.join("vendor"), "0x1002\n");
        write(&card.join("gpu_busy_percent"), "37\n");
        write(&card.join("mem_info_vram_used"), "1073741824\n");
        write(&card.join("mem_info_vram_total"), "12868124672\n");
        write(&card.join("hwmon/hwmon3/temp1_input"), "51000\n");
        write(&card.join("hwmon/hwmon3/power1_average"), "45000000\n");
        // Conector e GPU de outro fabricante ficam de fora.
        write(&root.join("card1-DP-1/status"), "connected\n");
        write(&root.join("card0/device/vendor"), "0x10de\n");

        let list = read(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(list.len(), 1);
        let g = &list[0];
        assert_eq!(g.name, "GPU AMD (card1)");
        assert!((g.usage - 37.0).abs() < f32::EPSILON);
        assert_eq!(g.memory_used, 1 << 30);
        assert_eq!(g.memory_total, 12_868_124_672);
        assert_eq!(g.temperature_c, Some(51.0));
        assert_eq!(g.power_w, Some(45.0));
    }

    #[test]
    fn sem_drm_devolve_vazio() {
        assert_eq!(read(Path::new("/caminho/que/nao/existe")).len(), 0);
    }
}
