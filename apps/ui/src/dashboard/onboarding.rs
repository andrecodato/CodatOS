//! Primeira execução (RF-APP-012): nome, iniciar com o SO, pastas de projetos e cidade do
//! clima. Tudo pode ser pulado; no fim monta o dashboard padrão só com widgets que têm
//! fonte de dados neste PC (RF-DASH-001). Cofre local e conta chegam com o M3 e o M4.

use codatos_protocol::dashboard::{
    Dashboard as Saved, METHOD_DASHBOARDS_SAVE, METHOD_SETTINGS_SET, SETTING_PROFILE_NAME,
    SetSetting, SettingScope,
};
use codatos_protocol::projects::{ProjectsConfig, SETTING_PROJECTS};
use codatos_protocol::system::{TOPIC_DOCKER, TOPIC_GPU, TOPIC_WSL};
use codatos_protocol::weather::{TOPIC_WEATHER, WeatherLocation};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
use wasm_bindgen::JsValue;

use super::default_dashboard;
use super::widgets::weather::CitySearch;
use crate::api::{self, Sample};
use crate::components::greeting::Profile;
use crate::components::icon::{Icon, Svg};

/// Tópicos testados enquanto o onboarding está aberto e o widget que cada um libera.
type Probes = Vec<(&'static str, ReadSignal<Option<Sample>>)>;

const PROBES: &[(&str, &str)] = &[
    (TOPIC_WSL, "wsl"),
    (TOPIC_DOCKER, "docker"),
    (TOPIC_GPU, "system.gpu"),
    (TOPIC_WEATHER, "weather"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Name,
    Autostart,
    Projects,
    City,
}

impl Step {
    fn next(self) -> Option<Self> {
        match self {
            Self::Name if api::in_tauri() => Some(Self::Autostart),
            Self::Name | Self::Autostart => Some(Self::Projects),
            Self::Projects => Some(Self::City),
            Self::City => None,
        }
    }
}

async fn set_setting(key: &str, value: Value, scope: SettingScope) -> Result<(), String> {
    let params = serde_json::to_value(SetSetting {
        key: key.to_owned(),
        value,
        scope,
    })
    .unwrap_or_default();
    api::call(METHOD_SETTINGS_SET, params)
        .await
        .map(|_| ())
        .map_err(|e| e.message)
}

/// Uma pasta por linha; linhas vazias saem.
fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

#[component]
pub fn Onboarding(on_done: Callback<Saved>) -> impl IntoView {
    let profile = use_context::<Profile>();
    let step = RwSignal::new(Step::Name);
    let name = RwSignal::new(String::new());
    let autostart = RwSignal::new(true);
    let roots = RwSignal::new(String::new());
    let wsl_roots = RwSignal::new(String::new());
    let city = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Assina os tópicos opcionais já no começo: até o fim do onboarding cada um respondeu
    // se tem dados neste PC.
    let probes: Probes = PROBES
        .iter()
        .map(|(topic, kind)| (*kind, api::subscribe(topic, None)))
        .collect();
    let probes = StoredValue::new(probes);

    if api::in_tauri() {
        spawn_local(async move {
            if let Ok(on) = api::invoke("plugin:autostart|is_enabled", &JsValue::UNDEFINED).await {
                autostart.set(on.as_bool().unwrap_or(true));
            }
        });
    }

    // Grava o passo atual e vai para o próximo; no último, monta o dashboard.
    let advance = move |skip: bool| {
        let current = step.get_untracked();
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = if skip {
                Ok(())
            } else {
                save_step(current, name, autostart, roots, wsl_roots).await
            };
            if let Err(e) = result {
                busy.set(false);
                error.set(Some(e));
                return;
            }
            if current == Step::Name
                && !skip
                && let Some(p) = profile
            {
                let n = name.get_untracked().trim().to_owned();
                p.0.set((!n.is_empty()).then_some(n));
            }
            match current.next() {
                Some(next) => {
                    busy.set(false);
                    step.set(next);
                }
                None => finish(probes, city, busy, error, on_done).await,
            }
        });
    };

    view! {
        <section class="mx-auto mt-6 max-w-xl rounded-xl border border-line bg-card p-6 shadow-card" aria-label="Primeiros passos">
            <p class="text-xs text-muted">"Primeiros passos"</p>
            {move || match step.get() {
                Step::Name => view! { <NameStep name=name /> }.into_any(),
                Step::Autostart => view! { <AutostartStep autostart=autostart /> }.into_any(),
                Step::Projects => view! { <ProjectsStep roots=roots wsl_roots=wsl_roots /> }.into_any(),
                Step::City => view! { <CityStep city=city /> }.into_any(),
            }}
            {move || error.get().map(|e| view! { <p role="alert" class="mt-3 text-sm text-danger">{e}</p> })}
            <div class="mt-6 flex items-center gap-2">
                <button
                    type="button"
                    class="rounded-md px-3 py-1.5 text-sm text-muted hover:bg-card-hover hover:text-fg disabled:opacity-50"
                    disabled=move || busy.get()
                    on:click=move |_| advance(true)
                >
                    "Pular"
                </button>
                <button
                    type="button"
                    class="ml-auto rounded-md bg-primary px-4 py-1.5 text-sm font-medium text-white hover:bg-primary-hover disabled:opacity-60"
                    disabled=move || busy.get()
                    on:click=move |_| advance(false)
                >
                    {move || match (busy.get(), step.get().next()) {
                        (true, None) => "Montando…",
                        (_, None) => "Montar meu dashboard",
                        _ => "Continuar",
                    }}
                </button>
            </div>
        </section>
    }
}

const FIELD: &str = "w-full rounded-md border border-line bg-input px-3 py-2 text-sm placeholder:text-subtle focus:outline-none";

#[component]
fn NameStep(name: RwSignal<String>) -> impl IntoView {
    view! {
        <h2 class="mt-1 text-lg font-semibold">"Como quer ser chamado?"</h2>
        <p class="mt-1 text-sm text-muted">"Aparece na saudação do topo."</p>
        <input
            class=format!("mt-4 {FIELD}")
            placeholder="Seu nome"
            aria-label="Seu nome"
            prop:value=name
            on:input=move |e| name.set(event_target_value(&e))
        />
    }
}

#[component]
fn AutostartStep(autostart: RwSignal<bool>) -> impl IntoView {
    view! {
        <h2 class="mt-1 text-lg font-semibold">"Iniciar com o Windows?"</h2>
        <p class="mt-1 text-sm text-muted">
            "O CodatOS abre na bandeja, sem janela, e os widgets ficam prontos quando você precisar."
        </p>
        <label class="mt-4 flex items-center gap-3 text-sm">
            <input
                type="checkbox"
                class="size-4 accent-primary"
                prop:checked=autostart
                on:change=move |e| autostart.set(event_target_checked(&e))
            />
            "Iniciar junto com o sistema"
        </label>
    }
}

#[component]
fn ProjectsStep(roots: RwSignal<String>, wsl_roots: RwSignal<String>) -> impl IntoView {
    view! {
        <h2 class="mt-1 text-lg font-semibold">"Onde ficam seus projetos?"</h2>
        <p class="mt-1 text-sm text-muted">
            "Uma pasta por linha. Em branco, vale o padrão: C:\\dev no Windows e ~ dentro das distros WSL."
        </p>
        <label class="mt-4 block text-xs text-muted" for="onb-roots">"Pastas neste PC"</label>
        <textarea
            id="onb-roots"
            rows="3"
            class=format!("mt-1 font-mono {FIELD}")
            placeholder="C:\\dev"
            prop:value=roots
            on:input=move |e| roots.set(event_target_value(&e))
        ></textarea>
        <label class="mt-3 block text-xs text-muted" for="onb-wsl">"Pastas dentro do WSL"</label>
        <textarea
            id="onb-wsl"
            rows="2"
            class=format!("mt-1 font-mono {FIELD}")
            placeholder="~"
            prop:value=wsl_roots
            on:input=move |e| wsl_roots.set(event_target_value(&e))
        ></textarea>
    }
}

#[component]
fn CityStep(city: RwSignal<Option<String>>) -> impl IntoView {
    let saved = Callback::new(move |loc: WeatherLocation| city.set(Some(loc.name)));
    view! {
        <h2 class="mt-1 text-lg font-semibold">"Qual a sua cidade?"</h2>
        <p class="mt-1 text-sm text-muted">"Para o widget de tempo (dados do Open-Meteo)."</p>
        <div class="mt-4 flex h-48 flex-col">
            {move || match city.get() {
                Some(c) => view! {
                    <p class="flex items-center gap-2 text-sm">
                        <span class="text-success"><Svg icon=Icon::Check size=16 /></span>
                        {format!("Cidade: {c}")}
                    </p>
                }
                .into_any(),
                None => view! { <CitySearch on_saved=saved /> }.into_any(),
            }}
        </div>
    }
}

async fn save_step(
    step: Step,
    name: RwSignal<String>,
    autostart: RwSignal<bool>,
    roots: RwSignal<String>,
    wsl_roots: RwSignal<String>,
) -> Result<(), String> {
    match step {
        Step::Name => {
            let n = name.get_untracked().trim().to_owned();
            if n.is_empty() {
                return Ok(());
            }
            set_setting(SETTING_PROFILE_NAME, json!(n), SettingScope::Global).await
        }
        Step::Autostart => {
            let cmd = if autostart.get_untracked() {
                "plugin:autostart|enable"
            } else {
                "plugin:autostart|disable"
            };
            api::invoke(cmd, &JsValue::UNDEFINED)
                .await
                .map(|_| ())
                .map_err(|_| "Não consegui mudar o início com o sistema".to_owned())
        }
        Step::Projects => {
            let (r, w) = (
                lines(&roots.get_untracked()),
                lines(&wsl_roots.get_untracked()),
            );
            if r.is_empty() && w.is_empty() {
                return Ok(());
            }
            // O padrão vem do núcleo (depende do SO dele); só grava o que foi preenchido.
            let mut config = serde_json::to_value(ProjectsConfig::default()).unwrap_or_default();
            if !r.is_empty() {
                config["roots"] = json!(r);
            } else if let Some(o) = config.as_object_mut() {
                o.remove("roots");
            }
            if !w.is_empty() {
                config["wslRoots"] = json!(w);
            }
            set_setting(SETTING_PROJECTS, config, SettingScope::Device).await
        }
        // A cidade é gravada pela própria busca.
        Step::City => Ok(()),
    }
}

/// Monta e grava o dashboard padrão com os widgets que têm dados neste PC.
async fn finish(
    probes: StoredValue<Probes>,
    city: RwSignal<Option<String>>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_done: Callback<Saved>,
) {
    let mut extra: Vec<&str> = probes.with_value(|p| {
        p.iter()
            .filter(|(_, s)| matches!(s.get_untracked(), Some(Ok(_))))
            .map(|(kind, _)| *kind)
            .collect()
    });
    if city.get_untracked().is_some() && !extra.contains(&"weather") {
        extra.push("weather");
    }
    let saved = default_dashboard(&extra);
    match api::call(
        METHOD_DASHBOARDS_SAVE,
        serde_json::to_value(&saved).unwrap_or_default(),
    )
    .await
    {
        Ok(_) => on_done.run(saved),
        Err(e) => {
            busy.set(false);
            error.set(Some(format!(
                "Não consegui salvar o dashboard: {}",
                e.message
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uma_pasta_por_linha() {
        assert_eq!(lines(" C:\\dev \n\n D:\\code\r\n"), ["C:\\dev", "D:\\code"]);
        assert_eq!(lines("  \n"), Vec::<String>::new());
    }
}
