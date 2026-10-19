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

/// GPUs (RF-SYS-003): NVIDIA via NVML; AMD via ADL no Windows e `amdgpu` no Linux. Sem
/// nenhuma, o tópico responde `unavailable`.
pub const TOPIC_GPU: &str = "system.gpu";
/// Temperatura e potência da CPU (Q-13). No Windows vêm do serviço `codatos-sensors`, que lê
/// pelo driver PawnIO.
pub const TOPIC_CPU_SENSORS: &str = "system.cpuSensors";
/// Erro de `system.cpuSensors` quando o serviço não está instalado: o widget oferece
/// [`METHOD_ENABLE_CPU_SENSORS`].
pub const CPU_SENSORS_NOT_INSTALLED: &str = "cpu_sensors_not_installed";
/// Instala o PawnIO e o serviço `codatos-sensors` (um pedido de administrador do Windows).
/// Sem parâmetros; devolve `null` quando o serviço já responde. Só existe no Windows.
pub const METHOD_ENABLE_CPU_SENSORS: &str = "system.enableCpuSensors";

/// Item da amostra de `system.gpu`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    /// Uso em % (0–100).
    pub usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature_c: Option<f32>,
    pub power_w: Option<f32>,
}

/// Amostra de `system.cpuSensors`. Valores ausentes aparecem como "indisponível".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSensors {
    pub temperature_c: Option<f32>,
    pub power_w: Option<f32>,
    pub source: SensorSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SensorSource {
    /// Driver PawnIO, pelo serviço `codatos-sensors` (Windows).
    PawnIo,
    /// Sensores do próprio SO (hwmon no Linux, SMC no macOS).
    Os,
}

/// Distros WSL (RF-SYS-020 a 022). Só existe no Windows.
pub const TOPIC_WSL: &str = "wsl.distros";

/// Ações do WSL (Q-14: ligar e desligar no M1). Só existem no Windows.
pub const METHOD_WSL_START: &str = "wsl.start";
pub const METHOD_WSL_TERMINATE: &str = "wsl.terminate";
/// Desliga a VM inteira (`wsl --shutdown`), com todas as distros.
pub const METHOD_WSL_SHUTDOWN: &str = "wsl.shutdown";
pub const METHOD_WSL_SET_DEFAULT: &str = "wsl.setDefault";

/// Parâmetros de `wsl.start`, `wsl.terminate` e `wsl.setDefault`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslDistroName {
    pub distro: String,
}

/// Amostra de `wsl.distros`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslStatus {
    pub distros: Vec<WslDistro>,
    /// Memória e CPU da VM do WSL 2 (processo `vmmemWSL` no host), quando ela está ligada.
    pub vm: Option<WslVm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslDistro {
    pub name: String,
    /// 1 ou 2.
    pub version: u8,
    pub default: bool,
    pub running: bool,
    /// Tamanho do `ext4.vhdx` no host (WSL 2), com a distro ligada ou não.
    pub vhdx_bytes: Option<u64>,
    /// Uso de `/` dentro da distro; só para distros em execução (RF-SYS-024).
    pub disk_used: Option<u64>,
    pub disk_total: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslVm {
    pub memory: u64,
    /// % do total da máquina (0–100), como no Gerenciador de Tarefas.
    pub cpu: f32,
}

/// Containers de cada engine Docker: o do host e os de dentro das distros WSL ligadas
/// (RF-SYS-030 a 032).
pub const TOPIC_DOCKER: &str = "docker.containers";

/// Amostra de `docker.containers`. Cada engine aparece uma vez, mesmo visto do host e do
/// WSL (Docker Desktop com integração WSL): a chave é o ID do engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub engines: Vec<DockerEngine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerEngine {
    /// ID do `docker info`.
    pub id: String,
    /// Ex.: "docker-desktop", ou o hostname da distro.
    pub name: String,
    pub version: String,
    /// Distro WSL onde o engine roda; `None` para o do host.
    pub wsl_distro: Option<String>,
    pub containers: Vec<Container>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    /// `running`, `exited`, `paused`…
    pub state: String,
    /// Texto do Docker, ex.: "Up 2 hours".
    pub status: String,
    pub ports: String,
    /// Projeto Compose (`com.docker.compose.project`), para agrupar.
    pub compose_project: Option<String>,
    /// Só para containers em execução.
    pub cpu: Option<f32>,
    pub memory: Option<u64>,
}
