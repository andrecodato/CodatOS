//! Clima pelo Open-Meteo (`docs/modulos/integracoes.md#clima`, Q-30).
//!
//! Unidades sempre métricas (°C, km/h); a UI converte se o usuário preferir.

use serde::{Deserialize, Serialize};

/// Tempo agora e previsão de 3 dias para a cidade em [`SETTING_WEATHER`].
pub const TOPIC_WEATHER: &str = "weather";
/// Preferência `global` com a [`WeatherLocation`] escolhida.
pub const SETTING_WEATHER: &str = "weather.location";
/// Busca de cidades pelo nome (geocodificação do Open-Meteo).
pub const METHOD_WEATHER_SEARCH: &str = "weather.searchCity";

/// Cidade escolhida (e item do resultado de `weather.searchCity`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherLocation {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Estado/região, ex.: "São Paulo".
    #[serde(default)]
    pub admin1: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
}

/// Parâmetros de `weather.searchCity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherSearch {
    pub query: String,
}

/// Amostra de `weather`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    pub location: String,
    pub temperature_c: f32,
    pub apparent_c: f32,
    /// Umidade relativa em %.
    pub humidity: f32,
    pub wind_kmh: f32,
    /// Código WMO do tempo (0 = céu limpo, 61 = chuva fraca…).
    pub code: u8,
    pub is_day: bool,
    pub daily: Vec<DailyForecast>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyForecast {
    /// `AAAA-MM-DD` no fuso da cidade.
    pub date: String,
    pub min_c: f32,
    pub max_c: f32,
    pub code: u8,
}
