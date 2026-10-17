//! O serviço `codatos-sensors`: atende o pipe até o Windows pedir para parar.

use std::ffi::OsString;
use std::io;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::oneshot;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

use crate::sampler::Sampler;
use crate::{PIPE_NAME, Reply, SERVICE_NAME};

define_windows_service!(ffi_service_main, service_main);

/// Entrega o processo ao gerenciador de serviços (só funciona iniciado por ele).
pub fn run() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

/// Uma leitura em JSON, para diagnóstico (`codatos.exe --sensors-once` como administrador).
#[must_use]
pub fn once() -> String {
    serde_json::to_string(&Sampler::new().sample()).unwrap_or_default()
}

fn service_main(_args: Vec<OsString>) {
    // Sem onde mostrar o erro: o gerenciador de serviços registra a parada.
    let _ = run_service();
}

fn run_service() -> windows_service::Result<()> {
    let (stop_tx, stop_rx) = oneshot::channel();
    let mut stop_tx = Some(stop_tx);
    let status = service_control_handler::register(SERVICE_NAME, move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            if let Some(tx) = stop_tx.take() {
                let _ = tx.send(());
            }
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })?;
    let set = |state, accept| {
        status.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
    };
    set(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    )?;

    let served = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .and_then(|rt| rt.block_on(serve(stop_rx)));
    set(ServiceState::Stopped, ServiceControlAccept::empty())?;
    served.map_err(windows_service::Error::Winapi)
}

/// Responde cada conexão com uma leitura e desconecta. Sem clientes, fica parado.
async fn serve(mut stop: oneshot::Receiver<()>) -> io::Result<()> {
    let mut sampler = Sampler::new();
    // `first_pipe_instance` falha se outro processo já criou o pipe com este nome.
    let mut server = options().first_pipe_instance(true).create(PIPE_NAME)?;
    loop {
        tokio::select! {
            _ = &mut stop => return Ok(()),
            connected = server.connect() => {
                let client = std::mem::replace(&mut server, options().create(PIPE_NAME)?);
                if connected.is_ok() {
                    reply(client, &sampler.sample()).await;
                }
            }
        }
    }
}

/// Só saída: o serviço nunca lê o que o cliente escreve. Clientes remotos ficam de fora
/// (padrão do tokio) e a ACL padrão dá leitura a todos os usuários locais.
fn options() -> ServerOptions {
    let mut options = ServerOptions::new();
    options.access_inbound(false).access_outbound(true);
    options
}

async fn reply(mut client: NamedPipeServer, reply: &Reply) {
    let mut line = serde_json::to_vec(reply).expect("Reply sempre serializa");
    line.push(b'\n');
    let write = async {
        client.write_all(&line).await?;
        client.flush().await
    };
    // Um cliente que não lê não segura o serviço. Fechar (em vez de `disconnect`, que
    // descarta o que o cliente ainda não leu) deixa a linha no buffer até ele ler.
    let _ = tokio::time::timeout(Duration::from_secs(1), write).await;
    drop(client);
}
