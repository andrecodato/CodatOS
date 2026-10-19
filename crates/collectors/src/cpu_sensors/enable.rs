//! `system.enableCpuSensors`: instala o PawnIO e o serviço `codatos-sensors` com um único
//! pedido de administrador (Q-13).
//!
//! O instalador do PawnIO vem de uma versão fixada e só roda se o SHA-256 bater, conferido
//! aqui e de novo no script elevado (o `%TEMP%` é gravável pelo usuário). O serviço é uma
//! cópia do próprio `codatos.exe` em `%ProgramFiles%\CodatOS\sensors`, onde só
//! administradores gravam.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use base64::Engine;
use codatos_core::{Method, MethodError, MethodFuture};
use codatos_protocol::system::METHOD_ENABLE_CPU_SENSORS;
use codatos_sensors::ryzen::CpuId;
use codatos_sensors::{NOT_INSTALLED, Reply, SERVICE_ARG, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::http;

/// PawnIO 2.2.0 (`namazso/PawnIO.Setup`), o mesmo hash do manifesto do winget.
const SETUP_URL: &str =
    "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe";
const SETUP_SHA256: &str = "1F519A22E47187F70A1379A48CA604981C4FCF694F4E65B734AAA74A9FBA3032";
const MAX_SETUP: usize = 32 << 20;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// `ERROR_CANCELLED`: o usuário recusou o pedido de administrador.
const CANCELLED: i32 = 1223;
const HASH_MISMATCH: i32 = 10;
const SETUP_FAILED: i32 = 11;

#[derive(Debug)]
pub(crate) struct EnableCpuSensors;

impl Method for EnableCpuSensors {
    fn name(&self) -> &'static str {
        METHOD_ENABLE_CPU_SENSORS
    }

    fn call(&self, _params: Value) -> MethodFuture<'_> {
        Box::pin(async {
            if !not_installed(&codatos_sensors::client::read().await) {
                return Ok(Value::Null);
            }
            let cpu = CpuId::current();
            if !cpu.supported() {
                return Err(MethodError::new(
                    codatos_sensors::UNSUPPORTED,
                    format!("Por enquanto só Ryzen: {} ainda não tem leitura", cpu.brand),
                ));
            }
            let dir = std::env::temp_dir().join("codatos-sensors");
            tokio::fs::create_dir_all(&dir).await.map_err(io_error)?;
            let setup = if pawnio_installed() {
                None
            } else {
                Some(download_setup(&dir).await?)
            };
            let exe = std::env::current_exe().map_err(io_error)?;
            let script = dir.join("enable.ps1");
            // BOM: o PowerShell 5.1 lê arquivo sem BOM na página de código ANSI.
            let mut body = "\u{feff}".to_owned();
            body.push_str(&script_body(setup.as_deref(), &exe));
            tokio::fs::write(&script, body).await.map_err(io_error)?;
            run_elevated(&script).await?;
            wait_for_service().await
        })
    }
}

fn not_installed(reply: &Reply) -> bool {
    matches!(reply, Reply::Error { code, .. } if code == NOT_INSTALLED)
}

/// Chave de desinstalação que o instalador do PawnIO cria (vista de 64 e de 32 bits).
fn pawnio_installed() -> bool {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};

    let path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";
    [KEY_WOW64_64KEY, KEY_WOW64_32KEY].into_iter().any(|view| {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(path, KEY_READ | view)
            .and_then(|k| k.get_value::<String, _>("DisplayVersion"))
            .is_ok()
    })
}

async fn download_setup(dir: &Path) -> Result<PathBuf, MethodError> {
    let failed = |e: &dyn std::fmt::Display| {
        MethodError::new(
            "pawnio_download_failed",
            format!("Não foi possível baixar o PawnIO: {e}"),
        )
    };
    let response = http::client()
        .get(SETUP_URL)
        .timeout(Duration::from_mins(2))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| failed(&e))?;
    let bytes = response.bytes().await.map_err(|e| failed(&e))?;
    if bytes.len() > MAX_SETUP || !hash_matches(&bytes) {
        return Err(failed(&"o arquivo não confere com a versão esperada"));
    }
    let path = dir.join("PawnIO_setup.exe");
    tokio::fs::write(&path, &bytes).await.map_err(io_error)?;
    Ok(path)
}

fn hash_matches(bytes: &[u8]) -> bool {
    use std::fmt::Write;

    let hex = Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02X}");
            s
        });
    hex == SETUP_SHA256
}

/// Texto literal do PowerShell entre aspas simples.
fn ps_literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Script que roda elevado: PawnIO (se preciso) e o serviço, criado ou atualizado.
fn script_body(setup: Option<&Path>, exe: &Path) -> String {
    let setup = setup.map_or_else(|| "''".to_owned(), |p| ps_literal(&p.to_string_lossy()));
    let source = ps_literal(&exe.to_string_lossy());
    let name = ps_literal(SERVICE_NAME);
    let display = ps_literal(SERVICE_DISPLAY_NAME);
    let arg = SERVICE_ARG;
    format!(
        r#"$ErrorActionPreference = 'Stop'
$Setup = {setup}
$Source = {source}
$Name = {name}
if ($Setup) {{
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $Setup).Hash -ne '{SETUP_SHA256}') {{ exit {HASH_MISMATCH} }}
    $p = Start-Process -FilePath $Setup -ArgumentList '-install', '-silent' -Wait -PassThru
    if ($p.ExitCode -ne 0 -and $p.ExitCode -ne 3010) {{ exit {SETUP_FAILED} }}
}}
$Dir = Join-Path $env:ProgramFiles 'CodatOS\sensors'
New-Item -ItemType Directory -Force -Path $Dir | Out-Null
$Exe = Join-Path $Dir 'codatos-sensors.exe'
$Bin = '"' + $Exe + '" {arg}'
$Existing = Get-Service -Name $Name -ErrorAction SilentlyContinue
if ($Existing) {{
    Stop-Service -Name $Name -Force -ErrorAction SilentlyContinue
}}
# O processo do serviço pode levar um instante para soltar o .exe.
for ($i = 0; ; $i++) {{
    try {{ Copy-Item -Force -LiteralPath $Source -Destination $Exe; break }}
    catch {{ if ($i -ge 10) {{ throw }}; Start-Sleep -Milliseconds 300 }}
}}
if ($Existing) {{
    Set-Service -Name $Name -StartupType Automatic
}} else {{
    New-Service -Name $Name -DisplayName {display} -BinaryPathName $Bin -StartupType Automatic `
        -Description 'Le a temperatura e a potencia da CPU pelo PawnIO para o CodatOS.' | Out-Null
}}
Start-Service -Name $Name
exit 0
"#
    )
}

/// Abre o pedido de administrador (UAC) e espera o script terminar.
async fn run_elevated(script: &Path) -> Result<(), MethodError> {
    let file = ps_literal(&format!("\"{}\"", script.to_string_lossy()));
    let outer = format!(
        "try {{ $p = Start-Process -FilePath powershell.exe -Verb RunAs -Wait -PassThru \
         -WindowStyle Hidden -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', \
         '-File', {file}; exit $p.ExitCode }} catch {{ exit {CANCELLED} }}"
    );
    let utf16: Vec<u8> = outer.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
    let status = tokio::time::timeout(
        // O pedido do UAC espera o usuário.
        Duration::from_mins(10),
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .kill_on_drop(true)
            .status(),
    )
    .await
    .map_err(|_| MethodError::new("cancelled", "O pedido de administrador expirou"))?
    .map_err(io_error)?;
    match status.code() {
        Some(0) => Ok(()),
        Some(CANCELLED) => Err(MethodError::new(
            "cancelled",
            "Pedido de administrador recusado",
        )),
        Some(HASH_MISMATCH) => Err(MethodError::new(
            "pawnio_download_failed",
            "O instalador do PawnIO mudou antes de rodar; tente de novo",
        )),
        Some(SETUP_FAILED) => Err(MethodError::new(
            "pawnio_setup_failed",
            "O instalador do PawnIO falhou",
        )),
        code => Err(MethodError::new(
            "cpu_sensors_install_failed",
            format!("A instalação do serviço de sensores falhou (código {code:?})"),
        )),
    }
}

/// O serviço sobe em instantes; o driver recém-instalado pode pedir reinício.
async fn wait_for_service() -> Result<Value, MethodError> {
    let mut last = None;
    for _ in 0..20 {
        match codatos_sensors::client::read().await {
            Reply::Ok { .. } => return Ok(Value::Null),
            Reply::Error { code, title } if code != NOT_INSTALLED => {
                last = Some(MethodError::new(
                    code,
                    format!("{title}. Se o PawnIO acabou de ser instalado, reinicie o Windows."),
                ));
            }
            Reply::Error { .. } => {}
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(last.unwrap_or_else(|| {
        MethodError::new(
            "cpu_sensors_install_failed",
            "O serviço de sensores foi instalado mas não respondeu",
        )
    }))
}

#[allow(clippy::needless_pass_by_value)] // usado em `map_err`
fn io_error(e: std::io::Error) -> MethodError {
    MethodError::new("cpu_sensors_install_failed", e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_do_powershell() {
        assert_eq!(
            ps_literal(r"C:\Users\André Codato"),
            r"'C:\Users\André Codato'"
        );
        assert_eq!(ps_literal("O'Brien"), "'O''Brien'");
    }

    #[test]
    fn script_sem_instalador_nao_roda_setup() {
        let s = script_body(None, Path::new(r"C:\Users\A B\codatos.exe"));
        assert!(s.contains("$Setup = ''"));
        assert!(s.contains(r"$Source = 'C:\Users\A B\codatos.exe'"));
        assert!(s.contains("--sensors-service"));
        assert!(s.contains(SETUP_SHA256));
    }

    #[test]
    fn hash_do_instalador() {
        assert!(!hash_matches(b"outro arquivo"));
    }
}
