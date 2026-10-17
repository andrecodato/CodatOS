//! Distros WSL (RF-SYS-020 a 024). Só no Windows.
//!
//! A lista, a versão, a distro padrão e a pasta de cada uma vêm do Registro
//! (`HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss`); quais estão ligadas, de
//! `wsl.exe --list --running --quiet`. A saída verbosa do `wsl.exe` é traduzida conforme
//! o idioma do Windows ("Em execução"), por isso não é usada.
//!
//! **Uma distro parada nunca é ligada pela coleta** (RF-SYS-024): comandos dentro da
//! distro (`df`) só rodam nas que acabaram de aparecer como ligadas.

#[cfg(windows)]
mod collector;
#[cfg(any(windows, test))]
mod parse;

#[cfg(windows)]
pub use collector::WslCollector;
