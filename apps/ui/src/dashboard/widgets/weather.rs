//! Tempo: agora e próximos dias pelo Open-Meteo (`weather`, Q-30). A cidade é uma só para o
//! app todo (preferência global `weather.location`), escolhida aqui mesmo no widget.

use codatos_protocol::dashboard::{METHOD_SETTINGS_SET, SetSetting, SettingScope};
use codatos_protocol::weather::{
    DailyForecast, METHOD_WEATHER_SEARCH, SETTING_WEATHER, TOPIC_WEATHER, Weather, WeatherLocation,
    WeatherSearch,
};
use leptos::{ev, prelude::*, task::spawn_local};
use serde_json::Value;
use wasm_bindgen::JsValue;

use super::format::decimal;
use crate::api;
use crate::components::icon::{Icon, Svg};
use crate::dashboard::state::{Retry, sample_view};

const NOT_CONFIGURED: &str = "weather_not_configured";
const ATTRIBUTION_URL: &str = "https://open-meteo.com/";

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_WEATHER, None);
    let picking = RwSignal::new(false);
    let unset =
        Memo::new(move |_| matches!(sample.get(), Some(Err(ref p)) if p.code == NOT_CONFIGURED));
    (move || {
        if unset.get() || picking.get() {
            view! { <CityPicker picking=picking /> }.into_any()
        } else {
            sample_view(
                sample,
                move |w: Weather| view! { <Now weather=w picking=picking /> },
            )
        }
    })
    .into_any()
}

/// Descrição e ícone de um código WMO (tabela da documentação do Open-Meteo).
fn describe(code: u8, is_day: bool) -> (&'static str, Icon) {
    match code {
        0 => ("Céu limpo", if is_day { Icon::Sun } else { Icon::Moon }),
        1 => ("Quase limpo", if is_day { Icon::Sun } else { Icon::Moon }),
        2 => ("Parcialmente nublado", Icon::CloudSun),
        3 => ("Nublado", Icon::Cloud),
        45 | 48 => ("Neblina", Icon::CloudFog),
        51 | 53 | 55 | 56 | 57 => ("Garoa", Icon::CloudRain),
        61 | 66 | 80 => ("Chuva fraca", Icon::CloudRain),
        63 | 81 => ("Chuva", Icon::CloudRain),
        65 | 67 | 82 => ("Chuva forte", Icon::CloudRain),
        71 | 73 | 75 | 77 | 85 | 86 => ("Neve", Icon::CloudSnow),
        95 | 96 | 99 => ("Tempestade", Icon::CloudLightning),
        _ => ("Tempo indefinido", Icon::Cloud),
    }
}

/// "sáb." para `AAAA-MM-DD` (meio-dia local, para não cair no dia anterior pelo fuso).
fn weekday(date: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(&format!("{date}T12:00:00")));
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"weekday".into(), &"short".into());
    String::from(d.to_locale_date_string("pt-BR", &opts))
}

#[component]
fn Now(weather: Weather, picking: RwSignal<bool>) -> impl IntoView {
    let (label, icon) = describe(weather.code, weather.is_day);
    let days: Vec<DailyForecast> = weather.daily.into_iter().take(3).collect();
    view! {
        <div class="flex h-full flex-col gap-2">
            <div class="flex items-center gap-3">
                <span class="text-warning"><Svg icon=icon size=36 /></span>
                <div class="min-w-0">
                    <p class="text-3xl leading-none font-semibold tabular-nums">
                        {format!("{}°", decimal(f64::from(weather.temperature_c), 0))}
                    </p>
                    <p class="truncate text-xs text-muted">{label}</p>
                </div>
                <div class="ml-auto text-right text-xs text-subtle">
                    <p>{format!("Sensação {}°", decimal(f64::from(weather.apparent_c), 0))}</p>
                    <p>{format!("Umidade {}%", decimal(f64::from(weather.humidity), 0))}</p>
                    <p>{format!("Vento {} km/h", decimal(f64::from(weather.wind_kmh), 0))}</p>
                </div>
            </div>
            <ul class="grid min-h-0 flex-1 grid-cols-3 gap-2">
                {days
                    .into_iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let (label, icon) = describe(d.code, true);
                        let day = if i == 0 { "Hoje".to_owned() } else { weekday(&d.date) };
                        view! {
                            <li class="flex flex-col items-center justify-center rounded-md bg-card-hover/50 text-xs" title=label>
                                <span class="text-muted first-letter:uppercase">{day}</span>
                                <span class="my-1 text-subtle"><Svg icon=icon size=18 /></span>
                                <span class="tabular-nums">
                                    {format!("{}° ", decimal(f64::from(d.max_c), 0))}
                                    <span class="text-subtle">{format!("{}°", decimal(f64::from(d.min_c), 0))}</span>
                                </span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <div class="flex items-center gap-2 text-[11px] text-subtle">
                <button
                    type="button"
                    class="flex min-w-0 items-center gap-1 hover:text-fg"
                    title="Trocar cidade"
                    on:click=move |_| picking.set(true)
                >
                    <Svg icon=Icon::MapPin size=12 />
                    <span class="truncate">{weather.location}</span>
                </button>
                <Attribution />
            </div>
        </div>
    }
}

/// Crédito exigido pela licença dos dados (CC BY 4.0).
#[component]
fn Attribution() -> impl IntoView {
    view! {
        <a
            href=ATTRIBUTION_URL
            target="_blank"
            rel="noopener noreferrer"
            class="ml-auto shrink-0 hover:text-fg"
            on:click=|e: ev::MouseEvent| {
                e.prevent_default();
                api::open_external(ATTRIBUTION_URL);
            }
        >
            "Dados: Open-Meteo"
        </a>
    }
}

/// Troca de cidade dentro do widget: grava e recarrega.
#[component]
fn CityPicker(picking: RwSignal<bool>) -> impl IntoView {
    let retry = use_context::<Retry>();
    let saved = Callback::new(move |_: WeatherLocation| {
        picking.set(false);
        // Remonta o widget: a assinatura nova já coleta com a cidade escolhida.
        if let Some(r) = retry {
            r.again();
        }
    });
    view! {
        <div class="flex h-full flex-col gap-2">
            <CitySearch on_saved=saved />
            <Show when=move || picking.get()>
                <button type="button" class="self-start text-xs text-muted hover:text-fg" on:click=move |_| picking.set(false)>
                    "Cancelar"
                </button>
            </Show>
        </div>
    }
}

/// Busca de cidade pelo nome; escolher grava a preferência global (também no onboarding).
#[component]
pub fn CitySearch(on_saved: Callback<WeatherLocation>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(None::<Result<Vec<WeatherLocation>, String>>);
    let busy = RwSignal::new(false);

    let search = move || {
        let q = query.get_untracked().trim().to_owned();
        if q.chars().count() < 2 {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            let params = serde_json::to_value(WeatherSearch { query: q }).unwrap_or_default();
            let found = api::call(METHOD_WEATHER_SEARCH, params)
                .await
                .map_err(|e| e.message)
                .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()));
            busy.set(false);
            results.set(Some(found));
        });
    };
    let choose = move |location: WeatherLocation| {
        spawn_local(async move {
            let params = serde_json::to_value(SetSetting {
                key: SETTING_WEATHER.to_owned(),
                value: serde_json::to_value(location.clone()).unwrap_or_default(),
                scope: SettingScope::Global,
            })
            .unwrap_or_default();
            match api::call(METHOD_SETTINGS_SET, params).await {
                Ok(_) => on_saved.run(location),
                Err(e) => results.set(Some(Err(e.message))),
            }
        });
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col gap-2 text-sm">
            <form
                class="flex gap-2"
                on:submit=move |e: ev::SubmitEvent| {
                    e.prevent_default();
                    search();
                }
            >
                <input
                    type="search"
                    placeholder="Sua cidade"
                    aria-label="Cidade para o clima"
                    class="min-w-0 flex-1 rounded-md border border-line bg-input px-2.5 py-1 text-sm placeholder:text-subtle focus:outline-none"
                    prop:value=query
                    on:input=move |e| query.set(event_target_value(&e))
                />
                <button
                    type="submit"
                    class="rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-white hover:bg-primary-hover disabled:opacity-60"
                    disabled=move || busy.get()
                >
                    {move || if busy.get() { "Buscando…" } else { "Buscar" }}
                </button>
            </form>
            <ul class="min-h-0 flex-1 space-y-0.5 overflow-auto">
                {move || match results.get() {
                    None => view! { <li class="text-xs text-subtle">"Escolha a cidade da previsão do tempo."</li> }.into_any(),
                    Some(Err(e)) => view! { <li role="alert" class="text-xs text-danger">{e}</li> }.into_any(),
                    Some(Ok(list)) if list.is_empty() => view! { <li class="text-xs text-muted">"Nenhuma cidade com esse nome."</li> }.into_any(),
                    Some(Ok(list)) => list
                        .into_iter()
                        .map(|loc| {
                            let place = [loc.admin1.clone(), loc.country.clone()]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(", ");
                            let name = loc.name.clone();
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class="w-full truncate rounded px-1.5 py-1 text-left hover:bg-card-hover"
                                        on:click=move |_| choose(loc.clone())
                                    >
                                        {name}
                                        <span class="text-xs text-subtle">{format!(" {place}")}</span>
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()
                        .into_any(),
                }}
            </ul>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigos_wmo_em_portugues() {
        assert_eq!(describe(0, true), ("Céu limpo", Icon::Sun));
        assert_eq!(describe(0, false).1, Icon::Moon);
        assert_eq!(describe(63, true).0, "Chuva");
        assert_eq!(describe(95, true).1, Icon::CloudLightning);
        assert_eq!(describe(200, true).0, "Tempo indefinido");
    }
}
