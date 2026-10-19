//! Decodificação do SPD (Serial Presence Detect) dos módulos de memória.
//!
//! No Linux os drivers `ee1004` (DDR4) e `spd5118` (DDR5) expõem o EEPROM em
//! `/sys/bus/i2c/devices/<barramento>-<endereço>/eeprom`, legível sem root.
//! O layout segue a JEDEC 21-C, anexo L (DDR4 SPD, revisão 1.1).

use super::memory::jedec_vendor;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpdModule {
    /// Endereço I²C, ex.: "6-0050".
    pub address: String,
    /// Número do slot deduzido do endereço (0x50 = slot 1).
    pub slot: u8,
    pub kind: String,
    pub module_type: Option<&'static str>,
    pub decoded: Option<Ddr4>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ddr4 {
    pub spd_revision: String,
    pub crc_ok: bool,
    pub size_mb: u64,
    pub max_speed_mts: u32,
    pub pc_name: String,
    pub ranks: u8,
    pub device_width: u8,
    pub bus_width: u8,
    pub die_density_gbit: f32,
    pub bank_groups: u8,
    pub banks_per_group: u8,
    pub voltage: &'static str,
    pub module_manufacturer: Option<String>,
    pub dram_manufacturer: Option<String>,
    pub part_number: Option<String>,
    /// Ano e semana de fabricação.
    pub manufactured: Option<(u16, u8)>,
    pub cas_supported: Vec<u8>,
    pub timings_ns: TimingsNs,
    /// Timings em ciclos nas velocidades padrão JEDEC, da maior para a menor.
    pub table: Vec<SpeedBin>,
    pub xmp: Option<Xmp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingsNs {
    pub tck_min: f64,
    pub taa: f64,
    pub trcd: f64,
    pub trp: f64,
    pub tras: f64,
    pub trc: f64,
    pub trfc1: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeedBin {
    pub speed_mts: u32,
    pub clock_mhz: f64,
    pub cl: u32,
    pub trcd: u32,
    pub trp: u32,
    pub tras: u32,
    pub trc: u32,
}

/// Só a presença e quantidade de perfis por enquanto; a decodificação dos
/// timings XMP ainda precisa ser validada com módulos reais.
#[derive(Debug, Clone, Serialize)]
pub struct Xmp {
    pub version: String,
    pub profiles: u8,
}

pub fn read() -> Vec<SpdModule> {
    platform::read()
}

/// Velocidades padrão DDR4 e seus tCK em picossegundos.
const DDR4_BINS: [(u32, u32); 7] =
    [(3200, 625), (2933, 682), (2666, 750), (2400, 833), (2133, 938), (1866, 1071), (1600, 1250)];

pub fn decode(address: &str, data: &[u8]) -> SpdModule {
    let slot = address
        .rsplit_once('-')
        .and_then(|(_, a)| u8::from_str_radix(a.trim_start_matches("00"), 16).ok())
        .map(|a| a.saturating_sub(0x50) + 1)
        .unwrap_or(0);

    let kind = match data.get(2) {
        Some(0x0C) => "DDR4",
        Some(0x12) => "DDR5",
        Some(0x0B) => "DDR3",
        _ => "Desconhecido",
    };

    let (module_type, decoded) = if kind == "DDR4" && data.len() >= 384 {
        (ddr4_module_type(data[3]), Some(decode_ddr4(data)))
    } else {
        (None, None)
    };

    SpdModule { address: address.to_string(), slot, kind: kind.to_string(), module_type, decoded }
}

fn ddr4_module_type(byte: u8) -> Option<&'static str> {
    Some(match byte & 0x0F {
        0x1 => "RDIMM",
        0x2 => "UDIMM",
        0x3 => "SO-DIMM",
        0x4 => "LRDIMM",
        0x8 => "72b-SO-UDIMM",
        0x9 => "72b-SO-RDIMM",
        _ => return None,
    })
}

/// Tempo em ps a partir do valor em MTB (125 ps) e do ajuste fino (1 ps, com sinal).
fn ps(mtb: u32, fine: u8) -> i64 {
    mtb as i64 * 125 + fine as i8 as i64
}

/// Algoritmo de arredondamento da JEDEC: ciclos = trunc((t * 1000 / tCK + 974) / 1000).
fn cycles(t_ps: i64, tck_ps: u32) -> u32 {
    ((t_ps * 1000 / tck_ps as i64 + 974) / 1000) as u32
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

fn manufacturer(continuation: u8, code: u8) -> Option<String> {
    if continuation == 0 && code == 0 {
        return None;
    }
    let bank = (continuation & 0x7F) + 1;
    Some(jedec_vendor(bank, code).map(String::from).unwrap_or_else(|| format!("JEDEC banco {bank}, 0x{code:02X}")))
}

fn bcd(b: u8) -> u8 {
    (b >> 4) * 10 + (b & 0x0F)
}

fn decode_ddr4(d: &[u8]) -> Ddr4 {
    let crc_ok = crc16(&d[0..126]) == u16::from_le_bytes([d[126], d[127]])
        && crc16(&d[128..254]) == u16::from_le_bytes([d[254], d[255]]);

    let tck = ps(d[18] as u32, d[125]);
    let taa = ps(d[24] as u32, d[123]);
    let trcd = ps(d[25] as u32, d[122]);
    let trp = ps(d[26] as u32, d[121]);
    let tras = ps(((d[27] as u32 & 0x0F) << 8) | d[28] as u32, 0);
    let trc = ps(((d[27] as u32 & 0xF0) << 4) | d[29] as u32, d[120]);
    let trfc1 = ps(u16::from_le_bytes([d[30], d[31]]) as u32, 0);

    // Bytes 20–23: bit n = CL (7 + n); com o bit 7 do byte 23 ligado a faixa começa em 23.
    let base = if d[23] & 0x80 != 0 { 23 } else { 7 };
    let cas_supported: Vec<u8> = (0..30)
        .filter(|bit| d[20 + bit / 8] & (1 << (bit % 8)) != 0)
        .map(|bit| base + bit as u8)
        .collect();

    let max_speed_mts = (2_000_000 / tck) as u32;
    let max_speed_mts = DDR4_BINS.iter().map(|(s, _)| *s).find(|s| s.abs_diff(max_speed_mts) < 15).unwrap_or(max_speed_mts);

    let table = DDR4_BINS
        .iter()
        .filter(|(_, bin_tck)| *bin_tck as i64 >= tck)
        .map(|&(speed, bin_tck)| {
            let mut cl = cycles(taa, bin_tck);
            // A CAS precisa ser uma das suportadas pelo módulo.
            if let Some(&supported) = cas_supported.iter().find(|c| **c as u32 >= cl) {
                cl = supported as u32;
            }
            SpeedBin {
                speed_mts: speed,
                clock_mhz: speed as f64 / 2.0,
                cl,
                trcd: cycles(trcd, bin_tck),
                trp: cycles(trp, bin_tck),
                tras: cycles(tras, bin_tck),
                trc: cycles(trc, bin_tck),
            }
        })
        .collect();

    let die_density_gbit = match d[4] & 0x0F {
        n @ 0..=7 => 0.25 * (1u32 << n) as f32,
        8 => 12.0,
        9 => 24.0,
        _ => 0.0,
    };
    let device_width = 4u8 << (d[12] & 0x07);
    let ranks = ((d[12] >> 3) & 0x07) + 1;
    let bus_width = 8u8 << (d[13] & 0x07);
    let size_mb = (die_density_gbit * 1024.0 / 8.0) as u64 * (bus_width / device_width) as u64 * ranks as u64;

    let part = String::from_utf8_lossy(&d[329..349]).trim().to_string();
    let xmp = (d[384] == 0x0C && d[385] == 0x4A).then(|| Xmp {
        version: format!("{}.{}", d[387] >> 4, d[387] & 0x0F),
        profiles: (d[386] & 0x03).count_ones() as u8,
    });

    Ddr4 {
        spd_revision: format!("{}.{}", d[1] >> 4, d[1] & 0x0F),
        crc_ok,
        size_mb,
        max_speed_mts,
        pc_name: format!("PC4-{}", max_speed_mts * 8),
        ranks,
        device_width,
        bus_width,
        die_density_gbit,
        bank_groups: 1 << (d[4] >> 6),
        banks_per_group: 4 << ((d[4] >> 4) & 0x03),
        voltage: if d[11] & 0x01 != 0 { "1.2 V" } else { "—" },
        module_manufacturer: manufacturer(d[320], d[321]),
        dram_manufacturer: manufacturer(d[350], d[351]),
        part_number: (!part.is_empty()).then_some(part),
        manufactured: (d[323] != 0).then(|| (2000 + bcd(d[323]) as u16, bcd(d[324]))),
        cas_supported,
        timings_ns: TimingsNs {
            tck_min: tck as f64 / 1000.0,
            taa: taa as f64 / 1000.0,
            trcd: trcd as f64 / 1000.0,
            trp: trp as f64 / 1000.0,
            tras: tras as f64 / 1000.0,
            trc: trc as f64 / 1000.0,
            trfc1: trfc1 as f64 / 1000.0,
        },
        table,
        xmp,
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::SpdModule;
    use std::fs;

    const DRIVERS: [&str; 2] = ["/sys/bus/i2c/drivers/ee1004", "/sys/bus/i2c/drivers/spd5118"];

    pub fn read() -> Vec<SpdModule> {
        let mut modules: Vec<SpdModule> = DRIVERS
            .iter()
            .filter_map(|dir| fs::read_dir(dir).ok())
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let data = fs::read(entry.path().join("eeprom")).ok()?;
                Some(super::decode(&name, &data))
            })
            .collect();
        modules.sort_by_key(|m| m.slot);
        modules
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use super::SpdModule;

    pub fn read() -> Vec<SpdModule> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SPD real de um SO-DIMM DDR4-3200 (número de série zerado).
    // Valores esperados conferidos com o `decode-dimms` 4.4.
    const DDR4: &[u8] = include_bytes!("../../tests/fixtures/ddr4-sodimm-3200.spd");

    fn module() -> Ddr4 {
        let m = decode("6-0050", DDR4);
        assert_eq!(m.slot, 1);
        assert_eq!(m.kind, "DDR4");
        assert_eq!(m.module_type, Some("SO-DIMM"));
        m.decoded.unwrap()
    }

    #[test]
    fn caracteristicas() {
        let d = module();
        assert!(d.crc_ok);
        assert_eq!(d.spd_revision, "1.1");
        assert_eq!(d.size_mb, 8192);
        assert_eq!(d.max_speed_mts, 3200);
        assert_eq!(d.pc_name, "PC4-25600");
        assert_eq!((d.ranks, d.device_width, d.bus_width), (1, 8, 64));
        assert_eq!(d.module_manufacturer.as_deref(), Some("Smart Modular"));
        assert_eq!(d.part_number.as_deref(), Some("SMS4WEC8C1K0446FCG"));
        assert_eq!(d.manufactured, Some((2023, 25)));
        assert_eq!(d.cas_supported, vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24]);
        assert!(d.xmp.is_none());
    }

    #[test]
    fn timings_em_ns() {
        let t = module().timings_ns;
        assert_eq!(t.tck_min, 0.625);
        assert_eq!(t.taa, 13.75);
        assert_eq!(t.tras, 32.0);
        assert_eq!(t.trc, 45.75);
        assert_eq!(t.trfc1, 350.0);
    }

    #[test]
    fn tabela_de_velocidades() {
        let table = module().table;
        let row = |speed| {
            let b = table.iter().find(|b| b.speed_mts == speed).unwrap();
            (b.cl, b.trcd, b.trp, b.tras)
        };
        assert_eq!(table.len(), 7);
        assert_eq!(row(3200), (22, 22, 22, 52));
        assert_eq!(row(2933), (21, 21, 21, 47));
        assert_eq!(row(2666), (19, 19, 19, 43));
        assert_eq!(row(2400), (17, 17, 17, 39));
        assert_eq!(row(2133), (15, 15, 15, 35));
        assert_eq!(row(1866), (13, 13, 13, 30));
        assert_eq!(row(1600), (11, 11, 11, 26));
    }
}
