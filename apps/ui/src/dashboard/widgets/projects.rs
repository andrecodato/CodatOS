//! Projetos recentes (RF-PROJ-003): nome, branch, alterações e último acesso (`projects`).

use codatos_protocol::projects::{Project, TOPIC_PROJECTS};
use leptos::prelude::*;
use serde_json::Value;

use crate::api;
use crate::dashboard::state::sample_view;

pub fn render(_config: &Value) -> AnyView {
    let sample = api::subscribe(TOPIC_PROJECTS, None);
    sample_view(sample, |projects: Vec<Project>| {
        let today = today_start();
        view! {
            <ul class="h-full space-y-1 overflow-auto pr-1">
                {projects.into_iter().map(|p| view! { <Row project=p today=today /> }).collect_view()}
            </ul>
        }
    })
}

/// Início do dia de hoje no fuso local, em segundos Unix.
#[allow(clippy::cast_possible_truncation)] // segundos Unix cabem num i64
fn today_start() -> i64 {
    let now = js_sys::Date::new_0();
    now.set_hours(0);
    now.set_minutes(0);
    now.set_seconds(0);
    now.set_milliseconds(0);
    (now.get_time() / 1000.0) as i64
}

/// "hoje", "ontem", "3 dias atrás"…, em dias de calendário.
fn relative_day(timestamp: i64, today_start: i64) -> String {
    let days = if timestamp >= today_start {
        0
    } else {
        (today_start - timestamp - 1) / 86_400 + 1
    };
    match days {
        0 => "hoje".to_owned(),
        1 => "ontem".to_owned(),
        2..=29 => format!("{days} dias atrás"),
        30..=59 => "há 1 mês".to_owned(),
        _ => format!("há {} meses", days / 30),
    }
}

#[component]
fn Row(project: Project, today: i64) -> impl IntoView {
    let last = project
        .last_opened_at
        .into_iter()
        .chain(project.git.as_ref().and_then(|g| g.last_commit_at))
        .max();
    let git = project.git.clone();
    let branch = git.as_ref().map(|g| {
        g.branch
            .clone()
            .unwrap_or_else(|| "HEAD destacado".to_owned())
    });
    let changes = git.as_ref().map_or(0, |g| g.changes);
    let sync = git.as_ref().and_then(|g| {
        let parts: Vec<String> = [('↑', g.ahead), ('↓', g.behind)]
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(|(arrow, n)| format!("{arrow}{n}"))
            .collect();
        (!parts.is_empty()).then(|| parts.join(" "))
    });
    let title = match &project.wsl_distro {
        Some(d) => format!("{} (WSL: {d})", project.path),
        None => project.path.clone(),
    };
    view! {
        <li class="flex items-center gap-2 rounded-md px-1 py-1 text-sm hover:bg-card-hover" title=title>
            <div class="min-w-0 flex-1">
                <p class="flex items-center gap-1.5">
                    <span class="truncate font-medium">{project.name.clone()}</span>
                    {project.wsl_distro.as_ref().map(|_| view! {
                        <span class="shrink-0 rounded bg-card-hover px-1 text-[10px] text-muted">"WSL"</span>
                    })}
                    {(changes > 0).then(|| view! {
                        <span class="shrink-0 rounded bg-warning/15 px-1 text-[10px] text-warning" title="Alterações pendentes">
                            {format!("{changes} alteradas")}
                        </span>
                    })}
                </p>
                <p class="truncate text-xs text-subtle">
                    {[project.language.clone(), branch, sync]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(" · ")}
                </p>
            </div>
            <span class="shrink-0 text-xs text-subtle">{last.map(|t| relative_day(t, today))}</span>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dias_de_calendario() {
        let today = 1_000_000;
        assert_eq!(relative_day(today + 10, today), "hoje");
        assert_eq!(relative_day(today - 1, today), "ontem");
        assert_eq!(relative_day(today - 86_400, today), "ontem");
        assert_eq!(relative_day(today - 86_401, today), "2 dias atrás");
        assert_eq!(relative_day(today - 40 * 86_400, today), "há 1 mês");
        assert_eq!(relative_day(today - 100 * 86_400, today), "há 3 meses");
    }
}
