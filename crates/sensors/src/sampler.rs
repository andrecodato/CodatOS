//! Uma leitura de temperatura e potência por conexão ao pipe.

use std::time::Instant;

use crate::pawnio::{self, PawnIo};
use crate::ryzen::{self, CpuId, PowerMeter};
use crate::{PAWNIO_UNAVAILABLE, Reply, UNSUPPORTED};

/// Módulo assinado do PawnIO para Ryzen (LGPL-2.1, `modules/NOTICE.md`).
const AMD_FAMILY_17: &[u8] = include_bytes!("../modules/AMDFamily17.bin");

/// Abre o PawnIO na primeira leitura e de novo depois de uma falha (driver instalado com o
/// serviço já rodando, por exemplo).
pub(crate) struct Sampler {
    cpu: CpuId,
    open: Option<Open>,
}

struct Open {
    pawnio: PawnIo,
    meter: PowerMeter,
}

impl Sampler {
    pub(crate) fn new() -> Self {
        Self {
            cpu: CpuId::current(),
            open: None,
        }
    }

    pub(crate) fn sample(&mut self) -> Reply {
        if !self.cpu.supported() {
            return Reply::error(
                UNSUPPORTED,
                format!(
                    "Temperatura indisponível para esta CPU ({}): por enquanto só Ryzen",
                    self.cpu.brand
                ),
            );
        }
        match self.read() {
            Ok(reply) => reply,
            Err(e) => Reply::error(PAWNIO_UNAVAILABLE, format!("Temperatura indisponível: {e}")),
        }
    }

    fn read(&mut self) -> Result<Reply, pawnio::Error> {
        // Em caso de erro, fica fechado e reabre na próxima leitura.
        let mut open = if let Some(open) = self.open.take() {
            open
        } else {
            let mut pawnio = PawnIo::open(AMD_FAMILY_17)?;
            let unit = ryzen::energy_unit(read_msr(&mut pawnio, ryzen::MSR_PWR_UNIT)?);
            Open {
                pawnio,
                meter: PowerMeter::new(unit),
            }
        };
        let raw = {
            // Outros programas de sensores usam o mesmo índice/dado do SMN.
            let _bus = pci_bus::lock();
            read_smn(&mut open.pawnio, ryzen::SMN_THM_TCON_CUR_TMP)?
        };
        let temperature_c = ryzen::tctl(raw) + ryzen::tdie_offset(&self.cpu.brand);
        #[allow(clippy::cast_possible_truncation)] // contador de 32 bits
        let energy = read_msr(&mut open.pawnio, ryzen::MSR_PKG_ENERGY_STAT)? as u32;
        let power_w = open.meter.update(energy, Instant::now());
        self.open = Some(open);
        Ok(Reply::Ok {
            temperature_c: Some(temperature_c),
            power_w,
        })
    }
}

fn read_smn(pawnio: &mut PawnIo, addr: u32) -> Result<u32, pawnio::Error> {
    let out = pawnio.execute("ioctl_read_smn", &[i64::from(addr)], 1)?;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // registrador de 32 bits
    out.first()
        .map(|v| *v as u32)
        .ok_or(pawnio::Error::Execute(0))
}

fn read_msr(pawnio: &mut PawnIo, msr: u32) -> Result<u64, pawnio::Error> {
    let out = pawnio.execute("ioctl_read_msr", &[i64::from(msr)], 1)?;
    #[allow(clippy::cast_sign_loss)] // edx:eax
    out.first()
        .map(|v| *v as u64)
        .ok_or(pawnio::Error::Execute(0))
}

/// Mutex global `Access_PCI`, a convenção dos programas de sensores para não intercalar
/// acessos ao barramento.
#[allow(unsafe_code)]
mod pci_bus {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

    pub(super) struct Guard(Option<HANDLE>);

    /// Espera até 10 ms; sem o mutex, lê assim mesmo (é só leitura).
    pub(super) fn lock() -> Guard {
        let name: Vec<u16> = r"Global\Access_PCI".encode_utf16().chain([0]).collect();
        // SAFETY: `name` termina em NUL; atributos nulos dão a segurança padrão.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Guard(None);
        }
        // SAFETY: `handle` acabou de ser aberto.
        match unsafe { WaitForSingleObject(handle, 10) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Guard(Some(handle)),
            _ => {
                // SAFETY: idem; não é mais usado.
                unsafe { CloseHandle(handle) };
                Guard(None)
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(handle) = self.0 {
                // SAFETY: este processo detém o mutex desde `lock`.
                unsafe {
                    ReleaseMutex(handle);
                    CloseHandle(handle);
                }
            }
        }
    }
}
