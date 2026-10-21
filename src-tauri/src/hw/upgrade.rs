//! Sugestões de upgrade a partir do hardware detectado: que placa de vídeo cabe
//! no computador, quanta memória dá para instalar e até onde o soquete do
//! processador vai.
//!
//! As tabelas vêm das páginas públicas de especificação da AMD, Intel e NVIDIA
//! (soquete, linhas PCIe, fonte recomendada). Os exemplos de placa de vídeo
//! envelhecem a cada geração: revise `ENTRY`, `MID` e `HIGH` quando sair uma nova.
//!
//! O backend devolve só códigos e números; a interface monta as frases.

use super::board::{self, BoardInfo};
use super::cpu::{self, CpuInfo};
use super::gpu::Gpu;
use super::memory::{self, MemoryInfo};
use serde::Serialize;

const GIB: u64 = 1 << 30;

#[derive(Debug, Clone, Serialize)]
pub struct Upgrades {
    /// "desktop", "laptop", "compact" (mini PC, all-in-one) ou "unknown".
    pub form: &'static str,
    pub gpu: GpuUpgrade,
    pub memory: MemoryUpgrade,
    pub cpu: CpuUpgrade,
    /// Sugestão de maior impacto: "memory-dual-channel", "memory-capacity",
    /// "gpu", "cpu" ou "none".
    pub top: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuUpgrade {
    /// "slot" (PCIe interno), "external" (Thunderbolt/USB4) ou "none".
    pub path: &'static str,
    pub slot_gen: Option<u8>,
    pub external_port: Option<&'static str>,
    /// Placa dedicada já instalada, se houver.
    pub current: Option<String>,
    /// Faixa que combina com o processador: "entry", "mid" ou "high".
    pub tier: &'static str,
    /// Fonte recomendada para a faixa, em watts.
    pub psu_watts: u32,
    pub suggestions: Vec<GpuSuggestion>,
    /// Slot PCIe 3.0 ou anterior: placas de 8 linhas perdem desempenho nele.
    pub narrow_slot: bool,
    /// Processador provavelmente sem Resizable BAR (as Intel Arc dependem dela).
    pub no_rebar: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct GpuSuggestion {
    pub name: &'static str,
    pub lanes: u8,
    pub vram_gb: u8,
    #[serde(skip)]
    needs_rebar: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryUpgrade {
    /// "soldered", "dual-channel", "add", "replace", "maxed" ou "unknown".
    pub action: &'static str,
    pub kind: Option<String>,
    pub form_factor: Option<String>,
    pub speed_mts: Option<u32>,
    pub installed_bytes: u64,
    pub max_bytes: Option<u64>,
    pub slots_total: usize,
    pub slots_used: usize,
    /// Módulos sugeridos: quantidade × tamanho.
    pub module_count: usize,
    pub module_bytes: u64,
    /// Total depois do upgrade.
    pub target_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuUpgrade {
    /// "soldered", "socket" ou "unknown".
    pub path: &'static str,
    pub socket: Option<&'static str>,
    /// Os mais rápidos que o soquete aceita.
    pub ceiling: Vec<&'static str>,
    /// A plataforma não recebe mais processadores novos.
    pub platform_end: bool,
    /// O suporte depende do chipset e da versão do BIOS.
    pub check_bios: bool,
    /// O processador atual já é um dos do topo do soquete.
    pub at_ceiling: bool,
}

const fn g(name: &'static str, lanes: u8, vram_gb: u8, needs_rebar: bool) -> GpuSuggestion {
    GpuSuggestion {
        name,
        lanes,
        vram_gb,
        needs_rebar,
    }
}

// Linhas PCIe e VRAM das páginas de especificação de cada fabricante.
const ENTRY: [GpuSuggestion; 3] = [
    g("Radeon RX 9060 XT 8 GB", 16, 8, false),
    g("GeForce RTX 5050", 8, 8, false),
    g("Intel Arc B570", 8, 10, true),
];
const MID: [GpuSuggestion; 3] = [
    g("Radeon RX 9060 XT 16 GB", 16, 16, false),
    g("GeForce RTX 5060 Ti 16 GB", 8, 16, false),
    g("GeForce RTX 5070", 16, 12, false),
];
const HIGH: [GpuSuggestion; 3] = [
    g("Radeon RX 9070 XT", 16, 16, false),
    g("GeForce RTX 5070 Ti", 16, 16, false),
    g("GeForce RTX 5080", 16, 16, false),
];

pub fn read(gpus: &[Gpu]) -> Upgrades {
    advise(&cpu::read_static(), &board::read(), &memory::read(), gpus)
}

pub fn advise(cpu: &CpuInfo, board: &BoardInfo, memory: &MemoryInfo, gpus: &[Gpu]) -> Upgrades {
    let form = form(board.system.chassis);
    let gpu = gpu(form, cpu, board, gpus);
    let memory = memory_upgrade(form, memory);
    let cpu = cpu_upgrade(form, cpu);

    let top = if memory.action == "dual-channel" {
        "memory-dual-channel"
    } else if matches!(memory.action, "add" | "replace") && memory.installed_bytes < 16 * GIB {
        "memory-capacity"
    } else if gpu.path != "none" && gpu.current.is_none() {
        "gpu"
    } else if matches!(memory.action, "add" | "replace") {
        "memory-capacity"
    } else if cpu.path == "socket" && !cpu.at_ceiling {
        "cpu"
    } else {
        "none"
    };

    Upgrades {
        form,
        gpu,
        memory,
        cpu,
        top,
    }
}

/// Agrupa os tipos de chassi de `board::chassis_name` pelo que dá para trocar.
fn form(chassis: Option<&str>) -> &'static str {
    match chassis {
        Some("desktop" | "pizza-box" | "server") => "desktop",
        Some("notebook" | "convertible" | "detachable" | "tablet" | "handheld") => "laptop",
        Some("all-in-one" | "mini-pc" | "stick-pc") => "compact",
        _ => "unknown",
    }
}

fn microarch(cpu: &CpuInfo) -> &str {
    cpu.codename.as_ref().map_or("", |c| c.microarch)
}

/// Faixa de placa de vídeo que o processador consegue alimentar sem gargalo:
/// 0 = entrada, 1 = intermediária, 2 = alta.
fn cpu_tier(cpu: &CpuInfo) -> u8 {
    if cpu.cores <= 4 {
        return 0;
    }
    let arch = microarch(cpu);
    let hybrid = ["Golden Cove", "Raptor Cove", "Redwood Cove", "Cougar Cove"];
    match arch {
        "Zen 4" | "Zen 4c" | "Zen 5" | "Zen 5c" => 2,
        a if a.starts_with("Lion Cove") => 2,
        a if hybrid.iter().any(|h| a.starts_with(h)) => {
            if cpu.cores >= 10 {
                2
            } else {
                1
            }
        }
        "Zen 2" | "Zen 3" | "Zen 3+" | "Cypress Cove" | "Sunny Cove" | "Willow Cove" => 1,
        "" if cpu.cores >= 8 => 1,
        _ => 0,
    }
}

/// Resizable BAR chegou com o Zen 2 na AMD e com a 10ª geração (Comet Lake) na Intel.
fn has_rebar(cpu: &CpuInfo) -> bool {
    let Some(codename) = &cpu.codename else {
        return false;
    };
    match cpu.vendor.as_str() {
        "AMD" => codename.microarch.starts_with("Zen "),
        "Intel" => {
            let old = [
                "Sandy Bridge",
                "Ivy Bridge",
                "Haswell",
                "Broadwell",
                "Skylake",
            ];
            codename.name.starts_with("Comet") || !old.contains(&codename.microarch)
        }
        _ => false,
    }
}

fn gpu(form: &'static str, cpu: &CpuInfo, board: &BoardInfo, gpus: &[Gpu]) -> GpuUpgrade {
    let path = match form {
        "desktop" | "unknown" => "slot",
        _ if board.external_pcie.is_some() => "external",
        _ => "none",
    };
    let mut tier = cpu_tier(cpu);
    // Thunderbolt/USB4 entrega o equivalente a PCIe x4: placas de topo ficam presas.
    if path == "external" {
        tier = tier.min(1);
    }
    let no_rebar = !has_rebar(cpu);
    let table = match tier {
        0 => &ENTRY,
        1 => &MID,
        _ => &HIGH,
    };
    let suggestions = if path == "none" {
        Vec::new()
    } else {
        table
            .iter()
            .filter(|s| !(no_rebar && s.needs_rebar))
            .copied()
            .collect()
    };
    let slot_gen = (path == "slot").then_some(board.gpu_slot_gen).flatten();

    GpuUpgrade {
        path,
        slot_gen,
        external_port: board.external_pcie,
        current: gpus
            .iter()
            .find(|g| g.integrated == Some(false))
            .and_then(|g| g.name.clone()),
        tier: ["entry", "mid", "high"][tier as usize],
        psu_watts: [550, 650, 850][tier as usize],
        suggestions,
        narrow_slot: slot_gen.is_some_and(|g| g <= 3),
        no_rebar,
    }
}

/// Módulo de tamanho "comercial" (potência de dois em GiB) que não passa de `bytes`.
fn module_size(bytes: u64) -> u64 {
    let gib = (bytes / GIB).max(1);
    (1u64 << (63 - gib.leading_zeros())) * GIB
}

fn memory_upgrade(form: &str, m: &MemoryInfo) -> MemoryUpgrade {
    let populated: Vec<_> = m.slots.iter().filter(|s| s.size_bytes > 0).collect();
    let installed = match populated.iter().map(|s| s.size_bytes).sum::<u64>() {
        0 => m.total_bytes,
        n => n,
    };
    let kind = m.kind.clone();
    let form_factor = populated.iter().find_map(|s| s.form_factor.clone());
    // LPDDR e "Row of chips" são sempre soldadas na placa.
    let soldered = kind.as_deref().is_some_and(|k| k.starts_with("LPDDR"))
        || form_factor
            .as_deref()
            .is_some_and(|f| f.to_lowercase().contains("chip"))
        || (populated.is_empty() && form == "laptop");
    // Alguns BIOS informam um máximo menor do que o já instalado: aí ele não vale.
    let max = m.max_capacity_bytes.filter(|&b| b >= installed);
    let fits = |total: u64| max.is_none_or(|mx| total <= mx);
    let module = populated.iter().map(|s| s.size_bytes).max().unwrap_or(0);
    let total = m.slots.len();
    let used = populated.len();
    let free = total.saturating_sub(used);

    let mut up = MemoryUpgrade {
        action: "maxed",
        kind,
        form_factor,
        speed_mts: populated.iter().filter_map(|s| s.speed_mts).max(),
        installed_bytes: installed,
        max_bytes: max,
        slots_total: total,
        slots_used: used,
        module_count: 0,
        module_bytes: 0,
        target_bytes: installed,
    };

    if soldered {
        up.action = "soldered";
        return up;
    }
    if populated.is_empty() {
        up.action = "unknown";
        return up;
    }
    if free > 0 && used == 1 && m.channels < 2 && fits(installed + module) {
        up.action = "dual-channel";
        up.module_count = 1;
        up.module_bytes = module;
        up.target_bytes = installed + module;
        return up;
    }
    let count = (1..=free)
        .rev()
        .find(|&n| fits(installed + n as u64 * module))
        .unwrap_or(0);
    if count > 0 {
        up.action = "add";
        up.module_count = count;
        up.module_bytes = module;
        up.target_bytes = installed + count as u64 * module;
        return up;
    }
    // Slots cheios: troca por um kit com o dobro, até o máximo da placa.
    let target = max.map_or(installed * 2, |mx| (installed * 2).min(mx));
    let per_slot = module_size(target / total.max(1) as u64);
    if per_slot * total as u64 > installed {
        up.action = "replace";
        up.module_count = total;
        up.module_bytes = per_slot;
        up.target_bytes = per_slot * total as u64;
    }
    up
}

struct Socket {
    name: &'static str,
    ceiling: &'static [&'static str],
    end: bool,
}

/// Soquete de desktop pelo codinome (tabela de `codenames.rs`).
fn socket(codename: &str) -> Option<Socket> {
    let s = |name, ceiling, end| Some(Socket { name, ceiling, end });
    match codename {
        "Summit Ridge / Naples"
        | "Pinnacle Ridge"
        | "Raven Ridge"
        | "Picasso"
        | "Matisse"
        | "Vermeer"
        | "Cezanne"
        | "Renoir" => s("AM4", &["Ryzen 7 5700X3D", "Ryzen 9 5950X"], true),
        "Raphael" | "Phoenix" | "Granite Ridge" => {
            s("AM5", &["Ryzen 7 9800X3D", "Ryzen 9 9950X3D"], false)
        }
        "Sandy Bridge" | "Ivy Bridge" => s("LGA1155", &["Core i7-3770K"], true),
        "Haswell" | "Broadwell" => s("LGA1150", &["Core i7-4790K"], true),
        "Skylake" | "Kaby Lake / Coffee Lake" => {
            s("LGA1151", &["Core i7-7700K", "Core i9-9900K"], true)
        }
        "Comet Lake-S" | "Rocket Lake" => s("LGA1200", &["Core i9-10900K", "Core i9-11900K"], true),
        "Alder Lake" | "Raptor Lake" => s("LGA1700", &["Core i7-14700K", "Core i9-14900K"], true),
        "Arrow Lake" => s("LGA1851", &["Core Ultra 9 285K"], false),
        _ => None,
    }
}

fn cpu_upgrade(form: &str, cpu: &CpuInfo) -> CpuUpgrade {
    let mut up = CpuUpgrade {
        path: "unknown",
        socket: None,
        ceiling: Vec::new(),
        platform_end: false,
        check_bios: false,
        at_ceiling: false,
    };
    // Notebooks, mini PCs e all-in-ones usam processador soldado (BGA).
    if matches!(form, "laptop" | "compact") {
        up.path = "soldered";
        return up;
    }
    let Some(socket) = cpu.codename.as_ref().and_then(|c| socket(c.name)) else {
        return up;
    };
    up.path = "socket";
    up.socket = Some(socket.name);
    up.ceiling = socket.ceiling.to_vec();
    up.platform_end = socket.end;
    up.check_bios = true;
    // "Ryzen 9 5950X" -> procura "5950X" no nome do processador atual.
    up.at_ceiling = socket.ceiling.iter().any(|c| {
        c.rsplit(' ')
            .next()
            .is_some_and(|model| cpu.brand.contains(model))
    });
    up
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::board::{Bios, Board, System};
    use crate::hw::codenames::Codename;
    use crate::hw::cpu::{ClockRange, Signature};
    use crate::hw::memory::Slot;

    fn cpu(
        brand: &str,
        vendor: &str,
        codename: Option<(&'static str, &'static str)>,
        cores: usize,
    ) -> CpuInfo {
        CpuInfo {
            brand: brand.into(),
            vendor: vendor.into(),
            vendor_id: String::new(),
            codename: codename.map(|(name, microarch)| Codename {
                name,
                microarch,
                process: "",
            }),
            signature: Signature::default(),
            microcode: None,
            cores,
            threads: cores * 2,
            clocks: ClockRange::default(),
            caches: Vec::new(),
            topology: Vec::new(),
            instructions: Vec::new(),
        }
    }

    fn board(
        chassis: &'static str,
        slot_gen: Option<u8>,
        external: Option<&'static str>,
    ) -> BoardInfo {
        BoardInfo {
            system: System {
                chassis: Some(chassis),
                ..Default::default()
            },
            board: Board::default(),
            bios: Bios::default(),
            chipset: None,
            southbridge: None,
            graphics: Vec::new(),
            gpu_slot_gen: slot_gen,
            external_pcie: external,
        }
    }

    fn slot(gib: u64, kind: &str, form: &str) -> Slot {
        Slot {
            size_bytes: gib * GIB,
            kind: Some(kind.into()),
            form_factor: Some(form.into()),
            speed_mts: Some(3200),
            ..Default::default()
        }
    }

    fn memory(slots: Vec<Slot>, channels: usize, max_gib: Option<u64>) -> MemoryInfo {
        MemoryInfo {
            total_bytes: slots.iter().map(|s| s.size_bytes).sum(),
            max_capacity_bytes: max_gib.map(|g| g * GIB),
            kind: slots.iter().find_map(|s| s.kind.clone()),
            speed_mts: Some(3200),
            channels,
            slots,
        }
    }

    #[test]
    fn notebook_com_slots_cheios_troca_o_kit() {
        // Ryzen 7 5700U num VAIO: 2 × 8 GB SODIMM, máximo 32 GB, sem Thunderbolt.
        let up = advise(
            &cpu("AMD Ryzen 7 5700U", "AMD", Some(("Lucienne", "Zen 2")), 8),
            &board("notebook", Some(3), None),
            &memory(
                vec![slot(8, "DDR4", "SODIMM"), slot(8, "DDR4", "SODIMM")],
                2,
                Some(32),
            ),
            &[],
        );
        assert_eq!(up.form, "laptop");
        assert_eq!(up.gpu.path, "none");
        assert!(up.gpu.suggestions.is_empty());
        assert_eq!(up.cpu.path, "soldered");
        assert_eq!(up.memory.action, "replace");
        assert_eq!(up.memory.module_count, 2);
        assert_eq!(up.memory.module_bytes, 16 * GIB);
        assert_eq!(up.top, "memory-capacity");
    }

    #[test]
    fn um_modulo_sugere_dual_channel_primeiro() {
        let up = advise(
            &cpu("AMD Ryzen 5 5600G", "AMD", Some(("Cezanne", "Zen 3")), 6),
            &board("desktop", Some(3), None),
            &memory(
                vec![
                    slot(8, "DDR4", "DIMM"),
                    Slot::default(),
                    Slot::default(),
                    Slot::default(),
                ],
                1,
                Some(128),
            ),
            &[],
        );
        assert_eq!(up.memory.action, "dual-channel");
        assert_eq!(up.memory.target_bytes, 16 * GIB);
        assert_eq!(up.top, "memory-dual-channel");
        // APU em PCIe 3.0: avisa sobre placas de 8 linhas.
        assert_eq!(up.gpu.path, "slot");
        assert_eq!(up.gpu.tier, "mid");
        assert!(up.gpu.narrow_slot);
        assert_eq!(up.cpu.socket, Some("AM4"));
        assert!(up.cpu.platform_end);
    }

    #[test]
    fn processador_antigo_sem_rebar_nao_recebe_arc() {
        let up = advise(
            &cpu(
                "Intel Core i5-7400",
                "Intel",
                Some(("Kaby Lake / Coffee Lake", "Skylake")),
                4,
            ),
            &board("desktop", Some(3), None),
            &memory(
                vec![slot(8, "DDR4", "DIMM"), slot(8, "DDR4", "DIMM")],
                2,
                Some(64),
            ),
            &[],
        );
        assert_eq!(up.gpu.tier, "entry");
        assert!(up.gpu.no_rebar);
        assert!(up.gpu.suggestions.iter().all(|s| !s.name.contains("Arc")));
        assert_eq!(up.cpu.socket, Some("LGA1151"));
    }

    #[test]
    fn topo_do_soquete_e_reconhecido() {
        let up = advise(
            &cpu(
                "AMD Ryzen 7 9800X3D 8-Core Processor",
                "AMD",
                Some(("Granite Ridge", "Zen 5")),
                8,
            ),
            &board("desktop", Some(5), None),
            &memory(
                vec![slot(16, "DDR5", "DIMM"), slot(16, "DDR5", "DIMM")],
                2,
                Some(192),
            ),
            &[Gpu {
                name: Some("GeForce RTX 5080".into()),
                integrated: Some(false),
                ..Default::default()
            }],
        );
        assert!(up.cpu.at_ceiling);
        assert_eq!(up.gpu.tier, "high");
        assert!(!up.gpu.narrow_slot);
        assert_eq!(up.memory.action, "replace");
        assert_eq!(up.memory.target_bytes, 64 * GIB);
        assert_eq!(up.top, "memory-capacity");
    }

    #[test]
    fn memoria_soldada_e_gpu_externa() {
        let up = advise(
            &cpu(
                "Intel Core Ultra 7 258V",
                "Intel",
                Some(("Lunar Lake", "Lion Cove + Skymont")),
                8,
            ),
            &board("notebook", Some(4), Some("thunderbolt")),
            &memory(vec![slot(32, "LPDDR5", "Row Of Chips")], 2, None),
            &[],
        );
        assert_eq!(up.memory.action, "soldered");
        assert_eq!(up.gpu.path, "external");
        assert_eq!(up.gpu.tier, "mid");
        assert_eq!(up.gpu.slot_gen, None);
        assert_eq!(up.top, "gpu");
    }

    #[test]
    fn tamanho_de_modulo_e_potencia_de_dois() {
        assert_eq!(module_size(16 * GIB), 16 * GIB);
        assert_eq!(module_size(24 * GIB), 16 * GIB);
        assert_eq!(module_size(GIB / 2), GIB);
    }
}
