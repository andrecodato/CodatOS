//! Fórmulas dos sensores do Ryzen (família 17h em diante), as mesmas do `k10temp` do Linux
//! e do `LibreHardwareMonitor`. Puras, para testar em qualquer SO.

use std::time::{Duration, Instant};

/// `THM_TCON_CUR_TMP` no SMN: temperatura atual em `[31:21]`, passos de 0,125 °C.
pub const SMN_THM_TCON_CUR_TMP: u32 = 0x0005_9800;
/// Unidade de energia em `[12:8]` (ESU): cada incremento vale `0,5^ESU` J.
pub const MSR_PWR_UNIT: u32 = 0xC001_0299;
/// Energia acumulada do pacote, contador de 32 bits.
pub const MSR_PKG_ENERGY_STAT: u32 = 0xC001_029B;

const RANGE_SEL: u32 = 1 << 19;
const TJ_SEL: u32 = 0b11 << 16;

/// Tctl em °C a partir do registrador. Com `RANGE_SEL` (ou `TJ_SEL` = 11, nos Zen novos) a
/// escala começa em −49 °C.
#[must_use]
pub fn tctl(raw: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // 11 bits cabem num f32
    let mut t = ((raw >> 21) * 125) as f32 / 1000.0;
    if raw & RANGE_SEL != 0 || raw & TJ_SEL == TJ_SEL {
        t -= 49.0;
    }
    t
}

/// Diferença entre Tctl e Tdie nos modelos que somam um desvio ao Tctl (tabela do
/// `k10temp`). Os demais, como o Ryzen 5 3600, já entregam Tdie.
#[must_use]
pub fn tdie_offset(brand: &str) -> f32 {
    if ["1600X", "1700X", "1800X"]
        .iter()
        .any(|m| brand.contains(m))
    {
        -20.0
    } else if ["Threadripper 19", "Threadripper 29"]
        .iter()
        .any(|m| brand.contains(m))
    {
        -27.0
    } else if brand.contains("2700X") {
        -10.0
    } else {
        0.0
    }
}

/// Joules por incremento do contador de energia.
#[must_use]
pub fn energy_unit(pwr_unit: u64) -> f64 {
    #[allow(clippy::cast_possible_truncation)] // 5 bits
    let esu = ((pwr_unit >> 8) & 0x1F) as i32;
    0.5_f64.powi(esu)
}

/// Potência do pacote a partir de leituras sucessivas do contador de energia.
#[derive(Debug, Clone)]
pub struct PowerMeter {
    unit: f64,
    last: Option<(u32, Instant)>,
    watts: Option<f32>,
}

impl PowerMeter {
    /// Leituras mais próximas que isso repetem o último valor (o contador anda em ~1 ms e
    /// dois clientes juntos dariam números ruidosos).
    const MIN_INTERVAL: Duration = Duration::from_millis(200);
    /// Acima disso o contador pode ter dado mais de uma volta: recomeça.
    const MAX_INTERVAL: Duration = Duration::from_mins(5);

    #[must_use]
    pub fn new(unit: f64) -> Self {
        Self {
            unit,
            last: None,
            watts: None,
        }
    }

    /// Registra uma leitura do contador e devolve a potência média desde a anterior.
    pub fn update(&mut self, energy: u32, now: Instant) -> Option<f32> {
        match self.last {
            Some((_, at)) if now.duration_since(at) < Self::MIN_INTERVAL => return self.watts,
            Some((prev, at)) if now.duration_since(at) <= Self::MAX_INTERVAL => {
                let joules = f64::from(energy.wrapping_sub(prev)) * self.unit;
                #[allow(clippy::cast_possible_truncation)]
                let watts = (joules / now.duration_since(at).as_secs_f64()) as f32;
                self.watts = Some(watts);
            }
            _ => self.watts = None,
        }
        self.last = Some((energy, now));
        self.watts
    }
}

/// Fabricante, família e nome da CPU, pelo `cpuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuId {
    pub vendor: String,
    pub family: u32,
    pub brand: String,
}

impl CpuId {
    /// Família efetiva a partir do `eax` da folha 1 (base + estendida quando base = 0xF).
    #[must_use]
    pub fn family_of(eax: u32) -> u32 {
        let base = (eax >> 8) & 0xF;
        if base == 0xF {
            base + ((eax >> 20) & 0xFF)
        } else {
            base
        }
    }

    /// Ryzen e EPYC (Zen em diante): o que o módulo `AMDFamily17` atende.
    #[must_use]
    pub fn supported(&self) -> bool {
        self.vendor == "AuthenticAMD" && self.family >= 0x17
    }

    #[cfg(target_arch = "x86_64")]
    #[must_use]
    pub fn current() -> Self {
        use std::arch::x86_64::__cpuid;

        let words =
            |regs: [u32; 4]| -> Vec<u8> { regs.iter().flat_map(|r| r.to_le_bytes()).collect() };
        let leaf0 = __cpuid(0);
        let vendor = words([leaf0.ebx, leaf0.edx, leaf0.ecx, 0]);
        let vendor = String::from_utf8_lossy(&vendor[..12]).into_owned();
        let family = Self::family_of(__cpuid(1).eax);
        let brand = if __cpuid(0x8000_0000).eax >= 0x8000_0004 {
            let bytes: Vec<u8> = (0x8000_0002..=0x8000_0004)
                .flat_map(|leaf| {
                    let r = __cpuid(leaf);
                    words([r.eax, r.ebx, r.ecx, r.edx])
                })
                .collect();
            String::from_utf8_lossy(&bytes)
                .trim_matches(char::from(0))
                .trim()
                .to_owned()
        } else {
            String::new()
        };
        Self {
            vendor,
            family,
            brand,
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    #[must_use]
    pub fn current() -> Self {
        Self {
            vendor: String::new(),
            family: 0,
            brand: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tctl_com_e_sem_desvio_de_49() {
        // 45 °C: 360 passos de 0,125 °C.
        assert!((tctl(360 << 21) - 45.0).abs() < f32::EPSILON);
        // Mesma leitura com RANGE_SEL: escala começa em −49 °C.
        assert!((tctl((752 << 21) | RANGE_SEL) - 45.0).abs() < f32::EPSILON);
        assert!((tctl((752 << 21) | TJ_SEL) - 45.0).abs() < f32::EPSILON);
        // Só um bit de TJ_SEL não muda a escala.
        assert!((tctl((360 << 21) | (1 << 16)) - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn desvio_do_tdie_por_modelo() {
        assert!((tdie_offset("AMD Ryzen 7 1700X Eight-Core Processor") + 20.0).abs() < 1e-6);
        assert!((tdie_offset("AMD Ryzen 7 2700X Eight-Core Processor") + 10.0).abs() < 1e-6);
        assert!((tdie_offset("AMD Ryzen Threadripper 2950X") + 27.0).abs() < 1e-6);
        assert!(tdie_offset("AMD Ryzen 5 3600 6-Core Processor").abs() < 1e-6);
    }

    #[test]
    fn unidade_de_energia() {
        // ESU = 16 (padrão do Zen): 15,3 µJ.
        assert!((energy_unit(0x000A_1003) - 1.0 / 65536.0).abs() < 1e-12);
    }

    #[test]
    fn potencia_entre_leituras_e_na_volta_do_contador() {
        let unit = 1.0 / 65536.0;
        let t0 = Instant::now();
        let mut meter = PowerMeter::new(unit);
        assert_eq!(meter.update(1000, t0), None);
        // 65 W durante 1 s.
        let w = meter.update(1000 + 65 * 65536, t0 + Duration::from_secs(1));
        assert!((w.unwrap() - 65.0).abs() < 1e-3);
        // Leitura colada na anterior repete o valor.
        assert_eq!(
            meter.update(0, t0 + Duration::from_millis(1050)),
            w,
            "não deveria recalcular"
        );
        // Contador de 32 bits dá a volta.
        let mut meter = PowerMeter::new(unit);
        meter.update(u32::MAX - 65535, t0);
        let w = meter.update(65536, t0 + Duration::from_secs(2)).unwrap();
        assert!((w - 1.0).abs() < 1e-3, "{w}");
        // Depois de muito tempo parado, recomeça.
        assert_eq!(meter.update(0, t0 + Duration::from_mins(10)), None);
    }

    #[test]
    fn familia_e_suporte() {
        // Ryzen 5 3600 (Zen 2): família 0xF + 0x8 = 0x17.
        assert_eq!(CpuId::family_of(0x0087_0F10), 0x17);
        // Intel Core: família 6.
        assert_eq!(CpuId::family_of(0x000A_0655), 6);
        let cpu = |vendor: &str, family| CpuId {
            vendor: vendor.into(),
            family,
            brand: String::new(),
        };
        assert!(cpu("AuthenticAMD", 0x19).supported());
        assert!(!cpu("AuthenticAMD", 0x15).supported());
        assert!(!cpu("GenuineIntel", 6).supported());
        let here = CpuId::current();
        assert_eq!(
            here.supported(),
            here.vendor == "AuthenticAMD" && here.family >= 0x17
        );
    }
}
