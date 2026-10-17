//! Clima pelo Open-Meteo (Q-30: gratuito para uso não comercial; revisar antes do M4).
//! O widget mostra a atribuição "Open-Meteo" (licença CC BY 4.0 dos dados).

use std::time::Duration;

use codatos_core::{
    CollectError, CollectFuture, Collector, Intervals, Method, MethodError, MethodFuture, params,
};
use codatos_protocol::dashboard::SettingScope;
use codatos_protocol::weather::{
    DailyForecast, METHOD_WEATHER_SEARCH, SETTING_WEATHER, TOPIC_WEATHER, Weather, WeatherLocation,
    WeatherSearch,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{SettingsReader, http};

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";

/// `weather`. A cidade vem da preferência `weather.location` (escopo `global`).
#[derive(Default)]
pub struct WeatherCollector {
    settings: Option<SettingsReader>,
}

impl std::fmt::Debug for WeatherCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeatherCollector").finish_non_exhaustive()
    }
}

impl WeatherCollector {
    pub fn new(settings: Option<SettingsReader>) -> Self {
        Self { settings }
    }
}

impl Collector for WeatherCollector {
    fn topic(&self) -> &'static str {
        TOPIC_WEATHER
    }

    /// O tempo muda devagar e a API é gratuita: sem martelar.
    fn intervals(&self) -> Intervals {
        Intervals {
            default: Duration::from_mins(15),
            min: Duration::from_mins(5),
        }
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(15)
    }

    fn collect(&self) -> CollectFuture<'_> {
        Box::pin(async move {
            let location: Option<WeatherLocation> = match &self.settings {
                Some(settings) => settings(SETTING_WEATHER, SettingScope::Global)
                    .await
                    .and_then(|v| serde_json::from_value(v).ok()),
                None => None,
            };
            let location = location.ok_or_else(|| {
                CollectError::new("weather_not_configured", "Escolha uma cidade para o clima")
            })?;
            let unreachable =
                |_| CollectError::new("weather_unreachable", "Não consegui falar com o Open-Meteo");
            let body: Value = http::client()
                .get(FORECAST_URL)
                .query(&forecast_query(&location))
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(unreachable)?
                .json()
                .await
                .map_err(unreachable)?;
            let weather = parse_forecast(&location.name, body).ok_or_else(|| {
                CollectError::new("weather_invalid", "Resposta inesperada do Open-Meteo")
            })?;
            Ok(serde_json::to_value(weather).expect("Weather sempre serializa"))
        })
    }
}

fn forecast_query(location: &WeatherLocation) -> Vec<(&'static str, String)> {
    vec![
        ("latitude", location.latitude.to_string()),
        ("longitude", location.longitude.to_string()),
        (
            "current",
            "temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,is_day"
                .to_owned(),
        ),
        ("daily", "temperature_2m_max,temperature_2m_min,weather_code".to_owned()),
        ("timezone", "auto".to_owned()),
        ("forecast_days", "3".to_owned()),
    ]
}

fn parse_forecast(name: &str, body: Value) -> Option<Weather> {
    #[derive(Deserialize)]
    struct Body {
        current: Current,
        daily: Daily,
    }
    #[derive(Deserialize)]
    struct Current {
        temperature_2m: f32,
        relative_humidity_2m: f32,
        apparent_temperature: f32,
        weather_code: u8,
        wind_speed_10m: f32,
        is_day: u8,
    }
    #[derive(Deserialize)]
    struct Daily {
        time: Vec<String>,
        temperature_2m_max: Vec<f32>,
        temperature_2m_min: Vec<f32>,
        weather_code: Vec<u8>,
    }
    let body: Body = serde_json::from_value(body).ok()?;
    let d = body.daily;
    let daily = d
        .time
        .into_iter()
        .zip(d.temperature_2m_min)
        .zip(d.temperature_2m_max)
        .zip(d.weather_code)
        .map(|(((date, min_c), max_c), code)| DailyForecast {
            date,
            min_c,
            max_c,
            code,
        })
        .collect();
    let c = body.current;
    Some(Weather {
        location: name.to_owned(),
        temperature_c: c.temperature_2m,
        apparent_c: c.apparent_temperature,
        humidity: c.relative_humidity_2m,
        wind_kmh: c.wind_speed_10m,
        code: c.weather_code,
        is_day: c.is_day == 1,
        daily,
    })
}

/// `weather.searchCity`: até 5 cidades com o nome, em português.
#[derive(Debug)]
pub(crate) struct SearchCity;

impl Method for SearchCity {
    fn name(&self) -> &'static str {
        METHOD_WEATHER_SEARCH
    }

    fn call(&self, value: Value) -> MethodFuture<'_> {
        Box::pin(async move {
            let p: WeatherSearch = params(value)?;
            let query = p.query.trim();
            if query.chars().count() < 2 {
                return Ok(Value::Array(Vec::new()));
            }
            let unreachable =
                |_| MethodError::new("weather_unreachable", "Não consegui falar com o Open-Meteo");
            let body: Value = http::client()
                .get(GEOCODING_URL)
                .query(&[
                    ("name", query),
                    ("count", "5"),
                    ("language", "pt"),
                    ("format", "json"),
                ])
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(unreachable)?
                .json()
                .await
                .map_err(unreachable)?;
            Ok(serde_json::to_value(parse_geocoding(body)).expect("lista sempre serializa"))
        })
    }
}

/// Sem resultados, a API omite `results`.
fn parse_geocoding(mut body: Value) -> Vec<WeatherLocation> {
    serde_json::from_value(body["results"].take()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn le_a_previsao() {
        let body = json!({
            "latitude": -23.55, "longitude": -46.63, "timezone": "America/Sao_Paulo",
            "current": {"time": "2026-10-17T10:00", "temperature_2m": 24.3, "relative_humidity_2m": 61,
                "apparent_temperature": 25.1, "weather_code": 2, "wind_speed_10m": 9.4, "is_day": 1},
            "daily": {"time": ["2026-10-17", "2026-10-18"], "temperature_2m_max": [28.0, 30.5],
                "temperature_2m_min": [17.2, 18.0], "weather_code": [2, 61]}
        });
        let w = parse_forecast("São Paulo", body).unwrap();
        assert_eq!(w.location, "São Paulo");
        assert!((w.temperature_c - 24.3).abs() < f32::EPSILON);
        assert!(w.is_day);
        assert_eq!(w.daily.len(), 2);
        assert_eq!(w.daily[1].code, 61);
        assert_eq!(w.daily[1].date, "2026-10-18");
        assert!(parse_forecast("x", json!({"error": true})).is_none());
    }

    #[test]
    fn le_a_busca_de_cidades() {
        let body = json!({"results": [
            {"id": 3_448_439, "name": "São Paulo", "latitude": -23.5475, "longitude": -46.63611,
             "country": "Brasil", "admin1": "São Paulo", "timezone": "America/Sao_Paulo"}
        ]});
        let list = parse_geocoding(body);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].country.as_deref(), Some("Brasil"));
        assert_eq!(
            parse_geocoding(json!({"generationtime_ms": 0.5})),
            Vec::new()
        );
    }

    #[test]
    fn parametros_da_previsao() {
        let q = forecast_query(&WeatherLocation {
            name: "x".to_owned(),
            latitude: -23.5,
            longitude: -46.6,
            admin1: None,
            country: None,
        });
        assert!(q.contains(&("latitude", "-23.5".to_owned())));
        assert!(q.contains(&("timezone", "auto".to_owned())));
    }

    #[tokio::test]
    async fn sem_cidade_responde_erro_claro() {
        let error = WeatherCollector::new(None).collect().await.unwrap_err();
        assert_eq!(error.code(), "weather_not_configured");
    }
}
