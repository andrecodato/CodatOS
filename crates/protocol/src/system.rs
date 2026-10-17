//! Tópicos e amostras do monitor do sistema (`docs/modulos/sistema.md`).

use serde::{Deserialize, Serialize};

/// Uso de CPU, memória e rede. Padrão: a cada 2 s.
pub const TOPIC_METRICS: &str = "system.metrics";
/// SO, máquina e CPU (RF-SYS-006). Muda pouco: a cada 60 s.
pub const TOPIC_INFO: &str = "system.info";
/// Volumes montados (RF-SYS-010). A cada 10 s.
pub const TOPIC_DISKS: &str = "disks";

/// Amostra de `system.metrics`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    pub cpu: CpuUsage,
    pub memory: Memory,
    pub network: Vec<NetworkInterface>,
}

/// Uso de CPU em % (0–100), total e por núcleo lógico (RF-SYS-001).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuUsage {
    pub total: f32,
    pub cores: Vec<f32>,
    /// Frequência média dos núcleos.
    pub frequency_mhz: u64,
}

/// Memória em bytes (RF-SYS-002). No Windows, `swap` é o arquivo de paginação.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub used: u64,
    pub total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

/// Taxa de uma interface desde a amostra anterior (RF-SYS-005). Só IPs locais.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    pub name: String,
    pub rx_bps: u64,
    pub tx_bps: u64,
    pub addresses: Vec<String>,
}

/// Amostra de `system.info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    /// Ex.: "Windows 11 Pro 23H2", "Ubuntu 24.04".
    pub os: String,
    pub kernel: Option<String>,
    pub hostname: Option<String>,
    pub arch: String,
    pub uptime_s: u64,
    pub cpu_model: String,
    pub logical_cores: usize,
    pub physical_cores: Option<usize>,
}

/// Item da amostra de `disks` (lista de volumes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    /// Letra no Windows (`C:\`), ponto de montagem nos outros.
    pub mount_point: String,
    pub name: String,
    pub file_system: String,
    pub kind: DiskKind,
    pub removable: bool,
    pub total: u64,
    pub available: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskKind {
    Ssd,
    Hdd,
    Unknown,
}
