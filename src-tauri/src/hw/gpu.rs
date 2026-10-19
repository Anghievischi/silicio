//! Placas de vídeo: dados fixos (nome, VRAM, BIOS, PCIe) e leituras ao vivo
//! (clocks, uso, temperatura, consumo).
//!
//! - AMD e Intel no Linux: sysfs do driver (`amdgpu`, `i915`, `xe`) e hwmon.
//! - NVIDIA em qualquer sistema: NVML, carregada em tempo de execução.
//! - Windows: WMI para listar os adaptadores.

use super::pci::PcieLink;
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Gpu {
    /// Endereço PCI ("0000:04:00.0"); liga o dado fixo às leituras ao vivo.
    pub id: String,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub integrated: Option<bool>,
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    pub vbios: Option<String>,
    pub vram_bytes: Option<u64>,
    /// Memória do sistema que a GPU pode usar (GTT na AMD).
    pub shared_bytes: Option<u64>,
    pub max_core_mhz: Option<u32>,
    pub max_memory_mhz: Option<u32>,
    pub link: Option<PcieLink>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GpuLive {
    pub id: String,
    pub core_mhz: Option<u32>,
    pub memory_mhz: Option<u32>,
    pub usage: Option<u32>,
    pub vram_used_bytes: Option<u64>,
    pub temperature_c: Option<f32>,
    pub power_w: Option<f32>,
    pub voltage_mv: Option<u32>,
    pub fan_rpm: Option<u32>,
    pub fan_percent: Option<u32>,
}

/// Mantém a NVML carregada entre leituras (inicializá-la é lento).
pub struct GpuSampler {
    nvml: Option<Nvml>,
}

impl GpuSampler {
    pub fn new() -> Self {
        Self {
            nvml: Nvml::init().ok(),
        }
    }

    pub fn info(&self) -> Vec<Gpu> {
        let mut gpus = platform::list();
        if let Some(nvml) = &self.nvml {
            nvidia::merge_info(nvml, &mut gpus);
        }
        gpus
    }

    pub fn live(&self) -> Vec<GpuLive> {
        let mut samples = platform::live();
        if let Some(nvml) = &self.nvml {
            nvidia::merge_live(nvml, &mut samples);
        }
        samples
    }
}

/// Compara endereços PCI ignorando o domínio e a caixa ("00000000:01:00.0" = "0000:01:00.0").
fn same_pci(a: &str, b: &str) -> bool {
    let tail = |s: &str| {
        s.rsplitn(3, ':')
            .take(2)
            .collect::<Vec<_>>()
            .join(":")
            .to_lowercase()
    };
    tail(a) == tail(b)
}

/// Extrai o LUID do nome de um contador de GPU do Windows
/// ("pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_3D").
#[cfg_attr(not(windows), allow(dead_code))]
fn perf_luid(name: &str) -> Option<String> {
    let start = name.find("luid_")?;
    name.get(start..start + 26).map(str::to_lowercase)
}

/// Uso por GPU como no Gerenciador de Tarefas: soma os processos em cada motor
/// (3D, cópia, vídeo…) e fica com o motor mais ocupado.
#[cfg_attr(not(windows), allow(dead_code))]
fn engine_usage(rows: &[(String, f64)]) -> HashMap<String, f64> {
    let mut per_engine: HashMap<(String, String), f64> = HashMap::new();
    for (name, value) in rows {
        let (Some(luid), Some(engine)) = (perf_luid(name), name.find("phys_").map(|i| &name[i..]))
        else {
            continue;
        };
        *per_engine.entry((luid, engine.to_string())).or_default() += value;
    }
    let mut per_gpu: HashMap<String, f64> = HashMap::new();
    for ((luid, _), value) in per_engine {
        let best = per_gpu.entry(luid).or_default();
        *best = best.max(value);
    }
    per_gpu
}

mod nvidia {
    use super::*;

    pub fn merge_info(nvml: &Nvml, gpus: &mut Vec<Gpu>) {
        let driver_version = nvml.sys_driver_version().ok();
        for i in 0..nvml.device_count().unwrap_or(0) {
            let Ok(dev) = nvml.device_by_index(i) else {
                continue;
            };
            let Ok(pci) = dev.pci_info() else { continue };
            // No Windows o WMI identifica a placa pelo PNPDeviceID; nesse caso casamos pelo fabricante.
            let found = gpus
                .iter()
                .position(|g| same_pci(&g.id, &pci.bus_id))
                .or_else(|| {
                    gpus.iter().position(|g| {
                        !g.id.contains(':')
                            && g.vendor
                                .as_deref()
                                .is_some_and(|v| v.to_uppercase().contains("NVIDIA"))
                    })
                });
            let gpu = match found {
                Some(idx) => {
                    gpus[idx].id = pci.bus_id.to_lowercase();
                    &mut gpus[idx]
                }
                None => {
                    gpus.push(Gpu {
                        id: pci.bus_id.to_lowercase(),
                        ..Default::default()
                    });
                    gpus.last_mut().expect("acabou de ser inserido")
                }
            };
            gpu.name = dev.name().ok().or(gpu.name.take());
            gpu.vendor = Some("NVIDIA".into());
            gpu.integrated = Some(false);
            gpu.driver_version = driver_version.clone();
            gpu.vbios = dev.vbios_version().ok();
            gpu.vram_bytes = dev.memory_info().ok().map(|m| m.total);
            gpu.max_core_mhz = dev.max_clock_info(Clock::Graphics).ok();
            gpu.max_memory_mhz = dev.max_clock_info(Clock::Memory).ok();
            if gpu.link.is_none() {
                gpu.link = Some(PcieLink {
                    current_gen: dev.current_pcie_link_gen().ok().map(|v| v as u8),
                    current_width: dev.current_pcie_link_width().ok().map(|v| v as u8),
                    max_gen: dev.max_pcie_link_gen().ok().map(|v| v as u8),
                    max_width: dev.max_pcie_link_width().ok().map(|v| v as u8),
                });
            }
        }
    }

    pub fn merge_live(nvml: &Nvml, samples: &mut Vec<GpuLive>) {
        for i in 0..nvml.device_count().unwrap_or(0) {
            let Ok(dev) = nvml.device_by_index(i) else {
                continue;
            };
            let Ok(pci) = dev.pci_info() else { continue };
            let sample = GpuLive {
                id: pci.bus_id.to_lowercase(),
                core_mhz: dev.clock_info(Clock::Graphics).ok(),
                memory_mhz: dev.clock_info(Clock::Memory).ok(),
                usage: dev.utilization_rates().ok().map(|u| u.gpu),
                vram_used_bytes: dev.memory_info().ok().map(|m| m.total - m.free),
                temperature_c: dev
                    .temperature(TemperatureSensor::Gpu)
                    .ok()
                    .map(|t| t as f32),
                power_w: dev.power_usage().ok().map(|mw| mw as f32 / 1000.0),
                fan_percent: dev.fan_speed(0).ok(),
                ..Default::default()
            };
            match samples.iter_mut().find(|s| same_pci(&s.id, &sample.id)) {
                Some(existing) => {
                    *existing = GpuLive {
                        id: existing.id.clone(),
                        ..sample
                    }
                }
                None => samples.push(sample),
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{Gpu, GpuLive};
    use crate::hw::pci;
    use std::fs;
    use std::path::{Path, PathBuf};

    struct Card {
        card: PathBuf,
        device: PathBuf,
        address: String,
        driver: Option<String>,
    }

    fn cards() -> Vec<Card> {
        let Ok(entries) = fs::read_dir("/sys/class/drm") else {
            return Vec::new();
        };
        let mut cards: Vec<Card> = entries
            .flatten()
            .filter(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.strip_prefix("card")
                    .is_some_and(|n| n.chars().all(|c| c.is_ascii_digit()))
            })
            .filter_map(|e| {
                let device = fs::canonicalize(e.path().join("device")).ok()?;
                let address = device.file_name()?.to_string_lossy().into_owned();
                let driver = fs::read_link(device.join("driver"))
                    .ok()
                    .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()));
                Some(Card {
                    card: e.path(),
                    device,
                    address,
                    driver,
                })
            })
            .collect();
        cards.sort_by(|a, b| a.address.cmp(&b.address));
        cards.dedup_by(|a, b| a.address == b.address);
        cards
    }

    fn text(path: &Path) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn num<T: std::str::FromStr>(path: &Path) -> Option<T> {
        text(path)?.parse().ok()
    }

    fn hwmon(device: &Path) -> Option<PathBuf> {
        fs::read_dir(device.join("hwmon"))
            .ok()?
            .flatten()
            .next()
            .map(|e| e.path())
    }

    /// Lê os níveis de `pp_dpm_*` ("1: 400Mhz *") e devolve (atual, máximo) em MHz.
    fn dpm_levels(path: &Path) -> (Option<u32>, Option<u32>) {
        let Some(table) = text(path) else {
            return (None, None);
        };
        let mhz = |line: &str| -> Option<u32> {
            line.split_whitespace()
                .nth(1)?
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()
        };
        let current = table
            .lines()
            .find(|l| l.trim_end().ends_with('*'))
            .and_then(mhz);
        let max = table.lines().filter_map(mhz).max();
        (current, max)
    }

    pub fn list() -> Vec<Gpu> {
        let devices = pci::list();
        cards()
            .into_iter()
            .map(|c| {
                let pci = devices.iter().find(|d| d.address == c.address);
                let mut gpu = Gpu {
                    id: c.address.clone(),
                    name: pci.and_then(|p| p.name.clone()),
                    vendor: pci.and_then(|p| p.vendor.clone()),
                    driver: c.driver.clone(),
                    // Drivers que vêm no kernel não têm versão própria; vale a do kernel.
                    driver_version: text(&c.device.join("driver/module/version")).or_else(|| {
                        sysinfo::System::kernel_version().map(|k| format!("Linux {k}"))
                    }),
                    link: pci.and_then(|p| p.link.clone()),
                    ..Default::default()
                };
                match c.driver.as_deref() {
                    Some("amdgpu") => {
                        gpu.vram_bytes = num(&c.device.join("mem_info_vram_total"));
                        gpu.shared_bytes = num(&c.device.join("mem_info_gtt_total"));
                        gpu.vbios = text(&c.device.join("vbios_version"));
                        gpu.max_core_mhz = dpm_levels(&c.device.join("pp_dpm_sclk")).1;
                        gpu.max_memory_mhz = dpm_levels(&c.device.join("pp_dpm_mclk")).1;
                        // APUs reservam pouca VRAM dedicada e usam a RAM do sistema.
                        gpu.integrated = gpu.vram_bytes.map(|v| v <= 2 << 30);
                    }
                    Some("i915") | Some("xe") => {
                        gpu.max_core_mhz = num(&c.card.join("gt_max_freq_mhz"));
                        let arc = gpu
                            .name
                            .as_deref()
                            .is_some_and(|n| n.contains("Arc") || n.contains("DG2"));
                        gpu.integrated = Some(!arc);
                    }
                    Some("nvidia") | Some("nouveau") => gpu.integrated = Some(false),
                    _ => {}
                }
                gpu
            })
            .collect()
    }

    pub fn live() -> Vec<GpuLive> {
        cards()
            .into_iter()
            .map(|c| {
                let mut s = GpuLive {
                    id: c.address.clone(),
                    ..Default::default()
                };
                let mon = hwmon(&c.device);
                let mon_num = |file: &str| mon.as_ref().and_then(|m| num::<u64>(&m.join(file)));

                s.temperature_c = mon_num("temp1_input").map(|m| m as f32 / 1000.0);
                s.power_w = mon_num("power1_average")
                    .or_else(|| mon_num("power1_input"))
                    .map(|uw| uw as f32 / 1e6);
                s.fan_rpm = mon_num("fan1_input").map(|r| r as u32).filter(|r| *r > 0);

                match c.driver.as_deref() {
                    Some("amdgpu") => {
                        s.usage = num(&c.device.join("gpu_busy_percent"));
                        s.vram_used_bytes = num(&c.device.join("mem_info_vram_used"));
                        s.core_mhz = mon_num("freq1_input")
                            .map(|hz| (hz / 1_000_000) as u32)
                            .or_else(|| dpm_levels(&c.device.join("pp_dpm_sclk")).0);
                        s.memory_mhz = dpm_levels(&c.device.join("pp_dpm_mclk")).0;
                        s.voltage_mv = mon_num("in0_input").map(|v| v as u32);
                    }
                    Some("i915") => {
                        s.core_mhz = num(&c.card.join("gt_act_freq_mhz"))
                            .or_else(|| num(&c.card.join("gt_cur_freq_mhz")))
                    }
                    Some("xe") => s.core_mhz = num(&c.device.join("tile0/gt0/freq0/act_freq")),
                    _ => {}
                }
                s
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn le_niveis_dpm() {
            let dir = std::env::temp_dir().join(format!("silicio-dpm-{}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let file = dir.join("pp_dpm_sclk");
            fs::write(&file, "0: 200Mhz \n1: 400Mhz *\n2: 1900Mhz \n").unwrap();
            assert_eq!(dpm_levels(&file), (Some(400), Some(1900)));
            fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::{Gpu, GpuLive, engine_usage, perf_luid};
    use crate::hw::{pci, wmi};
    use std::collections::HashMap;
    use windows::Win32::Foundation::LUID;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
    };

    /// Mesmo formato dos nomes dos contadores de desempenho ("luid_0x00000000_0x0000d1b5").
    fn luid_id(luid: LUID) -> String {
        format!("luid_0x{:08x}_0x{:08x}", luid.HighPart as u32, luid.LowPart)
    }

    fn vendor_name(id: u32) -> Option<String> {
        Some(
            match id {
                0x10DE => "NVIDIA",
                0x1002 | 0x1022 => "AMD",
                0x8086 => "Intel",
                0x1414 => "Microsoft",
                0x5143 => "Qualcomm",
                _ => return None,
            }
            .to_string(),
        )
    }

    /// Lista os adaptadores pelo DXGI, que informa VRAM acima de 4 GB (o WMI satura)
    /// e o LUID usado pelos contadores de uso. Driver e link PCIe vêm do WMI e do PCI.
    pub fn list() -> Vec<Gpu> {
        let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
            return list_wmi();
        };
        let devices = pci::list();
        let controllers =
            wmi::query("SELECT PNPDeviceID, DriverVersion FROM Win32_VideoController");

        let mut gpus = Vec::new();
        for index in 0.. {
            let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
                break;
            };
            let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
                continue;
            };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue; // "Microsoft Basic Render Driver"
            }
            let ids = format!("VEN_{:04X}&DEV_{:04X}", desc.VendorId, desc.DeviceId);
            let matches = |id: &str| id.to_uppercase().contains(&ids);
            let name = String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .trim()
                .to_string();
            gpus.push(Gpu {
                id: luid_id(desc.AdapterLuid),
                name: (!name.is_empty()).then_some(name),
                vendor: vendor_name(desc.VendorId),
                // GPUs integradas reservam pouca memória dedicada e usam a RAM do sistema.
                integrated: Some(desc.VendorId != 0x10DE && desc.DedicatedVideoMemory < 1 << 30),
                driver_version: controllers
                    .iter()
                    .find(|row| wmi::text(row, "PNPDeviceID").is_some_and(|id| matches(&id)))
                    .and_then(|row| wmi::text(row, "DriverVersion")),
                vram_bytes: Some(desc.DedicatedVideoMemory as u64),
                shared_bytes: Some(desc.SharedSystemMemory as u64).filter(|v| *v > 0),
                link: devices
                    .iter()
                    .find(|d| matches(&d.address))
                    .and_then(|d| d.link.clone()),
                ..Default::default()
            });
        }
        if gpus.is_empty() { list_wmi() } else { gpus }
    }

    /// Reserva para quando o DXGI não lista nada (ex.: máquinas virtuais só com
    /// o adaptador básico). O AdapterRAM do WMI é uint32 e satura em 4 GB.
    fn list_wmi() -> Vec<Gpu> {
        wmi::query("SELECT Name, AdapterCompatibility, DriverVersion, AdapterRAM, PNPDeviceID FROM Win32_VideoController")
            .iter()
            .map(|row| Gpu {
                id: wmi::text(row, "PNPDeviceID").unwrap_or_default(),
                name: wmi::text(row, "Name"),
                vendor: wmi::text(row, "AdapterCompatibility"),
                driver_version: wmi::text(row, "DriverVersion"),
                vram_bytes: wmi::number(row, "AdapterRAM").filter(|v| *v > 0),
                ..Default::default()
            })
            .collect()
    }

    /// Uso e VRAM em uso pelos contadores de desempenho do Windows (os mesmos do
    /// Gerenciador de Tarefas). Funcionam com qualquer fabricante, sem driver extra.
    pub fn live() -> Vec<GpuLive> {
        let engines: Vec<(String, f64)> = wmi::query(
            "SELECT Name, UtilizationPercentage FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine",
        )
        .iter()
        .filter_map(|row| Some((wmi::text(row, "Name")?, wmi::number(row, "UtilizationPercentage")? as f64)))
        .collect();
        let usage = engine_usage(&engines);

        let mut memory: HashMap<String, u64> = HashMap::new();
        for row in wmi::query(
            "SELECT Name, DedicatedUsage FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory",
        ) {
            if let (Some(luid), Some(used)) = (
                wmi::text(&row, "Name").and_then(|n| perf_luid(&n)),
                wmi::number(&row, "DedicatedUsage"),
            ) {
                *memory.entry(luid).or_default() += used;
            }
        }

        let mut ids: Vec<&String> = usage.keys().chain(memory.keys()).collect();
        ids.sort();
        ids.dedup();
        ids.into_iter()
            .map(|id| GpuLive {
                id: id.clone(),
                usage: usage.get(id).map(|u| u.round().min(100.0) as u32),
                vram_used_bytes: memory.get(id).copied(),
                ..Default::default()
            })
            .collect()
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::{Gpu, GpuLive};

    pub fn list() -> Vec<Gpu> {
        Vec::new()
    }

    pub fn live() -> Vec<GpuLive> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{engine_usage, perf_luid, same_pci};

    #[test]
    fn soma_processos_por_motor_e_fica_com_o_maior() {
        let rows = [
            (
                "pid_1_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D".to_string(),
                30.0,
            ),
            (
                "pid_2_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D".to_string(),
                25.0,
            ),
            (
                "pid_2_luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_VideoDecode".to_string(),
                40.0,
            ),
            (
                "pid_9_luid_0x00000000_0x0000AAAA_phys_0_eng_0_engtype_3D".to_string(),
                5.0,
            ),
        ];
        let usage = engine_usage(&rows);
        assert_eq!(usage["luid_0x00000000_0x0000d1b5"], 55.0);
        assert_eq!(usage["luid_0x00000000_0x0000aaaa"], 5.0);
        assert_eq!(
            perf_luid("luid_0x00000000_0x0000D1B5_phys_0").as_deref(),
            Some("luid_0x00000000_0x0000d1b5")
        );
    }

    #[test]
    fn compara_enderecos_pci() {
        assert!(same_pci("00000000:01:00.0", "0000:01:00.0"));
        assert!(same_pci("0000:0A:00.0", "0000:0a:00.0"));
        assert!(!same_pci("0000:01:00.0", "0000:02:00.0"));
    }
}
