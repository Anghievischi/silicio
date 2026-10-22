//! Leituras que exigem root no Linux: registradores MSR e a tabela SMBIOS bruta.
//!
//! Rodam no binário `silicio-helper`, iniciado pelo app via `pkexec`, que lê
//! caminhos fixos e imprime JSON na saída padrão. No Windows só a tabela SMBIOS
//! é lida, pelo `GetSystemFirmwareTable`, que não exige administrador: o
//! Silício não instala driver para ler os MSRs.

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

#[cfg(windows)]
pub use win::read;

#[cfg(not(any(target_os = "linux", windows)))]
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod msr {
    use super::{PrivilegedInfo, ThreadMsr};
    use raw_cpuid::CpuId;
    use std::collections::BTreeSet;
    use std::io;
    use std::time::{Duration, Instant};

    const IA32_PERF_STATUS: u64 = 0x198;
    const MSR_PLATFORM_INFO: u64 = 0xCE;
    const MSR_TURBO_RATIO_LIMIT: u64 = 0x1AD;
    const AMD_PSTATE_DEF0: u64 = 0xC001_0064;
    const AMD_HW_PSTATE_STATUS: u64 = 0xC001_0293;
    const AMD_CPPC_CAP1: u64 = 0xC001_02B0;

    /// Origem das leituras (`/dev/cpu/N/msr`); separada para os testes.
    pub trait Msr {
        fn read(&self, cpu: usize, reg: u64) -> io::Result<u64>;
    }

    pub struct DevCpu;

    impl Msr for DevCpu {
        fn read(&self, cpu: usize, reg: u64) -> io::Result<u64> {
            use std::os::unix::fs::FileExt;
            let file = std::fs::File::open(format!("/dev/cpu/{cpu}/msr"))?;
            let mut buf = [0u8; 8];
            file.read_exact_at(&mut buf, reg)?;
            Ok(u64::from_le_bytes(buf))
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

}

/// No Windows só a tabela SMBIOS, que qualquer usuário pode ler.
#[cfg(windows)]
mod win {
    use super::PrivilegedInfo;
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};

    /// SMBIOS pelo `GetSystemFirmwareTable`. Os 8 primeiros bytes são o
    /// cabeçalho `RawSMBIOSData`.
    fn smbios() -> Option<Vec<u8>> {
        let size = unsafe { GetSystemFirmwareTable(RSMB, 0, None) };
        let mut buf = vec![0u8; size as usize];
        let read = unsafe { GetSystemFirmwareTable(RSMB, 0, Some(&mut buf)) };
        (read as usize == buf.len() && buf.len() > 8).then(|| buf.split_off(8))
    }

    pub fn read() -> PrivilegedInfo {
        let mut info = PrivilegedInfo::default();
        match smbios() {
            Some(table) => super::fill_smbios(&mut info, &table),
            None => info.errors.push("SMBIOS: tabela indisponível".into()),
        }
        info
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


    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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
