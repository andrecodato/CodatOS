//! Shell Tauri do CodatOS (ADR-0003): sobe o núcleo no runtime Tokio do próprio
//! processo, mantém o app na bandeja e cria/destrói a janela sob demanda.

mod autostart;
mod settings;
mod tray;
mod window;

use std::sync::{Arc, OnceLock};

use codatos_core::{DEFAULT_PORT, ServerConfig};
use tauri::{AppHandle, Manager, RunEvent};

/// Token do app, preenchido quando o núcleo termina de subir.
#[derive(Default, Clone)]
struct CoreToken(Arc<OnceLock<String>>);

/// Entrega à janela o token do app (IPC `get_session_token`, ver `codatos-protocol`).
/// Só a janela do próprio app tem essa permissão (`capabilities/default.json`).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // o Tauri entrega `State` por valor
fn get_session_token(token: tauri::State<'_, CoreToken>) -> Result<String, String> {
    token
        .0
        .get()
        .cloned()
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
        .manage(CoreToken::default())
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
        let config = ServerConfig {
            port: DEFAULT_PORT,
            ui_dir: None,
            extra_origins: dev_origins(),
            methods: store
                .as_ref()
                .map(codatos_store::methods)
                .unwrap_or_default()
                .into_iter()
                .chain(codatos_collectors::methods())
                .collect(),
            collectors: codatos_collectors::all(store.map(settings_reader)),
        };
        match codatos_core::start(config).await {
            Ok(server) => {
                let _ = app
                    .state::<CoreToken>()
                    .0
                    .set(server.app_token().to_owned());
                // O servidor para quando o processo termina.
                std::future::pending::<()>().await;
                drop(server);
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
