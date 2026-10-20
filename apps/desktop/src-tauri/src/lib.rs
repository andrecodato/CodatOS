//! Shell Tauri do CodatOS (ADR-0003): sobe o núcleo no runtime Tokio do próprio
//! processo, mantém o app na bandeja e cria/destrói a janela sob demanda.

mod autostart;
mod settings;
mod tray;
mod ui_assets;
mod window;

use std::sync::{Arc, OnceLock};

use codatos_core::{DEFAULT_PORT, RunningServer, ServerConfig};
use tauri::{AppHandle, Manager, RunEvent};

/// API local, preenchida quando o núcleo termina de subir. Fica guardada aqui até o app sair.
#[derive(Default, Clone)]
pub(crate) struct Core(Arc<OnceLock<RunningServer>>);

impl Core {
    /// Endereço de uso único para abrir a UI no navegador (tray "Abrir no navegador").
    pub(crate) fn browser_url(&self) -> Option<String> {
        let server = self.0.get()?;
        Some(format!(
            "http://127.0.0.1:{}/?token={}",
            server.local_addr().port(),
            server.issue_browser_token()
        ))
    }
}

/// Entrega à janela o token do app (IPC `get_session_token`, ver `codatos-protocol`).
/// Só a janela do próprio app tem essa permissão (`capabilities/default.json`).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // o Tauri entrega `State` por valor
fn get_session_token(core: tauri::State<'_, Core>) -> Result<String, String> {
    core.0
        .get()
        .map(|server| server.app_token().to_owned())
        .ok_or_else(|| "núcleo iniciando".to_owned())
}

/// Roda o app até o usuário escolher "Sair".
///
/// # Panics
///
/// Se o Tauri não conseguir iniciar (sem o `WebView2` no Windows, por exemplo).
pub fn run() {
    let autostarted = std::env::args().any(|arg| arg == autostart::FLAG);

    let app = tauri::Builder::default()
        // Primeiro plugin: uma segunda execução só traz a janela da primeira.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::show(app);
        }))
        .plugin(autostart::plugin())
        .plugin(window::state_plugin())
        // Links dos widgets abrem no navegador padrão (só http/https, ver capabilities).
        .plugin(tauri_plugin_opener::init())
        .manage(Core::default())
        .invoke_handler(tauri::generate_handler![get_session_token])
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut prefs = settings::load(&handle);
            autostart::configure_first_run(&handle, &mut prefs);
            tray::create(&handle)?;

            let delay = if autostarted {
                prefs.autostart_delay()
            } else {
                window::show(&handle);
                std::time::Duration::ZERO
            };
            start_core(handle, delay);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("falha ao iniciar o Tauri");

    app.run(|_app, event| {
        // Fechar a última janela não encerra o app: ele segue na bandeja (RF-APP-017).
        // Só "Sair" no menu da bandeja chama `exit(0)`, que chega aqui com `code`.
        if let RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}

/// Banco local em `<dados do app>/codatos.db`. Sem ele, o app abre sem dashboards salvos.
async fn open_store(app: &AppHandle) -> Option<codatos_store::Store> {
    let path = app.path().app_data_dir().ok()?.join("codatos.db");
    match codatos_store::Store::open(&path).await {
        Ok(store) => Some(store),
        Err(error) => {
            tracing::error!(%error, path = %path.display(), "banco local não abriu");
            None
        }
    }
}

/// Cofre local (header no banco, chave no keychain no modo sessão). Sem ele, as conexões
/// não guardam segredos.
async fn open_vault(store: codatos_store::Store) -> Option<codatos_store::Vault> {
    match codatos_store::Vault::open(store, codatos_store::os_keychain()).await {
        Ok(vault) => Some(vault),
        Err(error) => {
            tracing::error!(%error, "cofre local não abriu");
            None
        }
    }
}

/// Leitura de preferências do banco, para os coletores (projetos, cidade do clima).
fn settings_reader(store: codatos_store::Store) -> codatos_collectors::SettingsReader {
    std::sync::Arc::new(move |key, scope| {
        let store = store.clone();
        Box::pin(async move { store.setting(key, scope).await.ok().flatten() })
    })
}

/// Sobe a API local depois do atraso de autostart e a mantém viva até o app sair.
fn start_core(app: AppHandle, delay: std::time::Duration) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        let store = open_store(&app).await;
        let vault = match &store {
            Some(store) => open_vault(store.clone()).await,
            None => None,
        };
        let ui_dir = {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || ui_assets::extract(&app))
                .await
                .ok()
                .flatten()
        };
        let terminal =
            codatos_connections::OpenTerminalMethod::new(store.clone().zip(vault.clone()));
        let sftp = store
            .as_ref()
            .zip(vault.as_ref())
            .map(|(s, v)| codatos_connections::sftp_methods(s, v));
        let config = ServerConfig {
            port: DEFAULT_PORT,
            ui_dir,
            extra_origins: dev_origins(),
            methods: store
                .as_ref()
                .map(codatos_store::methods)
                .unwrap_or_default()
                .into_iter()
                .chain(vault.iter().flat_map(codatos_store::vault_methods))
                .chain(
                    store
                        .as_ref()
                        .zip(vault.as_ref())
                        .map(|(s, v)| {
                            let mut methods = codatos_store::connection_methods(s, v);
                            methods.extend(codatos_connections::key_methods(s, v));
                            methods.extend(codatos_connections::ssh_config_methods(s, v));
                            if let Some((_, sftp)) = &sftp {
                                methods.extend(sftp.iter().cloned());
                            }
                            methods
                        })
                        .unwrap_or_default(),
                )
                .chain(store.iter().flat_map(codatos_store::snippet_methods))
                .chain(codatos_connections::local_file_methods())
                .chain(codatos_collectors::methods())
                .collect(),
            collectors: codatos_collectors::all(store.map(settings_reader))
                .into_iter()
                .chain(vault.map(|v| {
                    std::sync::Arc::new(codatos_store::VaultCollector(v))
                        as std::sync::Arc<dyn codatos_core::Collector>
                }))
                .collect(),
            channel_methods: std::iter::once(
                std::sync::Arc::new(terminal) as std::sync::Arc<dyn codatos_core::ChannelMethod>
            )
            .chain(sftp.map(|(open, _)| open))
            .collect(),
        };
        match codatos_core::start(config).await {
            // O servidor para quando o processo termina.
            Ok(server) => {
                let _ = app.state::<Core>().0.set(server);
            }
            Err(error) => tracing::error!(%error, "núcleo não subiu"),
        }
    });
}

/// No `cargo tauri dev` a janela carrega do `trunk serve` (devUrl), outra origem.
fn dev_origins() -> Vec<String> {
    if cfg!(debug_assertions) {
        vec!["http://127.0.0.1:1420".to_owned()]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nome_do_comando_bate_com_o_protocolo() {
        assert_eq!(
            codatos_protocol::api::IPC_GET_SESSION_TOKEN,
            "get_session_token"
        );
    }
}
