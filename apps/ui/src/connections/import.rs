//! Importar e exportar o `~/.ssh/config` (RF-CON-005).

use codatos_protocol::connections::{
    ImportReport, ImportSshConfig, METHOD_SSH_CONFIG_EXPORT, METHOD_SSH_CONFIG_IMPORT,
    SshConfigText,
};
use leptos::{prelude::*, task::spawn_local};
use serde_json::json;

use super::Data;
use super::ui::{self, BUTTON, Dialog, FIELD, FormError, LABEL, PRIMARY};
use crate::components::icon::{Icon, Svg};

/// Lê o `~/.ssh/config` do PC do núcleo, ou o texto colado.
#[component]
pub fn ImportDialog(open: RwSignal<bool>) -> impl IntoView {
    let data = expect_context::<Data>();
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let report = RwSignal::new(None::<ImportReport>);
    let close = Callback::new(move |()| open.set(false));

    let run = move |pasted: bool| {
        let params = ImportSshConfig {
            text: pasted.then(|| text.get_untracked()),
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call(METHOD_SSH_CONFIG_IMPORT, &params)
                .await
                .and_then(|v| serde_json::from_value::<ImportReport>(v).map_err(|e| e.to_string()));
            busy.set(false);
            match result {
                Ok(r) => {
                    report.set(Some(r));
                    data.reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <Dialog title="Importar ~/.ssh/config".to_owned() on_close=close>
            <div class="space-y-4 overflow-auto p-5">
                {move || match report.get() {
                    Some(r) => view! { <Report report=r /> }.into_any(),
                    None => view! {
                        <p class="text-sm text-muted">
                            "Hosts, usuários, chaves sem passphrase e jump hosts entram no CodatOS. Hosts com um nome já cadastrado ficam como estão."
                        </p>
                        <button type="button" class=PRIMARY disabled=busy on:click=move |_| run(false)>
                            <Svg icon=Icon::Download size=16 />
                            "Ler o ~/.ssh/config deste PC"
                        </button>
                        <div>
                            <label class=LABEL for="ssh-config-text">"Ou cole o conteúdo"</label>
                            <textarea
                                id="ssh-config-text"
                                rows="8"
                                class=format!("mt-1 font-mono {FIELD}")
                                placeholder="Host web\n  HostName 192.168.1.10\n  User deploy"
                                prop:value=text
                                on:input=move |e| text.set(event_target_value(&e))
                            ></textarea>
                        </div>
                        <button
                            type="button"
                            class=BUTTON
                            disabled=move || busy.get() || text.with(|t| t.trim().is_empty())
                            on:click=move |_| run(true)
                        >
                            "Importar o texto colado"
                        </button>
                    }
                    .into_any(),
                }}
                <FormError error=error />
            </div>
            <footer class="flex justify-end border-t border-line px-5 py-3">
                <button type="button" class=BUTTON on:click=move |_| close.run(())>"Fechar"</button>
            </footer>
        </Dialog>
    }
}

#[component]
fn Report(report: ImportReport) -> impl IntoView {
    view! {
        <p class="flex items-center gap-2 text-sm">
            <span class="text-success"><Svg icon=Icon::Check size=16 /></span>
            {format!(
                "{} hosts, {} identidades e {} chaves importados.",
                report.hosts, report.identities, report.keys
            )}
        </p>
        {(!report.skipped.is_empty()).then(|| view! {
            <p class="text-sm text-muted">{format!("Já existiam: {}.", report.skipped.join(", "))}</p>
        })}
        {(!report.warnings.is_empty()).then(|| view! {
            <ul class="list-disc space-y-1 pl-5 text-sm text-warning">
                {report.warnings.into_iter().map(|w| view! { <li>{w}</li> }).collect_view()}
            </ul>
        })}
    }
}

/// Mostra o `ssh_config` dos hosts cadastrados para copiar. Não grava arquivo.
#[component]
pub fn ExportDialog(open: RwSignal<bool>) -> impl IntoView {
    let text = RwSignal::new(None::<Result<String, String>>);
    let copied = RwSignal::new(false);
    let close = Callback::new(move |()| open.set(false));
    spawn_local(async move {
        let result = ui::call(METHOD_SSH_CONFIG_EXPORT, &json!({}))
            .await
            .and_then(|v| serde_json::from_value::<SshConfigText>(v).map_err(|e| e.to_string()));
        text.set(Some(result.map(|t| t.text)));
    });

    view! {
        <Dialog title="Exportar para ~/.ssh/config".to_owned() on_close=close width="w-[min(40rem,calc(100vw-2rem))]">
            <div class="space-y-3 overflow-auto p-5">
                {move || match text.get() {
                    None => view! { <p class="text-sm text-muted">"Gerando…"</p> }.into_any(),
                    Some(Err(e)) => view! { <p role="alert" class="text-sm text-danger">{e}</p> }.into_any(),
                    Some(Ok(t)) => view! {
                        <p class="text-sm text-muted">
                            "Cole no seu ~/.ssh/config. As chaves ficam no cofre do CodatOS e aparecem só como comentário."
                        </p>
                        <textarea readonly rows="14" class=format!("font-mono {FIELD}") aria-label="ssh_config gerado">{t}</textarea>
                    }
                    .into_any(),
                }}
            </div>
            <footer class="flex justify-end gap-2 border-t border-line px-5 py-3">
                <button type="button" class=BUTTON on:click=move |_| close.run(())>"Fechar"</button>
                <button
                    type="button"
                    class=PRIMARY
                    disabled=move || !matches!(text.get(), Some(Ok(_)))
                    on:click=move |_| {
                        if let Some(Ok(t)) = text.get_untracked() {
                            ui::copy(&t);
                            copied.set(true);
                        }
                    }
                >
                    <Svg icon=Icon::Copy size=16 />
                    {move || if copied.get() { "Copiado" } else { "Copiar" }}
                </button>
            </footer>
        </Dialog>
    }
}
