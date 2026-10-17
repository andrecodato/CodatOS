//! Números no formato pt-BR (vírgula decimal).

/// Bytes em unidades binárias, como o Gerenciador de Tarefas: "15,8 GB".
#[allow(clippy::cast_precision_loss)] // uma casa decimal basta
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        decimal(value, 1) + " " + UNITS[unit]
    }
}

/// Número com `places` casas e vírgula decimal.
pub fn decimal(value: f64, places: usize) -> String {
    format!("{value:.places$}").replace('.', ",")
}

/// Fração `part / total` em %, 0 quando `total` é 0.
#[allow(clippy::cast_precision_loss)]
pub fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_em_pt_br() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1,5 KB");
        assert_eq!(bytes(17_000_000_000), "15,8 GB");
    }

    #[test]
    fn percentual_sem_divisao_por_zero() {
        assert!((percent(1, 4) - 25.0).abs() < f64::EPSILON);
        assert!(percent(1, 0).abs() < f64::EPSILON);
    }
}
