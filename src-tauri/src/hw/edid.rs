//! Decodificador de EDID (VESA E-EDID 1.3/1.4 + extensão CTA-861): o bloco
//! que cada tela envia ao computador com fabricante, modelo, tamanho e modos.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mode {
    pub width: u16,
    pub height: u16,
    pub refresh_hz: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edid {
    /// Código PNP de três letras ("SAM", "DEL", "BOE").
    pub vendor_code: String,
    pub vendor: Option<&'static str>,
    /// Nome do monitor ou, em painéis de notebook, o código do painel.
    pub name: Option<String>,
    pub product_code: u16,
    pub year: Option<u16>,
    pub week: Option<u8>,
    pub version: String,
    pub width_cm: Option<u8>,
    pub height_cm: Option<u8>,
    pub diagonal_in: Option<f64>,
    /// Modo preferido (a resolução nativa do painel).
    pub native: Option<Mode>,
    pub modes: Vec<Mode>,
    pub refresh_min: Option<u16>,
    pub refresh_max: Option<u16>,
    pub bit_depth: Option<u8>,
    pub interface: Option<&'static str>,
    pub hdr: bool,
    pub bt2020: bool,
}

pub fn parse(data: &[u8]) -> Option<Edid> {
    const HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    if data.len() < 128 || data[..8] != HEADER {
        return None;
    }
    let id = u16::from_be_bytes([data[8], data[9]]);
    let letter = |shift: u16| (((id >> shift) & 0x1F) as u8 + b'@') as char;
    let vendor_code: String = [letter(10), letter(5), letter(0)].iter().collect();

    let (major, minor) = (data[18], data[19]);
    let digital = data[20] & 0x80 != 0;
    let v14 = major == 1 && minor >= 4;
    let (bit_depth, interface) = if digital && v14 {
        let depth = match (data[20] >> 4) & 0x07 {
            n @ 1..=6 => Some(4 + 2 * n),
            _ => None,
        };
        let iface = match data[20] & 0x0F {
            1 => Some("DVI"),
            2 | 3 => Some("HDMI"),
            5 => Some("DisplayPort"),
            _ => None,
        };
        (depth, iface)
    } else {
        (None, None)
    };

    let width_cm = (data[21] > 0).then_some(data[21]);
    let height_cm = (data[22] > 0).then_some(data[22]);
    let diagonal_in = match (width_cm, height_cm) {
        (Some(w), Some(h)) => Some(((w as f64).hypot(h as f64) / 2.54 * 10.0).round() / 10.0),
        _ => None,
    };

    let (week, year) = match (data[16], data[17]) {
        (_, 0) => (None, None),
        (0xFF, y) => (None, Some(1990 + y as u16)),
        (0, y) => (None, Some(1990 + y as u16)),
        (w, y) => (Some(w), Some(1990 + y as u16)),
    };

    let mut edid = Edid {
        vendor: pnp_vendor(&vendor_code),
        vendor_code,
        name: None,
        product_code: u16::from_le_bytes([data[10], data[11]]),
        year,
        week,
        version: format!("{major}.{minor}"),
        width_cm,
        height_cm,
        diagonal_in,
        native: None,
        modes: Vec::new(),
        refresh_min: None,
        refresh_max: None,
        bit_depth,
        interface,
        hdr: false,
        bt2020: false,
    };

    let mut texts = Vec::new();
    for d in data[54..126].as_chunks::<18>().0 {
        if d[0] != 0 || d[1] != 0 {
            if let Some(mode) = timing(d) {
                edid.native.get_or_insert(mode.clone());
                push_mode(&mut edid.modes, mode);
            }
            continue;
        }
        match d[3] {
            0xFC => edid.name = text(&d[5..]),
            0xFE => texts.extend(text(&d[5..])),
            0xFD => {
                // Na 1.4 os bits 0 e 1 somam 255 aos limites de frequência vertical.
                let offset = |bit: u8| if v14 && d[4] & bit != 0 { 255 } else { 0 };
                edid.refresh_min = Some(d[5] as u16 + offset(0x01));
                edid.refresh_max = Some(d[6] as u16 + offset(0x02));
            }
            _ => {}
        }
    }
    // Painéis de notebook não têm nome: o último texto livre costuma ser o código do painel.
    if edid.name.is_none() {
        edid.name = texts.pop();
    }

    for block in data[128..].as_chunks::<128>().0.iter().take(data[126] as usize) {
        if block[0] == 0x02 {
            cta(block, &mut edid);
        }
    }

    let from_modes = edid.modes.iter().map(|m| m.refresh_hz.round() as u16).max();
    edid.refresh_max = edid.refresh_max.max(from_modes);
    Some(edid)
}

/// Bloco de extensão CTA-861: modos extras, HDR e espaço de cor.
fn cta(block: &[u8], edid: &mut Edid) {
    let dtd_start = (block[2] as usize).clamp(4, 127);
    let mut i = 4;
    while i < dtd_start {
        let tag = block[i] >> 5;
        let len = (block[i] & 0x1F) as usize;
        let payload = &block[(i + 1).min(127)..(i + 1 + len).min(127)];
        if tag == 7 && !payload.is_empty() {
            match payload[0] {
                0x05 if payload.len() > 1 => edid.bt2020 |= payload[1] & 0xE0 != 0,
                0x06 => edid.hdr = true,
                _ => {}
            }
        }
        i += 1 + len;
    }
    let mut j = dtd_start;
    while j + 18 <= 127 && (block[j] != 0 || block[j + 1] != 0) {
        if let Some(mode) = timing(&block[j..j + 18]) {
            push_mode(&mut edid.modes, mode);
        }
        j += 18;
    }
}

fn timing(d: &[u8]) -> Option<Mode> {
    let clock_hz = u16::from_le_bytes([d[0], d[1]]) as f64 * 10_000.0;
    let h_active = d[2] as u32 | ((d[4] as u32 & 0xF0) << 4);
    let h_blank = d[3] as u32 | ((d[4] as u32 & 0x0F) << 8);
    let v_active = d[5] as u32 | ((d[7] as u32 & 0xF0) << 4);
    let v_blank = d[6] as u32 | ((d[7] as u32 & 0x0F) << 8);
    let total = ((h_active + h_blank) * (v_active + v_blank)) as f64;
    if h_active == 0 || v_active == 0 || total == 0.0 {
        return None;
    }
    let interlaced = d[17] & 0x80 != 0;
    let refresh = clock_hz / total * if interlaced { 2.0 } else { 1.0 };
    Some(Mode {
        width: h_active as u16,
        height: if interlaced { v_active * 2 } else { v_active } as u16,
        refresh_hz: (refresh * 100.0).round() / 100.0,
    })
}

fn push_mode(modes: &mut Vec<Mode>, mode: Mode) {
    if !modes.contains(&mode) {
        modes.push(mode);
    }
}

fn text(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0x0A || b == 0).unwrap_or(bytes.len());
    let s = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Fabricantes mais comuns de monitores e painéis (registro PNP da UEFI).
fn pnp_vendor(code: &str) -> Option<&'static str> {
    Some(match code {
        "ACR" => "Acer",
        "AOC" => "AOC",
        "APP" => "Apple",
        "AUO" => "AU Optronics",
        "AUS" => "ASUS",
        "BNQ" => "BenQ",
        "BOE" => "BOE",
        "CMN" => "Innolux",
        "CSO" => "CSOT",
        "DEL" => "Dell",
        "ENC" => "EIZO",
        "GBT" => "Gigabyte",
        "GSM" => "LG",
        "HKC" => "HKC",
        "HPN" | "HWP" => "HP",
        "HSD" => "HannStar",
        "IVM" => "iiyama",
        "IVO" => "InfoVision",
        "LEN" => "Lenovo",
        "LGD" => "LG Display",
        "MEI" => "Panasonic",
        "MSI" => "MSI",
        "NEC" => "NEC",
        "PHL" => "Philips",
        "SAM" => "Samsung",
        "SDC" => "Samsung Display",
        "SHP" => "Sharp",
        "SNY" => "Sony",
        "TMA" => "Tianma",
        "VSC" => "ViewSonic",
        "XMI" => "Xiaomi",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Painel BOE NT156FHM-N61 (15,6" Full HD) de um notebook; o número de série já vem zerado.
    const PANEL: &str = "00ffffffffffff0009e51208000000001f1c0104a5221378030980955c5a912921505400000001010101010101010101010101010101033a803671381e403020360058c21000001a000000000000000000000000000000000000000000fe00424f452043510a202020202020000000fe004e5431353646484d2d4e36310a00ed";

    fn bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn decodifica_painel_de_notebook() {
        let e = parse(&bytes(PANEL)).unwrap();
        assert_eq!(e.vendor_code, "BOE");
        assert_eq!(e.vendor, Some("BOE"));
        assert_eq!(e.name.as_deref(), Some("NT156FHM-N61"));
        assert_eq!(e.version, "1.4");
        assert_eq!((e.year, e.week), (Some(2018), Some(31)));
        assert_eq!((e.width_cm, e.height_cm), (Some(34), Some(19)));
        assert_eq!(e.diagonal_in, Some(15.3));
        let native = e.native.unwrap();
        assert_eq!((native.width, native.height), (1920, 1080));
        assert!((native.refresh_hz - 60.0).abs() < 0.1);
        assert_eq!(e.bit_depth, Some(8));
        assert_eq!(e.interface, Some("DisplayPort"));
    }

    #[test]
    fn rejeita_dados_invalidos() {
        assert!(parse(&[0u8; 128]).is_none());
        assert!(parse(&bytes(PANEL)[..100]).is_none());
    }
}
