//! Volumes montados via `sysinfo` (RF-SYS-010).

use std::time::Duration;

use codatos_core::{CollectFuture, Collector, Intervals};
use codatos_protocol::system::{Disk, DiskKind, TOPIC_DISKS};
use sysinfo::Disks;

use crate::blocking;

/// Sistemas de arquivos que não são discos de verdade (Linux/macOS).
const PSEUDO_FS: &[&str] = &[
    "tmpfs",
    "devtmpfs",
    "overlay",
    "squashfs",
    "proc",
    "sysfs",
    "devfs",
    "autofs",
    "ramfs",
    "efivarfs",
    "nsfs",
    "fuse.snapfuse",
];

/// `disks`: a lista é relida a cada coleta, para pegar pendrives e discos novos.
#[derive(Debug, Default)]
pub struct DisksCollector;

impl Collector for DisksCollector {
    fn topic(&self) -> &'static str {
        TOPIC_DISKS
    }

    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_secs(10),
            min: Duration::from_secs(2),
        }
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(blocking::run(|| {
            let disks = Disks::new_with_refreshed_list();
            Ok(volumes(disks.list().iter().map(|d| Disk {
                mount_point: d.mount_point().to_string_lossy().into_owned(),
                name: d.name().to_string_lossy().into_owned(),
                file_system: d.file_system().to_string_lossy().into_owned(),
                kind: match d.kind() {
                    sysinfo::DiskKind::SSD => DiskKind::Ssd,
                    sysinfo::DiskKind::HDD => DiskKind::Hdd,
                    sysinfo::DiskKind::Unknown(_) => DiskKind::Unknown,
                },
                removable: d.is_removable(),
                total: d.total_space(),
                available: d.available_space(),
            })))
        }))
    }
}

/// Tira pseudo-sistemas de arquivos, volumes vazios e montagens repetidas do mesmo
/// dispositivo (snaps, bind mounts), e ordena pelo ponto de montagem. No Windows o nome é
/// o rótulo do volume, que pode se repetir: lá não há deduplicação.
fn volumes(disks: impl Iterator<Item = Disk>) -> Vec<Disk> {
    let mut list: Vec<Disk> = disks
        .filter(|d| d.total > 0 && !PSEUDO_FS.contains(&d.file_system.as_str()))
        .collect();
    list.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    let mut seen = std::collections::HashSet::new();
    // O mais curto (ex.: `/` antes de `/var/snap/...`) fica, por causa da ordenação.
    list.retain(|d| !d.name.starts_with("/dev/") || seen.insert(d.name.clone()));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(mount: &str, name: &str, fs: &str, total: u64) -> Disk {
        Disk {
            mount_point: mount.to_owned(),
            name: name.to_owned(),
            file_system: fs.to_owned(),
            kind: DiskKind::Ssd,
            removable: false,
            total,
            available: total / 2,
        }
    }

    #[test]
    fn tira_pseudo_fs_vazios_e_repetidos() {
        let list = volumes(
            [
                disk("/var/snap/x", "/dev/nvme0n1p2", "ext4", 100),
                disk("/", "/dev/nvme0n1p2", "ext4", 100),
                disk("/run", "tmpfs", "tmpfs", 10),
                disk("/boot/efi", "/dev/nvme0n1p1", "vfat", 1),
                disk("/mnt/x", "x", "ext4", 0),
                disk("C:\\", "", "NTFS", 500),
                disk("D:\\", "Dados", "NTFS", 500),
                disk("E:\\", "Dados", "NTFS", 500),
            ]
            .into_iter(),
        );
        let mounts: Vec<_> = list.iter().map(|d| d.mount_point.as_str()).collect();
        assert_eq!(mounts, ["/", "/boot/efi", "C:\\", "D:\\", "E:\\"]);
    }
}
