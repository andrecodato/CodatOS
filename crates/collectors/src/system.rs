//! CPU, memória, rede e dados da máquina via `sysinfo` (RF-SYS-001/002/005/006).

use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use codatos_core::{CollectFuture, Collector, Intervals};
use codatos_protocol::system::{
    CpuUsage, Memory, NetworkInterface, SystemInfo, SystemMetrics, TOPIC_INFO, TOPIC_METRICS,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

use crate::blocking;

/// `system.metrics`. Guarda o estado do `sysinfo` entre coletas: o uso de CPU e as taxas
/// de rede são diferenças em relação à coleta anterior.
#[derive(Debug, Default)]
pub struct SystemMetricsCollector {
    state: Arc<Mutex<Option<MetricsState>>>,
}

#[derive(Debug)]
struct MetricsState {
    system: System,
    networks: Networks,
    last: Instant,
}

impl Collector for SystemMetricsCollector {
    fn topic(&self) -> &'static str {
        TOPIC_METRICS
    }

    fn collect(&self) -> CollectFuture<'_> {
        let state = Arc::clone(&self.state);
        Box::pin(blocking::run(move || {
            let mut guard = state.lock().unwrap_or_else(PoisonError::into_inner);
            let state = guard.get_or_insert_with(MetricsState::new);
            Ok(state.sample())
        }))
    }
}

impl MetricsState {
    /// A primeira leitura de CPU do `sysinfo` é sempre 0: lê duas vezes com o intervalo
    /// mínimo entre elas.
    fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(cpu_refresh())
                .with_memory(MemoryRefreshKind::everything()),
        );
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_specifics(cpu_refresh());
        Self {
            system,
            networks: Networks::new_with_refreshed_list(),
            last: Instant::now(),
        }
    }

    fn sample(&mut self) -> SystemMetrics {
        self.system.refresh_cpu_specifics(cpu_refresh());
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::everything());
        self.networks.refresh(true);
        let now = Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;

        let cpus = self.system.cpus();
        let frequency_mhz = if cpus.is_empty() {
            0
        } else {
            cpus.iter().map(sysinfo::Cpu::frequency).sum::<u64>() / cpus.len() as u64
        };

        let mut network: Vec<NetworkInterface> = self
            .networks
            .list()
            .iter()
            .filter_map(|(name, data)| {
                let addresses: Vec<String> = data
                    .ip_networks()
                    .iter()
                    .map(|n| n.addr)
                    .filter(|addr| is_local_address(*addr))
                    .map(|addr| addr.to_string())
                    .collect();
                // Loopback e interfaces sem endereço nem tráfego não interessam ao widget.
                if addresses.is_empty() && data.total_received() == 0 {
                    return None;
                }
                Some(NetworkInterface {
                    name: name.clone(),
                    rx_bps: per_second(data.received(), elapsed),
                    tx_bps: per_second(data.transmitted(), elapsed),
                    addresses,
                })
            })
            .collect();
        network.sort_by(|a, b| a.name.cmp(&b.name));

        SystemMetrics {
            cpu: CpuUsage {
                total: self.system.global_cpu_usage(),
                cores: cpus.iter().map(sysinfo::Cpu::cpu_usage).collect(),
                frequency_mhz,
            },
            memory: Memory {
                used: self.system.used_memory(),
                total: self.system.total_memory(),
                swap_used: self.system.used_swap(),
                swap_total: self.system.total_swap(),
            },
            network,
        }
    }
}

fn cpu_refresh() -> CpuRefreshKind {
    CpuRefreshKind::nothing().with_cpu_usage().with_frequency()
}

/// Bytes no intervalo → bytes por segundo.
fn per_second(bytes: u64, elapsed: Duration) -> u64 {
    let ms = elapsed.as_millis();
    if ms == 0 {
        return 0;
    }
    u64::try_from(u128::from(bytes) * 1000 / ms).unwrap_or(u64::MAX)
}

/// Endereços da rede local; loopback e IPv6 de enlace (`fe80::/10`) ficam de fora.
fn is_local_address(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_unspecified() && !v4.is_link_local(),
        IpAddr::V6(v6) => {
            !v6.is_loopback() && !v6.is_unspecified() && (v6.segments()[0] & 0xffc0) != 0xfe80
        }
    }
}

/// `system.info`: dados que quase não mudam, lidos a cada 60 s.
#[derive(Debug, Default)]
pub struct SystemInfoCollector;

impl Collector for SystemInfoCollector {
    fn topic(&self) -> &'static str {
        TOPIC_INFO
    }

    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_secs(60),
            min: Duration::from_secs(5),
        }
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(blocking::run(|| Ok(system_info())))
    }
}

fn system_info() -> SystemInfo {
    let system =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    let cpu_model = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim().to_owned())
        .unwrap_or_default();
    SystemInfo {
        os: System::long_os_version()
            .or_else(System::name)
            .unwrap_or_else(|| std::env::consts::OS.to_owned()),
        kernel: System::kernel_version(),
        hostname: System::host_name(),
        arch: System::cpu_arch(),
        uptime_s: System::uptime(),
        cpu_model,
        logical_cores: system.cpus().len(),
        physical_cores: System::physical_core_count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxa_por_segundo() {
        assert_eq!(per_second(1000, Duration::from_millis(500)), 2000);
        assert_eq!(per_second(1000, Duration::ZERO), 0);
        assert_eq!(per_second(0, Duration::from_secs(2)), 0);
    }

    #[test]
    fn so_enderecos_da_rede_local() {
        let ok = |s: &str| is_local_address(s.parse().unwrap());
        assert!(ok("192.168.0.10"));
        assert!(ok("2804:14c::1"));
        assert!(!ok("127.0.0.1"));
        assert!(!ok("::1"));
        assert!(!ok("169.254.10.1"));
        assert!(!ok("fe80::1"));
    }

    #[test]
    fn coleta_de_verdade() {
        let mut state = MetricsState::new();
        let m = state.sample();
        assert!(m.memory.total > 0);
        assert!(m.memory.used <= m.memory.total);
        assert!((0.0..=100.0).contains(&m.cpu.total));

        let info = system_info();
        assert!(info.logical_cores > 0);
        assert_eq!(m.cpu.cores.len(), info.logical_cores);
        assert_ne!(info.os, "");
    }
}
