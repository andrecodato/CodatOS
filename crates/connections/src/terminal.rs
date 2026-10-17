//! Terminais locais (PowerShell, cmd, `$SHELL`), WSL e Docker num PTY (`portable-pty`,
//! `ConPTY` no Windows), ligados a um canal binário do WebSocket. Sessões SSH ficam em
//! [`crate::ssh`].
//!
//! O PTY é bloqueante: uma thread lê a saída e outra escreve a entrada; o canal faz a
//! ponte com o runtime. O conteúdo do terminal nunca vai para log (R-06).

use std::io::{Read, Write};
use std::sync::mpsc as std_mpsc;

use codatos_core::{Channel, ChannelEvent, ChannelMethod, MethodError, MethodFuture, params};
use codatos_protocol::terminal::{
    LocalShell, METHOD_TERMINAL_OPEN, OpenTerminal, TerminalTarget, errors,
};
use codatos_store::{Store, Vault};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use zeroize::Zeroizing;

use crate::ssh::{self, Ssh};

/// Saída lida do PTY por vez.
const READ_CHUNK: usize = 16 * 1024;
/// Pedaços de saída esperando o canal; a thread de leitura para quando enche.
const OUTPUT_QUEUE: usize = 16;

/// `terminal.open`.
#[derive(Debug, Default)]
pub struct OpenTerminalMethod {
    ssh: Option<Ssh>,
}

impl OpenTerminalMethod {
    /// Com banco e cofre, abre também sessões SSH; sem eles, só terminais deste PC.
    pub fn new(connections: Option<(Store, Vault)>) -> Self {
        Self {
            ssh: connections.map(|(store, vault)| Ssh { store, vault }),
        }
    }
}

impl ChannelMethod for OpenTerminalMethod {
    fn name(&self) -> &'static str {
        METHOD_TERMINAL_OPEN
    }

    fn open(&self, value: Value, channel: Channel) -> MethodFuture<'_> {
        Box::pin(async move {
            let p: OpenTerminal = params(value)?;
            if let TerminalTarget::Ssh {
                host_id,
                username,
                password,
                trust_host_key,
            } = p.target
            {
                let ssh = self.ssh.as_ref().ok_or_else(|| {
                    MethodError::new("vault_absent", "Banco local ou cofre indisponível")
                })?;
                let request = ssh::Request {
                    host_id,
                    username,
                    password: password.map(Zeroizing::new),
                    trust_host_key,
                    cols: p.cols,
                    rows: p.rows,
                };
                return ssh.open(request, channel).await;
            }
            let (command, title) = command_for(&p.target)?;
            let size = PtySize {
                rows: p.rows.clamp(2, 500),
                cols: p.cols.clamp(10, 1000),
                pixel_width: 0,
                pixel_height: 0,
            };
            // Abrir o PTY e iniciar o processo bloqueia um pouco (`ConPTY`).
            let pty = tokio::task::spawn_blocking(move || spawn(command, size))
                .await
                .map_err(|_| spawn_failed(&"tarefa interrompida"))??;
            tokio::spawn(pump(pty, channel));
            Ok(json!({ "title": title }))
        })
    }
}

/// Processo rodando num PTY.
struct Pty {
    master: Box<dyn MasterPty + Send>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

fn spawn(command: CommandBuilder, size: PtySize) -> Result<Pty, MethodError> {
    let pair = native_pty_system()
        .openpty(size)
        .map_err(|e| spawn_failed(&e))?;
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|e| spawn_failed(&e))?;
    // Sem o lado escravo aberto aqui, o leitor recebe fim de arquivo quando o processo sai.
    drop(pair.slave);
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| spawn_failed(&e))?;
    let writer = pair.master.take_writer().map_err(|e| spawn_failed(&e))?;
    Ok(Pty {
        master: pair.master,
        reader,
        writer,
        child,
    })
}

/// Liga o PTY ao canal até um dos lados acabar.
async fn pump(pty: Pty, mut channel: Channel) {
    let Pty {
        master,
        mut reader,
        mut writer,
        mut child,
    } = pty;
    let mut killer = child.clone_killer();

    // Saída do processo → canal.
    let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(OUTPUT_QUEUE);
    std::thread::spawn(move || {
        let mut buf = vec![0_u8; READ_CHUNK];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Canal → entrada do processo.
    let (in_tx, in_rx) = std_mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        while let Ok(data) = in_rx.recv() {
            if writer
                .write_all(&data)
                .and_then(|()| writer.flush())
                .is_err()
            {
                break;
            }
        }
    });

    // Espera o processo sair numa thread, para saber quando fechar o canal.
    let (exit_tx, mut exit_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = exit_tx.send(());
    });

    let sender = channel.sender.clone();
    let output = tokio::spawn(async move {
        while let Some(data) = out_rx.recv().await {
            if sender.send(&data).await.is_err() {
                return;
            }
        }
    });

    loop {
        tokio::select! {
            event = channel.incoming.recv() => match event {
                Some(ChannelEvent::Data(data)) => {
                    if in_tx.send(data).is_err() {
                        break;
                    }
                }
                Some(ChannelEvent::Resize { cols, rows }) => {
                    let _ = master.resize(PtySize {
                        rows: rows.clamp(2, 500),
                        cols: cols.clamp(10, 1000),
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                // A UI fechou a aba ou a conexão: encerra o processo.
                None => {
                    let _ = killer.kill();
                    break;
                }
            },
            _ = &mut exit_rx => break,
        }
    }
    // Entrega o que o processo escreveu antes de sair, depois avisa o fim.
    let _ = output.await;
    channel.sender.close().await;
}

fn spawn_failed(e: &dyn std::fmt::Display) -> MethodError {
    MethodError::new(
        errors::SPAWN_FAILED,
        format!("Não foi possível abrir o terminal: {e}"),
    )
}

/// Nome de distro ou container que vira argumento: sem espaços nem cara de opção.
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Comando e título da aba para o alvo.
fn command_for(target: &TerminalTarget) -> Result<(CommandBuilder, String), MethodError> {
    let (mut command, title) = match target {
        TerminalTarget::Local { shell } => local_shell(shell.unwrap_or(LocalShell::Default))?,
        TerminalTarget::Wsl { distro } => {
            if !cfg!(windows) {
                return Err(MethodError::invalid_params("WSL só existe no Windows"));
            }
            if !safe_name(distro) {
                return Err(MethodError::invalid_params("Nome de distro inválido"));
            }
            let mut c = CommandBuilder::new("wsl.exe");
            c.args(["-d", distro.as_str(), "--cd", "~"]);
            (c, distro.clone())
        }
        TerminalTarget::Docker { container } => {
            if !safe_name(container) {
                return Err(MethodError::invalid_params("Nome de container inválido"));
            }
            let mut c = CommandBuilder::new("docker");
            c.args([
                "exec",
                "-it",
                container.as_str(),
                "/bin/sh",
                "-c",
                "if command -v bash >/dev/null 2>&1; then exec bash; else exec sh; fi",
            ]);
            (c, container.clone())
        }
        TerminalTarget::Ssh { .. } => unreachable!("sessões SSH não usam PTY local"),
    };
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    if let Some(home) = std::env::home_dir() {
        command.cwd(home);
    }
    Ok((command, title))
}

fn local_shell(shell: LocalShell) -> Result<(CommandBuilder, String), MethodError> {
    let (program, title) = match shell {
        LocalShell::Powershell | LocalShell::Default if cfg!(windows) => {
            ("powershell.exe".to_owned(), "PowerShell")
        }
        LocalShell::Pwsh => (
            if cfg!(windows) { "pwsh.exe" } else { "pwsh" }.to_owned(),
            "PowerShell 7",
        ),
        LocalShell::Cmd if cfg!(windows) => ("cmd.exe".to_owned(), "Prompt de Comando"),
        LocalShell::Default => (
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()),
            "Terminal",
        ),
        LocalShell::Powershell | LocalShell::Cmd => {
            return Err(MethodError::invalid_params("Shell só existe no Windows"));
        }
    };
    let mut command = CommandBuilder::new(program);
    if !cfg!(windows) && shell == LocalShell::Default {
        // Shell de login, como um terminal comum abre.
        command.arg("-l");
    }
    if cfg!(windows) && matches!(shell, LocalShell::Powershell | LocalShell::Default) {
        command.arg("-NoLogo");
    }
    Ok((command, title.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_seguros() {
        assert!(safe_name("Ubuntu-24.04"));
        assert!(safe_name("meu_app.web-1"));
        assert!(!safe_name("-it"));
        assert!(!safe_name("a b"));
        assert!(!safe_name("a;rm"));
        assert!(!safe_name(""));
    }

    #[test]
    fn alvos_invalidos() {
        let err = command_for(&TerminalTarget::Docker {
            container: "--privileged".into(),
        })
        .unwrap_err();
        assert_eq!(err.code(), "invalid_params");
        #[cfg(not(windows))]
        assert!(
            command_for(&TerminalTarget::Wsl {
                distro: "Ubuntu".into()
            })
            .is_err()
        );
        #[cfg(not(windows))]
        assert!(local_shell(LocalShell::Cmd).is_err());
    }
}
