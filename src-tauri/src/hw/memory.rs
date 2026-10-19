//! Memória RAM: total e uso ao vivo (sysinfo) e módulos instalados (SMBIOS).
//!
//! No Linux os módulos vêm da base do udev (`/run/udev/data/+dmi:id`), onde o
//! systemd já decodificou a tabela SMBIOS tipo 17. Assim não precisamos de root
//! para ler `/sys/firmware/dmi/tables/DMI`.

use serde::Serialize;
use std::collections::BTreeMap;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub max_capacity_bytes: Option<u64>,
    pub kind: Option<String>,
    pub speed_mts: Option<u32>,
    pub channels: usize,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Slot {
    pub locator: Option<String>,
    pub bank: Option<String>,
    pub size_bytes: u64,
    pub kind: Option<String>,
    pub detail: Option<String>,
    pub form_factor: Option<String>,
    pub speed_mts: Option<u32>,
    pub configured_mts: Option<u32>,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub rank: Option<u8>,
    pub data_width: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryLive {
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
}

pub fn read() -> MemoryInfo {
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
    );
    let (slots, max_capacity_bytes) = platform::slots();

    let populated: Vec<&Slot> = slots.iter().filter(|s| s.size_bytes > 0).collect();
    let channels = {
        let mut banks: Vec<&str> = populated.iter().filter_map(|s| channel_of(s)).collect();
        banks.sort();
        banks.dedup();
        banks.len().max(usize::from(!populated.is_empty()))
    };

    MemoryInfo {
        total_bytes: sys.total_memory(),
        max_capacity_bytes,
        kind: populated.iter().find_map(|s| s.kind.clone()),
        speed_mts: populated
            .iter()
            .filter_map(|s| s.configured_mts.or(s.speed_mts))
            .min(),
        channels,
        slots,
    }
}

pub fn read_live(sys: &mut System) -> MemoryLive {
    sys.refresh_memory();
    MemoryLive {
        used_bytes: sys.used_memory(),
        available_bytes: sys.available_memory(),
        total_bytes: sys.total_memory(),
        swap_used_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
    }
}

/// "P0 CHANNEL A" -> "A". Placas que não informam canal contam como um só.
/// No Windows o canal costuma vir no DeviceLocator ("ChannelA-DIMM0").
fn channel_of(slot: &Slot) -> Option<&str> {
    [slot.bank.as_deref(), slot.locator.as_deref()]
        .into_iter()
        .flatten()
        .find_map(|text| {
            let idx = text.to_uppercase().find("CHANNEL")?;
            let rest = text.get(idx + "CHANNEL".len()..)?.trim();
            rest.split(['-', ' ', '_']).next().filter(|c| !c.is_empty())
        })
}

/// Fabricantes de módulo pelo código JEDEC JEP106 (banco, código com paridade).
pub(super) fn jedec_vendor(bank: u8, code: u8) -> Option<&'static str> {
    Some(match (bank, code) {
        (1, 0x2C) => "Micron",
        (1, 0xCE) => "Samsung",
        (1, 0xAD) => "SK hynix",
        (1, 0xFE) => "Elpida",
        (2, 0x94) => "Smart Modular",
        (2, 0x98) => "Kingston",
        (3, 0x9E) => "Corsair",
        (5, 0xCB) => "ADATA",
        (5, 0xCD) => "G.Skill",
        (6, 0x9B) => "Crucial",
        _ => return None,
    })
}

/// Lê o formato do udev: "Bank 2, Hex 0x94".
fn decode_manufacturer_id(raw: &str) -> Option<&'static str> {
    let (bank, hex) = raw.split_once(',')?;
    let bank: u8 = bank.trim().strip_prefix("Bank")?.trim().parse().ok()?;
    let code = u8::from_str_radix(
        hex.trim()
            .strip_prefix("Hex")?
            .trim()
            .trim_start_matches("0x"),
        16,
    )
    .ok()?;
    jedec_vendor(bank, code)
}

fn useful(value: &str) -> Option<String> {
    let v = value.trim();
    let lower = v.to_lowercase();
    let junk = v.is_empty()
        || ["unknown", "not specified", "none"].contains(&lower.as_str())
        || lower.starts_with("undefined");
    (!junk).then(|| v.to_string())
}

/// Monta os slots a partir das chaves MEMORY_DEVICE_<n>_<CAMPO>=valor.
fn parse_udev(text: &str) -> (Vec<Slot>, Option<u64>) {
    let mut fields: BTreeMap<u32, BTreeMap<&str, &str>> = BTreeMap::new();
    let mut max_capacity = None;

    for line in text.lines() {
        let line = line.strip_prefix("E:").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key == "MEMORY_ARRAY_MAX_CAPACITY" {
            max_capacity = value.parse().ok();
        } else if let Some(rest) = key.strip_prefix("MEMORY_DEVICE_")
            && let Some((idx, field)) = rest.split_once('_')
            && let Ok(idx) = idx.parse()
        {
            fields.entry(idx).or_default().insert(field, value);
        }
    }

    let slots = fields
        .into_values()
        .map(|f| {
            fn num<T: std::str::FromStr>(f: &BTreeMap<&str, &str>, k: &str) -> Option<T> {
                f.get(k).and_then(|v| v.parse().ok())
            }
            let get = |k: &str| f.get(k).and_then(|v| useful(v));
            Slot {
                locator: get("LOCATOR"),
                bank: get("BANK_LOCATOR"),
                size_bytes: num(&f, "SIZE").unwrap_or(0),
                kind: get("TYPE"),
                detail: get("TYPE_DETAIL"),
                form_factor: get("FORM_FACTOR"),
                speed_mts: num(&f, "SPEED_MTS"),
                configured_mts: num(&f, "CONFIGURED_SPEED_MTS"),
                manufacturer: get("MANUFACTURER").or_else(|| {
                    f.get("MODULE_MANUFACTURER_ID")
                        .and_then(|id| decode_manufacturer_id(id))
                        .map(String::from)
                }),
                part_number: get("PART_NUMBER"),
                rank: num(&f, "RANK"),
                data_width: num(&f, "DATA_WIDTH"),
            }
        })
        .collect();
    (slots, max_capacity)
}

#[cfg(target_os = "linux")]
mod platform {
    use super::Slot;

    pub fn slots() -> (Vec<Slot>, Option<u64>) {
        std::fs::read_to_string("/run/udev/data/+dmi:id")
            .map(|text| super::parse_udev(&text))
            .unwrap_or_default()
    }
}

#[cfg(windows)]
mod platform {
    use super::{Slot, useful};
    use crate::hw::wmi;

    /// Tipos de memória do SMBIOS (tabela 7.18.2).
    fn memory_type(code: u64) -> Option<&'static str> {
        Some(match code {
            24 => "DDR3",
            26 => "DDR4",
            29 => "LPDDR3",
            30 => "LPDDR4",
            34 => "DDR5",
            35 => "LPDDR5",
            _ => return None,
        })
    }

    fn form_factor(code: u64) -> Option<&'static str> {
        Some(match code {
            8 => "DIMM",
            12 => "SODIMM",
            _ => return None,
        })
    }

    pub fn slots() -> (Vec<Slot>, Option<u64>) {
        let rows = wmi::query(
            "SELECT BankLabel, DeviceLocator, Capacity, SMBIOSMemoryType, FormFactor, Speed, \
             ConfiguredClockSpeed, Manufacturer, PartNumber, DataWidth, Attributes FROM Win32_PhysicalMemory",
        );
        let slots = rows
            .iter()
            .map(|r| Slot {
                locator: wmi::text(r, "DeviceLocator").and_then(|v| useful(&v)),
                bank: wmi::text(r, "BankLabel").and_then(|v| useful(&v)),
                size_bytes: wmi::number(r, "Capacity").unwrap_or(0),
                kind: wmi::number(r, "SMBIOSMemoryType")
                    .and_then(memory_type)
                    .map(String::from),
                detail: None,
                form_factor: wmi::number(r, "FormFactor")
                    .and_then(form_factor)
                    .map(String::from),
                speed_mts: wmi::number(r, "Speed").map(|v| v as u32).filter(|v| *v > 0),
                configured_mts: wmi::number(r, "ConfiguredClockSpeed")
                    .map(|v| v as u32)
                    .filter(|v| *v > 0),
                manufacturer: wmi::text(r, "Manufacturer").and_then(|v| useful(&v)),
                part_number: wmi::text(r, "PartNumber").and_then(|v| useful(&v)),
                rank: wmi::number(r, "Attributes")
                    .map(|v| v as u8)
                    .filter(|v| *v > 0),
                data_width: wmi::number(r, "DataWidth").map(|v| v as u16),
            })
            .collect();
        // MaxCapacityEx vem em KB.
        let max = wmi::query("SELECT MaxCapacityEx FROM Win32_PhysicalMemoryArray")
            .first()
            .and_then(|r| wmi::number(r, "MaxCapacityEx"))
            .map(|kb| kb * 1024);
        (slots, max)
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::Slot;

    pub fn slots() -> (Vec<Slot>, Option<u64>) {
        (Vec::new(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UDEV: &str = "I:1\nE:MEMORY_ARRAY_MAX_CAPACITY=34359738368\n\
E:MEMORY_DEVICE_0_SIZE=8589934592\nE:MEMORY_DEVICE_0_BANK_LOCATOR=P0 CHANNEL A\n\
E:MEMORY_DEVICE_0_TYPE=DDR4\nE:MEMORY_DEVICE_0_SPEED_MTS=3200\nE:MEMORY_DEVICE_0_MANUFACTURER=Unknown\n\
E:MEMORY_DEVICE_0_MODULE_MANUFACTURER_ID=Bank 2, Hex 0x94\n\
E:MEMORY_DEVICE_1_SIZE=8589934592\nE:MEMORY_DEVICE_1_BANK_LOCATOR=P0 CHANNEL B\n\
E:MEMORY_DEVICE_1_MANUFACTURER=Kingston\n";

    #[test]
    fn le_slots_do_udev() {
        let (slots, max) = parse_udev(UDEV);
        assert_eq!(max, Some(34359738368));
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].kind.as_deref(), Some("DDR4"));
        assert_eq!(slots[0].manufacturer.as_deref(), Some("Smart Modular"));
        assert_eq!(slots[1].manufacturer.as_deref(), Some("Kingston"));
        assert_eq!(channel_of(&slots[0]), Some("A"));
        assert_eq!(channel_of(&slots[1]), Some("B"));
    }
}
