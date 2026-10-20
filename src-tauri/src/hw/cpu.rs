//! Leitura do processador: dados estáticos (CPUID, cache, topologia) e dados
//! ao vivo (clock, uso e temperatura por thread).

use super::codenames::{self, Codename};
use serde::Serialize;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub vendor_id: String,
    pub codename: Option<Codename>,
    pub signature: Signature,
    pub microcode: Option<String>,
    pub cores: usize,
    pub threads: usize,
    pub clocks: ClockRange,
    pub caches: Vec<Cache>,
    pub topology: Vec<Core>,
    pub instructions: Vec<InstructionGroup>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Signature {
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    pub base_family: u32,
    pub base_model: u32,
    pub ext_family: u32,
    pub ext_model: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ClockRange {
    pub min_mhz: Option<u64>,
    pub max_mhz: Option<u64>,
    pub driver: Option<String>,
    pub governor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cache {
    pub level: u8,
    pub kind: &'static str,
    pub size_kib: u64,
    pub ways: usize,
    pub line_bytes: usize,
    pub instances: usize,
}

/// Um núcleo físico e as threads lógicas (índices de `sysinfo`) que ele roda.
#[derive(Debug, Clone, Serialize)]
pub struct Core {
    pub id: usize,
    pub threads: Vec<usize>,
    /// Grupo de núcleos que divide o mesmo cache L3 (CCX na AMD).
    pub cluster: usize,
    /// "performance" ou "efficiency" em processadores híbridos (Intel P/E).
    pub kind: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionGroup {
    pub name: &'static str,
    pub items: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuLive {
    pub threads: Vec<ThreadLive>,
    pub avg_mhz: u64,
    pub peak_mhz: u64,
    pub usage: f32,
    pub temperature_c: Option<f32>,
    /// "sensor" (sensor do processador) ou "acpi" (zona térmica da placa, no
    /// Windows sem driver), que nem sempre corresponde ao núcleo.
    pub temperature_source: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThreadLive {
    pub mhz: u64,
    pub usage: f32,
}

pub fn read_static() -> CpuInfo {
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_frequency()),
    );
    let threads = sys.cpus().len().max(1);
    let cores = System::physical_core_count().unwrap_or(threads);
    let brand_fallback = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let vendor_fallback = sys
        .cpus()
        .first()
        .map(|c| c.vendor_id().to_string())
        .unwrap_or_default();

    let mut info = CpuInfo {
        brand: brand_fallback,
        vendor: String::new(),
        vendor_id: vendor_fallback,
        codename: None,
        signature: Signature::default(),
        microcode: platform::microcode(),
        cores,
        threads,
        clocks: platform::clock_range(),
        caches: Vec::new(),
        topology: platform::topology(threads, cores),
        instructions: Vec::new(),
    };

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    x86::fill(&mut info);

    info.vendor = vendor_name(&info.vendor_id).to_string();
    info.codename = codenames::lookup(&info.vendor_id, info.signature.family, info.signature.model);
    info
}

fn vendor_name(vendor_id: &str) -> &str {
    match vendor_id {
        "GenuineIntel" => "Intel",
        "AuthenticAMD" => "AMD",
        "HygonGenuine" => "Hygon",
        "CentaurHauls" | "  Shanghai  " => "Zhaoxin",
        other => other,
    }
}

/// Mantém um `System` entre chamadas: o uso de CPU é calculado pela diferença
/// entre duas leituras, então a primeira amostra sempre vem zerada.
pub struct LiveSampler {
    sys: System,
    /// Criado na primeira leitura. No Windows os sensores usam WMI, que inicializa
    /// o COM em modo multithread; se isso acontecer no thread principal antes da
    /// janela abrir, o `OleInitialize` do Tauri falha e o app fecha.
    components: Option<Components>,
}

impl LiveSampler {
    pub fn new() -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::nothing().with_cpu(CpuRefreshKind::everything()),
            ),
            components: None,
        }
    }

    pub fn sample(&mut self) -> CpuLive {
        self.sys.refresh_cpu_specifics(CpuRefreshKind::everything());
        match &mut self.components {
            Some(components) => components.refresh(false),
            None => self.components = Some(Components::new_with_refreshed_list()),
        }

        let threads: Vec<ThreadLive> = self
            .sys
            .cpus()
            .iter()
            .map(|c| ThreadLive {
                mhz: c.frequency(),
                usage: c.cpu_usage(),
            })
            .collect();
        let n = threads.len().max(1) as u64;
        let (temperature_c, temperature_source) = match self.cpu_temperature() {
            Some(t) => (Some(t), Some("sensor")),
            None => match platform::thermal_zone_celsius() {
                Some(t) => (Some(t), Some("acpi")),
                None => (None, None),
            },
        };

        CpuLive {
            avg_mhz: threads.iter().map(|t| t.mhz).sum::<u64>() / n,
            peak_mhz: threads.iter().map(|t| t.mhz).max().unwrap_or(0),
            usage: self.sys.global_cpu_usage(),
            temperature_c,
            temperature_source,
            threads,
        }
    }

    fn cpu_temperature(&self) -> Option<f32> {
        // Rótulos dos drivers hwmon de CPU (k10temp na AMD, coretemp na Intel).
        const LABELS: [&str; 4] = ["tctl", "tdie", "package id", "cpu"];
        self.components
            .iter()
            .flatten()
            .find(|c| {
                let label = c.label().to_lowercase();
                LABELS.iter().any(|l| label.contains(l))
            })
            .and_then(|c| c.temperature())
            .filter(|t| t.is_finite() && *t > 0.0)
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86 {
    use super::{Cache, CpuInfo, InstructionGroup};
    use raw_cpuid::{CacheType, CpuId};

    pub fn fill(info: &mut CpuInfo) {
        let cpuid = CpuId::new();

        if let Some(v) = cpuid.get_vendor_info() {
            info.vendor_id = v.as_str().to_string();
        }
        if let Some(b) = cpuid.get_processor_brand_string() {
            let brand = b.as_str().trim();
            if !brand.is_empty() {
                info.brand = brand.to_string();
            }
        }
        if let Some(f) = cpuid.get_feature_info() {
            let s = &mut info.signature;
            s.family = f.family_id() as u32;
            s.model = f.model_id() as u32;
            s.stepping = f.stepping_id() as u32;
            s.base_family = f.base_family_id() as u32;
            s.base_model = f.base_model_id() as u32;
            s.ext_family = f.extended_family_id() as u32;
            s.ext_model = f.extended_model_id() as u32;
        }

        if let Some(params) = cpuid.get_cache_parameters() {
            info.caches = params
                .map(|c| {
                    let size = c.associativity()
                        * c.physical_line_partitions()
                        * c.coherency_line_size()
                        * c.sets();
                    let shared = c.max_cores_for_cache().max(1);
                    Cache {
                        level: c.level(),
                        kind: match c.cache_type() {
                            CacheType::Data => "data",
                            CacheType::Instruction => "instruction",
                            _ => "unified",
                        },
                        size_kib: (size / 1024) as u64,
                        ways: c.associativity(),
                        line_bytes: c.coherency_line_size(),
                        instances: (info.threads / shared).max(1),
                    }
                })
                .collect();
        }

        info.instructions = instructions(&cpuid);
    }

    fn instructions(cpuid: &CpuId<raw_cpuid::CpuIdReaderNative>) -> Vec<InstructionGroup> {
        let f = cpuid.get_feature_info();
        let e = cpuid.get_extended_feature_info();
        let x = cpuid.get_extended_processor_and_feature_identifiers();

        let mut simd = Vec::new();
        let mut crypto = Vec::new();
        let mut virt = Vec::new();
        let mut bits = Vec::new();
        let mut security = Vec::new();

        macro_rules! add {
            ($src:expr, $list:ident, $( $method:ident => $name:literal ),+ $(,)?) => {
                if let Some(s) = &$src {
                    $( if s.$method() { $list.push($name); } )+
                }
            };
        }

        add!(f, simd,
            has_mmx => "MMX", has_sse => "SSE", has_sse2 => "SSE2", has_sse3 => "SSE3",
            has_ssse3 => "SSSE3", has_sse41 => "SSE4.1", has_sse42 => "SSE4.2");
        add!(x, simd, has_sse4a => "SSE4A");
        add!(f, simd, has_avx => "AVX", has_fma => "FMA3", has_f16c => "F16C");
        add!(e, simd,
            has_avx2 => "AVX2", has_avx_vnni => "AVX-VNNI", has_avx512f => "AVX-512F",
            has_avx512dq => "AVX-512DQ", has_avx512bw => "AVX-512BW", has_avx512vl => "AVX-512VL",
            has_avx512vnni => "AVX-512 VNNI", has_avx512_bf16 => "AVX-512 BF16",
            has_amx_tile => "AMX");

        add!(f, crypto, has_aesni => "AES-NI", has_pclmulqdq => "PCLMULQDQ", has_rdrand => "RDRAND");
        add!(e, crypto, has_sha => "SHA", has_vaes => "VAES", has_vpclmulqdq => "VPCLMULQDQ",
            has_gfni => "GFNI", has_rdseed => "RDSEED");

        add!(f, virt, has_vmx => "VT-x");
        add!(x, virt, has_svm => "AMD-V");

        add!(x, bits, has_64bit_mode => "x86-64", has_lzcnt => "LZCNT/ABM");
        add!(f, bits, has_popcnt => "POPCNT", has_movbe => "MOVBE");
        add!(e, bits, has_bmi1 => "BMI1", has_bmi2 => "BMI2", has_adx => "ADX");

        add!(x, security, has_execute_disable => "NX");
        add!(e, security, has_smep => "SMEP", has_smap => "SMAP", has_sgx => "SGX", has_cet_ss => "CET");

        [
            ("simd", simd),
            ("crypto", crypto),
            ("virtualization", virt),
            ("integer", bits),
            ("security", security),
        ]
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(name, items)| InstructionGroup { name, items })
        .collect()
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{ClockRange, Core};
    use std::collections::BTreeMap;
    use std::fs;

    fn read(path: &str) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn khz_to_mhz(path: &str) -> Option<u64> {
        read(path)?.parse::<u64>().ok().map(|khz| khz / 1000)
    }

    pub fn microcode() -> Option<String> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo").ok()?;
        cpuinfo
            .lines()
            .find(|l| l.starts_with("microcode"))
            .and_then(|l| l.split(':').nth(1))
            .map(|v| v.trim().to_string())
    }

    pub fn clock_range() -> ClockRange {
        let base = "/sys/devices/system/cpu/cpu0/cpufreq";
        ClockRange {
            min_mhz: khz_to_mhz(&format!("{base}/cpuinfo_min_freq")),
            max_mhz: khz_to_mhz(&format!("{base}/cpuinfo_max_freq")),
            driver: read(&format!("{base}/scaling_driver")),
            governor: read(&format!("{base}/scaling_governor")),
        }
    }

    /// No Linux o hwmon já traz o sensor do processador.
    pub fn thermal_zone_celsius() -> Option<f32> {
        None
    }

    /// Agrupa as threads pelo par (pacote, core_id) que o kernel expõe.
    pub fn topology(threads: usize, cores: usize) -> Vec<Core> {
        let mut map: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        for cpu in 0..threads {
            let topo = format!("/sys/devices/system/cpu/cpu{cpu}/topology");
            let pkg = read(&format!("{topo}/physical_package_id")).and_then(|v| v.parse().ok());
            let core = read(&format!("{topo}/core_id")).and_then(|v| v.parse().ok());
            match (pkg, core) {
                (Some(p), Some(c)) => map.entry((p, c)).or_default().push(cpu),
                _ => return super::sequential_topology(threads, cores),
            }
        }
        let mut cores: Vec<Core> = map
            .into_values()
            .enumerate()
            .map(|(id, threads)| Core {
                id,
                threads,
                cluster: 0,
                kind: None,
            })
            .collect();
        super::assign_clusters(&mut cores, l3_group);

        // Processadores híbridos da Intel registram um PMU por tipo de núcleo.
        let performance = read("/sys/devices/cpu_core/cpus").map(|l| super::parse_cpu_list(&l));
        let efficiency = read("/sys/devices/cpu_atom/cpus").map(|l| super::parse_cpu_list(&l));
        if let (Some(p), Some(e)) = (performance, efficiency) {
            for core in &mut cores {
                let first = core.threads[0];
                core.kind = if p.contains(&first) {
                    Some("performance")
                } else if e.contains(&first) {
                    Some("efficiency")
                } else {
                    None
                };
            }
        }
        cores
    }

    /// Lista de threads que dividem o L3 com `cpu`, usada como chave do grupo.
    fn l3_group(cpu: usize) -> Option<String> {
        let base = format!("/sys/devices/system/cpu/cpu{cpu}/cache");
        (0..8).find_map(|i| {
            let index = format!("{base}/index{i}");
            (read(&format!("{index}/level"))? == "3")
                .then(|| read(&format!("{index}/shared_cpu_list")))
                .flatten()
        })
    }
}

#[cfg(windows)]
mod platform {
    use super::{ClockRange, Core};
    use crate::hw::wmi;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_BINARY, RegGetValueW};
    use windows::Win32::System::SystemInformation::{
        GetLogicalProcessorInformationEx, LOGICAL_PROCESSOR_RELATIONSHIP, RelationAll,
        RelationCache, RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };
    use windows::core::w;

    /// Microcódigo do Registro ("Update Revision", 8 bytes). A Intel guarda a
    /// revisão nos 4 bytes altos; a AMD, nos baixos.
    pub fn microcode() -> Option<String> {
        let mut data = [0u8; 8];
        let mut size = data.len() as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                w!(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0"),
                w!("Update Revision"),
                RRF_RT_REG_BINARY,
                None,
                Some(data.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if result != ERROR_SUCCESS || size < 8 {
            return None;
        }
        let low = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let high = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let revision = if high != 0 { high } else { low };
        (revision != 0).then(|| format!("0x{revision:X}"))
    }

    /// Só o CPUID 0x16 (Intel) informa o clock máximo sem driver; o "~MHz" do
    /// Registro é o clock base, que mostraríamos errado como máximo.
    pub fn clock_range() -> ClockRange {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        let max = raw_cpuid::CpuId::new()
            .get_processor_frequency_info()
            .map(|f| f.processor_max_frequency() as u64)
            .filter(|mhz| *mhz > 0);
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        let max = None;
        ClockRange {
            max_mhz: max,
            ..ClockRange::default()
        }
    }

    /// Núcleos, grupos de L3 e classe de eficiência pelo GetLogicalProcessorInformationEx.
    pub fn topology(threads: usize, cores: usize) -> Vec<Core> {
        let entries = processor_relations();
        let mut found: Vec<Core> = Vec::new();
        let mut classes: Vec<u8> = Vec::new();
        let mut l3: Vec<Vec<usize>> = Vec::new();
        for (relation, info) in &entries {
            if *relation == RelationProcessorCore {
                let p = unsafe { &info.Anonymous.Processor };
                let mask = p.GroupMask[0];
                found.push(Core {
                    id: found.len(),
                    threads: mask_threads(mask.Group, mask.Mask),
                    cluster: 0,
                    kind: None,
                });
                classes.push(p.EfficiencyClass);
            } else if *relation == RelationCache {
                let c = unsafe { &info.Anonymous.Cache };
                if c.Level == 3 {
                    let mask = unsafe { c.Anonymous.GroupMask };
                    l3.push(mask_threads(mask.Group, mask.Mask));
                }
            }
        }
        if found.is_empty() || found.iter().any(|c| c.threads.iter().any(|&t| t >= threads)) {
            return super::sequential_topology(threads, cores);
        }
        super::assign_clusters(&mut found, |t| l3.iter().position(|g| g.contains(&t)));

        // EfficiencyClass maior = núcleo mais rápido; só faz sentido se houver mais de uma.
        let (min, max) = (classes.iter().min(), classes.iter().max());
        if min != max {
            for (core, class) in found.iter_mut().zip(&classes) {
                core.kind = Some(if Some(class) == max { "performance" } else { "efficiency" });
            }
        }
        found.sort_by_key(|c| c.threads[0]);
        found
    }

    fn mask_threads(group: u16, mask: usize) -> Vec<usize> {
        let base = group as usize * usize::BITS as usize;
        (0..usize::BITS as usize)
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| base + bit)
            .collect()
    }

    /// Copia cada registro de tamanho variável devolvido pela API.
    fn processor_relations() -> Vec<(LOGICAL_PROCESSOR_RELATIONSHIP, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)> {
        let mut len = 0u32;
        let _ = unsafe { GetLogicalProcessorInformationEx(RelationAll, None, &mut len) };
        if len == 0 {
            return Vec::new();
        }
        let mut buf = vec![0u8; len as usize];
        let ok = unsafe {
            GetLogicalProcessorInformationEx(RelationAll, Some(buf.as_mut_ptr().cast()), &mut len)
        };
        if ok.is_err() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut offset = 0usize;
        let header = std::mem::size_of::<LOGICAL_PROCESSOR_RELATIONSHIP>() + 4;
        while offset + header <= len as usize {
            let size = u32::from_ne_bytes(buf[offset + 4..offset + 8].try_into().unwrap()) as usize;
            if size < header || offset + size > len as usize {
                break;
            }
            // O registro pode ser menor que a struct (union com tamanhos diferentes).
            let mut info = SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX::default();
            let n = size.min(std::mem::size_of_val(&info));
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr().add(offset),
                    (&mut info as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX).cast::<u8>(),
                    n,
                );
            }
            out.push((info.Relationship, info));
            offset += size;
        }
        out
    }

    /// Zona térmica ACPI pelos contadores de desempenho (não exige administrador).
    pub fn thermal_zone_celsius() -> Option<f32> {
        wmi::query(
            "SELECT Temperature FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation",
        )
        .iter()
        .filter_map(|row| wmi::number(row, "Temperature"))
        .map(|kelvin| kelvin as f32 - 273.15)
        .filter(|c| (1.0..130.0).contains(c))
        .reduce(f32::max)
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::{ClockRange, Core};

    pub fn microcode() -> Option<String> {
        None
    }

    pub fn clock_range() -> ClockRange {
        ClockRange::default()
    }

    pub fn topology(threads: usize, cores: usize) -> Vec<Core> {
        super::sequential_topology(threads, cores)
    }

    pub fn thermal_zone_celsius() -> Option<f32> {
        None
    }
}

/// Windows e macOS numeram as threads irmãs (SMT) em sequência.
fn sequential_topology(threads: usize, cores: usize) -> Vec<Core> {
    let per_core = (threads / cores.max(1)).max(1);
    (0..threads)
        .collect::<Vec<_>>()
        .chunks(per_core)
        .enumerate()
        .map(|(id, t)| Core {
            id,
            threads: t.to_vec(),
            cluster: 0,
            kind: None,
        })
        .collect()
}

/// Numera os grupos de L3 na ordem em que aparecem, pela primeira thread de cada núcleo.
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
fn assign_clusters<K: PartialEq>(cores: &mut [Core], key_of: impl Fn(usize) -> Option<K>) {
    let mut keys: Vec<K> = Vec::new();
    for core in cores {
        let Some(key) = key_of(core.threads[0]) else { continue };
        core.cluster = match keys.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                keys.push(key);
                keys.len() - 1
            }
        };
    }
}

/// Lê listas de CPUs no formato do kernel ("0-3,8,10-11").
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_cpu_list(list: &str) -> Vec<usize> {
    list.trim()
        .split(',')
        .filter_map(|part| match part.split_once('-') {
            Some((a, b)) => Some((a.trim().parse().ok()?..=b.trim().parse().ok()?).collect::<Vec<_>>()),
            None => part.trim().parse().ok().map(|n| vec![n]),
        })
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_lista_de_cpus() {
        assert_eq!(parse_cpu_list("0-3,8,10-11\n"), vec![0, 1, 2, 3, 8, 10, 11]);
        assert_eq!(parse_cpu_list(""), Vec::<usize>::new());
    }

    #[test]
    fn agrupa_nucleos_por_l3() {
        let mut cores = sequential_topology(16, 8);
        assign_clusters(&mut cores, |t| Some(t / 8));
        let clusters: Vec<usize> = cores.iter().map(|c| c.cluster).collect();
        assert_eq!(clusters, vec![0, 0, 0, 0, 1, 1, 1, 1]);
    }
}
