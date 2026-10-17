//! Saudação do topo (RF-DASH-008): "Bom dia, André".

use std::time::Duration;

use codatos_protocol::dashboard::{
    METHOD_SETTINGS_GET, SETTING_PROFILE_NAME, SettingKey, SettingScope,
};
use leptos::{prelude::*, task::spawn_local};

use crate::api::{self, CoreStatus};

/// Nome do usuário, lido do núcleo ao conectar e trocado pelo onboarding.
#[derive(Debug, Clone, Copy)]
pub struct Profile(pub RwSignal<Option<String>>);

impl Profile {
    /// Cria o perfil e o carrega na primeira vez que o núcleo conectar.
    pub fn load(status: ReadSignal<CoreStatus>) -> Self {
        let name = RwSignal::new(None);
        let loaded = StoredValue::new(false);
        Effect::new(move |_| {
            if status.get() != CoreStatus::Connected || loaded.get_value() {
                return;
            }
            loaded.set_value(true);
            spawn_local(async move {
                let params = serde_json::to_value(SettingKey {
                    key: SETTING_PROFILE_NAME.to_owned(),
                    scope: SettingScope::default(),
                })
                .unwrap_or_default();
                match api::call(METHOD_SETTINGS_GET, params).await {
                    Ok(value) => name.set(value.as_str().map(str::to_owned)),
                    Err(_) => loaded.set_value(false),
                }
            });
        });
        Self(name)
    }
}

/// Período do dia pela hora local.
fn period(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Bom dia",
        12..=17 => "Boa tarde",
        _ => "Boa noite",
    }
}

#[component]
pub fn Greeting() -> impl IntoView {
    let profile = expect_context::<Profile>();
    let (hour, set_hour) = signal(js_sys::Date::new_0().get_hours());
    let handle = set_interval_with_handle(
        move || set_hour.set(js_sys::Date::new_0().get_hours()),
        Duration::from_secs(60),
    )
    .ok();
    on_cleanup(move || {
        if let Some(h) = handle {
            h.clear();
        }
    });
    let text = move || match profile.0.get() {
        Some(name) => format!("{}, {name}", period(hour.get())),
        None => period(hour.get()).to_owned(),
    };
    view! {
        <h1 class="text-[26px] leading-8 font-semibold">{text}</h1>
        <p class="mt-1 text-sm text-muted">"Seu universo, em um só lugar."</p>
    }
}

#[cfg(test)]
mod tests {
    use super::period;

    #[test]
    fn periodo_do_dia() {
        assert_eq!(period(4), "Boa noite");
        assert_eq!(period(5), "Bom dia");
        assert_eq!(period(12), "Boa tarde");
        assert_eq!(period(18), "Boa noite");
    }
}
