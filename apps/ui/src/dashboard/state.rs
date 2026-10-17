//! Estados padronizados dos widgets (RF-DASH-014): carregando, erro com "tentar de novo"
//! (RF-DASH-009) e desatualizado quando o núcleo cai.

use leptos::prelude::*;
use serde::de::DeserializeOwned;

use crate::api::{CoreStatus, Sample};
use crate::components::icon::{Icon, Svg};

/// Erros que não são falha: a fonte não existe neste PC ou não tem nada para mostrar.
/// Viram um aviso discreto em vez do alerta vermelho.
const SOFT: &[&str] = &[
    "unavailable",
    "lhm_not_running",
    "unknown_topic",
    "docker_not_running",
    "no_projects",
];

/// Dado por cada widget da grade: incrementar remonta o conteúdo, refazendo as assinaturas.
#[derive(Debug, Clone, Copy)]
pub struct Retry(pub RwSignal<u32>);

impl Retry {
    pub fn again(self) {
        self.0.update(|n| *n += 1);
    }
}

/// Mostra a última amostra de um tópico com `render`, ou o estado em que ela está.
pub fn sample_view<T, F, V>(sample: ReadSignal<Option<Sample>>, render: F) -> AnyView
where
    T: DeserializeOwned + 'static,
    F: Fn(T) -> V + Send + Sync + 'static,
    V: IntoView + 'static,
{
    let status = use_context::<ReadSignal<CoreStatus>>();
    (move || match sample.get() {
        None => view! { <Skeleton /> }.into_any(),
        // Fonte que não existe nesta máquina (sem GPU, sem sensor, LHM fechado): não é falha.
        Some(Err(problem)) if SOFT.contains(&problem.code.as_str()) => view! {
            <Unavailable code=problem.code title=problem.title />
        }
        .into_any(),
        Some(Err(problem)) => view! {
            <ErrorState title=problem.title detail=problem.detail.unwrap_or_default() />
        }
        .into_any(),
        Some(Ok(value)) => match serde_json::from_value::<T>(value) {
            Ok(data) => {
                let stale = status.is_some_and(|s| s.get() != CoreStatus::Connected);
                view! {
                    <div class="relative h-full">
                        {render(data)}
                        {stale.then(|| view! { <Stale /> })}
                    </div>
                }
                .into_any()
            }
            Err(error) => view! {
                <ErrorState title="Resposta inesperada do núcleo".to_owned() detail=error.to_string() />
            }
            .into_any(),
        },
    })
    .into_any()
}

#[component]
pub fn Skeleton() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col gap-2" aria-busy="true" aria-label="Carregando">
            <div class="h-6 w-1/3 animate-pulse rounded bg-card-hover"></div>
            <div class="h-3 w-2/3 animate-pulse rounded bg-card-hover"></div>
            <div class="h-3 w-1/2 animate-pulse rounded bg-card-hover"></div>
        </div>
    }
}

#[component]
pub fn ErrorState(title: String, #[prop(optional)] detail: String) -> impl IntoView {
    let retry = use_context::<Retry>();
    view! {
        <div role="alert" class="flex h-full flex-col items-start justify-center gap-1 text-sm">
            <p class="flex items-center gap-1.5 font-medium text-danger">
                <Svg icon=Icon::Alert size=14 />
                {title}
            </p>
            {(!detail.is_empty()).then(|| view! { <p class="line-clamp-2 text-xs text-muted">{detail}</p> })}
            {retry.map(|r| view! {
                <button
                    type="button"
                    on:click=move |_| r.again()
                    class="mt-1 inline-flex items-center gap-1.5 rounded-md border border-line px-2 py-1 text-xs text-fg hover:bg-card-hover"
                >
                    <Svg icon=Icon::Refresh size=12 />
                    "Tentar de novo"
                </button>
            })}
        </div>
    }
}

/// Selo de "desatualizado": o núcleo caiu e o valor exibido é o último recebido.
#[component]
fn Stale() -> impl IntoView {
    view! {
        <span class="absolute top-0 right-0 rounded bg-warning/15 px-1.5 py-0.5 text-[11px] text-warning">
            "Desatualizado"
        </span>
    }
}

#[component]
fn Unavailable(code: String, title: String) -> impl IntoView {
    let (heading, detail) = match code.as_str() {
        "unknown_topic" => (
            "Indisponível neste sistema",
            "Este widget depende de um recurso de outro sistema operacional.".to_owned(),
        ),
        "unavailable" => ("Indisponível neste PC", title),
        "lhm_not_running" => ("Precisa do LibreHardwareMonitor", title),
        _ => ("Nada por aqui", title),
    };
    view! {
        <div class="flex h-full flex-col items-start justify-center gap-1 text-sm">
            <p class="font-medium text-muted">{heading}</p>
            <p class="text-xs text-subtle">{detail}</p>
        </div>
    }
}
