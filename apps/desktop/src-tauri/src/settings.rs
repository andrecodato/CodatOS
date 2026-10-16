//! Preferências do M0 em `settings.json` na pasta de config do app.
//! O SQLite chega no M1; até lá, só o que o shell precisa.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE: &str = "settings.json";
const MAX_AUTOSTART_DELAY_SECS: u64 = 60;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// O autostart já foi ligado na primeira execução (depois, quem manda é o usuário).
    pub autostart_configured: bool,
    /// Espera antes de subir o núcleo quando o SO inicia o app (0–60 s).
    pub autostart_delay_secs: u64,
}

impl Settings {
    pub fn autostart_delay(&self) -> Duration {
        Duration::from_secs(self.autostart_delay_secs.min(MAX_AUTOSTART_DELAY_SECS))
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

/// Arquivo ausente ou inválido vira o padrão (sem travar a abertura do app).
pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) {
    let Some(path) = path(app) else { return };
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)
    };
    if let Err(error) = write() {
        tracing::warn!(%error, "não consegui salvar as preferências");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atraso_fica_entre_0_e_60_s() {
        let mut s = Settings::default();
        assert_eq!(s.autostart_delay(), Duration::ZERO);
        s.autostart_delay_secs = 600;
        assert_eq!(s.autostart_delay(), Duration::from_secs(60));
    }

    #[test]
    fn campos_ausentes_usam_o_padrao() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings::default());
    }
}
