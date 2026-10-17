//! Dashboards, widgets e preferências (`docs/modulos/dashboard.md`, `docs/05-modelo-de-dados.md`).
//!
//! O layout é configuração (sincroniza no M4); os dados exibidos ficam no PC.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Métodos JSON-RPC.
pub const METHOD_DASHBOARDS_LIST: &str = "dashboards.list";
pub const METHOD_DASHBOARDS_GET: &str = "dashboards.get";
pub const METHOD_DASHBOARDS_SAVE: &str = "dashboards.save";
pub const METHOD_DASHBOARDS_DELETE: &str = "dashboards.delete";
pub const METHOD_SETTINGS_GET: &str = "settings.get";
pub const METHOD_SETTINGS_SET: &str = "settings.set";

/// Preferência `global` com o nome da saudação (RF-DASH-008).
pub const SETTING_PROFILE_NAME: &str = "profile.name";

/// Item de `dashboards.list`, em ordem de `position`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub id: String,
    pub name: String,
    pub position: i64,
}

/// Resultado de `dashboards.get` e parâmetro de `dashboards.save`. Salvar troca todos os
/// widgets do dashboard de uma vez (sair do modo edição salva o layout inteiro).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    /// UUID v7 gerado pelo cliente.
    pub id: String,
    pub name: String,
    pub position: i64,
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    /// UUID v7 gerado pelo cliente.
    pub id: String,
    /// Tipo do catálogo, ex.: `system.usage`, `disks`, `weather`.
    pub kind: String,
    /// Posição e tamanho por breakpoint, ex.: `{"lg": {"x": 0, "y": 0, "w": 3, "h": 2}}`.
    pub layout: Value,
    /// Configuração da instância (RF-DASH-004).
    pub config: Value,
}

/// Parâmetros de `dashboards.get` e `dashboards.delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardId {
    pub id: String,
}

/// Parâmetros de `settings.get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingKey {
    pub key: String,
    #[serde(default)]
    pub scope: SettingScope,
}

/// Parâmetros de `settings.set`. `value: null` apaga a preferência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetSetting {
    pub key: String,
    pub value: Value,
    #[serde(default)]
    pub scope: SettingScope,
}

/// `global` sincroniza entre dispositivos (M4); `device` fica neste PC.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingScope {
    #[default]
    Global,
    Device,
}
