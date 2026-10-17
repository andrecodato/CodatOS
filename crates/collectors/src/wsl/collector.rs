//! Coletor `wsl.distros` (Windows).

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codatos_core::{CollectError, CollectFuture, Collector, Intervals};
use codatos_protocol::system::{TOPIC_WSL, WslDistro, WslStatus, WslVm};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use super::parse;
use crate::command;

const LXSS: &str = r"Software\Microsoft\Windows\CurrentVersion\Lxss";

#[derive(Debug, Default)]
pub struct WslCollector {
    vm: Arc<Mutex<VmProcess>>,
}

/// Processo da VM no host. O PID fica guardado para não varrer todos os processos.
#[derive(Debug, Default)]
struct VmProcess {
    system: Option<System>,
    pid: Option<Pid>,
}

/// Distro como está no Registro.
pub(crate) struct Registered {
    pub(crate) name: String,
    version: u8,
    default: bool,
    vhdx: Option<PathBuf>,
}

impl Collector for WslCollector {
    fn topic(&self) -> &'static str {
        TOPIC_WSL
    }

    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_secs(5),
            min: Duration::from_secs(2),
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(15)
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async move {
            let registered = tokio::task::spawn_blocking(registered)
                .await
                .map_err(|_| CollectError::new("collector_panicked", "O coletor falhou"))??;
            let running = parse::running_names(
                &wsl(&["--list", "--running", "--quiet"])
                    .await
                    .unwrap_or_default(),
            );

            let mut distros = Vec::with_capacity(registered.len());
            for r in registered {
                let is_running = running.iter().any(|n| n.eq_ignore_ascii_case(&r.name));
                let vhdx_bytes = r
                    .vhdx
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| m.len());
                // RF-SYS-024: `wsl -d` liga a distro; só nas que estão ligadas.
                let disk = if is_running {
                    wsl(&["-d", &r.name, "-e", "df", "-B1", "--output=used,size", "/"])
                        .await
                        .as_deref()
                        .and_then(parse::df_used_total)
                } else {
                    None
                };
                distros.push(WslDistro {
                    name: r.name,
                    version: r.version,
                    default: r.default,
                    running: is_running,
                    vhdx_bytes,
                    disk_used: disk.map(|d| d.0),
                    disk_total: disk.map(|d| d.1),
                });
            }

            let vm = Arc::clone(&self.vm);
            let vm = tokio::task::spawn_blocking(move || {
                vm.lock().unwrap_or_else(PoisonError::into_inner).sample()
            })
            .await
            .ok()
            .flatten();

            Ok(
                serde_json::to_value(WslStatus { distros, vm })
                    .expect("WslStatus sempre serializa"),
            )
        })
    }
}

pub(crate) fn registered() -> Result<Vec<Registered>, CollectError> {
    let not_installed = || CollectError::unavailable("O WSL não está instalado");
    let lxss = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(LXSS)
        .map_err(|_| not_installed())?;
    let default: String = lxss.get_value("DefaultDistribution").unwrap_or_default();

    let mut list = Vec::new();
    for guid in lxss.enum_keys().flatten() {
        let Ok(key) = lxss.open_subkey(&guid) else {
            continue;
        };
        let Ok(name) = key.get_value::<String, _>("DistributionName") else {
            continue;
        };
        let version: u32 = key.get_value("Version").unwrap_or(2);
        let base: Option<String> = key.get_value("BasePath").ok();
        let file: String = key
            .get_value("VhdFileName")
            .unwrap_or_else(|_| "ext4.vhdx".to_owned());
        list.push(Registered {
            name,
            version: if version == 1 { 1 } else { 2 },
            default: guid.eq_ignore_ascii_case(&default),
            vhdx: base
                .filter(|_| version != 1)
                .map(|b| PathBuf::from(b.trim_start_matches(r"\\?\")).join(file)),
        });
    }
    if list.is_empty() {
        return Err(not_installed());
    }
    list.sort_by(|a, b| b.default.cmp(&a.default).then_with(|| a.name.cmp(&b.name)));
    Ok(list)
}

/// Roda o `wsl.exe`; `None` em qualquer falha.
async fn wsl(args: &[&str]) -> Option<String> {
    command::output("wsl.exe", args, command::TIMEOUT).await
}

/// Distros ligadas agora.
pub(crate) async fn running() -> Vec<String> {
    parse::running_names(
        &wsl(&["--list", "--running", "--quiet"])
            .await
            .unwrap_or_default(),
    )
}

impl VmProcess {
    fn sample(&mut self) -> Option<WslVm> {
        let system = self.system.get_or_insert_with(System::new);
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        let found = self.pid.is_some_and(|pid| {
            system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, refresh);
            system.process(pid).is_some()
        });
        if !found {
            system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
            self.pid = system
                .processes()
                .values()
                .find(|p| {
                    let name = p.name().to_string_lossy().to_ascii_lowercase();
                    name == "vmmemwsl" || name == "vmmem" || name == "vmmem.exe"
                })
                .map(sysinfo::Process::pid);
        }
        let process = system.process(self.pid?)?;
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        #[allow(clippy::cast_precision_loss)]
        Some(WslVm {
            memory: process.memory(),
            cpu: process.cpu_usage() / cores as f32,
        })
    }
}
