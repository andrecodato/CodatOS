//! Cliente do driver PawnIO (`\\?\GLOBALROOT\Device\PawnIO`).
//!
//! Cada handle carrega um módulo assinado (`IOCTL_PIO_LOAD_BINARY`) e chama as funções dele
//! pelo nome (`IOCTL_PIO_EXECUTE_FN`): a entrada é o nome em 32 bytes ASCII seguido de
//! `i64`s, e a saída, `i64`s. Abrir o dispositivo exige administrador.

#![allow(unsafe_code)] // chamadas Win32 com buffers que este módulo controla

use std::ffi::c_void;
use std::fmt;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, GENERIC_READ,
    GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const DEVICE: &str = r"\\?\GLOBALROOT\Device\PawnIO";
const DEVICE_TYPE: u32 = 41394 << 16;
const IOCTL_LOAD_BINARY: u32 = DEVICE_TYPE | (0x821 << 2);
const IOCTL_EXECUTE: u32 = DEVICE_TYPE | (0x841 << 2);
const NAME_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    /// O driver não está instalado.
    NotInstalled,
    /// O processo não roda como administrador.
    AccessDenied,
    /// O driver recusou o módulo (CPU errada, módulo sem assinatura válida).
    Load(u32),
    /// Uma chamada ao módulo falhou.
    Execute(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => f.write_str("o PawnIO não está instalado"),
            Self::AccessDenied => f.write_str("o PawnIO exige administrador"),
            Self::Load(code) => write!(f, "o PawnIO recusou o módulo (erro {code})"),
            Self::Execute(code) => write!(f, "o módulo do PawnIO falhou (erro {code})"),
        }
    }
}

/// Handle do PawnIO com um módulo carregado.
pub(crate) struct PawnIo {
    handle: HANDLE,
}

// O handle é de kernel e o driver serializa as chamadas; quem usa guarda o `PawnIo` atrás de
// um `&mut`, então não há chamadas simultâneas por este lado.
unsafe impl Send for PawnIo {}

impl PawnIo {
    pub(crate) fn open(module: &[u8]) -> Result<Self, Error> {
        let path: Vec<u16> = DEVICE.encode_utf16().chain([0]).collect();
        // SAFETY: `path` termina em NUL e vive até o fim da chamada.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            // SAFETY: sem pré-condições.
            return Err(match unsafe { GetLastError() } {
                ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Error::NotInstalled,
                ERROR_ACCESS_DENIED => Error::AccessDenied,
                code => Error::Load(code),
            });
        }
        let pawnio = Self { handle };
        let len = u32::try_from(module.len()).map_err(|_| Error::Load(0))?;
        // SAFETY: o buffer de entrada tem `len` bytes; não há saída.
        let ok = unsafe {
            DeviceIoControl(
                pawnio.handle,
                IOCTL_LOAD_BINARY,
                module.as_ptr().cast::<c_void>(),
                len,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            // SAFETY: sem pré-condições.
            return Err(Error::Load(unsafe { GetLastError() }));
        }
        Ok(pawnio)
    }

    /// Chama `name` no módulo e devolve até `out_len` valores.
    pub(crate) fn execute(
        &mut self,
        name: &str,
        input: &[i64],
        out_len: usize,
    ) -> Result<Vec<i64>, Error> {
        let request = request(name, input);
        let mut output = vec![0_i64; out_len];
        let mut returned = 0_u32;
        let in_len = u32::try_from(request.len()).map_err(|_| Error::Execute(0))?;
        let out_bytes = u32::try_from(std::mem::size_of_val(output.as_slice()))
            .map_err(|_| Error::Execute(0))?;
        // SAFETY: os tamanhos passados são os dos buffers, que vivem até o fim da chamada.
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_EXECUTE,
                request.as_ptr().cast::<c_void>(),
                in_len,
                output.as_mut_ptr().cast::<c_void>(),
                out_bytes,
                &raw mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            // SAFETY: sem pré-condições.
            return Err(Error::Execute(unsafe { GetLastError() }));
        }
        output.truncate(returned as usize / size_of::<i64>());
        Ok(output)
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        // SAFETY: o handle veio de `CreateFileW` e só é fechado aqui.
        unsafe { CloseHandle(self.handle) };
    }
}

/// Nome em 32 bytes ASCII (com NUL no fim) seguido dos argumentos em little-endian.
fn request(name: &str, input: &[i64]) -> Vec<u8> {
    let mut buf = vec![0_u8; NAME_LEN];
    let name = name.as_bytes();
    let n = name.len().min(NAME_LEN - 1);
    buf[..n].copy_from_slice(&name[..n]);
    buf.extend(input.iter().flat_map(|v| v.to_le_bytes()));
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_do_pedido() {
        let r = request("ioctl_read_smn", &[0x59800]);
        assert_eq!(r.len(), 40);
        assert_eq!(&r[..14], b"ioctl_read_smn");
        assert!(r[14..32].iter().all(|b| *b == 0));
        assert_eq!(&r[32..], &0x59800_i64.to_le_bytes());
        assert_eq!(request(&"x".repeat(40), &[]).len(), 32);
        assert_eq!(request(&"x".repeat(40), &[])[31], 0);
    }

    #[test]
    fn codigos_de_controle() {
        assert_eq!(IOCTL_LOAD_BINARY, 0xA1B2_2084);
        assert_eq!(IOCTL_EXECUTE, 0xA1B2_2104);
    }
}
