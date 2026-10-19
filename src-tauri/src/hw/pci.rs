//! Dispositivos PCI lidos do sysfs, com nomes da base `pci.ids` do sistema.

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

pub fn list() -> Vec<PciDevice> {
    let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") else {
        return Vec::new();
    };
    let mut devices: Vec<PciDevice> = entries
        .flatten()
        .filter_map(|e| read_device(&e.path()))
        .collect();
    devices.sort_by(|a, b| a.address.cmp(&b.address));

    let wanted: Vec<(u16, u16)> = devices.iter().map(|d| (d.vendor_id, d.device_id)).collect();
    let names = PCI_IDS
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())
        .map(|db| lookup_names(&db, &wanted));
    if let Some((vendors, products)) = names {
        for d in &mut devices {
            d.vendor = vendors.get(&d.vendor_id).cloned();
            d.name = products.get(&(d.vendor_id, d.device_id)).cloned();
        }
    }
    devices
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
    fn converte_velocidade_em_geracao() {
        assert_eq!(pcie_gen("16.0 GT/s PCIe"), Some(4));
        assert_eq!(pcie_gen("2.5 GT/s PCIe"), Some(1));
        assert_eq!(pcie_gen("Unknown"), None);
    }
}
