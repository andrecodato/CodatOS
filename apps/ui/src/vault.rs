//! Cofre (`docs/04-seguranca.md` §7): estado no topo da janela, diálogo para desbloquear e
//! a seção de Configurações para criar, trocar a proteção e bloquear. A senha vai ao núcleo
//! e nunca volta; a UI só vê o [`VaultStatus`].

use codatos_protocol::dashboard::{METHOD_SETTINGS_SET, SetSetting, SettingScope};
use codatos_protocol::vault::{
    CreateVault, METHOD_VAULT_CREATE, METHOD_VAULT_LOCK, METHOD_VAULT_SET_PROTECTION,
    METHOD_VAULT_STATUS, METHOD_VAULT_UNLOCK, MIN_PASSWORD_CHARS, SETTING_VAULT_AUTO_LOCK,
    SetProtection, TOPIC_VAULT, UnlockVault, VaultProtection, VaultState, VaultStatus, errors,
};
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};

use crate::api::CoreStatus;
use crate::components::icon::{Icon, Svg};
use crate::connections::ui::{self, BUTTON, Dialog, FIELD, FormError, LABEL, PRIMARY};

/// Opções do bloqueio automático, em minutos (`0` = nunca).
const AUTO_LOCK_CHOICES: [(u32, &str); 5] = [
    (5, "5 minutos"),
    (15, "15 minutos"),
    (30, "30 minutos"),
    (60, "1 hora"),
    (0, "Nunca"),
];

thread_local! {
    /// O mesmo [`Vault`] do contexto, para quem roda fora do dono reativo (ex.: depois de um
    /// `await` em `spawn_local`, onde `use_context` não acha nada).
    static CURRENT: std::cell::Cell<Option<Vault>> = const { std::cell::Cell::new(None) };
}

/// Estado do cofre compartilhado pela janela.
#[derive(Debug, Clone, Copy)]
pub struct Vault {
    pub status: RwSignal<Option<VaultStatus>>,
    /// Diálogo de desbloqueio aberto.
    pub unlocking: RwSignal<bool>,
}

impl Vault {
    /// Cria o contexto, lê o estado a cada conexão com o núcleo e acompanha o tópico
    /// `vault.state` (o bloqueio automático acontece no núcleo, sem pedido da UI).
    pub fn provide(core: ReadSignal<CoreStatus>) -> Self {
        let vault = Self {
            status: RwSignal::new(None),
            unlocking: RwSignal::new(false),
        };
        Effect::new(move |_| {
            if core.get() == CoreStatus::Connected {
                vault.refresh();
            }
        });
        let samples = crate::api::subscribe(TOPIC_VAULT, Some(5_000));
        Effect::new(move |_| {
            if let Some(Ok(value)) = samples.get()
                && let Ok(status) = serde_json::from_value::<VaultStatus>(value)
            {
                vault.set(status);
            }
        });
        provide_context(vault);
        CURRENT.with(|c| c.set(Some(vault)));
        vault
    }

    fn set(self, status: VaultStatus) {
        if status.state != VaultState::Locked {
            self.unlocking.set(false);
        }
        self.status.set(Some(status));
    }

    /// Lê o estado de novo, depois de criar, abrir ou trancar.
    pub fn refresh(self) {
        spawn_local(async move {
            if let Ok(status) = ui::call(METHOD_VAULT_STATUS, &json!({}))
                .await
                .and_then(|v| serde_json::from_value::<VaultStatus>(v).map_err(|e| e.to_string()))
            {
                self.set(status);
            }
        });
    }

    /// O cofre da janela, de qualquer lugar.
    pub fn current() -> Option<Self> {
        CURRENT.with(std::cell::Cell::get)
    }

    /// Abre o diálogo de desbloqueio (ex.: um método respondeu `vault_locked`).
    pub fn ask_unlock(self) {
        self.unlocking.set(true);
    }

    fn lock(self) {
        spawn_local(async move {
            let _ = ui::call(METHOD_VAULT_LOCK, &json!({})).await;
            self.refresh();
        });
    }
}

/// Cadeado no topo: trancado abre o diálogo; aberto com senha tranca na hora. Sem cofre ou
/// no modo sessão do SO não aparece, porque não há o que fazer ali.
#[component]
pub fn VaultIndicator() -> impl IntoView {
    let vault = expect_context::<Vault>();
    move || {
        let status = vault.status.get()?;
        match (status.state, status.protection) {
            (VaultState::Locked, _) => Some(
                view! {
                    <button
                        type="button"
                        class="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-warning hover:bg-card-hover"
                        on:click=move |_| vault.ask_unlock()
                    >
                        <Svg icon=Icon::Lock size=16 />
                        "Cofre trancado"
                    </button>
                }
                .into_any(),
            ),
            (VaultState::Unlocked, Some(VaultProtection::Password)) => Some(
                view! {
                    <button
                        type="button"
                        aria-label="Trancar o cofre"
                        title="Cofre aberto. Clique para trancar."
                        class="rounded-md p-1.5 hover:bg-card-hover hover:text-fg"
                        on:click=move |_| vault.lock()
                    >
                        <Svg icon=Icon::LockOpen />
                    </button>
                }
                .into_any(),
            ),
            _ => None,
        }
    }
}

/// Diálogo de desbloqueio, aberto por [`Vault::ask_unlock`].
#[component]
pub fn UnlockDialog() -> impl IntoView {
    let vault = expect_context::<Vault>();
    move || vault.unlocking.get().then(|| view! { <UnlockForm /> })
}

#[component]
fn UnlockForm() -> impl IntoView {
    let vault = expect_context::<Vault>();
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let close = Callback::new(move |()| vault.unlocking.set(false));

    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let params = UnlockVault {
            password: password.get_untracked(),
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call_coded(METHOD_VAULT_UNLOCK, &params).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    password.set(String::new());
                    vault.unlocking.set(false);
                    vault.refresh();
                }
                Err((code, message)) => error.set(Some(if code == errors::WRONG_PASSWORD {
                    "Senha errada.".to_owned()
                } else {
                    message
                })),
            }
        });
    };

    view! {
        <Dialog title="Desbloquear o cofre".to_owned() on_close=close>
            <form class="space-y-4 p-5" on:submit=submit>
                <p class="text-sm text-muted">
                    "Senhas e chaves das conexões ficam no cofre. Digite a senha do cofre para usá-las."
                </p>
                <div>
                    <label class=LABEL for="vault-unlock-password">"Senha do cofre"</label>
                    <input
                        id="vault-unlock-password"
                        type="password"
                        autocomplete="current-password"
                        autofocus
                        class=format!("mt-1 {FIELD}")
                        prop:value=password
                        on:input=move |e| password.set(event_target_value(&e))
                    />
                </div>
                <FormError error=error />
                <div class="flex justify-end gap-2">
                    <button type="button" class=BUTTON on:click=move |_| close.run(())>"Cancelar"</button>
                    <button
                        type="submit"
                        class=PRIMARY
                        disabled=move || busy.get() || password.with(String::is_empty)
                    >
                        <Svg icon=Icon::LockOpen size=16 />
                        {move || if busy.get() { "Abrindo…" } else { "Desbloquear" }}
                    </button>
                </div>
            </form>
        </Dialog>
    }
}

/// Seção "Cofre" de Configurações.
#[component]
pub fn VaultSettings() -> impl IntoView {
    let vault = expect_context::<Vault>();
    view! {
        <section class="max-w-2xl space-y-4 rounded-lg border border-line bg-card p-5">
            <header class="flex items-center gap-3">
                <span class="rounded-md bg-primary-soft p-2 text-primary"><Svg icon=Icon::Lock size=18 /></span>
                <div>
                    <h2 class="text-sm font-semibold">"Cofre"</h2>
                    <p class="text-xs text-muted">"Guarda as senhas e chaves das conexões, cifradas neste PC."</p>
                </div>
            </header>
            {move || match vault.status.get() {
                None => view! { <p class="text-sm text-muted">"Carregando…"</p> }.into_any(),
                Some(status) if status.state == VaultState::Absent => {
                    view! { <CreateForm status=status /> }.into_any()
                }
                Some(status) => view! { <Manage status=status /> }.into_any(),
            }}
        </section>
    }
}

/// Erro de senha nova, antes de mandar ao núcleo.
fn check_new_password(password: &str, confirm: &str) -> Result<(), String> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(format!(
            "A senha precisa de pelo menos {MIN_PASSWORD_CHARS} caracteres."
        ));
    }
    if password != confirm {
        return Err("As senhas não conferem.".to_owned());
    }
    Ok(())
}

/// Escolha do modo com a explicação de cada um.
#[component]
fn ProtectionChoice(
    protection: RwSignal<VaultProtection>,
    os_session_available: bool,
) -> impl IntoView {
    let option = move |value: VaultProtection, title: &'static str, text: &'static str| {
        let disabled = value == VaultProtection::OsSession && !os_session_available;
        view! {
            <label
                class="flex cursor-pointer gap-3 rounded-md border p-3"
                class=("border-primary", move || protection.get() == value)
                class=("border-line", move || protection.get() != value)
                class=("opacity-50", disabled)
            >
                <input
                    type="radio"
                    name="vault-protection"
                    class="mt-1"
                    disabled=disabled
                    prop:checked=move || protection.get() == value
                    on:change=move |_| protection.set(value)
                />
                <span>
                    <span class="block text-sm font-medium">{title}</span>
                    <span class="block text-xs text-muted">{text}</span>
                </span>
            </label>
        }
    };
    view! {
        <fieldset class="space-y-2">
            <legend class=LABEL>"Como proteger"</legend>
            {option(
                VaultProtection::Password,
                "Senha do cofre (recomendado)",
                "Você digita a senha ao abrir o CodatOS e depois de um tempo sem uso. Quem usa o seu PC logado não vê os segredos.",
            )}
            {option(
                VaultProtection::OsSession,
                "Sessão do Windows",
                if os_session_available {
                    "O cofre abre sozinho quando você entra no Windows. Mais prático, mas quem usar o seu PC logado tem acesso."
                } else {
                    "Indisponível neste sistema."
                },
            )}
        </fieldset>
    }
}

#[component]
fn NewPassword(password: RwSignal<String>, confirm: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="grid gap-3 sm:grid-cols-2">
            <div>
                <label class=LABEL for="vault-new-password">"Senha"</label>
                <input
                    id="vault-new-password"
                    type="password"
                    autocomplete="new-password"
                    class=format!("mt-1 {FIELD}")
                    prop:value=password
                    on:input=move |e| password.set(event_target_value(&e))
                />
            </div>
            <div>
                <label class=LABEL for="vault-confirm-password">"Repita a senha"</label>
                <input
                    id="vault-confirm-password"
                    type="password"
                    autocomplete="new-password"
                    class=format!("mt-1 {FIELD}")
                    prop:value=confirm
                    on:input=move |e| confirm.set(event_target_value(&e))
                />
            </div>
        </div>
        <p class="text-xs text-muted">
            {format!("Pelo menos {MIN_PASSWORD_CHARS} caracteres. Não há como recuperar a senha: sem ela, os segredos guardados se perdem.")}
        </p>
    }
}

#[component]
fn CreateForm(status: VaultStatus) -> impl IntoView {
    let vault = expect_context::<Vault>();
    let protection = RwSignal::new(VaultProtection::Password);
    let password = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let protection = protection.get_untracked();
        let password = if protection == VaultProtection::Password {
            let (p, c) = (password.get_untracked(), confirm.get_untracked());
            if let Err(e) = check_new_password(&p, &c) {
                error.set(Some(e));
                return;
            }
            Some(p)
        } else {
            None
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call(
                METHOD_VAULT_CREATE,
                &CreateVault {
                    protection,
                    password,
                },
            )
            .await;
            busy.set(false);
            match result {
                Ok(_) => vault.refresh(),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <form class="space-y-4" on:submit=submit>
            <p class="text-sm">"Ainda não há cofre neste PC. Crie um para guardar senhas e chaves SSH."</p>
            <ProtectionChoice protection=protection os_session_available=status.os_session_available />
            {move || (protection.get() == VaultProtection::Password).then(|| view! {
                <NewPassword password=password confirm=confirm />
            })}
            <FormError error=error />
            <button type="submit" class=PRIMARY disabled=busy>
                {move || if busy.get() { "Criando…" } else { "Criar cofre" }}
            </button>
        </form>
    }
}

#[component]
fn Manage(status: VaultStatus) -> impl IntoView {
    let vault = expect_context::<Vault>();
    let changing = RwSignal::new(false);
    let with_password = status.protection == Some(VaultProtection::Password);
    let locked = status.state == VaultState::Locked;

    view! {
        <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
            <dt class="text-muted">"Estado"</dt>
            <dd class="flex items-center gap-2">
                <span class=if locked { "text-warning" } else { "text-success" }>
                    <Svg icon=if locked { Icon::Lock } else { Icon::LockOpen } size=14 />
                </span>
                {if locked { "Trancado" } else { "Aberto" }}
            </dd>
            <dt class="text-muted">"Proteção"</dt>
            <dd>{if with_password { "Senha do cofre" } else { "Sessão do Windows" }}</dd>
        </dl>
        {with_password.then(|| view! { <AutoLock minutes=status.auto_lock_minutes /> })}
        <div class="flex flex-wrap gap-2">
            {if locked {
                view! {
                    <button type="button" class=PRIMARY on:click=move |_| vault.ask_unlock()>
                        <Svg icon=Icon::LockOpen size=16 />
                        "Desbloquear"
                    </button>
                }
                .into_any()
            } else {
                view! {
                    {with_password.then(|| view! {
                        <button type="button" class=BUTTON on:click=move |_| vault.lock()>
                            <Svg icon=Icon::Lock size=16 />
                            "Trancar agora"
                        </button>
                    })}
                    <button type="button" class=BUTTON on:click=move |_| changing.set(true)>
                        <Svg icon=Icon::Key size=16 />
                        {if with_password { "Trocar senha ou modo" } else { "Trocar modo" }}
                    </button>
                }
                .into_any()
            }}
        </div>
        {move || changing.get().then(|| view! { <ChangeDialog status=status.clone() open=changing /> })}
    }
}

#[component]
fn AutoLock(minutes: u32) -> impl IntoView {
    let vault = expect_context::<Vault>();
    let error = RwSignal::new(None::<String>);
    let on_change = move |e: leptos::ev::Event| {
        let Ok(minutes) = event_target_value(&e).parse::<u32>() else {
            return;
        };
        let params = SetSetting {
            key: SETTING_VAULT_AUTO_LOCK.to_owned(),
            value: Value::from(minutes),
            scope: SettingScope::Device,
        };
        spawn_local(async move {
            match ui::call(METHOD_SETTINGS_SET, &params).await {
                Ok(_) => {
                    error.set(None);
                    vault.refresh();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    view! {
        <div class="max-w-xs">
            <label class=LABEL for="vault-auto-lock">"Trancar sozinho depois de"</label>
            <select id="vault-auto-lock" class=format!("mt-1 {FIELD}") on:change=on_change>
                {AUTO_LOCK_CHOICES
                    .iter()
                    .map(|(value, label)| view! {
                        <option value=value.to_string() selected=*value == minutes>{*label}</option>
                    })
                    .collect_view()}
            </select>
            <p class="mt-1 text-xs text-muted">"Contado sem uso das senhas e chaves."</p>
            <FormError error=error />
        </div>
    }
}

/// Troca a senha ou o modo; pede a senha atual quando o cofre usa senha (reautenticação).
#[component]
fn ChangeDialog(status: VaultStatus, open: RwSignal<bool>) -> impl IntoView {
    let vault = expect_context::<Vault>();
    let had_password = status.protection == Some(VaultProtection::Password);
    let protection = RwSignal::new(status.protection.unwrap_or(VaultProtection::Password));
    let current = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let close = Callback::new(move |()| open.set(false));

    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let protection = protection.get_untracked();
        let new_password = if protection == VaultProtection::Password {
            let (p, c) = (password.get_untracked(), confirm.get_untracked());
            if let Err(e) = check_new_password(&p, &c) {
                error.set(Some(e));
                return;
            }
            Some(p)
        } else {
            None
        };
        let params = SetProtection {
            protection,
            current_password: had_password.then(|| current.get_untracked()),
            new_password,
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call_coded(METHOD_VAULT_SET_PROTECTION, &params).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    open.set(false);
                    vault.refresh();
                }
                Err((code, message)) => error.set(Some(if code == errors::WRONG_PASSWORD {
                    "A senha atual está errada.".to_owned()
                } else {
                    message
                })),
            }
        });
    };

    view! {
        <Dialog title="Trocar a proteção do cofre".to_owned() on_close=close>
            <form class="space-y-4 overflow-auto p-5" on:submit=submit>
                {had_password.then(|| view! {
                    <div>
                        <label class=LABEL for="vault-current-password">"Senha atual"</label>
                        <input
                            id="vault-current-password"
                            type="password"
                            autocomplete="current-password"
                            class=format!("mt-1 {FIELD}")
                            prop:value=current
                            on:input=move |e| current.set(event_target_value(&e))
                        />
                    </div>
                })}
                <ProtectionChoice protection=protection os_session_available=status.os_session_available />
                {move || (protection.get() == VaultProtection::Password).then(|| view! {
                    <NewPassword password=password confirm=confirm />
                })}
                <FormError error=error />
                <div class="flex justify-end gap-2">
                    <button type="button" class=BUTTON on:click=move |_| close.run(())>"Cancelar"</button>
                    <button type="submit" class=PRIMARY disabled=busy>
                        {move || if busy.get() { "Salvando…" } else { "Salvar" }}
                    </button>
                </div>
            </form>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confere_a_senha_nova() {
        assert!(check_new_password("curta", "curta").is_err());
        assert!(check_new_password("senha-boa-1", "senha-boa-2").is_err());
        assert_eq!(check_new_password("senha-boa-1", "senha-boa-1"), Ok(()));
        // Conta caracteres, não bytes.
        assert_eq!(check_new_password("áéíóúãõç", "áéíóúãõç"), Ok(()));
    }
}
