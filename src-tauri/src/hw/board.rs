//! Sistema, placa-mãe, BIOS, chipset e interface gráfica.

use super::pci::{self, PciDevice};
use serde::Serialize;
use sysinfo::{Motherboard, Product};

#[derive(Debug, Clone, Serialize)]
pub struct BoardInfo {
    pub system: System,
    pub board: Board,
    pub bios: Bios,
    pub chipset: Option<PciDevice>,
    pub southbridge: Option<PciDevice>,
    pub graphics: Vec<PciDevice>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct System {
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub family: Option<String>,
    pub chassis: Option<&'static str>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Board {
    pub vendor: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Bios {
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
    pub release: Option<String>,
}

// Classes PCI (classe << 8 | subclasse).
const HOST_BRIDGE: u32 = 0x0600;
const ISA_BRIDGE: u32 = 0x0601;
const DISPLAY: u32 = 0x03;

pub fn read() -> BoardInfo {
    let mb = Motherboard::new();
    let devices = pci::list();

    BoardInfo {
        system: System {
            vendor: clean(Product::vendor_name()),
            product: clean(Product::name()),
            family: clean(Product::family()),
            chassis: platform::chassis_type().map(chassis_name),
        },
        board: Board {
            vendor: clean(mb.as_ref().and_then(|m| m.vendor_name())),
            name: clean(mb.as_ref().and_then(|m| m.name())),
            version: clean(mb.as_ref().and_then(|m| m.version())),
        },
        bios: platform::bios(),
        // O primeiro host bridge (00:00.0) identifica o chipset / root complex.
        chipset: devices.iter().find(|d| d.class == HOST_BRIDGE).cloned(),
        southbridge: devices.iter().find(|d| d.class == ISA_BRIDGE).cloned(),
        graphics: devices
            .iter()
            .filter(|d| d.class >> 8 == DISPLAY)
            .cloned()
            .collect(),
    }
}

/// Descarta os valores de preenchimento que os fabricantes deixam no SMBIOS.
fn clean(value: Option<String>) -> Option<String> {
    const PLACEHOLDERS: [&str; 6] = [
        "to be filled by o.e.m.",
        "default string",
        "not applicable",
        "not specified",
        "system product name",
        "none",
    ];
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !PLACEHOLDERS.contains(&v.to_lowercase().as_str()))
}

/// Tipos de chassi da especificação SMBIOS (tabela 3.3.4.1), como códigos que a
/// interface traduz.
fn chassis_name(kind: u8) -> &'static str {
    match kind {
        3 | 4 | 6 | 7 => "desktop",
        5 => "pizza-box",
        8 | 9 | 10 | 14 => "notebook",
        11 => "handheld",
        13 => "all-in-one",
        17 | 23 | 28 => "server",
        30 => "tablet",
        31 => "convertible",
        32 => "detachable",
        35 => "mini-pc",
        36 => "stick-pc",
        _ => "other",
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{Bios, clean};
    use std::fs;

    fn dmi(file: &str) -> Option<String> {
        clean(fs::read_to_string(format!("/sys/class/dmi/id/{file}")).ok())
    }

    pub fn bios() -> Bios {
        Bios {
            vendor: dmi("bios_vendor"),
            version: dmi("bios_version"),
            date: dmi("bios_date"),
            release: dmi("bios_release"),
        }
    }

    pub fn chassis_type() -> Option<u8> {
        dmi("chassis_type")?.parse().ok()
    }
}

#[cfg(windows)]
mod platform {
    use super::{Bios, clean};
    use crate::hw::wmi;

    /// Data CIM ("20221213000000.000000+000") no formato SMBIOS (MM/DD/AAAA).
    fn cim_date(raw: &str) -> Option<String> {
        let (y, rest) = raw.split_at_checked(4)?;
        let (m, rest) = rest.split_at_checked(2)?;
        let d = rest.get(..2)?;
        Some(format!("{m}/{d}/{y}"))
    }

    pub fn bios() -> Bios {
        let rows =
            wmi::query("SELECT Manufacturer, SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS");
        let Some(row) = rows.first() else {
            return Bios::default();
        };
        Bios {
            vendor: clean(wmi::text(row, "Manufacturer")),
            version: clean(wmi::text(row, "SMBIOSBIOSVersion")),
            date: wmi::text(row, "ReleaseDate").and_then(|d| cim_date(&d)),
            release: None,
        }
    }

    pub fn chassis_type() -> Option<u8> {
        let rows = wmi::query("SELECT ChassisTypes FROM Win32_SystemEnclosure");
        match rows.first()?.get("ChassisTypes")? {
            ::wmi::Variant::Array(items) => items.iter().find_map(|v| match v {
                ::wmi::Variant::UI2(n) => u8::try_from(*n).ok(),
                ::wmi::Variant::I4(n) => u8::try_from(*n).ok(),
                _ => None,
            }),
            _ => None,
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::Bios;

    pub fn bios() -> Bios {
        Bios::default()
    }

    pub fn chassis_type() -> Option<u8> {
        None
    }
}
