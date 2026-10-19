//! Aba Chaveiro: identidades (usuário, senha e chave) e chaves SSH (RF-CON-010/011).
//! Senhas e chaves privadas vão ao núcleo uma vez, seladas pelo cofre; daqui só se vê se
//! há senha e a chave pública.

use codatos_protocol::connections::{
    ConnectionId, GenerateKey, Identity, ImportKey, KeyAlgorithm, METHOD_IDENTITY_DELETE,
    METHOD_IDENTITY_SAVE, METHOD_KEY_DELETE, METHOD_KEY_GENERATE, METHOD_KEY_IMPORT,
    METHOD_KEY_RENAME, PasswordChange, RenameKey, SaveIdentity, SshKey,
};
use leptos::{prelude::*, task::spawn_local};

use super::Data;
use super::ui::{self, BUTTON, DANGER, Dialog, Edit, FIELD, FormError, GHOST, LABEL, PRIMARY};
use crate::components::icon::{Icon, Svg};
use crate::ids::new_id;

#[component]
pub fn Keychain() -> impl IntoView {
    let data = expect_context::<Data>();
    let identity = RwSignal::new(None::<Edit<Identity>>);
    let generating = RwSignal::new(false);
    let importing = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    view! {
        <div class="grid min-h-0 flex-1 gap-6 overflow-auto xl:grid-cols-2">
            <section class="space-y-3">
                <header class="flex items-center gap-2">
                    <h2 class="text-sm font-semibold">"Identidades"</h2>
                    <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| identity.set(Some(Edit::New))>
                        <Svg icon=Icon::Plus size=16 />
                        "Nova identidade"
                    </button>
                </header>
                <ul class="space-y-2">
                    {move || {
                        let list = data.get();
                        if list.identities.is_empty() {
                            return view! {
                                <li class="rounded-lg border border-dashed border-line p-4 text-sm text-muted">
                                    "Uma identidade junta usuário, senha e chave para reaproveitar em vários hosts."
                                </li>
                            }
                            .into_any();
                        }
                        list.identities
                            .into_iter()
                            .map(|i| {
                                let key = i
                                    .key_id
                                    .as_ref()
                                    .and_then(|k| list.keys.iter().find(|x| x.id == *k))
                                    .map(|k| k.name.clone());
                                let auth = match (i.has_password, key) {
                                    (true, Some(k)) => format!("senha e chave {k}"),
                                    (true, None) => "senha".to_owned(),
                                    (false, Some(k)) => format!("chave {k}"),
                                    (false, None) => "sem senha nem chave".to_owned(),
                                };
                                let edit = i.clone();
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            class="flex w-full items-center gap-3 rounded-lg border border-line bg-card p-3 text-left hover:bg-card-hover"
                                            on:click=move |_| identity.set(Some(Edit::Existing(edit.clone())))
                                        >
                                            <span class="rounded-md bg-primary-soft p-2 text-primary"><Svg icon=Icon::User size=16 /></span>
                                            <span class="min-w-0 flex-1">
                                                <span class="block truncate text-sm font-medium">{i.name.clone()}</span>
                                                <span class="block truncate text-xs text-muted">{format!("{} · {auth}", i.username)}</span>
                                            </span>
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </ul>
            </section>
            <section class="space-y-3">
                <header class="flex flex-wrap items-center gap-2">
                    <h2 class="text-sm font-semibold">"Chaves SSH"</h2>
                    <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| importing.set(true)>
                        <Svg icon=Icon::Download size=16 />
                        "Importar"
                    </button>
                    <button type="button" class=PRIMARY on:click=move |_| generating.set(true)>
                        <Svg icon=Icon::Plus size=16 />
                        "Gerar chave"
                    </button>
                </header>
                <FormError error=error />
                <ul class="space-y-2">
                    {move || {
                        let keys = data.get().keys;
                        if keys.is_empty() {
                            return view! {
                                <li class="rounded-lg border border-dashed border-line p-4 text-sm text-muted">
                                    "Gere uma chave Ed25519 ou importe uma chave OpenSSH ou PuTTY. A chave privada fica selada no cofre."
                                </li>
                            }
                            .into_any();
                        }
                        keys.into_iter().map(|k| view! { <KeyRow key=k error=error /> }).collect_view().into_any()
                    }}
                </ul>
            </section>
        </div>
        {move || identity.get().map(|i| view! { <IdentityDialog identity=i open=identity /> })}
        {move || generating.get().then(|| view! { <GenerateDialog open=generating /> })}
        {move || importing.get().then(|| view! { <ImportKeyDialog open=importing /> })}
    }
}

#[component]
fn KeyRow(key: SshKey, error: RwSignal<Option<String>>) -> impl IntoView {
    let data = expect_context::<Data>();
    let copied = RwSignal::new(false);
    let public = StoredValue::new(key.public_key.clone());
    let id = StoredValue::new(key.id.clone());
    let name = StoredValue::new(key.name.clone());

    let rename = move |_| {
        let Ok(Some(new_name)) =
            window().prompt_with_message_and_default("Novo nome da chave", &name.get_value())
        else {
            return;
        };
        if new_name.trim().is_empty() {
            return;
        }
        let params = RenameKey {
            id: id.get_value(),
            name: new_name.trim().to_owned(),
        };
        spawn_local(async move {
            match ui::call(METHOD_KEY_RENAME, &params).await {
                Ok(_) => data.reload(),
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete = move |_| {
        if !window()
            .confirm_with_message(&format!(
                "Apagar a chave {}? Identidades que a usam ficam sem chave.",
                name.get_value()
            ))
            .unwrap_or(false)
        {
            return;
        }
        let params = ConnectionId { id: id.get_value() };
        spawn_local(async move {
            match ui::call(METHOD_KEY_DELETE, &params).await {
                Ok(_) => data.reload(),
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <li class="rounded-lg border border-line bg-card p-3">
            <div class="flex items-center gap-3">
                <span class="rounded-md bg-primary-soft p-2 text-primary"><Svg icon=Icon::Key size=16 /></span>
                <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-medium">{key.name.clone()}</p>
                    <p class="truncate font-mono text-xs text-muted">{format!("{} · {}", key.algorithm, key.fingerprint)}</p>
                </div>
                <button
                    type="button"
                    class=GHOST
                    title="Copiar a chave pública"
                    aria-label=format!("Copiar a chave pública de {}", key.name)
                    on:click=move |_| {
                        ui::copy(&public.get_value());
                        copied.set(true);
                    }
                >
                    {move || view! { <Svg icon=if copied.get() { Icon::Check } else { Icon::Copy } size=16 /> }}
                </button>
                <button type="button" class=GHOST title="Renomear" aria-label=format!("Renomear {}", key.name) on:click=rename>
                    <Svg icon=Icon::Pencil size=16 />
                </button>
                <button type="button" class=GHOST title="Apagar" aria-label=format!("Apagar {}", key.name) on:click=delete>
                    <Svg icon=Icon::Trash size=16 />
                </button>
            </div>
        </li>
    }
}

/// Como a senha fica ao salvar: a tela nunca recebe a senha guardada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasswordMode {
    Keep,
    Set,
    Clear,
}

#[component]
fn IdentityDialog(
    identity: Edit<Identity>,
    open: RwSignal<Option<Edit<Identity>>>,
) -> impl IntoView {
    let data = expect_context::<Data>();
    let identity = identity.existing();
    let is_new = identity.is_none();
    let had_password = identity.as_ref().is_some_and(|i| i.has_password);
    let id = StoredValue::new(identity.as_ref().map_or_else(new_id, |i| i.id.clone()));
    let name = RwSignal::new(
        identity
            .as_ref()
            .map(|i| i.name.clone())
            .unwrap_or_default(),
    );
    let username = RwSignal::new(
        identity
            .as_ref()
            .map(|i| i.username.clone())
            .unwrap_or_default(),
    );
    let key_id = RwSignal::new(
        identity
            .as_ref()
            .and_then(|i| i.key_id.clone())
            .unwrap_or_default(),
    );
    let mode = RwSignal::new(if had_password {
        PasswordMode::Keep
    } else {
        PasswordMode::Set
    });
    let password = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let close = Callback::new(move |()| open.set(None));

    let save = move |_| {
        let (n, u) = (name.get_untracked(), username.get_untracked());
        if n.trim().is_empty() || u.trim().is_empty() {
            error.set(Some("Preencha o nome e o usuário.".to_owned()));
            return;
        }
        let change = match mode.get_untracked() {
            PasswordMode::Keep => PasswordChange::Keep,
            PasswordMode::Clear => PasswordChange::Clear,
            // Campo vazio numa identidade nova: sem senha.
            PasswordMode::Set if password.with_untracked(String::is_empty) => {
                if had_password {
                    PasswordChange::Keep
                } else {
                    PasswordChange::Clear
                }
            }
            PasswordMode::Set => PasswordChange::Set(password.get_untracked()),
        };
        let params = SaveIdentity {
            id: id.get_value(),
            name: n.trim().to_owned(),
            username: u.trim().to_owned(),
            key_id: Some(key_id.get_untracked()).filter(|k| !k.is_empty()),
            password: change,
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call(METHOD_IDENTITY_SAVE, &params).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    data.reload();
                    open.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };
    let delete = move |_| {
        if !window()
            .confirm_with_message("Apagar a identidade? Hosts que a usam ficam sem identidade.")
            .unwrap_or(false)
        {
            return;
        }
        let params = ConnectionId { id: id.get_value() };
        spawn_local(async move {
            match ui::call(METHOD_IDENTITY_DELETE, &params).await {
                Ok(_) => {
                    data.reload();
                    open.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <Dialog title=if is_new { "Nova identidade".to_owned() } else { "Editar identidade".to_owned() } on_close=close>
            <div class="space-y-4 overflow-auto p-5">
                <div class="grid gap-4 sm:grid-cols-2">
                    <div>
                        <label class=LABEL for="identity-name">"Nome"</label>
                        <input
                            id="identity-name"
                            class=format!("mt-1 {FIELD}")
                            placeholder="Deploy produção"
                            prop:value=name
                            on:input=move |e| name.set(event_target_value(&e))
                        />
                    </div>
                    <div>
                        <label class=LABEL for="identity-user">"Usuário"</label>
                        <input
                            id="identity-user"
                            class=format!("mt-1 {FIELD}")
                            placeholder="deploy"
                            autocomplete="off"
                            prop:value=username
                            on:input=move |e| username.set(event_target_value(&e))
                        />
                    </div>
                </div>
                <div>
                    <label class=LABEL for="identity-key">"Chave SSH"</label>
                    <select
                        id="identity-key"
                        class=format!("mt-1 {FIELD}")
                        prop:value=key_id
                        on:change=move |e| key_id.set(event_target_value(&e))
                    >
                        <option value="">"Nenhuma"</option>
                        {data
                            .get_untracked()
                            .keys
                            .into_iter()
                            .map(|k| view! { <option value=k.id.clone()>{format!("{} ({})", k.name, k.algorithm)}</option> })
                            .collect_view()}
                    </select>
                </div>
                <fieldset class="space-y-2">
                    <legend class=LABEL>"Senha"</legend>
                    {had_password.then(|| view! {
                        <div class="flex flex-wrap gap-4 text-sm">
                            <label class="flex items-center gap-2">
                                <input type="radio" name="identity-password" class="accent-primary"
                                    prop:checked=move || mode.get() == PasswordMode::Keep
                                    on:change=move |_| mode.set(PasswordMode::Keep) />
                                "Manter a guardada"
                            </label>
                            <label class="flex items-center gap-2">
                                <input type="radio" name="identity-password" class="accent-primary"
                                    prop:checked=move || mode.get() == PasswordMode::Set
                                    on:change=move |_| mode.set(PasswordMode::Set) />
                                "Trocar"
                            </label>
                            <label class="flex items-center gap-2">
                                <input type="radio" name="identity-password" class="accent-primary"
                                    prop:checked=move || mode.get() == PasswordMode::Clear
                                    on:change=move |_| mode.set(PasswordMode::Clear) />
                                "Apagar"
                            </label>
                        </div>
                    })}
                    {move || (mode.get() == PasswordMode::Set).then(|| view! {
                        <input
                            type="password"
                            class=FIELD
                            aria-label="Senha"
                            autocomplete="new-password"
                            placeholder=if had_password { "Nova senha" } else { "Opcional: em branco, a senha é pedida ao conectar" }
                            prop:value=password
                            on:input=move |e| password.set(event_target_value(&e))
                        />
                    })}
                    <p class="flex items-center gap-1.5 text-xs text-muted">
                        <Svg icon=Icon::Lock size=12 />
                        "A senha é selada no cofre e não volta para esta tela."
                    </p>
                </fieldset>
                <FormError error=error />
            </div>
            <footer class="flex items-center gap-2 border-t border-line px-5 py-3">
                {(!is_new).then(|| view! {
                    <button type="button" class=DANGER on:click=delete>
                        <Svg icon=Icon::Trash size=16 />
                        "Apagar"
                    </button>
                })}
                <button type="button" class=format!("ml-auto {BUTTON}") on:click=move |_| close.run(())>"Cancelar"</button>
                <button type="button" class=PRIMARY disabled=busy on:click=save>
                    {move || if busy.get() { "Salvando…" } else { "Salvar" }}
                </button>
            </footer>
        </Dialog>
    }
}

#[component]
fn GenerateDialog(open: RwSignal<bool>) -> impl IntoView {
    let data = expect_context::<Data>();
    let name = RwSignal::new(String::new());
    let algorithm = RwSignal::new("ed25519".to_owned());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let created = RwSignal::new(None::<SshKey>);
    let close = Callback::new(move |()| open.set(false));

    let generate = move |_| {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() {
            error.set(Some("Dê um nome à chave.".to_owned()));
            return;
        }
        let algorithm = match algorithm.get_untracked().as_str() {
            "ecdsaP256" => KeyAlgorithm::EcdsaP256,
            "ecdsaP384" => KeyAlgorithm::EcdsaP384,
            "rsa4096" => KeyAlgorithm::Rsa4096,
            _ => KeyAlgorithm::Ed25519,
        };
        let params = GenerateKey {
            id: new_id(),
            name: n,
            algorithm,
            comment: String::new(),
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call(METHOD_KEY_GENERATE, &params)
                .await
                .and_then(|v| serde_json::from_value::<SshKey>(v).map_err(|e| e.to_string()));
            busy.set(false);
            match result {
                Ok(key) => {
                    created.set(Some(key));
                    data.reload();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <Dialog title="Gerar chave SSH".to_owned() on_close=close>
            <div class="space-y-4 overflow-auto p-5">
                {move || match created.get() {
                    Some(key) => view! { <CreatedKey key=key /> }.into_any(),
                    None => view! {
                        <div>
                            <label class=LABEL for="key-name">"Nome"</label>
                            <input
                                id="key-name"
                                class=format!("mt-1 {FIELD}")
                                placeholder="Notebook"
                                prop:value=name
                                on:input=move |e| name.set(event_target_value(&e))
                            />
                        </div>
                        <div>
                            <label class=LABEL for="key-algorithm">"Tipo"</label>
                            <select
                                id="key-algorithm"
                                class=format!("mt-1 {FIELD}")
                                prop:value=algorithm
                                on:change=move |e| algorithm.set(event_target_value(&e))
                            >
                                <option value="ed25519">"Ed25519 (recomendada)"</option>
                                <option value="ecdsaP256">"ECDSA P-256"</option>
                                <option value="ecdsaP384">"ECDSA P-384"</option>
                                <option value="rsa4096">"RSA 4096 (leva alguns segundos)"</option>
                            </select>
                        </div>
                        <button type="button" class=PRIMARY disabled=busy on:click=generate>
                            {move || if busy.get() { "Gerando…" } else { "Gerar" }}
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

/// Chave recém-criada: a pública para colar no `authorized_keys` do servidor.
#[component]
fn CreatedKey(key: SshKey) -> impl IntoView {
    let copied = RwSignal::new(false);
    let public = StoredValue::new(key.public_key.clone());
    view! {
        <p class="flex items-center gap-2 text-sm">
            <span class="text-success"><Svg icon=Icon::Check size=16 /></span>
            {format!("Chave {} pronta ({}).", key.name, key.fingerprint)}
        </p>
        <p class="text-sm text-muted">"Cole a chave pública no ~/.ssh/authorized_keys do servidor:"</p>
        <textarea readonly rows="4" class=format!("font-mono text-xs {FIELD}") aria-label="Chave pública">{key.public_key.clone()}</textarea>
        <button
            type="button"
            class=BUTTON
            on:click=move |_| {
                ui::copy(&public.get_value());
                copied.set(true);
            }
        >
            <Svg icon=Icon::Copy size=16 />
            {move || if copied.get() { "Copiada" } else { "Copiar chave pública" }}
        </button>
    }
}

#[component]
fn ImportKeyDialog(open: RwSignal<bool>) -> impl IntoView {
    let data = expect_context::<Data>();
    let name = RwSignal::new(String::new());
    let text = RwSignal::new(String::new());
    let passphrase = RwSignal::new(String::new());
    let needs_passphrase = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let close = Callback::new(move |()| open.set(false));

    let import = move |_| {
        let n = name.get_untracked().trim().to_owned();
        if n.is_empty() || text.with_untracked(|t| t.trim().is_empty()) {
            error.set(Some("Dê um nome e cole a chave privada.".to_owned()));
            return;
        }
        let params = ImportKey {
            id: new_id(),
            name: n,
            private_key: text.get_untracked(),
            passphrase: Some(passphrase.get_untracked()).filter(|p| !p.is_empty()),
        };
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            let result = ui::call_coded(METHOD_KEY_IMPORT, &params).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    data.reload();
                    open.set(false);
                }
                Err((code, message)) => {
                    if code == "key_passphrase_required" {
                        needs_passphrase.set(true);
                    }
                    error.set(Some(message));
                }
            }
        });
    };

    view! {
        <Dialog title="Importar chave SSH".to_owned() on_close=close>
            <div class="space-y-4 overflow-auto p-5">
                <div>
                    <label class=LABEL for="import-key-name">"Nome"</label>
                    <input
                        id="import-key-name"
                        class=format!("mt-1 {FIELD}")
                        placeholder="Chave do trabalho"
                        prop:value=name
                        on:input=move |e| name.set(event_target_value(&e))
                    />
                </div>
                <div>
                    <label class=LABEL for="import-key-text">"Chave privada (OpenSSH ou PuTTY .ppk)"</label>
                    <textarea
                        id="import-key-text"
                        rows="8"
                        spellcheck="false"
                        class=format!("mt-1 font-mono text-xs {FIELD}")
                        placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                        prop:value=text
                        on:input=move |e| text.set(event_target_value(&e))
                    ></textarea>
                </div>
                {move || needs_passphrase.get().then(|| view! {
                    <div>
                        <label class=LABEL for="import-key-pass">"Passphrase"</label>
                        <input
                            id="import-key-pass"
                            type="password"
                            autocomplete="off"
                            class=format!("mt-1 {FIELD}")
                            prop:value=passphrase
                            on:input=move |e| passphrase.set(event_target_value(&e))
                        />
                    </div>
                })}
                <p class="flex items-center gap-1.5 text-xs text-muted">
                    <Svg icon=Icon::Lock size=12 />
                    "A chave é guardada sem a passphrase, selada pelo cofre."
                </p>
                <FormError error=error />
            </div>
            <footer class="flex justify-end gap-2 border-t border-line px-5 py-3">
                <button type="button" class=BUTTON on:click=move |_| close.run(())>"Cancelar"</button>
                <button type="button" class=PRIMARY disabled=busy on:click=import>
                    {move || if busy.get() { "Importando…" } else { "Importar" }}
                </button>
            </footer>
        </Dialog>
    }
}
