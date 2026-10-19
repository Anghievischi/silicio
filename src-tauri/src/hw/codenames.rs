//! Banco próprio de codinomes, montado a partir de documentação pública
//! (manuais da Intel/AMD, kernel Linux `arch/x86/include/asm/intel-family.h`,
//! InstLatx64). Não contém dados extraídos do CPU-Z.
//!
//! A chave é (fornecedor, família de exibição, modelo de exibição). Quando um
//! modelo não está aqui, a interface mostra só os números de família/modelo.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Codename {
    pub name: &'static str,
    pub microarch: &'static str,
    pub process: &'static str,
}

const fn c(name: &'static str, microarch: &'static str, process: &'static str) -> Codename {
    Codename {
        name,
        microarch,
        process,
    }
}

pub fn lookup(vendor: &str, family: u32, model: u32) -> Option<Codename> {
    match vendor {
        "GenuineIntel" => intel(family, model),
        "AuthenticAMD" => amd(family, model),
        _ => None,
    }
}

fn intel(family: u32, model: u32) -> Option<Codename> {
    if family != 6 {
        return None;
    }
    Some(match model {
        0x2A => c("Sandy Bridge", "Sandy Bridge", "32 nm"),
        0x3A => c("Ivy Bridge", "Ivy Bridge", "22 nm"),
        0x3C | 0x45 | 0x46 => c("Haswell", "Haswell", "22 nm"),
        0x3D | 0x47 => c("Broadwell", "Broadwell", "14 nm"),
        0x4E | 0x5E => c("Skylake", "Skylake", "14 nm"),
        0x55 => c("Skylake-SP / Cascade Lake", "Skylake", "14 nm"),
        0x8E => c("Kaby Lake / Coffee Lake (mobile)", "Skylake", "14 nm"),
        0x9E => c("Kaby Lake / Coffee Lake", "Skylake", "14 nm"),
        0xA5 => c("Comet Lake-S", "Skylake", "14 nm"),
        0xA6 => c("Comet Lake-U", "Skylake", "14 nm"),
        0xA7 => c("Rocket Lake", "Cypress Cove", "14 nm"),
        0x7D | 0x7E => c("Ice Lake", "Sunny Cove", "10 nm"),
        0x6A | 0x6C => c("Ice Lake-SP", "Sunny Cove", "10 nm"),
        0x8C | 0x8D => c("Tiger Lake", "Willow Cove", "10 nm SuperFin"),
        0x97 | 0x9A => c("Alder Lake", "Golden Cove + Gracemont", "Intel 7"),
        0xB7 | 0xBA | 0xBF => c("Raptor Lake", "Raptor Cove + Gracemont", "Intel 7"),
        0x8F => c("Sapphire Rapids", "Golden Cove", "Intel 7"),
        0xCF => c("Emerald Rapids", "Raptor Cove", "Intel 7"),
        0xAA | 0xAC => c("Meteor Lake", "Redwood Cove + Crestmont", "Intel 4"),
        0xBD => c("Lunar Lake", "Lion Cove + Skymont", "TSMC N3B"),
        0xC5 | 0xC6 => c("Arrow Lake", "Lion Cove + Skymont", "TSMC N3B"),
        0xCC => c("Panther Lake", "Cougar Cove + Darkmont", "Intel 18A"),
        _ => return None,
    })
}

fn amd(family: u32, model: u32) -> Option<Codename> {
    Some(match (family, model) {
        (0x17, 0x01) => c("Summit Ridge / Naples", "Zen", "14 nm"),
        (0x17, 0x08) => c("Pinnacle Ridge", "Zen+", "12 nm"),
        (0x17, 0x11) => c("Raven Ridge", "Zen", "14 nm"),
        (0x17, 0x18) => c("Picasso", "Zen+", "12 nm"),
        (0x17, 0x20) => c("Dali", "Zen", "14 nm"),
        (0x17, 0x31) => c("Rome / Castle Peak", "Zen 2", "7 nm"),
        (0x17, 0x60) => c("Renoir", "Zen 2", "7 nm"),
        (0x17, 0x68) => c("Lucienne", "Zen 2", "7 nm"),
        (0x17, 0x71) => c("Matisse", "Zen 2", "7 nm"),
        (0x17, 0x90) => c("Van Gogh", "Zen 2", "7 nm"),
        (0x17, 0xA0) => c("Mendocino", "Zen 2", "6 nm"),
        (0x19, 0x01) => c("Milan", "Zen 3", "7 nm"),
        (0x19, 0x08) => c("Chagall", "Zen 3", "7 nm"),
        (0x19, 0x21) => c("Vermeer", "Zen 3", "7 nm"),
        (0x19, 0x40 | 0x44) => c("Rembrandt", "Zen 3+", "6 nm"),
        (0x19, 0x50) => c("Cezanne", "Zen 3", "7 nm"),
        (0x19, 0x10 | 0x11) => c("Genoa", "Zen 4", "5 nm"),
        (0x19, 0x18) => c("Storm Peak", "Zen 4", "5 nm"),
        (0x19, 0x61) => c("Raphael", "Zen 4", "5 nm"),
        (0x19, 0x74 | 0x78) => c("Phoenix", "Zen 4", "4 nm"),
        (0x19, 0x75) => c("Hawk Point", "Zen 4", "4 nm"),
        (0x19, 0xA0) => c("Bergamo / Siena", "Zen 4c", "5 nm"),
        (0x1A, 0x02) => c("Turin", "Zen 5", "4 nm"),
        (0x1A, 0x11) => c("Turin Dense", "Zen 5c", "3 nm"),
        (0x1A, 0x24) => c("Strix Point", "Zen 5", "4 nm"),
        (0x1A, 0x44) => c("Granite Ridge", "Zen 5", "4 nm"),
        (0x1A, 0x60) => c("Krackan Point", "Zen 5", "4 nm"),
        (0x1A, 0x70) => c("Strix Halo", "Zen 5", "4 nm"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ryzen_5700u_e_lucienne() {
        let c = lookup("AuthenticAMD", 0x17, 0x68).unwrap();
        assert_eq!(c.name, "Lucienne");
        assert_eq!(c.microarch, "Zen 2");
    }

    #[test]
    fn modelo_desconhecido_retorna_none() {
        assert!(lookup("GenuineIntel", 6, 0x01).is_none());
        assert!(lookup("HygonGenuine", 0x18, 0x00).is_none());
    }
}
