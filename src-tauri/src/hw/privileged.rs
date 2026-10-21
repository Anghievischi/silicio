//! Leituras que exigem administrador: registradores MSR, o sensor térmico das
//! CPUs AMD e a tabela SMBIOS bruta.
//!
//! No Linux rodam no binário `silicio-helper`, iniciado pelo app via `pkexec`,
//! que lê caminhos fixos e imprime JSON na saída padrão. No Windows passam pelo
//! driver PawnIO: direto no app quando ele já roda como administrador, ou pelo
//! mesmo helper elevado pelo UAC, que manda a temperatura a cada segundo por um
//! named pipe (ver [`win`]).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrivilegedInfo {
    pub processor: Option<SmbiosProcessor>,
    pub bios_vendor: Option<String>,
    /// Clock de referência medido (TSC ÷ multiplicador base).
    pub bus_mhz: Option<f64>,
    pub base_ratio: Option<f64>,
    /// Clock máximo (boost) calculado a partir dos MSRs.
    pub max_mhz: Option<u64>,
    pub threads: Vec<ThreadMsr>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SmbiosProcessor {
    pub socket: Option<String>,
    pub voltage: Option<f32>,
    pub external_clock_mhz: Option<u16>,
    pub max_speed_mhz: Option<u16>,
    pub current_speed_mhz: Option<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadMsr {
    /// Frequência do P-state atual pedida ao processador.
    pub pstate_mhz: Option<f64>,
    pub ratio: Option<f64>,
    pub voltage: Option<f64>,
}

/// Linha que o helper do Windows manda a cada segundo depois do [`PrivilegedInfo`].
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiveReading {
    pub temperature_c: Option<f32>,
}

#[cfg(target_os = "linux")]
pub fn read() -> PrivilegedInfo {
    let mut info = PrivilegedInfo::default();
    match std::fs::read("/sys/firmware/dmi/tables/DMI") {
        Ok(table) => fill_smbios(&mut info, &table),
        Err(e) => info.errors.push(format!("SMBIOS: {e}")),
    }

    #[cfg(target_arch = "x86_64")]
    {
        if !std::path::Path::new("/dev/cpu/0/msr").exists() {
            let _ = std::process::Command::new("modprobe").arg("msr").status();
        }
        msr::fill(&mut info, &msr::DevCpu);
    }

    info
}

#[cfg(all(windows, target_arch = "x86_64"))]
pub use win::read;

#[cfg(not(any(target_os = "linux", all(windows, target_arch = "x86_64"))))]
pub fn read() -> PrivilegedInfo {
    PrivilegedInfo::default()
}

#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
fn fill_smbios(info: &mut PrivilegedInfo, table: &[u8]) {
    info.processor = smbios_processor(table);
    info.bios_vendor =
        smbios_strings(table, 0).and_then(|(s, header)| string_at(&s, *header.get(4)?));
}

/// Percorre as estruturas SMBIOS e devolve (strings, área formatada) da primeira do tipo pedido.
fn smbios_strings(table: &[u8], kind: u8) -> Option<(Vec<String>, Vec<u8>)> {
    let mut pos = 0;
    while pos + 4 <= table.len() {
        let t = table[pos];
        let len = table[pos + 1] as usize;
        if len < 4 || pos + len > table.len() {
            return None;
        }
        let formatted = table[pos..pos + len].to_vec();
        // As strings terminam em um byte nulo duplo.
        let mut end = pos + len;
        while end + 1 < table.len() && !(table[end] == 0 && table[end + 1] == 0) {
            end += 1;
        }
        if t == kind {
            let strings = table[pos + len..end]
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).trim().to_string())
                .collect();
            return Some((strings, formatted));
        }
        if t == 127 {
            return None; // fim da tabela
        }
        pos = end + 2;
    }
    None
}

/// Índices de string do SMBIOS começam em 1; 0 significa "sem string".
fn string_at(strings: &[String], index: u8) -> Option<String> {
    let s = strings.get(index.checked_sub(1)? as usize)?;
    (!s.is_empty()).then(|| s.clone())
}

fn smbios_processor(table: &[u8]) -> Option<SmbiosProcessor> {
    let (strings, f) = smbios_strings(table, 4)?;
    let word = |at: usize| {
        f.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .filter(|v| *v != 0)
    };
    let voltage = f
        .get(0x11)
        .and_then(|v| (v & 0x80 != 0).then(|| (v & 0x7F) as f32 / 10.0));
    Some(SmbiosProcessor {
        socket: f.get(0x04).and_then(|i| string_at(&strings, *i)),
        voltage,
        external_clock_mhz: word(0x12),
        max_speed_mhz: word(0x14),
        current_speed_mhz: word(0x16),
    })
}

#[cfg(all(any(target_os = "linux", windows), target_arch = "x86_64"))]
// A temperatura e o SMN só são usados no Windows; no Linux vêm do `k10temp`.
#[cfg_attr(not(windows), allow(dead_code))]
mod msr {
    use super::{PrivilegedInfo, ThreadMsr};
    use raw_cpuid::CpuId;
    use std::collections::BTreeSet;
    use std::io;
    use std::time::{Duration, Instant};

    const IA32_PERF_STATUS: u64 = 0x198;
    const IA32_TEMPERATURE_TARGET: u64 = 0x1A2;
    const IA32_PACKAGE_THERM_STATUS: u64 = 0x1B1;
    const MSR_PLATFORM_INFO: u64 = 0xCE;
    const MSR_TURBO_RATIO_LIMIT: u64 = 0x1AD;
    const AMD_PSTATE_DEF0: u64 = 0xC001_0064;
    const AMD_HW_PSTATE_STATUS: u64 = 0xC001_0293;
    const AMD_CPPC_CAP1: u64 = 0xC001_02B0;
    /// THM_TCON_CUR_TMP, o mesmo registrador que o `k10temp` do Linux lê.
    const AMD_SMN_TEMPERATURE: u32 = 0x0005_9800;

    /// Origem das leituras: `/dev/cpu/N/msr` no Linux, PawnIO no Windows.
    pub trait Msr {
        fn read(&self, cpu: usize, reg: u64) -> io::Result<u64>;
        /// Registrador SMN da AMD; no Linux a temperatura já vem do `k10temp`.
        fn smn(&self, _address: u32) -> Option<u32> {
            None
        }
    }

    #[cfg(target_os = "linux")]
    pub struct DevCpu;

    #[cfg(target_os = "linux")]
    impl Msr for DevCpu {
        fn read(&self, cpu: usize, reg: u64) -> io::Result<u64> {
            use std::os::unix::fs::FileExt;
            let file = std::fs::File::open(format!("/dev/cpu/{cpu}/msr"))?;
            let mut buf = [0u8; 8];
            file.read_exact_at(&mut buf, reg)?;
            Ok(u64::from_le_bytes(buf))
        }
    }

    #[cfg(windows)]
    impl Msr for crate::hw::pawnio::PawnIo {
        fn read(&self, cpu: usize, reg: u64) -> io::Result<u64> {
            self.msr(cpu, reg)
        }
        fn smn(&self, address: u32) -> Option<u32> {
            crate::hw::pawnio::PawnIo::smn(self, address).ok()
        }
    }

    pub fn bits(value: u64, lo: u32, hi: u32) -> u64 {
        (value >> lo) & ((1 << (hi - lo + 1)) - 1)
    }

    /// Frequência do TSC medida contra o relógio do sistema.
    fn tsc_mhz() -> f64 {
        // SAFETY: RDTSC existe em todo x86-64 e não tem efeitos colaterais.
        let start = (Instant::now(), unsafe { core::arch::x86_64::_rdtsc() });
        std::thread::sleep(Duration::from_millis(200));
        let ticks = unsafe { core::arch::x86_64::_rdtsc() } - start.1;
        ticks as f64 / start.0.elapsed().as_secs_f64() / 1e6
    }

    struct Cpu {
        vendor: String,
        family: u8,
        model: u8,
        brand: String,
    }

    fn cpu() -> Cpu {
        let cpuid = CpuId::new();
        let (family, model) = cpuid
            .get_feature_info()
            .map(|f| (f.family_id(), f.model_id()))
            .unwrap_or((0, 0));
        Cpu {
            vendor: cpuid
                .get_vendor_info()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            family,
            model,
            brand: cpuid
                .get_processor_brand_string()
                .map(|b| b.as_str().trim().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn fill(info: &mut PrivilegedInfo, m: &impl Msr) {
        let c = cpu();
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let result = match c.vendor.as_str() {
            "GenuineIntel" => intel(info, m, threads),
            "AuthenticAMD" if (0x17..=0x1A).contains(&c.family) => {
                amd_zen(info, m, threads, c.family, c.model)
            }
            _ => {
                info.errors.push(format!(
                    "MSR: processador {} família {:#x} ainda não suportado",
                    c.vendor, c.family
                ));
                return;
            }
        };
        if let Err(e) = result {
            info.errors.push(format!("MSR: {e}"));
        }
    }

    fn intel(info: &mut PrivilegedInfo, m: &impl Msr, threads: usize) -> io::Result<()> {
        let base_ratio = bits(m.read(0, MSR_PLATFORM_INFO)?, 8, 15) as f64;
        if base_ratio > 0.0 {
            info.base_ratio = Some(base_ratio);
            info.bus_mhz = Some(tsc_mhz() / base_ratio);
        }
        // Bits 7–0: multiplicador máximo com um núcleo ativo.
        if let (Ok(limit), Some(bus)) = (m.read(0, MSR_TURBO_RATIO_LIMIT), info.bus_mhz) {
            let ratio = bits(limit, 0, 7) as f64;
            info.max_mhz = (ratio > 0.0).then(|| (ratio * bus / 100.0).round() as u64 * 100);
        }
        for cpu in 0..threads {
            let status = m.read(cpu, IA32_PERF_STATUS)?;
            let ratio = bits(status, 8, 15) as f64;
            let volts = bits(status, 32, 47) as f64 / 8192.0;
            info.threads.push(ThreadMsr {
                pstate_mhz: info.bus_mhz.map(|b| b * ratio),
                ratio: Some(ratio),
                voltage: (volts > 0.0).then_some(volts),
            });
        }
        Ok(())
    }

    /// Zen 1–4: FID/DID dão a frequência (FID ÷ DID × 200 MHz) e o VID a tensão.
    /// Zen 5 (família 1Ah): FID de 12 bits × 5 MHz.
    fn amd_zen(
        info: &mut PrivilegedInfo,
        m: &impl Msr,
        threads: usize,
        family: u8,
        model: u8,
    ) -> io::Result<()> {
        let freq = |v: u64| {
            if family >= 0x1A {
                let fid = bits(v, 0, 11) as f64;
                return (fid > 0.0).then_some(fid * 5.0);
            }
            let (fid, did) = (bits(v, 0, 7) as f64, bits(v, 8, 13) as f64);
            (did > 0.0).then(|| fid / did * 200.0)
        };
        // A tensão só é convertida nos modelos com SVI2, cuja fórmula é conhecida.
        let svi2 = family == 0x17 || (family == 0x19 && matches!(model, 0x01 | 0x08 | 0x21 | 0x50));

        let p0 = freq(m.read(0, AMD_PSTATE_DEF0)?);
        if let Some(p0) = p0 {
            let ratio = p0 / 100.0;
            info.base_ratio = Some(ratio);
            info.bus_mhz = Some(tsc_mhz() / ratio);
        }
        let caps: Vec<u64> = (0..threads)
            .filter_map(|cpu| m.read(cpu, AMD_CPPC_CAP1).ok())
            .collect();
        info.max_mhz = p0.and_then(|p0| amd_boost_mhz(p0, &caps, family, model));
        for cpu in 0..threads {
            let status = m.read(cpu, AMD_HW_PSTATE_STATUS)?;
            let mhz = freq(status);
            let vid = bits(status, 14, 21) as f64;
            info.threads.push(ThreadMsr {
                pstate_mhz: mhz,
                ratio: mhz.zip(info.bus_mhz).map(|(m, b)| m / b),
                voltage: svi2.then_some(1.55 - vid * 0.00625).filter(|v| *v > 0.0),
            });
        }
        Ok(())
    }

    /// Boost máximo pelo CPPC, como o `amd-pstate` do Linux: P0 (frequência
    /// nominal) × numerador ÷ desempenho nominal. Com "preferred cores" o
    /// highest_perf de cada núcleo vira um ranking, e o numerador é fixo (166,
    /// ou 196 nos Zen 4 de modelo 70h–7Fh).
    pub fn amd_boost_mhz(p0_mhz: f64, caps: &[u64], family: u8, model: u8) -> Option<u64> {
        let first = *caps.first()?;
        let nominal = bits(first, 16, 23) as f64;
        let highest: BTreeSet<u64> = caps.iter().map(|c| bits(*c, 24, 31)).collect();
        let numerator = if highest.len() == 1 {
            bits(first, 24, 31) as f64
        } else if family == 0x19 && (0x70..=0x7F).contains(&model) {
            196.0
        } else {
            166.0
        };
        if nominal == 0.0 {
            return None;
        }
        let mhz = (p0_mhz * numerator / nominal / 25.0).round() as u64 * 25;
        (p0_mhz as u64..=7000).contains(&mhz).then_some(mhz)
    }

    /// Temperatura do processador: Tctl na AMD (via SMN), pacote na Intel.
    pub fn temperature(m: &impl Msr) -> Option<f32> {
        let c = cpu();
        let celsius = match c.vendor.as_str() {
            "AuthenticAMD" => amd_tctl(m.smn(AMD_SMN_TEMPERATURE)?, &c.brand),
            "GenuineIntel" => {
                let tjmax = bits(m.read(0, IA32_TEMPERATURE_TARGET).ok()?, 16, 23) as f32;
                let below = bits(m.read(0, IA32_PACKAGE_THERM_STATUS).ok()?, 16, 22) as f32;
                tjmax - below
            }
            _ => return None,
        };
        Some(celsius).filter(|t| (1.0..125.0).contains(t))
    }

    /// Conversão do `k10temp`: bits 31–21 em 1/8 °C, menos 49 °C na faixa
    /// estendida, e o deslocamento que alguns Ryzen antigos somam ao Tctl.
    pub fn amd_tctl(reg: u32, brand: &str) -> f32 {
        const OFFSETS: [(&str, f32); 6] = [
            ("AMD Ryzen 5 1600X", 20.0),
            ("AMD Ryzen 7 1700X", 20.0),
            ("AMD Ryzen 7 1800X", 20.0),
            ("AMD Ryzen 7 2700X", 10.0),
            ("AMD Ryzen Threadripper 19", 27.0),
            ("AMD Ryzen Threadripper 29", 27.0),
        ];
        let mut t = (reg >> 21) as f32 * 0.125;
        if reg & (1 << 19) != 0 || reg & 0x3_0000 == 0x3_0000 {
            t -= 49.0;
        }
        let offset = OFFSETS
            .iter()
            .find(|(id, _)| brand.starts_with(id))
            .map_or(0.0, |(_, o)| *o);
        t - offset
    }
}

#[cfg(all(windows, target_arch = "x86_64"))]
pub mod win {
    //! Leituras pelo PawnIO e o canal com o helper elevado.

    use super::{LiveReading, PrivilegedInfo, msr};
    use crate::hw::pawnio::{OpenError, PawnIo};
    use std::io::{BufRead, BufReader, Write};
    use std::os::windows::io::FromRawHandle;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{ERROR_CANCELLED, ERROR_PIPE_CONNECTED, HANDLE};
    use windows::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_INBOUND};
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};
    use windows::Win32::System::Threading::{INFINITE, WaitForSingleObject};
    use windows::Win32::UI::Shell::{
        SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows::core::{HSTRING, PCWSTR, w};

    /// Resultado da primeira autorização, reaproveitado nas próximas chamadas.
    static INFO: Mutex<Option<PrivilegedInfo>> = Mutex::new(None);
    /// Última temperatura recebida e quando chegou.
    static TEMPERATURE: Mutex<Option<(f32, Instant)>> = Mutex::new(None);
    static PIPE_SERIAL: AtomicU32 = AtomicU32::new(0);

    /// SMBIOS pelo `GetSystemFirmwareTable` (não exige administrador). Os 8
    /// primeiros bytes são o cabeçalho `RawSMBIOSData`.
    fn smbios() -> Option<Vec<u8>> {
        let size = unsafe { GetSystemFirmwareTable(RSMB, 0, None) };
        let mut buf = vec![0u8; size as usize];
        let read = unsafe { GetSystemFirmwareTable(RSMB, 0, Some(&mut buf)) };
        (read as usize == buf.len() && buf.len() > 8).then(|| buf.split_off(8))
    }

    fn read_with(io: Result<&PawnIo, &OpenError>) -> PrivilegedInfo {
        let mut info = PrivilegedInfo::default();
        match smbios() {
            Some(table) => super::fill_smbios(&mut info, &table),
            None => info.errors.push("SMBIOS: tabela indisponível".into()),
        }
        match io {
            Ok(io) => msr::fill(&mut info, io),
            Err(e) => info.errors.push(e.to_string()),
        }
        info
    }

    /// Leitura única, usada pelo `silicio-helper` sem argumentos.
    pub fn read() -> PrivilegedInfo {
        let io = PawnIo::open();
        read_with(io.as_ref())
    }

    /// Temperatura recebida do helper (ou do próprio app elevado) há menos de 5 s.
    pub fn temperature() -> Option<f32> {
        let guard = TEMPERATURE.lock().ok()?;
        guard
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
            .map(|(t, _)| t)
    }

    fn set_temperature(t: Option<f32>) {
        if let Ok(mut guard) = TEMPERATURE.lock() {
            *guard = t.map(|t| (t, Instant::now()));
        }
    }

    /// Clock máximo lido pelos MSRs, se a autorização já foi dada.
    pub fn max_mhz() -> Option<u64> {
        INFO.lock().ok()?.as_ref()?.max_mhz
    }

    /// Chamado pelo app: lê tudo uma vez e passa a atualizar a temperatura.
    pub fn start() -> Result<PrivilegedInfo, String> {
        if let Some(info) = INFO.lock().map_err(|e| e.to_string())?.clone() {
            return Ok(info);
        }
        let info = match PawnIo::open() {
            // O app já roda como administrador: lê aqui mesmo.
            Ok(io) => {
                let info = read_with(Ok(&io));
                std::thread::spawn(move || {
                    loop {
                        set_temperature(msr::temperature(&io));
                        std::thread::sleep(Duration::from_secs(1));
                    }
                });
                info
            }
            Err(OpenError::AccessDenied) => start_helper()?,
            Err(e) => return Err(e.to_string()),
        };
        *INFO.lock().map_err(|e| e.to_string())? = Some(info.clone());
        Ok(info)
    }

    /// Cria um named pipe, roda o helper como administrador (o Windows mostra o
    /// UAC) e lê dele o `PrivilegedInfo` e depois uma [`LiveReading`] por segundo.
    fn start_helper() -> Result<PrivilegedInfo, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let helper = exe.with_file_name("silicio-helper.exe");
        if !helper.exists() {
            return Err("silicio-helper.exe não encontrado. Ele vem no instalador; a versão portátil não o inclui.".into());
        }

        let name = format!(
            r"\\.\pipe\silicio-{}-{}",
            std::process::id(),
            PIPE_SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let pipe = unsafe {
            CreateNamedPipeW(
                &HSTRING::from(&name),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                0,
                4096,
                0,
                None,
            )
        };
        if pipe.is_invalid() {
            return Err(format!("named pipe: {}", std::io::Error::last_os_error()));
        }
        // A partir daqui o File fecha o pipe em qualquer saída.
        let pipe_file = unsafe { std::fs::File::from_raw_handle(pipe.0) };

        let file = HSTRING::from(helper.as_os_str());
        let params = HSTRING::from(format!("--pipe {name}"));
        let mut exec = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            lpVerb: w!("runas"),
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: PCWSTR(params.as_ptr()),
            nShow: 0, // SW_HIDE
            ..Default::default()
        };
        if let Err(e) = unsafe { ShellExecuteExW(&mut exec) } {
            return Err(if e.code() == ERROR_CANCELLED.to_hresult() {
                "Autorização cancelada.".into()
            } else {
                format!("não foi possível iniciar o helper: {e}")
            });
        }

        // Se o helper morrer antes de conectar, o ConnectNamedPipe ficaria
        // esperando para sempre; conectar nós mesmos o destrava (e a leitura
        // seguinte encontra o fim do arquivo).
        let process = exec.hProcess.0 as usize;
        let pipe_name = name.clone();
        std::thread::spawn(move || {
            unsafe { WaitForSingleObject(HANDLE(process as *mut _), INFINITE) };
            let _ = std::fs::OpenOptions::new().write(true).open(&pipe_name);
        });

        if let Err(e) = unsafe { ConnectNamedPipe(pipe, None) }
            && e.code() != ERROR_PIPE_CONNECTED.to_hresult()
        {
            return Err(format!("named pipe: {e}"));
        }

        let mut reader = BufReader::new(pipe_file);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            return Err("o helper fechou sem responder.".into());
        }
        let info: PrivilegedInfo =
            serde_json::from_str(&line).map_err(|e| format!("resposta inválida do helper: {e}"))?;

        std::thread::spawn(move || {
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(r) = serde_json::from_str::<LiveReading>(&line) {
                    set_temperature(r.temperature_c);
                }
            }
            set_temperature(None);
        });
        Ok(info)
    }

    /// Lado do helper: conecta no pipe do app e escreve até o app fechar.
    pub fn serve(pipe_name: &str) -> std::io::Result<()> {
        if !pipe_name.starts_with(r"\\.\pipe\silicio-") {
            return Err(std::io::Error::other("pipe inválido"));
        }
        let mut pipe = std::fs::OpenOptions::new().write(true).open(pipe_name)?;
        let io = PawnIo::open();
        let info = read_with(io.as_ref());
        writeln!(pipe, "{}", serde_json::to_string(&info)?)?;
        let Ok(io) = io else { return Ok(()) };
        loop {
            let reading = LiveReading {
                temperature_c: msr::temperature(&io),
            };
            // Falha quando o app fecha o pipe: é a hora de sair.
            writeln!(pipe, "{}", serde_json::to_string(&reading)?)?;
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tabela com um tipo 0 (BIOS), um tipo 4 (processador) e o fim (127).
    fn table() -> Vec<u8> {
        let mut t = vec![0, 0x18, 0, 0, 1, 2, 0, 0];
        t.resize(0x18, 0);
        t.extend_from_slice(b"Insyde\0V1.0\0\0");

        let mut p = vec![4, 0x1A, 1, 0];
        p.resize(0x1A, 0);
        p[0x04] = 1; // socket
        p[0x11] = 0x80 | 12; // 1.2 V
        p[0x12..0x14].copy_from_slice(&100u16.to_le_bytes());
        p[0x14..0x16].copy_from_slice(&4375u16.to_le_bytes());
        p[0x16..0x18].copy_from_slice(&1800u16.to_le_bytes());
        t.extend_from_slice(&p);
        t.extend_from_slice(b"FP6\0\0");

        t.extend_from_slice(&[127, 4, 2, 0, 0, 0]);
        t
    }

    #[test]
    fn le_processador_do_smbios() {
        let p = smbios_processor(&table()).unwrap();
        assert_eq!(p.socket.as_deref(), Some("FP6"));
        assert_eq!(p.voltage, Some(1.2));
        assert_eq!(p.external_clock_mhz, Some(100));
        assert_eq!(p.max_speed_mhz, Some(4375));
        assert_eq!(p.current_speed_mhz, Some(1800));
    }

    #[test]
    fn le_fabricante_do_bios() {
        let (strings, header) = smbios_strings(&table(), 0).unwrap();
        assert_eq!(string_at(&strings, header[4]).as_deref(), Some("Insyde"));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn temperatura_amd() {
        // 45 °C direto, 94 − 49 °C na faixa estendida e 65 − 20 °C de offset no 1800X.
        assert_eq!(msr::amd_tctl(360 << 21, "AMD Ryzen 5 5600X"), 45.0);
        assert_eq!(
            msr::amd_tctl(752 << 21 | 1 << 19, "AMD Ryzen 9 7950X"),
            45.0
        );
        assert_eq!(
            msr::amd_tctl(520 << 21, "AMD Ryzen 7 1800X Eight-Core Processor"),
            45.0
        );
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn boost_amd_pelo_cppc() {
        let cap = |highest: u64, nominal: u64| highest << 24 | nominal << 16;
        // Sem preferred cores: o highest_perf é o numerador.
        assert_eq!(
            msr::amd_boost_mhz(3800.0, &[cap(150, 120); 4], 0x17, 0x71),
            Some(4750)
        );
        // Com ranking entre núcleos: numerador fixo de 166…
        let ranked = [cap(171, 133), cap(166, 133)];
        assert_eq!(msr::amd_boost_mhz(3700.0, &ranked, 0x19, 0x21), Some(4625));
        // …ou 196 nos Zen 4 de modelo 70h–7Fh.
        assert_eq!(msr::amd_boost_mhz(3700.0, &ranked, 0x19, 0x74), Some(5450));
        assert_eq!(msr::amd_boost_mhz(3700.0, &[cap(0, 0)], 0x19, 0x21), None);
        assert_eq!(msr::amd_boost_mhz(3700.0, &[], 0x19, 0x21), None);
    }
}
