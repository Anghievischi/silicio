//! Diagnóstico: cruza o que as outras abas já leram e aponta o que está
//! deixando o computador mais lento ou perto de dar problema, com a ação para
//! resolver.
//!
//! Cada regra só dispara com evidência clara no dado lido; na dúvida, fica
//! quieta. O backend devolve códigos e números; a interface monta as frases.

use super::battery::{self, Battery};
use super::board::{self, BoardInfo};
use super::cpu::{self, CpuInfo};
use super::gpu::Gpu;
use super::memory::{self, MemoryInfo};
use super::peripherals::{self, Device};
use super::spd::{self, SpdModule};
use super::storage::{self, Disk};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

const GIB: u64 = 1 << 30;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnosis {
    /// "good", "attention" (há avisos) ou "problem" (há problemas).
    pub status: &'static str,
    /// Nota de 0 a 100: cada problema tira 25 pontos e cada aviso, 10.
    pub score: u8,
    /// Do mais grave para o menos grave.
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// "info", "warning" ou "problem".
    pub severity: &'static str,
    /// Aba relacionada: "memory", "cpu", "gpu", "storage", "battery", "board" ou "peripherals".
    pub area: &'static str,
    /// Componente envolvido (disco, placa de vídeo, dispositivo), quando houver.
    pub subject: Option<String>,
    #[serde(flatten)]
    pub kind: Kind,
}

/// O que foi encontrado, com os números que a frase usa.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "kebab-case")]
pub enum Kind {
    /// Um só canal de memória: metade da largura de banda.
    MemorySingleChannel,
    /// O kit tem perfil XMP/EXPO mais rápido do que a velocidade em uso.
    MemoryProfileOff {
        profile: &'static str,
        current_mts: u32,
        profile_mts: u32,
    },
    /// Sem perfil, mas abaixo da velocidade nominal do módulo (SMBIOS).
    MemoryBelowRated {
        current_mts: u32,
        rated_mts: u32,
    },
    /// Módulos de modelos diferentes.
    MemoryMixed,
    MemoryLow {
        installed_bytes: u64,
    },
    /// Governor `powersave` num driver em que ele prende o clock no mínimo.
    CpuPowersave {
        driver: String,
    },
    /// Placa de vídeo dedicada com menos linhas PCIe do que suporta.
    GpuLinkNarrow {
        current_width: u8,
        max_width: u8,
    },
    /// Placa de vídeo dedicada sem driver carregado.
    GpuNoDriver,
    /// NVIDIA com o driver livre `nouveau`, bem mais lento.
    GpuNouveau,
    /// O próprio disco diz que está falhando.
    DiskFailing,
    /// Desgaste ou setores ruins.
    DiskCaution {
        life_left: Option<u8>,
        bad_sectors: Option<u64>,
    },
    /// Projeção da vida útil pelo ritmo de uso até aqui.
    DiskLife {
        life_left: u8,
        hours_left: u64,
    },
    DiskFull {
        mount_point: String,
        free_bytes: u64,
        free_pct: u8,
    },
    DiskHot {
        temperature_c: f32,
    },
    /// Sistema instalado num HD mecânico.
    SystemOnHdd {
        mount_point: String,
    },
    /// SSD NVMe com menos linhas do que suporta.
    DiskLinkNarrow {
        current_width: u8,
        max_width: u8,
    },
    /// Capacidade atual em % da original.
    BatteryWorn {
        health_pct: u8,
        cycles: Option<u32>,
    },
    /// BIOS com vários anos.
    BiosOld {
        years: u8,
    },
    /// Dispositivo USB 3 ligado numa porta (ou cabo) USB 2.
    UsbSlowPort {
        speed_mbps: u32,
    },
}

pub fn read(gpus: &[Gpu]) -> Diagnosis {
    let input = Input {
        cpu: &cpu::read_static(),
        board: &board::read(),
        memory: &memory::read(),
        spd: &spd::read(),
        gpus,
        disks: &storage::read(),
        batteries: &battery::read(),
        devices: &peripherals::read().devices,
        today: today(),
    };
    diagnose(&input)
}

/// Tudo o que as regras olham; separado da leitura para os testes.
pub struct Input<'a> {
    pub cpu: &'a CpuInfo,
    pub board: &'a BoardInfo,
    pub memory: &'a MemoryInfo,
    pub spd: &'a [SpdModule],
    pub gpus: &'a [Gpu],
    pub disks: &'a [Disk],
    pub batteries: &'a [Battery],
    pub devices: &'a [Device],
    /// Ano e mês de hoje.
    pub today: (i32, u32),
}

pub fn diagnose(input: &Input) -> Diagnosis {
    let mut findings = Vec::new();
    memory_checks(input, &mut findings);
    cpu_checks(input.cpu, &mut findings);
    gpu_checks(input, &mut findings);
    disk_checks(input.disks, &mut findings);
    battery_checks(input.batteries, &mut findings);
    bios_checks(input.board, input.today, &mut findings);
    usb_checks(input.devices, &mut findings);

    findings.sort_by_key(|f| rank(f.severity));
    let count = |s| findings.iter().filter(|f| f.severity == s).count() as u32;
    let (problems, warnings) = (count("problem"), count("warning"));
    Diagnosis {
        status: if problems > 0 {
            "problem"
        } else if warnings > 0 {
            "attention"
        } else {
            "good"
        },
        score: 100u32.saturating_sub(problems * 25 + warnings * 10) as u8,
        findings,
    }
}

fn rank(severity: &str) -> u8 {
    match severity {
        "problem" => 0,
        "warning" => 1,
        _ => 2,
    }
}

fn push(
    out: &mut Vec<Finding>,
    severity: &'static str,
    area: &'static str,
    subject: Option<String>,
    kind: Kind,
) {
    out.push(Finding {
        severity,
        area,
        subject,
        kind,
    });
}

/// Notebooks, mini PCs e all-in-ones raramente deixam mexer na memória pelo BIOS.
fn is_desktop(board: &BoardInfo) -> bool {
    matches!(
        board.system.chassis,
        Some("desktop" | "pizza-box" | "server") | None
    )
}

fn memory_checks(input: &Input, out: &mut Vec<Finding>) {
    let m = input.memory;
    let modules: Vec<_> = m.slots.iter().filter(|s| s.size_bytes > 0).collect();
    if modules.is_empty() {
        return;
    }
    // LPDDR soldada informa canais de 16/32 bits e não tem o que mudar.
    let soldered = m.kind.as_deref().is_some_and(|k| k.starts_with("LPDDR"));
    if m.channels < 2 && !soldered {
        push(out, "warning", "memory", None, Kind::MemorySingleChannel);
    }

    let current = modules
        .iter()
        .filter_map(|s| s.configured_mts)
        .min()
        .or(m.speed_mts);
    // O perfil mais rápido que todos os módulos têm (o kit roda no do mais lento).
    let profile = input
        .spd
        .iter()
        .filter_map(|s| s.decoded.as_ref())
        .map(|d| {
            let xmp = d.xmp.iter().flat_map(|x| &x.profiles).map(|p| p.speed_mts);
            let expo = d.expo.iter().flatten().map(|p| p.speed_mts);
            (xmp.max(), expo.max())
        })
        // `None` é menor que qualquer `Some`: basta um módulo sem o perfil para ele não valer.
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1)))
        .and_then(|(xmp, expo)| match (xmp, expo) {
            (_, Some(e)) if xmp.is_none_or(|x| e >= x) => Some(("expo", e)),
            (Some(x), _) => Some(("xmp", x)),
            _ => None,
        });
    let rated = modules.iter().filter_map(|s| s.speed_mts).min();

    match (current, profile, rated) {
        // Tolera arredondamentos do SMBIOS (3200 vs 3199).
        (Some(cur), Some((name, fast)), _) if fast > cur + 50 => push(
            out,
            "warning",
            "memory",
            None,
            Kind::MemoryProfileOff {
                profile: name,
                current_mts: cur,
                profile_mts: fast,
            },
        ),
        (Some(cur), None, Some(rated)) if rated > cur + 50 => push(
            out,
            if is_desktop(input.board) {
                "warning"
            } else {
                "info"
            },
            "memory",
            None,
            Kind::MemoryBelowRated {
                current_mts: cur,
                rated_mts: rated,
            },
        ),
        _ => {}
    }

    let mut parts: Vec<_> = modules
        .iter()
        .filter_map(|s| s.part_number.as_deref())
        .collect();
    parts.sort_unstable();
    parts.dedup();
    if parts.len() > 1 {
        push(out, "info", "memory", None, Kind::MemoryMixed);
    }

    let installed = match modules.iter().map(|s| s.size_bytes).sum::<u64>() {
        0 => m.total_bytes,
        n => n,
    };
    // Módulos de "8 GB" às vezes aparecem com um pouco menos.
    if installed < 15 * GIB {
        push(
            out,
            if installed < 7 * GIB {
                "warning"
            } else {
                "info"
            },
            "memory",
            None,
            Kind::MemoryLow {
                installed_bytes: installed,
            },
        );
    }
}

fn cpu_checks(cpu: &CpuInfo, out: &mut Vec<Finding>) {
    // Com intel_pstate e amd-pstate-epp, "powersave" é o padrão e escala bem.
    // Nos drivers passivos ele fixa o clock mínimo (Documentation/admin-guide/pm/cpufreq.rst).
    let (Some(driver), Some(governor)) = (&cpu.clocks.driver, &cpu.clocks.governor) else {
        return;
    };
    let passive = [
        "acpi-cpufreq",
        "intel_cpufreq",
        "amd-pstate",
        "cppc_cpufreq",
    ];
    if governor == "powersave" && passive.contains(&driver.as_str()) {
        push(
            out,
            "warning",
            "cpu",
            None,
            Kind::CpuPowersave {
                driver: driver.clone(),
            },
        );
    }
}

fn gpu_checks(input: &Input, out: &mut Vec<Finding>) {
    for gpu in input.gpus.iter().filter(|g| g.integrated == Some(false)) {
        let name = gpu.name.clone().or_else(|| gpu.vendor.clone());
        match gpu.driver.as_deref() {
            None if cfg!(target_os = "linux") => {
                push(out, "problem", "gpu", name.clone(), Kind::GpuNoDriver);
            }
            Some("nouveau") => push(out, "info", "gpu", name.clone(), Kind::GpuNouveau),
            _ => {}
        }
        // Em notebooks a placa dedicada costuma vir ligada com x8 ou x4 de propósito.
        if !is_desktop(input.board) {
            continue;
        }
        let Some(link) = &gpu.link else { continue };
        if let (Some(cur), Some(max)) = (link.current_width, link.max_width)
            && cur < max
        {
            push(
                out,
                if cur <= 4 && max >= 8 {
                    "warning"
                } else {
                    "info"
                },
                "gpu",
                name,
                Kind::GpuLinkNarrow {
                    current_width: cur,
                    max_width: max,
                },
            );
        }
    }
}

fn disk_name(disk: &Disk) -> String {
    let name = [disk.vendor.as_deref(), disk.model.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if name.is_empty() {
        disk.id.clone()
    } else {
        name
    }
}

fn is_system_volume(mount: &str) -> bool {
    mount == "/" || mount.eq_ignore_ascii_case("C:\\")
}

fn disk_checks(disks: &[Disk], out: &mut Vec<Finding>) {
    for disk in disks.iter().filter(|d| d.bus != Some("virtual")) {
        let name = Some(disk_name(disk));
        if let Some(h) = &disk.health {
            match h.status {
                "bad" => push(out, "problem", "storage", name.clone(), Kind::DiskFailing),
                "warning" => push(
                    out,
                    "warning",
                    "storage",
                    name.clone(),
                    Kind::DiskCaution {
                        life_left: h.life_left,
                        bad_sectors: h.bad_sectors.filter(|&n| n > 0),
                    },
                ),
                _ => {
                    // Projeção linear: as horas ligadas até aqui gastaram (100 − vida)%.
                    if let (Some(life), Some(hours)) = (h.life_left, h.power_on_hours) {
                        let used = 100u64.saturating_sub(life as u64);
                        if used >= 3 && hours >= 500 {
                            push(
                                out,
                                "info",
                                "storage",
                                name.clone(),
                                Kind::DiskLife {
                                    life_left: life,
                                    hours_left: hours * life as u64 / used,
                                },
                            );
                        }
                    }
                }
            }
        }

        // Limite em que o SSD começa a reduzir a velocidade; HDs aguentam menos.
        let hot = if disk.media == Some("hdd") {
            55.0
        } else {
            70.0
        };
        if let Some(t) = disk.temperature_c.filter(|&t| t >= hot) {
            push(
                out,
                "warning",
                "storage",
                name.clone(),
                Kind::DiskHot { temperature_c: t },
            );
        }

        if disk.bus == Some("nvme")
            && let Some(link) = &disk.link
            && let (Some(cur), Some(max)) = (link.current_width, link.max_width)
            && cur < max
        {
            push(
                out,
                "info",
                "storage",
                name.clone(),
                Kind::DiskLinkNarrow {
                    current_width: cur,
                    max_width: max,
                },
            );
        }

        for v in &disk.volumes {
            if disk.media == Some("hdd") && !disk.removable && is_system_volume(&v.mount_point) {
                push(
                    out,
                    "info",
                    "storage",
                    name.clone(),
                    Kind::SystemOnHdd {
                        mount_point: v.mount_point.clone(),
                    },
                );
            }
            // Partições pequenas (/boot, EFI) enchem por natureza.
            if v.total_bytes < 4 * GIB {
                continue;
            }
            let pct = (v.available_bytes as f64 / v.total_bytes as f64 * 100.0) as u8;
            if pct < 10 {
                push(
                    out,
                    if pct < 3 { "problem" } else { "warning" },
                    "storage",
                    name.clone(),
                    Kind::DiskFull {
                        mount_point: v.mount_point.clone(),
                        free_bytes: v.available_bytes,
                        free_pct: pct,
                    },
                );
            }
        }
    }
}

fn battery_checks(batteries: &[Battery], out: &mut Vec<Finding>) {
    for b in batteries {
        let (Some(full), Some(design)) = (b.full_wh, b.design_wh) else {
            continue;
        };
        if design <= 0.0 {
            continue;
        }
        // Mesmos limites da aba Bateria (`battery.rating`).
        let pct = (full / design * 100.0).round().min(100.0) as u8;
        if pct < 80 {
            push(
                out,
                if pct < 60 { "problem" } else { "warning" },
                "battery",
                b.model.clone(),
                Kind::BatteryWorn {
                    health_pct: pct,
                    cycles: b.cycles,
                },
            );
        }
    }
}

/// Data do SMBIOS (MM/DD/AAAA) em ano e mês.
fn smbios_month(date: &str) -> Option<(i32, u32)> {
    let mut parts = date.split('/');
    let month = parts.next()?.parse().ok()?;
    let _day: u32 = parts.next()?.parse().ok()?;
    let year = parts.next()?.parse().ok()?;
    (1..=12).contains(&month).then_some((year, month))
}

fn bios_checks(board: &BoardInfo, (year, month): (i32, u32), out: &mut Vec<Finding>) {
    let Some((y, m)) = board.bios.date.as_deref().and_then(smbios_month) else {
        return;
    };
    let months = (year - y) * 12 + month as i32 - m as i32;
    // As atualizações corrigem microcódigo, compatibilidade de memória e falhas de segurança.
    if months >= 36 {
        push(
            out,
            "info",
            "board",
            None,
            Kind::BiosOld {
                years: (months / 12) as u8,
            },
        );
    }
}

fn usb_checks(devices: &[Device], out: &mut Vec<Finding>) {
    for d in devices.iter().filter(|d| d.bus == "usb") {
        // Só pendrives e HDs externos: webcams e receptores USB 3 rodam bem em USB 2.
        if !d.kinds.contains(&"storage") {
            continue;
        }
        let usb3 = d
            .usb_version
            .as_deref()
            .and_then(|v| v.trim().split('.').next()?.parse::<u8>().ok())
            .is_some_and(|major| major >= 3);
        if let Some(speed) = d.speed_mbps.filter(|&s| usb3 && s <= 480.0) {
            push(
                out,
                "warning",
                "peripherals",
                d.name.clone(),
                Kind::UsbSlowPort {
                    speed_mbps: speed as u32,
                },
            );
        }
    }
}

/// Ano e mês de hoje (UTC), pelo algoritmo `civil_from_days` de Howard Hinnant.
fn today() -> (i32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    (year, month)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::board::{Bios, Board, System};
    use crate::hw::cpu::{ClockRange, Signature};
    use crate::hw::memory::Slot;
    use crate::hw::pci::PcieLink;
    use crate::hw::storage::{Health, Volume};

    fn cpu(driver: &str, governor: &str) -> CpuInfo {
        CpuInfo {
            brand: "AMD Ryzen 7 5700U".into(),
            vendor: "AMD".into(),
            vendor_id: String::new(),
            codename: None,
            signature: Signature::default(),
            microcode: None,
            cores: 8,
            threads: 16,
            clocks: ClockRange {
                driver: Some(driver.into()),
                governor: Some(governor.into()),
                ..Default::default()
            },
            caches: Vec::new(),
            topology: Vec::new(),
            instructions: Vec::new(),
        }
    }

    fn board(chassis: &'static str, bios_date: &str) -> BoardInfo {
        BoardInfo {
            system: System {
                chassis: Some(chassis),
                ..Default::default()
            },
            board: Board::default(),
            bios: Bios {
                date: Some(bios_date.into()),
                ..Default::default()
            },
            chipset: None,
            southbridge: None,
            graphics: Vec::new(),
            gpu_slot_gen: None,
            external_pcie: None,
        }
    }

    fn slot(gib: u64, rated: u32, configured: u32, part: &str) -> Slot {
        Slot {
            size_bytes: gib * GIB,
            kind: Some("DDR4".into()),
            speed_mts: Some(rated),
            configured_mts: Some(configured),
            part_number: Some(part.into()),
            ..Default::default()
        }
    }

    fn memory(slots: Vec<Slot>, channels: usize) -> MemoryInfo {
        MemoryInfo {
            total_bytes: slots.iter().map(|s| s.size_bytes).sum(),
            max_capacity_bytes: None,
            kind: Some("DDR4".into()),
            speed_mts: slots.iter().filter_map(|s| s.configured_mts).min(),
            channels,
            slots,
        }
    }

    fn disk(media: &'static str, health: Option<Health>, volumes: Vec<Volume>) -> Disk {
        Disk {
            id: "nvme0n1".into(),
            model: Some("SSD 1TB".into()),
            vendor: None,
            firmware: None,
            size_bytes: 1_000_000_000_000,
            bus: Some(if media == "hdd" { "sata" } else { "nvme" }),
            media: Some(media),
            removable: false,
            link: None,
            sata_speed: None,
            temperature_c: Some(40.0),
            health,
            volumes,
        }
    }

    fn volume(mount: &str, total_gib: u64, free_gib: u64) -> Volume {
        Volume {
            mount_point: mount.into(),
            label: None,
            file_system: None,
            total_bytes: total_gib * GIB,
            available_bytes: free_gib * GIB,
        }
    }

    struct Fixture {
        cpu: CpuInfo,
        board: BoardInfo,
        memory: MemoryInfo,
        spd: Vec<SpdModule>,
        gpus: Vec<Gpu>,
        disks: Vec<Disk>,
        batteries: Vec<Battery>,
        devices: Vec<Device>,
    }

    impl Fixture {
        /// Um desktop sem nada de errado.
        fn healthy() -> Self {
            Fixture {
                cpu: cpu("amd-pstate-epp", "powersave"),
                board: board("desktop", "06/10/2026"),
                memory: memory(
                    vec![slot(16, 3200, 3200, "A"), slot(16, 3200, 3200, "A")],
                    2,
                ),
                spd: Vec::new(),
                gpus: Vec::new(),
                disks: vec![disk("ssd", None, vec![volume("/", 500, 200)])],
                batteries: Vec::new(),
                devices: Vec::new(),
            }
        }

        fn run(&self) -> Diagnosis {
            diagnose(&Input {
                cpu: &self.cpu,
                board: &self.board,
                memory: &self.memory,
                spd: &self.spd,
                gpus: &self.gpus,
                disks: &self.disks,
                batteries: &self.batteries,
                devices: &self.devices,
                today: (2026, 10),
            })
        }
    }

    fn codes(d: &Diagnosis) -> Vec<&Kind> {
        d.findings.iter().map(|f| &f.kind).collect()
    }

    #[test]
    fn computador_saudavel_nao_tem_achados() {
        let d = Fixture::healthy().run();
        assert!(d.findings.is_empty(), "{:?}", d.findings);
        assert_eq!(d.status, "good");
        assert_eq!(d.score, 100);
    }

    #[test]
    fn notebook_do_vaio_com_bateria_gasta() {
        // Dados reais de um Ryzen 7 5700U: bateria com 60% da capacidade original,
        // SSD com 86% de vida após 4652 horas e BIOS de dezembro de 2022.
        let mut f = Fixture::healthy();
        f.board = board("notebook", "12/13/2022");
        f.memory = memory(
            vec![slot(8, 3200, 3200, "SMS4"), slot(8, 3200, 3200, "SMS4")],
            2,
        );
        f.batteries = vec![Battery {
            model: Some("575983".into()),
            design_wh: Some(55.209),
            full_wh: Some(33.113),
            cycles: Some(114),
            ..Default::default()
        }];
        f.disks = vec![disk(
            "ssd",
            Some(Health {
                status: "good",
                life_left: Some(86),
                power_on_hours: Some(4652),
                ..Default::default()
            }),
            vec![volume("/", 474, 198), volume("/boot", 1, 0)],
        )];
        let d = f.run();
        // 59,98% arredonda para 60: "desgastada", como na aba Bateria.
        assert_eq!(d.status, "attention");
        assert_eq!(d.score, 90);
        assert_eq!(d.findings[0].severity, "warning");
        assert_eq!(
            d.findings[0].kind,
            Kind::BatteryWorn {
                health_pct: 60,
                cycles: Some(114)
            }
        );
        assert!(codes(&d).contains(&&Kind::DiskLife {
            life_left: 86,
            hours_left: 28_576
        }));
        assert!(codes(&d).contains(&&Kind::BiosOld { years: 3 }));
        // /boot cheio não conta: é pequeno por natureza.
        assert!(!codes(&d).iter().any(|k| matches!(k, Kind::DiskFull { .. })));
    }

    #[test]
    fn memoria_em_um_canal_e_abaixo_do_nominal() {
        let mut f = Fixture::healthy();
        f.memory = memory(vec![slot(8, 3200, 2400, "A"), Slot::default()], 1);
        let d = f.run();
        let kinds = codes(&d);
        assert!(kinds.contains(&&Kind::MemorySingleChannel));
        assert!(kinds.contains(&&Kind::MemoryBelowRated {
            current_mts: 2400,
            rated_mts: 3200
        }));
        assert!(kinds.contains(&&Kind::MemoryLow {
            installed_bytes: 8 * GIB
        }));
        // Num desktop dá para corrigir pelo BIOS: é aviso, não só informação.
        assert_eq!(
            d.findings
                .iter()
                .find(|f| matches!(f.kind, Kind::MemoryBelowRated { .. }))
                .unwrap()
                .severity,
            "warning"
        );
    }

    #[test]
    fn perfil_xmp_desligado() {
        let mut f = Fixture::healthy();
        f.memory = memory(
            vec![slot(16, 2133, 2133, "A"), slot(16, 2133, 2133, "A")],
            2,
        );
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ddr4-sodimm-3200.spd"
        ))
        .unwrap();
        let mut module = spd::decode("0-0050", &data);
        // O fixture não tem XMP: acrescenta um perfil de 3600.
        let decoded = module.decoded.as_mut().unwrap();
        decoded.xmp = Some(spd::Xmp {
            version: "2.0".into(),
            profiles: vec![spd::Profile {
                number: 1,
                name: None,
                voltage: 1.35,
                speed_mts: 3600,
                clock_mhz: 1800.0,
                cl: 18,
                trcd: 22,
                trp: 22,
                tras: 42,
                trc: 64,
                trfc1: 630,
            }],
        });
        f.spd = vec![module];
        let d = f.run();
        assert_eq!(
            codes(&d),
            vec![&Kind::MemoryProfileOff {
                profile: "xmp",
                current_mts: 2133,
                profile_mts: 3600
            }]
        );
    }

    #[test]
    fn governor_powersave_so_avisa_em_driver_passivo() {
        let mut f = Fixture::healthy();
        f.cpu = cpu("acpi-cpufreq", "powersave");
        assert_eq!(
            codes(&f.run()),
            vec![&Kind::CpuPowersave {
                driver: "acpi-cpufreq".into()
            }]
        );
        f.cpu = cpu("intel_pstate", "powersave");
        assert!(f.run().findings.is_empty());
    }

    #[test]
    fn placa_de_video_em_x4_num_desktop() {
        let mut f = Fixture::healthy();
        f.gpus = vec![Gpu {
            name: Some("GeForce RTX 5070".into()),
            integrated: Some(false),
            driver: Some("nvidia".into()),
            link: Some(PcieLink {
                current_gen: Some(4),
                current_width: Some(4),
                max_gen: Some(5),
                max_width: Some(16),
            }),
            ..Default::default()
        }];
        let d = f.run();
        assert_eq!(d.findings.len(), 1);
        assert_eq!(d.findings[0].severity, "warning");
        assert_eq!(d.findings[0].subject.as_deref(), Some("GeForce RTX 5070"));
        // Em notebook a mesma leitura é esperada.
        f.board = board("notebook", "06/10/2026");
        assert!(f.run().findings.is_empty());
    }

    #[test]
    fn disco_cheio_quente_e_falhando() {
        let mut f = Fixture::healthy();
        let mut hdd = disk(
            "hdd",
            Some(Health {
                status: "bad",
                failing: true,
                ..Default::default()
            }),
            vec![volume("C:\\", 1000, 20)],
        );
        hdd.temperature_c = Some(58.0);
        f.disks = vec![hdd];
        let d = f.run();
        let kinds = codes(&d);
        assert_eq!(d.findings[0].kind, Kind::DiskFailing);
        assert!(kinds.contains(&&Kind::DiskHot {
            temperature_c: 58.0
        }));
        assert!(kinds.contains(&&Kind::SystemOnHdd {
            mount_point: "C:\\".into()
        }));
        assert!(
            kinds
                .iter()
                .any(|k| matches!(k, Kind::DiskFull { free_pct: 2, .. }))
        );
        // Dois problemas e um aviso.
        assert_eq!(d.score, 40);
    }

    #[test]
    fn pendrive_usb3_em_porta_usb2() {
        let mut f = Fixture::healthy();
        let device = |speed: f32, kinds: Vec<&'static str>| Device {
            bus: "usb",
            vendor_id: None,
            product_id: None,
            vendor: None,
            name: Some("Pendrive".into()),
            kinds,
            speed_mbps: Some(speed),
            usb_version: Some("3.20".into()),
            max_power_ma: None,
            driver: None,
        };
        f.devices = vec![
            device(480.0, vec!["storage"]),
            device(5000.0, vec!["storage"]),
            device(480.0, vec!["camera"]),
        ];
        assert_eq!(
            codes(&f.run()),
            vec![&Kind::UsbSlowPort { speed_mbps: 480 }]
        );
    }

    #[test]
    fn datas_do_smbios_e_de_hoje() {
        assert_eq!(smbios_month("12/13/2022"), Some((2022, 12)));
        assert_eq!(smbios_month("13/01/2022"), None);
        assert_eq!(smbios_month("V1.21"), None);
        let (year, month) = today();
        assert!(year >= 2026 && (1..=12).contains(&month));
    }
}
