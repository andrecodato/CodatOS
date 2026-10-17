//! GPUs AMD no Windows pela ADL, a biblioteca que o próprio driver da AMD instala
//! (`atiadlxx.dll` em `System32`). Roda sem admin e sem programa externo.
//!
//! Usa o registro de sensores do Overdrive 8 (`ADL2_New_QueryPMLogData_Get`, Radeon RX 5000
//! em diante): temperatura, atividade e potência; e a VRAM por
//! `ADL2_Adapter_DedicatedVRAMUsage_Get` e `ADL2_Adapter_MemoryInfoX4_Get`. Estruturas e
//! índices conferidos com o ADL SDK (`adl_structures.h`) e com o LibreHardwareMonitor.

use codatos_protocol::system::Gpu;

/// Fabricante AMD (ATI) no PCI.
const AMD_VENDOR: i32 = 0x1002;

/// Índices do registro de sensores (`ADL_PMLOG_*`).
const TEMPERATURE_EDGE: usize = 8;
const ACTIVITY_GFX: usize = 19;
const ASIC_POWER: usize = 23;
const TEMPERATURE_HOTSPOT: usize = 27;
const TEMPERATURE_GFX: usize = 28;
const BOARD_POWER: usize = 73;

/// Um adaptador da lista da ADL. A ADL lista um por saída de vídeo; a mesma placa aparece
/// várias vezes com o mesmo barramento.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Adapter {
    index: i32,
    name: String,
    vendor: i32,
    bus: i32,
    device: i32,
    has_udid: bool,
}

/// Uma entrada por placa AMD, na ordem da ADL.
fn unique_amd(adapters: Vec<Adapter>) -> Vec<Adapter> {
    let mut seen = Vec::new();
    adapters
        .into_iter()
        .filter(|a| a.vendor == AMD_VENDOR && a.has_udid)
        .filter(|a| {
            let slot = (a.bus, a.device);
            let new = !seen.contains(&slot);
            seen.push(slot);
            new
        })
        .collect()
}

/// Leitura de um sensor: `(suportado, valor)`.
type Sensor = (bool, i32);

#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn gpu(
    name: String,
    sensors: &[Sensor],
    vram_used_mb: Option<i32>,
    vram_total: Option<i64>,
) -> Gpu {
    let read = |index: usize| {
        sensors
            .get(index)
            .and_then(|&(supported, value)| supported.then_some(value as f32))
    };
    let first = |indexes: &[usize]| indexes.iter().find_map(|&i| read(i));
    Gpu {
        name,
        usage: read(ACTIVITY_GFX).unwrap_or(0.0).clamp(0.0, 100.0),
        memory_used: vram_used_mb.map_or(0, |mb| (mb.max(0) as u64) * 1024 * 1024),
        memory_total: vram_total.map_or(0, |b| b.max(0) as u64),
        temperature_c: first(&[TEMPERATURE_EDGE, TEMPERATURE_GFX, TEMPERATURE_HOTSPOT]),
        power_w: first(&[ASIC_POWER, BOARD_POWER]),
    }
}

#[cfg(windows)]
pub(super) use ffi::Adl;

#[cfg(windows)]
#[allow(unsafe_code)]
mod ffi {
    //! Chamadas à `atiadlxx.dll`. Todo `unsafe` do coletor fica aqui.

    use std::ffi::{CStr, c_char, c_void};
    use std::ptr;

    use codatos_protocol::system::Gpu;
    use libloading::os::windows::{LOAD_LIBRARY_SEARCH_SYSTEM32, Library, Symbol};

    use super::{Adapter, Sensor, gpu, unique_amd};

    const ADL_MAX_PATH: usize = 256;
    const ADL_PMLOG_MAX_SENSORS: usize = 256;

    /// `AdapterInfo` do `adl_structures.h` (campos do Windows incluídos).
    #[repr(C)]
    struct AdapterInfo {
        size: i32,
        adapter_index: i32,
        udid: [c_char; ADL_MAX_PATH],
        bus_number: i32,
        device_number: i32,
        function_number: i32,
        vendor_id: i32,
        adapter_name: [c_char; ADL_MAX_PATH],
        display_name: [c_char; ADL_MAX_PATH],
        present: i32,
        exist: i32,
        driver_path: [c_char; ADL_MAX_PATH],
        driver_path_ext: [c_char; ADL_MAX_PATH],
        pnp_string: [c_char; ADL_MAX_PATH],
        os_display_index: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SingleSensorData {
        supported: i32,
        value: i32,
    }

    #[repr(C)]
    struct PmLogDataOutput {
        size: i32,
        sensors: [SingleSensorData; ADL_PMLOG_MAX_SENSORS],
    }

    #[repr(C)]
    struct MemoryInfoX4 {
        memory_size: i64,
        memory_type: [c_char; ADL_MAX_PATH],
        memory_bandwidth: i64,
        hyper_memory_size: i64,
        invisible_memory_size: i64,
        visible_memory_size: i64,
        vram_vendor_rev_id: i64,
        memory_bandwidth_x2: i64,
        memory_bit_rate_x2: i64,
    }

    // Tamanhos do `adl_structures.h` no Windows x64; um campo errado aqui corromperia memória.
    const _: () = assert!(size_of::<AdapterInfo>() == 1572);
    const _: () = assert!(size_of::<PmLogDataOutput>() == 4 + 256 * 8);
    const _: () = assert!(size_of::<MemoryInfoX4>() == 320);

    type Context = *mut c_void;
    type Malloc = extern "C" fn(i32) -> *mut c_void;
    type ControlCreate = unsafe extern "C" fn(Malloc, i32, *mut Context) -> i32;
    type ControlDestroy = unsafe extern "C" fn(Context) -> i32;
    type NumberOfAdapters = unsafe extern "C" fn(Context, *mut i32) -> i32;
    type AdapterInfoGet = unsafe extern "C" fn(Context, *mut AdapterInfo, i32) -> i32;
    type PmLogGet = unsafe extern "C" fn(Context, i32, *mut PmLogDataOutput) -> i32;
    type IntGet = unsafe extern "C" fn(Context, i32, *mut i32) -> i32;
    type MemoryInfoGet = unsafe extern "C" fn(Context, i32, *mut MemoryInfoX4) -> i32;

    /// `ADL_OK` é 0 e avisos são positivos; erros são negativos.
    fn ok(status: i32) -> bool {
        status >= 0
    }

    /// Alocador que a ADL pede na criação do contexto. As funções usadas aqui escrevem em
    /// buffers nossos; a ADL só o usa em chamadas que este coletor não faz.
    extern "C" fn adl_malloc(size: i32) -> *mut c_void {
        let Ok(size) = usize::try_from(size) else {
            return ptr::null_mut();
        };
        let Ok(layout) = std::alloc::Layout::from_size_align(size.max(1), 16) else {
            return ptr::null_mut();
        };
        // SAFETY: tamanho não nulo; a memória nunca é liberada por nós (a ADL é dona dela).
        unsafe { std::alloc::alloc(layout).cast() }
    }

    /// Biblioteca carregada e contexto aberto. Fecha o contexto ao sair de escopo.
    pub(in crate::gpu) struct Adl {
        context: Context,
        destroy: Symbol<ControlDestroy>,
        adapters: Vec<Adapter>,
        pm_log: Option<Symbol<PmLogGet>>,
        vram_usage: Option<Symbol<IntGet>>,
        memory_info: Option<Symbol<MemoryInfoGet>>,
        // Por último: os símbolos acima apontam para dentro dela.
        library: Library,
    }

    // SAFETY: o contexto da ADL2 é por instância e o coletor o usa atrás de um `Mutex`, uma
    // chamada por vez.
    unsafe impl Send for Adl {}

    impl std::fmt::Debug for Adl {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Adl")
                .field("adapters", &self.adapters)
                .finish_non_exhaustive()
        }
    }

    impl Adl {
        /// `None` sem driver da AMD (a DLL não existe) ou sem placa AMD.
        pub(in crate::gpu) fn load() -> Option<Self> {
            // SAFETY: carrega só de `System32` (sem DLL plantada na pasta do app); a DLL é a
            // do driver da AMD e não roda nada de especial ao carregar.
            let library =
                unsafe { Library::load_with_flags("atiadlxx.dll", LOAD_LIBRARY_SEARCH_SYSTEM32) }
                    .ok()?;
            // SAFETY: nomes e assinaturas do `adl_sdk`; um símbolo ausente vira `None`.
            unsafe {
                let create: Symbol<ControlCreate> =
                    library.get(b"ADL2_Main_Control_Create\0").ok()?;
                let destroy: Symbol<ControlDestroy> =
                    library.get(b"ADL2_Main_Control_Destroy\0").ok()?;
                let mut context: Context = ptr::null_mut();
                // 1 = só adaptadores conectados.
                if !ok(create(adl_malloc, 1, &raw mut context)) || context.is_null() {
                    return None;
                }
                let mut adl = Self {
                    adapters: Vec::new(),
                    pm_log: library.get(b"ADL2_New_QueryPMLogData_Get\0").ok(),
                    vram_usage: library.get(b"ADL2_Adapter_DedicatedVRAMUsage_Get\0").ok(),
                    memory_info: library.get(b"ADL2_Adapter_MemoryInfoX4_Get\0").ok(),
                    context,
                    destroy,
                    library,
                };
                // Sem placa AMD, `adl` sai de escopo e o contexto é fechado.
                adl.adapters = unique_amd(adl.list_adapters()?);
                (!adl.adapters.is_empty()).then_some(adl)
            }
        }

        /// SAFETY: chamar com o contexto aberto.
        unsafe fn list_adapters(&self) -> Option<Vec<Adapter>> {
            let count_fn: Symbol<NumberOfAdapters> =
                unsafe { self.library.get(b"ADL2_Adapter_NumberOfAdapters_Get\0") }.ok()?;
            let info_fn: Symbol<AdapterInfoGet> =
                unsafe { self.library.get(b"ADL2_Adapter_AdapterInfo_Get\0") }.ok()?;
            let mut count = 0;
            if !ok(unsafe { count_fn(self.context, &raw mut count) }) || count <= 0 {
                return None;
            }
            let count = usize::try_from(count).ok()?.min(64);
            // SAFETY: `AdapterInfo` é só inteiros e bytes; zero é um valor válido.
            let mut infos: Vec<AdapterInfo> =
                (0..count).map(|_| unsafe { std::mem::zeroed() }).collect();
            let bytes = i32::try_from(count * size_of::<AdapterInfo>()).ok()?;
            if !ok(unsafe { info_fn(self.context, infos.as_mut_ptr(), bytes) }) {
                return None;
            }
            Some(
                infos
                    .iter()
                    .map(|i| Adapter {
                        index: i.adapter_index,
                        name: text(&i.adapter_name),
                        vendor: i.vendor_id,
                        bus: i.bus_number,
                        device: i.device_number,
                        has_udid: i.udid[0] != 0,
                    })
                    .collect(),
            )
        }

        pub(in crate::gpu) fn gpus(&self) -> Vec<Gpu> {
            self.adapters
                .iter()
                .map(|a| {
                    let sensors = self.sensors(a.index);
                    gpu(
                        a.name.clone(),
                        &sensors,
                        self.vram_used(a.index),
                        self.vram_total(a.index),
                    )
                })
                .collect()
        }

        fn sensors(&self, index: i32) -> Vec<Sensor> {
            let Some(get) = &self.pm_log else {
                return Vec::new();
            };
            // SAFETY: estrutura de inteiros, zerada; o tamanho vai no primeiro campo.
            let mut out: PmLogDataOutput = unsafe { std::mem::zeroed() };
            out.size = i32::try_from(size_of::<PmLogDataOutput>()).unwrap_or(i32::MAX);
            if !ok(unsafe { get(self.context, index, &raw mut out) }) {
                return Vec::new();
            }
            out.sensors
                .iter()
                .map(|s| (s.supported != 0, s.value))
                .collect()
        }

        fn vram_used(&self, index: i32) -> Option<i32> {
            let get = self.vram_usage.as_ref()?;
            let mut mb = 0;
            // SAFETY: escreve um `int`.
            ok(unsafe { get(self.context, index, &raw mut mb) }).then_some(mb)
        }

        fn vram_total(&self, index: i32) -> Option<i64> {
            let get = self.memory_info.as_ref()?;
            // SAFETY: estrutura de inteiros e bytes, zerada.
            let mut info: MemoryInfoX4 = unsafe { std::mem::zeroed() };
            ok(unsafe { get(self.context, index, &raw mut info) }).then_some(info.memory_size)
        }
    }

    impl Drop for Adl {
        fn drop(&mut self) {
            // SAFETY: contexto aberto em `load` e ainda não destruído.
            unsafe {
                (self.destroy)(self.context);
            }
        }
    }

    /// Texto C (ANSI) de tamanho fixo.
    fn text(raw: &[c_char; ADL_MAX_PATH]) -> String {
        let bytes: Vec<u8> = raw.iter().map(|&c| c.cast_unsigned()).collect();
        CStr::from_bytes_until_nul(&bytes)
            .map(|s| s.to_string_lossy().trim().to_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(index: i32, vendor: i32, bus: i32, udid: bool) -> Adapter {
        Adapter {
            index,
            name: format!("GPU {index}"),
            vendor,
            bus,
            device: 0,
            has_udid: udid,
        }
    }

    #[test]
    fn uma_entrada_por_placa_amd() {
        let list = unique_amd(vec![
            adapter(0, AMD_VENDOR, 3, true),
            adapter(1, AMD_VENDOR, 3, true), // outra saída da mesma placa
            adapter(2, 0x10de, 4, true),     // NVIDIA
            adapter(3, AMD_VENDOR, 5, false), // sem UDID
            adapter(4, AMD_VENDOR, 7, true),
        ]);
        let indexes: Vec<i32> = list.iter().map(|a| a.index).collect();
        assert_eq!(indexes, [0, 4]);
    }

    /// Registro de uma RX 6700 XT: borda 48 °C, hotspot 60 °C, 23% de uso, 41 W.
    #[test]
    fn le_os_sensores_do_registro() {
        let mut sensors = vec![(false, 0); 256];
        sensors[TEMPERATURE_EDGE] = (true, 48);
        sensors[TEMPERATURE_HOTSPOT] = (true, 60);
        sensors[ACTIVITY_GFX] = (true, 23);
        sensors[ASIC_POWER] = (true, 41);
        let g = gpu(
            "AMD Radeon RX 6700 XT".to_owned(),
            &sensors,
            Some(2048),
            Some(12_868_124_672),
        );
        assert_eq!(g.name, "AMD Radeon RX 6700 XT");
        assert!((g.usage - 23.0).abs() < f32::EPSILON);
        assert_eq!(g.memory_used, 2048 * 1024 * 1024);
        assert_eq!(g.memory_total, 12_868_124_672);
        assert_eq!(g.temperature_c, Some(48.0));
        assert_eq!(g.power_w, Some(41.0));
    }

    /// Placas novas (RX 7000) não têm borda nem ASIC: usa GFX/hotspot e a potência da placa.
    #[test]
    fn usa_os_sensores_alternativos() {
        let mut sensors = vec![(false, 0); 256];
        sensors[TEMPERATURE_HOTSPOT] = (true, 70);
        sensors[BOARD_POWER] = (true, 210);
        let g = gpu("RX 7900 XT".to_owned(), &sensors, None, None);
        assert_eq!(g.temperature_c, Some(70.0));
        assert_eq!(g.power_w, Some(210.0));
        assert_eq!(g.memory_total, 0);
        assert!(g.usage.abs() < f32::EPSILON);
    }

    #[test]
    fn sem_registro_fica_sem_sensores() {
        let g = gpu("RX 580".to_owned(), &[], Some(512), Some(8 << 30));
        assert_eq!(g.temperature_c, None);
        assert_eq!(g.power_w, None);
        assert_eq!(g.memory_used, 512 << 20);
    }
}
