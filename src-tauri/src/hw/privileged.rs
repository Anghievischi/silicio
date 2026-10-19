//! Leituras que exigem root: registradores MSR e a tabela SMBIOS bruta.
//!
//! Rodam no binário `silicio-helper`, iniciado pelo app via `pkexec`. O helper
//! não recebe argumentos, só lê caminhos fixos e imprime JSON na saída padrão.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrivilegedInfo {
    pub processor: Option<SmbiosProcessor>,
    pub bios_vendor: Option<String>,
    /// Clock de referência medido (TSC ÷ multiplicador base).
    pub bus_mhz: Option<f64>,
    pub base_ratio: Option<f64>,
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

pub fn read() -> PrivilegedInfo {
    let mut info = PrivilegedInfo::default();

    match std::fs::read("/sys/firmware/dmi/tables/DMI") {
        Ok(table) => {
            info.processor = smbios_processor(&table);
            info.bios_vendor =
                smbios_strings(&table, 0).and_then(|(s, header)| string_at(&s, *header.get(4)?));
        }
        Err(e) => info.errors.push(format!("SMBIOS: {e}")),
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    msr::fill(&mut info);

    info
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
    use std::fs::File;
    use std::os::unix::fs::FileExt;
    use std::time::{Duration, Instant};

    const IA32_PERF_STATUS: u64 = 0x198;
    const MSR_PLATFORM_INFO: u64 = 0xCE;
    const AMD_PSTATE_DEF0: u64 = 0xC001_0064;
    const AMD_HW_PSTATE_STATUS: u64 = 0xC001_0293;

    fn read(cpu: usize, reg: u64) -> std::io::Result<u64> {
        let file = File::open(format!("/dev/cpu/{cpu}/msr"))?;
        let mut buf = [0u8; 8];
        file.read_exact_at(&mut buf, reg)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn bits(value: u64, lo: u32, hi: u32) -> u64 {
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

    pub fn fill(info: &mut PrivilegedInfo) {
        if !std::path::Path::new("/dev/cpu/0/msr").exists() {
            let _ = std::process::Command::new("modprobe").arg("msr").status();
        }
        let cpuid = CpuId::new();
        let vendor = cpuid
            .get_vendor_info()
            .map(|v| v.as_str().to_string())
            .unwrap_or_default();
        let (family, model) = cpuid
            .get_feature_info()
            .map(|f| (f.family_id(), f.model_id()))
            .unwrap_or((0, 0));
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let result = match vendor.as_str() {
            "GenuineIntel" => intel(info, threads),
            "AuthenticAMD" if family == 0x17 || family == 0x19 => {
                amd_zen(info, threads, family, model)
            }
            _ => {
                info.errors.push(format!(
                    "MSR: processador {vendor} família {family:#x} ainda não suportado"
                ));
                return;
            }
        };
        if let Err(e) = result {
            info.errors.push(format!("MSR: {e}"));
        }
    }

    fn intel(info: &mut PrivilegedInfo, threads: usize) -> std::io::Result<()> {
        let base_ratio = bits(read(0, MSR_PLATFORM_INFO)?, 8, 15) as f64;
        if base_ratio > 0.0 {
            info.base_ratio = Some(base_ratio);
            info.bus_mhz = Some(tsc_mhz() / base_ratio);
        }
        for cpu in 0..threads {
            let status = read(cpu, IA32_PERF_STATUS)?;
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
    fn amd_zen(
        info: &mut PrivilegedInfo,
        threads: usize,
        family: u8,
        model: u8,
    ) -> std::io::Result<()> {
        let freq = |v: u64| {
            let (fid, did) = (bits(v, 0, 7) as f64, bits(v, 8, 13) as f64);
            (did > 0.0).then(|| fid / did * 200.0)
        };
        // A tensão só é convertida nos modelos com SVI2, cuja fórmula é conhecida.
        let svi2 = family == 0x17 || matches!(model, 0x01 | 0x08 | 0x21 | 0x50);

        if let Some(p0) = freq(read(0, AMD_PSTATE_DEF0)?) {
            let ratio = p0 / 100.0;
            info.base_ratio = Some(ratio);
            info.bus_mhz = Some(tsc_mhz() / ratio);
        }
        for cpu in 0..threads {
            let status = read(cpu, AMD_HW_PSTATE_STATUS)?;
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
}
