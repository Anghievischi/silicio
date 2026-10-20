//! Dispositivos PCI: sysfs no Linux e Configuration Manager no Windows, com
//! nomes da base `pci.ids` (a do sistema ou a embutida em `data/`).

// No Windows o leitor do sysfs não é usado.
#![cfg_attr(windows, allow(dead_code))]

use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct PciDevice {
    pub address: String,
    pub class: u32,
    pub vendor_id: u16,
    pub device_id: u16,
    pub revision: Option<u8>,
    pub vendor: Option<String>,
    pub name: Option<String>,
    pub link: Option<PcieLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PcieLink {
    pub current_gen: Option<u8>,
    pub current_width: Option<u8>,
    pub max_gen: Option<u8>,
    pub max_width: Option<u8>,
}

const PCI_IDS: [&str; 3] = [
    "/usr/share/hwdata/pci.ids",
    "/usr/share/misc/pci.ids",
    "/usr/share/pci.ids",
];

/// Subconjunto da `pci.ids` gerado por `packaging/scripts/update-pci-ids.py`.
const EMBEDDED_PCI_IDS: &str = include_str!("../../data/pci-subset.ids");

pub fn list() -> Vec<PciDevice> {
    #[cfg(windows)]
    let mut devices = windows_pci::list();
    #[cfg(not(windows))]
    let mut devices = list_sysfs();

    devices.sort_by(|a, b| a.address.cmp(&b.address));
    name_devices(&mut devices);
    devices
}

/// Preenche fabricante e nome pela `pci.ids`; o que a base não conhece mantém
/// o nome que o sistema já tinha dado (no Windows, a descrição do driver).
fn name_devices(devices: &mut [PciDevice]) {
    let wanted: Vec<(u16, u16)> = devices.iter().map(|d| (d.vendor_id, d.device_id)).collect();
    let db = PCI_IDS
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())
        .unwrap_or_else(|| EMBEDDED_PCI_IDS.to_string());
    let (vendors, products) = lookup_names(&db, &wanted);
    for d in devices {
        if let Some(vendor) = vendors.get(&d.vendor_id) {
            d.vendor = Some(vendor.clone());
        }
        if let Some(name) = products.get(&(d.vendor_id, d.device_id)) {
            d.name = Some(name.clone());
        }
    }
}

fn list_sysfs() -> Vec<PciDevice> {
    let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| read_device(&e.path()))
        .collect()
}

fn read_device(path: &Path) -> Option<PciDevice> {
    let hex = |file: &str| -> Option<u32> {
        let raw = fs::read_to_string(path.join(file)).ok()?;
        u32::from_str_radix(raw.trim().trim_start_matches("0x"), 16).ok()
    };
    Some(PciDevice {
        address: path.file_name()?.to_string_lossy().into_owned(),
        class: hex("class")? >> 8,
        vendor_id: hex("vendor")? as u16,
        device_id: hex("device")? as u16,
        revision: hex("revision").map(|r| r as u8),
        vendor: None,
        name: None,
        link: read_link(path),
    })
}

fn read_link(path: &Path) -> Option<PcieLink> {
    let text = |file: &str| {
        fs::read_to_string(path.join(file))
            .ok()
            .map(|s| s.trim().to_string())
    };
    let width = |file: &str| text(file).and_then(|w| w.parse().ok()).filter(|w| *w > 0);
    let link = PcieLink {
        current_gen: text("current_link_speed").and_then(|s| pcie_gen(&s)),
        current_width: width("current_link_width"),
        max_gen: text("max_link_speed").and_then(|s| pcie_gen(&s)),
        max_width: width("max_link_width"),
    };
    (link.max_gen.is_some() || link.current_gen.is_some()).then_some(link)
}

/// "16.0 GT/s PCIe" -> geração 4.
fn pcie_gen(speed: &str) -> Option<u8> {
    let gts: f32 = speed.split_whitespace().next()?.parse().ok()?;
    Some(match gts {
        x if x >= 64.0 => 6,
        x if x >= 32.0 => 5,
        x if x >= 16.0 => 4,
        x if x >= 8.0 => 3,
        x if x >= 5.0 => 2,
        x if x >= 2.5 => 1,
        _ => return None,
    })
}

type Names = (HashMap<u16, String>, HashMap<(u16, u16), String>);

/// Percorre o `pci.ids` uma vez e guarda só os nomes pedidos.
fn lookup_names(db: &str, wanted: &[(u16, u16)]) -> Names {
    let mut vendors = HashMap::new();
    let mut products = HashMap::new();
    let mut current: Option<u16> = None;

    for line in db.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        // A lista de classes no fim do arquivo usa o prefixo "C ".
        if line.starts_with("C ") {
            break;
        }
        if let Some(rest) = line.strip_prefix('\t') {
            if rest.starts_with('\t') {
                continue; // subsistemas
            }
            let Some(vendor) = current else { continue };
            if let Some((id, name)) = split_entry(rest)
                && wanted.contains(&(vendor, id))
            {
                products.insert((vendor, id), name.to_string());
            }
        } else if let Some((id, name)) = split_entry(line) {
            current = wanted.iter().any(|(v, _)| *v == id).then_some(id);
            if current.is_some() {
                vendors.insert(id, short_vendor(name));
            }
        }
    }
    (vendors, products)
}

fn split_entry(line: &str) -> Option<(u16, &str)> {
    let (id, name) = line.split_once("  ")?;
    Some((u16::from_str_radix(id.trim(), 16).ok()?, name.trim()))
}

/// "Advanced Micro Devices, Inc. [AMD]" -> "AMD".
fn short_vendor(name: &str) -> String {
    match (name.rfind('['), name.rfind(']')) {
        (Some(a), Some(b)) if b > a + 1 => name[a + 1..b].to_string(),
        _ => name
            .trim_end_matches(", Inc.")
            .trim_end_matches(" Corporation")
            .to_string(),
    }
}

#[cfg(windows)]
mod windows_pci {
    use super::{PciDevice, PcieLink};
    use crate::hw::devnode::{id_field, locate, number, present_ids, text};
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_Manufacturer};
    use windows::Win32::Foundation::DEVPROPKEY;
    use windows::core::GUID;

    /// Propriedades de dispositivos PCI (`DEVPKEY_PciDevice_*` em devpkey.h).
    const PCI_FMTID: GUID = GUID::from_u128(0x3ab22e31_8264_4b4e_9af5_a8d2d8e33e62);
    const fn pci_key(pid: u32) -> DEVPROPKEY {
        DEVPROPKEY {
            fmtid: PCI_FMTID,
            pid,
        }
    }
    const BASE_CLASS: DEVPROPKEY = pci_key(3);
    const SUB_CLASS: DEVPROPKEY = pci_key(4);
    const CURRENT_LINK_SPEED: DEVPROPKEY = pci_key(9);
    const CURRENT_LINK_WIDTH: DEVPROPKEY = pci_key(10);
    const MAX_LINK_SPEED: DEVPROPKEY = pci_key(11);
    const MAX_LINK_WIDTH: DEVPROPKEY = pci_key(12);

    /// No Windows a velocidade do link vem como a geração (1 = 2,5 GT/s … 5 = 32 GT/s).
    fn link(devinst: u32) -> Option<PcieLink> {
        let field = |key: &DEVPROPKEY| {
            number(devinst, key)
                .filter(|v| (1..=64).contains(v))
                .map(|v| v as u8)
        };
        let link = PcieLink {
            current_gen: field(&CURRENT_LINK_SPEED).filter(|g| *g <= 6),
            current_width: field(&CURRENT_LINK_WIDTH),
            max_gen: field(&MAX_LINK_SPEED).filter(|g| *g <= 6),
            max_width: field(&MAX_LINK_WIDTH),
        };
        (link.current_gen.is_some() || link.max_gen.is_some()).then_some(link)
    }

    pub fn list() -> Vec<PciDevice> {
        present_ids(Some("PCI")).iter().filter_map(|id| device(id)).collect()
    }

    /// Lê um dispositivo pelo ID de instância ("PCI\\VEN_1022&DEV_1630&...\\3&2411E6FE&0&00").
    fn device(instance: &str) -> Option<PciDevice> {
        let devinst = locate(instance)?;
        Some(PciDevice {
            address: instance.to_string(),
            class: (number(devinst, &BASE_CLASS)? << 8) | number(devinst, &SUB_CLASS).unwrap_or(0),
            vendor_id: id_field(instance, "VEN_")? as u16,
            device_id: id_field(instance, "DEV_")? as u16,
            revision: id_field(instance, "REV_").map(|r| r as u8),
            vendor: text(devinst, &DEVPKEY_Device_Manufacturer),
            name: text(devinst, &DEVPKEY_Device_DeviceDesc),
            link: link(devinst),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DB: &str = "# comentário\n1022  Advanced Micro Devices, Inc. [AMD]\n\t1630  Renoir/Cezanne Root Complex\n\t\t1022 1630  Sub\n10ec  Realtek Semiconductor Co., Ltd.\n\t8168  RTL8111/8168\nC 00  Unclassified\n";

    #[test]
    fn le_nomes_do_pci_ids() {
        let (v, p) = lookup_names(DB, &[(0x1022, 0x1630), (0x10ec, 0x8168)]);
        assert_eq!(v[&0x1022], "AMD");
        assert_eq!(v[&0x10ec], "Realtek Semiconductor Co., Ltd.");
        assert_eq!(p[&(0x1022, 0x1630)], "Renoir/Cezanne Root Complex");
        assert_eq!(p[&(0x10ec, 0x8168)], "RTL8111/8168");
    }

    #[test]
    fn base_embutida_conhece_chipsets_comuns() {
        let (v, p) = lookup_names(EMBEDDED_PCI_IDS, &[(0x1022, 0x1630), (0x8086, 0xa780)]);
        assert_eq!(v[&0x1022], "AMD");
        assert!(p.contains_key(&(0x1022, 0x1630)));
        assert_eq!(v[&0x8086], "Intel");
    }

    #[test]
    fn converte_velocidade_em_geracao() {
        assert_eq!(pcie_gen("16.0 GT/s PCIe"), Some(4));
        assert_eq!(pcie_gen("2.5 GT/s PCIe"), Some(1));
        assert_eq!(pcie_gen("Unknown"), None);
    }
}
