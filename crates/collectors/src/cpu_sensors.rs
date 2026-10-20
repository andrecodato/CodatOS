//! Temperatura e potência da CPU (Q-13).
//!
//! No Windows não há leitura sem driver: os valores vêm do serviço `codatos-sensors`, que
//! lê pelo driver PawnIO. Sem o serviço, o widget oferece
//! `system.enableCpuSensors`. Nos outros SOs, dos sensores do sistema (hwmon, SMC) via
//! `sysinfo`.

use codatos_core::{CollectFuture, Collector};
use codatos_protocol::system::TOPIC_CPU_SENSORS;

#[cfg(windows)]
mod enable;

#[cfg(windows)]
pub(crate) use enable::EnableCpuSensors;

/// `system.cpuSensors`.
#[derive(Debug, Default)]
pub struct CpuSensorsCollector;

impl Collector for CpuSensorsCollector {
    fn topic(&self) -> &'static str {
        TOPIC_CPU_SENSORS
    }

    #[cfg(windows)]
    fn collect(&self) -> CollectFuture<'_> {
        use codatos_core::CollectError;
        use codatos_protocol::system::{CpuSensors, SensorSource};
        use codatos_sensors::Reply;

        Box::pin(async {
            match codatos_sensors::client::read().await {
                Reply::Ok {
                    temperature_c,
                    power_w,
                } => Ok(serde_json::to_value(CpuSensors {
                    temperature_c,
                    power_w,
                    source: SensorSource::PawnIo,
                })
                .expect("CpuSensors sempre serializa")),
                Reply::Error { code, title } => Err(CollectError::new(code, title)),
            }
        })
    }

    #[cfg(not(windows))]
    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(crate::blocking::run(os::read))
    }
}

#[cfg(not(windows))]
mod os {
    use codatos_core::CollectError;
    use codatos_protocol::system::{CpuSensors, SensorSource};
    use sysinfo::Components;

    pub(super) fn read() -> Result<CpuSensors, CollectError> {
        let components = Components::new_with_refreshed_list();
        let readings = components
            .list()
            .iter()
            .filter_map(|c| Some((c.label(), c.temperature()?)));
        let temperature_c = pick_cpu_temperature(readings).ok_or_else(|| {
            CollectError::unavailable("Sensor de temperatura da CPU não encontrado")
        })?;
        Ok(CpuSensors {
            temperature_c: Some(temperature_c),
            // RAPL exige root na maioria das distros; fica para depois.
            power_w: None,
            source: SensorSource::Os,
        })
    }

    /// Prefere o sensor do pacote (`Package id 0`, `Tctl`, `Tdie`); senão, o núcleo mais quente.
    pub(super) fn pick_cpu_temperature<'a>(
        readings: impl Iterator<Item = (&'a str, f32)>,
    ) -> Option<f32> {
        let mut package = None;
        let mut hottest_core: Option<f32> = None;
        for (label, temperature) in readings {
            let label = label.to_ascii_lowercase();
            if ["package", "tctl", "tdie"]
                .iter()
                .any(|k| label.contains(k))
            {
                package.get_or_insert(temperature);
            } else if label.contains("core") || label.contains("cpu") {
                hottest_core = Some(hottest_core.map_or(temperature, |t| t.max(temperature)));
            }
        }
        package.or(hottest_core)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn prefere_o_pacote() {
            let r = [
                ("coretemp Core 0", 50.0),
                ("coretemp Package id 0", 55.0),
                ("nvme Composite", 40.0),
            ];
            assert_eq!(pick_cpu_temperature(r.into_iter()), Some(55.0));
            let r = [("k10temp Tctl", 61.5), ("amdgpu edge", 45.0)];
            assert_eq!(pick_cpu_temperature(r.into_iter()), Some(61.5));
        }

        #[test]
        fn sem_pacote_usa_o_nucleo_mais_quente() {
            let r = [("Core 0", 50.0), ("Core 1", 58.0), ("acpitz", 30.0)];
            assert_eq!(pick_cpu_temperature(r.into_iter()), Some(58.0));
            assert_eq!(pick_cpu_temperature([("acpitz", 30.0)].into_iter()), None);
        }
    }
}
