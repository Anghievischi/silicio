//! Decodificação do SPD (Serial Presence Detect) dos módulos de memória.
//!
//! No Linux os drivers `ee1004` (DDR4) e `spd5118` (DDR5) expõem o EEPROM em
//! `/sys/bus/i2c/devices/<barramento>-<endereço>/eeprom`, legível sem root.
//! O layout segue a JEDEC 21-C, anexo L (DDR4 SPD, revisão 1.1) e a JESD400-5
//! (DDR5). As áreas XMP e EXPO não têm especificação pública; os offsets batem
//! com o memtest86+ e com o `decode-dimms` em desenvolvimento.

// O decodificador não depende de plataforma, mas só o Linux lê o EEPROM.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

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
    pub decoded: Option<Decoded>,
}

/// Campos comuns a DDR4 e DDR5.
#[derive(Debug, Clone, Serialize)]
pub struct Decoded {
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
    /// Tensão nominal em volts (1,2 V no DDR4, 1,1 V no DDR5).
    pub voltage: Option<f64>,
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
    pub expo: Option<Vec<Profile>>,
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

/// Perfis Intel XMP: 2.0 nos bytes 384–511 do DDR4, 3.0 nos bytes 640–1023 do DDR5.
#[derive(Debug, Clone, Serialize)]
pub struct Xmp {
    pub version: String,
    pub profiles: Vec<Profile>,
}

/// Um perfil XMP ou AMD EXPO.
#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    /// Número do perfil, como aparece no BIOS.
    pub number: u8,
    /// Nome gravado pelo fabricante (só XMP 3.0).
    pub name: Option<String>,
    /// VDD em volts.
    pub voltage: f64,
    pub speed_mts: u32,
    pub clock_mhz: f64,
    pub cl: u32,
    pub trcd: u32,
    pub trp: u32,
    pub tras: u32,
    pub trc: u32,
    pub trfc1: u32,
}

pub fn read() -> Vec<SpdModule> {
    platform::read()
}

/// Velocidades padrão DDR5 e seus tCK em picossegundos (truncados, como no SPD).
const DDR5_BINS: [(u32, u32); 9] = [
    (6400, 312),
    (6000, 333),
    (5600, 357),
    (5200, 384),
    (4800, 416),
    (4400, 454),
    (4000, 500),
    (3600, 555),
    (3200, 625),
];

/// Velocidades padrão DDR4 e seus tCK em picossegundos.
const DDR4_BINS: [(u32, u32); 7] = [
    (3200, 625),
    (2933, 682),
    (2666, 750),
    (2400, 833),
    (2133, 938),
    (1866, 1071),
    (1600, 1250),
];

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
        _ => "unknown",
    };

    let (module_type, decoded) = match kind {
        "DDR4" if data.len() >= 384 => (ddr4_module_type(data[3]), Some(decode_ddr4(data))),
        "DDR5" if data.len() >= 640 => (ddr5_module_type(data[3]), Some(decode_ddr5(data))),
        _ => (None, None),
    };

    SpdModule {
        address: address.to_string(),
        slot,
        kind: kind.to_string(),
        module_type,
        decoded,
    }
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

fn ddr5_module_type(byte: u8) -> Option<&'static str> {
    Some(match byte & 0x0F {
        0x1 => "RDIMM",
        0x2 => "UDIMM",
        0x3 => "SO-DIMM",
        0x4 => "LRDIMM",
        0x5 => "CUDIMM",
        0x6 => "CSODIMM",
        0x7 => "MRDIMM",
        0x8 => "CAMM2",
        0xA => "DDIMM",
        0xB => "Soldada",
        _ => return None,
    })
}

/// Tempo em ps a partir do valor em MTB (125 ps) e do ajuste fino (1 ps, com sinal).
fn ps(mtb: u32, fine: u8) -> i64 {
    mtb as i64 * 125 + fine as i8 as i64
}

/// Velocidades de mercado são múltiplos de 100/3 MT/s (3200, 3466, 3600, 3733…).
fn snap_speed(tck_ps: i64) -> u32 {
    let steps = (2_000_000 * 3 + tck_ps * 50) / (tck_ps * 100);
    (steps * 100 / 3) as u32
}

/// Bytes de CAS suportadas: bit n = CL (7 + n); com o bit 7 do último byte ligado a faixa começa em 23.
fn cas_list(bytes: &[u8]) -> Vec<u8> {
    let base = if bytes[3] & 0x80 != 0 { 23 } else { 7 };
    (0..30)
        .filter(|bit| bytes[bit / 8] & (1 << (bit % 8)) != 0)
        .map(|bit| base + bit as u8)
        .collect()
}

/// Ciclos de CAS: o primeiro valor suportado que cobre tAA.
fn cas(taa_ps: i64, tck_ps: u32, supported: &[u8]) -> u32 {
    let cl = cycles(taa_ps, tck_ps);
    supported
        .iter()
        .map(|&c| c as u32)
        .find(|&c| c >= cl)
        .unwrap_or(cl)
}

/// Arredondamento do DDR5 (JESD400-5): tolera 0,3% e arredonda para cima.
fn ddr5_cycles(t_ps: i64, tck_ps: u32) -> u32 {
    ((t_ps * 997 / tck_ps as i64 + 1000) / 1000) as u32
}

/// No DDR5 só existem CAS pares: o primeiro valor suportado que cobre tAA.
fn ddr5_cas(taa_ps: i64, tck_ps: u32, supported: &[u8]) -> u32 {
    let cl = ddr5_cycles(taa_ps, tck_ps);
    let cl = cl + cl % 2;
    supported
        .iter()
        .map(|&c| c as u32)
        .find(|&c| c >= cl)
        .unwrap_or(cl)
}

/// Bytes de CAS suportadas no DDR5: bit n = CL (20 + 2n).
fn ddr5_cas_list(bytes: &[u8]) -> Vec<u8> {
    (0..40)
        .filter(|bit| bytes[bit / 8] & (1 << (bit % 8)) != 0)
        .map(|bit| 20 + 2 * bit as u8)
        .collect()
}

/// Valor de 16 bits little-endian, como o DDR5 grava os tempos (em ps ou ns).
fn le16(d: &[u8], i: usize) -> i64 {
    u16::from_le_bytes([d[i], d[i + 1]]) as i64
}

/// Tensão de perfil DDR5: bits 7–5 = volts, bits 4–0 = passos de 50 mV.
fn ddr5_voltage(b: u8) -> f64 {
    ((b >> 5) as u32 * 1000 + (b & 0x1F) as u32 * 50) as f64 / 1000.0
}

/// MT/s a partir do tCK do DDR5, arredondado para a centena (6000, 6400, 7200…).
fn ddr5_speed(tck_ps: i64) -> u32 {
    ((2_000_000 / tck_ps + 50) / 100 * 100) as u32
}

fn text(bytes: &[u8]) -> Option<String> {
    let s = String::from_utf8_lossy(bytes);
    let s = s.split(['\0', '\r', '\n']).next().unwrap_or("").trim();
    (!s.is_empty()).then(|| s.to_string())
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
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn manufacturer(continuation: u8, code: u8) -> Option<String> {
    if continuation == 0 && code == 0 {
        return None;
    }
    let bank = (continuation & 0x7F) + 1;
    Some(
        jedec_vendor(bank, code)
            .map(String::from)
            .unwrap_or_else(|| format!("JEDEC banco {bank}, 0x{code:02X}")),
    )
}

fn bcd(b: u8) -> u8 {
    (b >> 4) * 10 + (b & 0x0F)
}

fn decode_ddr4(d: &[u8]) -> Decoded {
    let crc_ok = crc16(&d[0..126]) == u16::from_le_bytes([d[126], d[127]])
        && crc16(&d[128..254]) == u16::from_le_bytes([d[254], d[255]]);

    let tck = ps(d[18] as u32, d[125]);
    let taa = ps(d[24] as u32, d[123]);
    let trcd = ps(d[25] as u32, d[122]);
    let trp = ps(d[26] as u32, d[121]);
    let tras = ps(((d[27] as u32 & 0x0F) << 8) | d[28] as u32, 0);
    let trc = ps(((d[27] as u32 & 0xF0) << 4) | d[29] as u32, d[120]);
    let trfc1 = ps(u16::from_le_bytes([d[30], d[31]]) as u32, 0);

    let cas_supported = cas_list(&d[20..24]);

    let max_speed_mts = (2_000_000 / tck) as u32;
    let max_speed_mts = DDR4_BINS
        .iter()
        .map(|(s, _)| *s)
        .find(|s| s.abs_diff(max_speed_mts) < 15)
        .unwrap_or(max_speed_mts);

    let table = DDR4_BINS
        .iter()
        .filter(|(_, bin_tck)| *bin_tck as i64 >= tck)
        .map(|&(speed, bin_tck)| SpeedBin {
            speed_mts: speed,
            clock_mhz: speed as f64 / 2.0,
            // A CAS precisa ser uma das suportadas pelo módulo.
            cl: cas(taa, bin_tck, &cas_supported),
            trcd: cycles(trcd, bin_tck),
            trp: cycles(trp, bin_tck),
            tras: cycles(tras, bin_tck),
            trc: cycles(trc, bin_tck),
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
    let size_mb =
        (die_density_gbit * 1024.0 / 8.0) as u64 * (bus_width / device_width) as u64 * ranks as u64;

    let xmp = (d.len() >= 487 && d[384] == 0x0C && d[385] == 0x4A).then(|| Xmp {
        version: format!("{}.{}", d[387] >> 4, d[387] & 0x0F),
        profiles: (0..2)
            .filter(|n| d[386] & (1 << n) != 0)
            .filter_map(|n| xmp_profile(n + 1, &d[393 + 47 * n as usize..]))
            .collect(),
    });

    Decoded {
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
        voltage: (d[11] & 0x01 != 0).then_some(1.2),
        module_manufacturer: manufacturer(d[320], d[321]),
        dram_manufacturer: manufacturer(d[350], d[351]),
        part_number: text(&d[329..349]),
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
        expo: None,
    }
}

fn decode_ddr5(d: &[u8]) -> Decoded {
    let crc_ok = crc16(&d[0..510]) == u16::from_le_bytes([d[510], d[511]]);

    let tck = le16(d, 20);
    let taa = le16(d, 30);
    let trcd = le16(d, 32);
    let trp = le16(d, 34);
    let tras = le16(d, 36);
    let trc = le16(d, 38);
    // tRFC vem em ns no DDR5.
    let trfc1 = le16(d, 42) * 1000;
    let cas_supported = ddr5_cas_list(&d[24..29]);

    let max_speed_mts = ddr5_speed(tck.max(1));
    let table = DDR5_BINS
        .iter()
        .filter(|(_, bin_tck)| *bin_tck as i64 >= tck)
        .map(|&(speed, bin_tck)| SpeedBin {
            speed_mts: speed,
            clock_mhz: speed as f64 / 2.0,
            cl: ddr5_cas(taa, bin_tck, &cas_supported),
            trcd: ddr5_cycles(trcd, bin_tck),
            trp: ddr5_cycles(trp, bin_tck),
            tras: ddr5_cycles(tras, bin_tck),
            trc: ddr5_cycles(trc, bin_tck),
        })
        .collect();

    // Densidade por die em Gb; o índice 0 é indefinido.
    const DENSITY: [f32; 9] = [0.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0];
    let die_density_gbit = DENSITY.get((d[4] & 0x1F) as usize).copied().unwrap_or(0.0);
    // Bits 7–5 do byte 4: 0 = monolítico, 2/3/4/5 = 3DS com 2/4/8/16 dies.
    let dies = match d[4] >> 5 {
        n @ 2..=5 => 1u64 << (n - 1),
        _ => 1,
    };
    let device_width = 4u8 << ((d[6] >> 5) & 0x07);
    let ranks = ((d[234] >> 3) & 0x07) + 1;
    // Cada DIMM DDR5 tem dois subcanais de 32 bits; o barramento total é a soma.
    let subchannels = ((d[235] >> 5) & 0x03) + 1;
    let channel_width = 8u8 << (d[235] & 0x07);
    let size_mb = (die_density_gbit * 1024.0 / 8.0) as u64
        * dies
        * (channel_width / device_width) as u64
        * subchannels as u64
        * ranks as u64;

    let (xmp, expo) = if d.len() >= 1024 {
        (ddr5_xmp(d), ddr5_expo(d))
    } else {
        (None, None)
    };

    Decoded {
        spd_revision: format!("{}.{}", d[1] >> 4, d[1] & 0x0F),
        crc_ok,
        size_mb,
        max_speed_mts,
        pc_name: format!("PC5-{}", max_speed_mts * 8),
        ranks,
        device_width,
        bus_width: channel_width * subchannels,
        die_density_gbit,
        bank_groups: 1 << (d[7] >> 5),
        banks_per_group: 1 << (d[7] & 0x07),
        voltage: (d[16] & 0xF0 == 0).then_some(1.1),
        module_manufacturer: manufacturer(d[512], d[513]),
        dram_manufacturer: manufacturer(d[552], d[553]),
        part_number: text(&d[521..551]),
        manufactured: (d[515] != 0).then(|| (2000 + bcd(d[515]) as u16, bcd(d[516]))),
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
        expo,
    }
}

/// Campos de um perfil DDR5, nas posições relativas ao início do bloco.
struct Ddr5Layout {
    vdd: usize,
    tck: usize,
    cas: Option<usize>,
    taa: usize,
    trfc1: usize,
}

/// Perfil DDR5: tAA, tRCD, tRP, tRAS e tRC ficam em sequência a partir de `taa`.
fn ddr5_profile(number: u8, name: Option<String>, p: &[u8], l: &Ddr5Layout) -> Option<Profile> {
    let tck = le16(p, l.tck);
    if tck < 100 {
        return None;
    }
    let tck_ps = tck as u32;
    let supported = l
        .cas
        .map(|i| ddr5_cas_list(&p[i..i + 5]))
        .unwrap_or_default();
    let t = |n: usize| le16(p, l.taa + 2 * n);
    let speed_mts = ddr5_speed(tck);
    Some(Profile {
        number,
        name,
        voltage: ddr5_voltage(p[l.vdd]),
        speed_mts,
        clock_mhz: speed_mts as f64 / 2.0,
        cl: ddr5_cas(t(0), tck_ps, &supported),
        trcd: ddr5_cycles(t(1), tck_ps),
        trp: ddr5_cycles(t(2), tck_ps),
        tras: ddr5_cycles(t(3), tck_ps),
        trc: ddr5_cycles(t(4), tck_ps),
        trfc1: ddr5_cycles(le16(p, l.trfc1) * 1000, tck_ps),
    })
}

/// XMP 3.0: cabeçalho em 640, até três perfis do fabricante de 64 bytes a partir
/// de 704, nomes de 16 caracteres a partir de 654. Os perfis 4 e 5 são do usuário
/// e ficam de fora.
fn ddr5_xmp(d: &[u8]) -> Option<Xmp> {
    if d[640] != 0x0C || d[641] != 0x4A {
        return None;
    }
    const LAYOUT: Ddr5Layout = Ddr5Layout {
        vdd: 1,
        tck: 5,
        cas: Some(7),
        taa: 13,
        trfc1: 25,
    };
    Some(Xmp {
        version: format!("{}.{}", d[642] >> 4, d[642] & 0x0F),
        profiles: (0..3)
            .filter(|n| d[643] & (1 << n) != 0)
            .filter_map(|n| {
                let name = text(&d[654 + 16 * n..670 + 16 * n]);
                ddr5_profile(n as u8 + 1, name, &d[704 + 64 * n..], &LAYOUT)
            })
            .collect(),
    })
}

/// AMD EXPO: "EXPO" em 832 e dois perfis de 40 bytes a partir de 842. Um perfil
/// sem tCK não existe.
fn ddr5_expo(d: &[u8]) -> Option<Vec<Profile>> {
    if &d[832..836] != b"EXPO" {
        return None;
    }
    const LAYOUT: Ddr5Layout = Ddr5Layout {
        vdd: 0,
        tck: 4,
        cas: None,
        taa: 6,
        trfc1: 18,
    };
    let profiles: Vec<Profile> = (0..2)
        .filter_map(|n| ddr5_profile(n as u8 + 1, None, &d[842 + 40 * n..], &LAYOUT))
        .collect();
    (!profiles.is_empty()).then_some(profiles)
}

/// Um perfil XMP 2.0 tem 47 bytes; o 1 começa no byte 393 e o 2 no 440. Os
/// campos seguem a ordem da área JEDEC (tAA no byte 24 ↔ +8 aqui) e os ajustes
/// finos ficam no fim, em ordem inversa, como nos bytes 120–125.
fn xmp_profile(number: u8, p: &[u8]) -> Option<Profile> {
    let tck = ps(p[3] as u32, p[38]);
    if tck <= 0 {
        return None;
    }
    let tck_ps = tck as u32;
    let supported = cas_list(&p[4..8]);
    let taa = ps(p[8] as u32, p[37]);
    let trcd = ps(p[9] as u32, p[36]);
    let trp = ps(p[10] as u32, p[35]);
    let tras = ps(((p[11] as u32 & 0x0F) << 8) | p[12] as u32, 0);
    let trc = ps(((p[11] as u32 & 0xF0) << 4) | p[13] as u32, p[34]);
    let trfc1 = ps(u16::from_le_bytes([p[14], p[15]]) as u32, 0);
    let speed_mts = snap_speed(tck);
    Some(Profile {
        number,
        name: None,
        // Bit 7 = 1 V, bits 6–0 = centésimos.
        voltage: (p[0] >> 7) as f64 + (p[0] & 0x7F) as f64 / 100.0,
        speed_mts,
        clock_mhz: speed_mts as f64 / 2.0,
        cl: cas(taa, tck_ps, &supported),
        trcd: cycles(trcd, tck_ps),
        trp: cycles(trp, tck_ps),
        tras: cycles(tras, tck_ps),
        trc: cycles(trc, tck_ps),
        trfc1: cycles(trfc1, tck_ps),
    })
}

#[cfg(target_os = "linux")]
mod platform {
    use super::SpdModule;
    use std::fs;

    const DRIVERS: [&str; 2] = [
        "/sys/bus/i2c/drivers/ee1004",
        "/sys/bus/i2c/drivers/spd5118",
    ];

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

    fn module() -> Decoded {
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
        assert_eq!(
            d.cas_supported,
            vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24]
        );
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

    /// O fixture não tem XMP; grava um perfil de um kit DDR4-3600 CL16-19-19-39
    /// 1,35 V (valores típicos de etiqueta) para conferir a decodificação.
    #[test]
    fn perfil_xmp() {
        let mut spd = DDR4.to_vec();
        spd[384..388].copy_from_slice(&[0x0C, 0x4A, 0x01, 0x20]);
        let p = &mut spd[393..440];
        p[0] = 0x80 | 35; // 1,35 V
        p[3] = 5; // tCK = 625 − 70 = 555 ps
        p[38] = (-70i8) as u8;
        p[4..8].copy_from_slice(&[0xFC, 0xFF, 0x03, 0x00]); // CL 9–24
        p[8] = 71; // tAA = 8875 − 10 = 8865 ps → 16 ciclos
        p[37] = (-10i8) as u8;
        p[9] = 85; // tRCD = tRP = 10625 − 85 = 10540 ps → 19 ciclos
        p[36] = (-85i8) as u8;
        p[10] = 85;
        p[35] = (-85i8) as u8;
        p[11] = 0x00;
        p[12] = 172; // tRAS = 21500 ps → 39 ciclos
        p[13] = 0xFF; // tRC = 31875 ps → 58 ciclos
        p[34] = 0;
        p[14..16].copy_from_slice(&2800u16.to_le_bytes()); // tRFC1 = 350 ns

        let xmp = decode("6-0050", &spd).decoded.unwrap().xmp.unwrap();
        assert_eq!(xmp.version, "2.0");
        assert_eq!(xmp.profiles.len(), 1);
        let p = &xmp.profiles[0];
        assert_eq!(p.number, 1);
        assert_eq!(p.voltage, 1.35);
        assert_eq!((p.speed_mts, p.clock_mhz), (3600, 1800.0));
        assert_eq!((p.cl, p.trcd, p.trp, p.tras), (16, 19, 19, 39));
        assert_eq!((p.trc, p.trfc1), (58, 631));
    }

    fn put16(d: &mut [u8], i: usize, v: u16) {
        d[i..i + 2].copy_from_slice(&v.to_le_bytes());
    }

    /// Não há dump DDR5 público com XMP/EXPO para usar de fixture; monta um
    /// UDIMM de 16 GB DDR5-4800 1Rx8 (chips de 16 Gb) com XMP 3.0 e EXPO a
    /// 6000 MT/s CL30-38-38-96 1,35 V, valores comuns em kits do mercado.
    fn ddr5() -> Vec<u8> {
        let mut d = vec![0u8; 1024];
        d[0..8].copy_from_slice(&[0x30, 0x10, 0x12, 0x02, 0x04, 0x00, 0x20, 0x62]);
        put16(&mut d, 20, 416); // tCK 4800
        d[24..29].copy_from_slice(&[0xFE, 0xFF, 0x07, 0x00, 0x00]); // CL 22–56
        for (i, t) in [16000, 16000, 16000, 32000, 48000].iter().enumerate() {
            put16(&mut d, 30 + 2 * i, *t);
        }
        put16(&mut d, 42, 295); // tRFC1 em ns
        d[235] = 0x22; // 2 subcanais de 32 bits
        d[512..514].copy_from_slice(&[0x80, 0xAD]);
        d[515..517].copy_from_slice(&[0x23, 0x10]);
        d[521..551].copy_from_slice(b"HMCG78MEBUA081N               ");
        let crc = crc16(&d[0..510]);
        put16(&mut d, 510, crc);

        let timings = |d: &mut [u8], at: usize| {
            for (i, t) in [10000, 12650, 12650, 32000, 44650].iter().enumerate() {
                put16(d, at + 2 * i, *t);
            }
        };
        // XMP 3.0, perfil 1.
        d[640..644].copy_from_slice(&[0x0C, 0x4A, 0x30, 0x01]);
        d[654..662].copy_from_slice(b"XMP 6000");
        d[705] = 0x27; // VDD 1,35 V
        put16(&mut d, 709, 333);
        d[711..716].copy_from_slice(&[0xFE, 0xFF, 0x07, 0x00, 0x00]);
        timings(&mut d, 717);
        put16(&mut d, 729, 295);
        // EXPO, perfil 1.
        d[832..836].copy_from_slice(b"EXPO");
        d[842] = 0x27;
        put16(&mut d, 846, 333);
        timings(&mut d, 848);
        put16(&mut d, 860, 295);
        d
    }

    #[test]
    fn ddr5_base() {
        let m = decode("0-0051", &ddr5());
        assert_eq!(
            (m.slot, m.kind.as_str(), m.module_type),
            (2, "DDR5", Some("UDIMM"))
        );
        let d = m.decoded.unwrap();
        assert!(d.crc_ok);
        assert_eq!(d.size_mb, 16384);
        assert_eq!((d.max_speed_mts, d.pc_name.as_str()), (4800, "PC5-38400"));
        assert_eq!((d.ranks, d.device_width, d.bus_width), (1, 8, 64));
        assert_eq!((d.bank_groups, d.banks_per_group), (8, 4));
        assert_eq!(d.voltage, Some(1.1));
        assert_eq!(d.part_number.as_deref(), Some("HMCG78MEBUA081N"));
        assert_eq!(d.manufactured, Some((2023, 10)));
        assert!(d.module_manufacturer.is_some());
        assert_eq!(d.timings_ns.trfc1, 295.0);
        // DDR5-4800B: 40-39-39-77.
        let top = &d.table[0];
        assert_eq!(top.speed_mts, 4800);
        assert_eq!(
            (top.cl, top.trcd, top.trp, top.tras, top.trc),
            (40, 39, 39, 77, 116)
        );
        assert_eq!(d.table.last().unwrap().speed_mts, 3200);
    }

    #[test]
    fn ddr5_xmp_e_expo() {
        let d = decode("0-0050", &ddr5()).decoded.unwrap();
        let xmp = d.xmp.unwrap();
        assert_eq!(xmp.version, "3.0");
        assert_eq!(xmp.profiles.len(), 1);
        let expo = d.expo.unwrap();
        assert_eq!(expo.len(), 1);
        for p in [&xmp.profiles[0], &expo[0]] {
            assert_eq!(p.voltage, 1.35);
            assert_eq!(p.speed_mts, 6000);
            assert_eq!((p.cl, p.trcd, p.trp, p.tras, p.trc), (30, 38, 38, 96, 134));
            assert_eq!(p.trfc1, 884);
        }
        assert_eq!(xmp.profiles[0].name.as_deref(), Some("XMP 6000"));
        assert_eq!(expo[0].name, None);
    }

    #[test]
    fn velocidades_de_mercado() {
        assert_eq!(snap_speed(625), 3200);
        assert_eq!(snap_speed(555), 3600);
        assert_eq!(snap_speed(536), 3733);
        assert_eq!(snap_speed(500), 4000);
        assert_eq!(snap_speed(682), 2933);
    }
}
