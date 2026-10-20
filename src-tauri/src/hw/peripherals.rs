//! Periféricos: telas (EDID), dispositivos USB e Bluetooth, baterias de
//! periféricos, áudio e câmeras. Nada aqui exige administrador.

use super::edid::{self, Edid};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Display {
    /// Saída de vídeo no Linux ("eDP-1", "HDMI-A-1").
    pub connector: Option<String>,
    /// Tela embutida (notebook, all-in-one).
    pub internal: Option<bool>,
    #[serde(flatten)]
    pub edid: Edid,
}

#[derive(Debug, Clone, Serialize)]
pub struct Device {
    /// "usb" ou "bluetooth".
    pub bus: &'static str,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub vendor: Option<String>,
    pub name: Option<String>,
    /// Funções do dispositivo: "keyboard", "mouse", "camera", "audio"… (um receptor pode ser teclado e mouse).
    pub kinds: Vec<&'static str>,
    pub speed_mbps: Option<f32>,
    pub usb_version: Option<String>,
    pub max_power_ma: Option<u32>,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Battery {
    pub name: String,
    pub capacity: Option<u8>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioDevice {
    pub name: String,
    /// Placa ou codec que fornece o dispositivo.
    pub card: Option<String>,
    /// "output" ou "input".
    pub direction: &'static str,
    /// "usb", "hdmi", "bluetooth" ou "internal".
    pub bus: Option<&'static str>,
    /// "speakers", "headphones", "headset", "microphone", "hdmi"…
    pub form: Option<&'static str>,
    pub default: bool,
    pub sample_rate: Option<u32>,
    pub bits: Option<u16>,
    pub channels: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraMode {
    pub width: u32,
    pub height: u32,
    pub max_fps: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraFormat {
    /// Código do formato ("MJPG", "YUYV").
    pub fourcc: String,
    pub description: String,
    pub modes: Vec<CameraMode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Camera {
    pub name: String,
    pub bus: Option<&'static str>,
    pub formats: Vec<CameraFormat>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Peripherals {
    pub displays: Vec<Display>,
    pub devices: Vec<Device>,
    pub batteries: Vec<Battery>,
    pub audio: Vec<AudioDevice>,
    pub cameras: Vec<Camera>,
}

pub fn read() -> Peripherals {
    platform::read()
}

/// Função de uma interface USB pelo trio classe/subclasse/protocolo.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn interface_kind(class: u8, subclass: u8, protocol: u8) -> Option<&'static str> {
    Some(match (class, subclass, protocol) {
        (0x03, 0x01, 0x01) => "keyboard",
        (0x03, 0x01, 0x02) => "mouse",
        (0x03, _, _) => "hid",
        (0x01, _, _) => "audio",
        (0x0E, _, _) => "camera",
        (0x08, _, _) => "storage",
        (0x0B, _, _) => "smartcard",
        (0xE0, 0x01, 0x01) => "bluetooth",
        (0xE0, _, _) | (0x02, _, _) | (0x0A, _, _) => "network",
        (0xFF, 0x5D, _) | (0x58, _, _) => "gamepad",
        _ => return None,
    })
}

/// Completa as funções pelo nome quando a interface é genérica (HID de fabricante).
fn kinds_from_name(name: &str, kinds: &mut Vec<&'static str>) {
    let lower = name.to_lowercase();
    let rules: [(&[&str], &'static str); 6] = [
        (&["keyboard", "teclado", "keypad"], "keyboard"),
        (&["mouse", "trackball", "touchpad"], "mouse"),
        (&["headset", "headphone", "earbuds", "speaker", "audio"], "audio"),
        (&["webcam", "camera"], "camera"),
        (&["controller", "gamepad", "joystick"], "gamepad"),
        (&["receiver", "unifying", "dongle"], "receiver"),
    ];
    for (words, kind) in rules {
        if words.iter().any(|w| lower.contains(w)) && !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    // Mouses gamer expõem um teclado para macros (e teclados, um mouse): vale o que o nome diz.
    let says = |words: &[&str]| words.iter().any(|w| lower.contains(w));
    let combo = says(&["combo", "receiver", "unifying", "dongle"]);
    if !combo && says(&["mouse"]) && !says(&["keyboard", "teclado"]) {
        kinds.retain(|k| *k != "keyboard");
    }
    if !combo && says(&["keyboard", "teclado"]) && !says(&["mouse"]) {
        kinds.retain(|k| *k != "mouse");
    }
    // Com uma função específica conhecida, "hid" genérico só polui.
    if kinds.len() > 1 {
        kinds.retain(|k| *k != "hid");
    }
}

fn push_kind(kinds: &mut Vec<&'static str>, kind: &'static str) {
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    fn sysfs(path: impl AsRef<Path>) -> Option<String> {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn hex(path: impl AsRef<Path>) -> Option<u32> {
        u32::from_str_radix(&sysfs(path)?, 16).ok()
    }

    fn entries(dir: &str) -> Vec<(String, std::path::PathBuf)> {
        let mut list: Vec<_> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .collect();
        list.sort();
        list
    }

    pub fn read() -> Peripherals {
        Peripherals {
            displays: displays(),
            devices: [usb(), bluetooth()].concat(),
            batteries: batteries(),
            audio: audio(),
            cameras: cameras(),
        }
    }

    fn displays() -> Vec<Display> {
        entries("/sys/class/drm")
            .into_iter()
            .filter(|(_, path)| sysfs(path.join("status")).as_deref() == Some("connected"))
            .filter_map(|(name, path)| {
                let connector = name.split_once('-')?.1.to_string();
                let edid = edid::parse(&fs::read(path.join("edid")).ok()?)?;
                let internal = ["eDP", "LVDS", "DSI"].iter().any(|p| connector.starts_with(p));
                Some(Display {
                    connector: Some(connector),
                    internal: Some(internal),
                    edid,
                })
            })
            .collect()
    }

    /// Nomes da base usb.ids do sistema, para dispositivos sem texto no descritor.
    struct UsbIds(Option<String>);

    impl UsbIds {
        fn load() -> Self {
            let paths = ["/usr/share/hwdata/usb.ids", "/usr/share/misc/usb.ids", "/usr/share/usb.ids"];
            Self(paths.iter().find_map(|p| fs::read_to_string(p).ok()))
        }

        fn lookup(&self, vendor: u16, product: u16) -> (Option<String>, Option<String>) {
            let Some(db) = &self.0 else { return (None, None) };
            let vendor_tag = format!("{vendor:04x}  ");
            let product_tag = format!("\t{product:04x}  ");
            let mut lines = db.lines().skip_while(|l| !l.starts_with(&vendor_tag));
            let Some(vendor_line) = lines.next() else { return (None, None) };
            let product = lines
                .take_while(|l| l.starts_with('\t') || l.starts_with('#'))
                .find_map(|l| l.strip_prefix(&product_tag).map(str::to_string));
            (Some(vendor_line[6..].to_string()), product)
        }
    }

    fn usb() -> Vec<Device> {
        let ids = UsbIds::load();
        let mut list = Vec::new();
        for (name, path) in entries("/sys/bus/usb/devices") {
            // Interfaces ("1-4:1.0") e controladoras ("usb1") ficam de fora.
            if name.contains(':') || name.starts_with("usb") {
                continue;
            }
            let (Some(vid), Some(pid)) = (hex(path.join("idVendor")), hex(path.join("idProduct"))) else {
                continue;
            };
            if hex(path.join("bDeviceClass")) == Some(0x09) {
                continue;
            }
            let mut kinds = Vec::new();
            let mut driver = None;
            for (iface, ipath) in entries(path.to_str().unwrap_or_default()) {
                if !iface.starts_with(&format!("{name}:")) {
                    continue;
                }
                let byte = |f: &str| hex(ipath.join(f)).unwrap_or(0) as u8;
                if let Some(kind) = interface_kind(byte("bInterfaceClass"), byte("bInterfaceSubClass"), byte("bInterfaceProtocol")) {
                    push_kind(&mut kinds, kind);
                }
                if driver.is_none() {
                    driver = fs::read_link(ipath.join("driver"))
                        .ok()
                        .and_then(|l| l.file_name().map(|n| n.to_string_lossy().into_owned()));
                }
            }
            let (db_vendor, db_product) = ids.lookup(vid as u16, pid as u16);
            let product = sysfs(path.join("product")).or(db_product);
            kinds_from_name(product.as_deref().unwrap_or_default(), &mut kinds);
            list.push(Device {
                bus: "usb",
                vendor_id: Some(vid as u16),
                product_id: Some(pid as u16),
                vendor: sysfs(path.join("manufacturer")).or(db_vendor),
                name: product,
                kinds,
                speed_mbps: sysfs(path.join("speed")).and_then(|s| s.parse().ok()),
                usb_version: sysfs(path.join("version")),
                max_power_ma: sysfs(path.join("bMaxPower"))
                    .and_then(|s| s.trim_end_matches("mA").trim().parse().ok()),
                driver,
            });
        }
        list
    }

    /// Dispositivos HID conectados por Bluetooth (barramento 0005 no kernel).
    fn bluetooth() -> Vec<Device> {
        let mut seen: HashMap<String, ()> = HashMap::new();
        let mut list = Vec::new();
        for (_, path) in entries("/sys/bus/hid/devices") {
            let Some(uevent) = sysfs(path.join("uevent")) else { continue };
            let field = |key: &str| {
                uevent
                    .lines()
                    .find_map(|l| l.strip_prefix(key).map(str::to_string))
            };
            let Some(id) = field("HID_ID=") else { continue };
            let mut parts = id.split(':');
            if parts.next() != Some("0005") {
                continue;
            }
            let vid = parts.next().and_then(|v| u32::from_str_radix(v, 16).ok());
            let pid = parts.next().and_then(|v| u32::from_str_radix(v, 16).ok());
            let name = field("HID_NAME=");
            let key = field("HID_UNIQ=").filter(|u| !u.is_empty()).or(name.clone()).unwrap_or(id.clone());
            if seen.insert(key, ()).is_some() {
                continue;
            }
            let mut kinds = vec!["hid"];
            kinds_from_name(name.as_deref().unwrap_or_default(), &mut kinds);
            list.push(Device {
                bus: "bluetooth",
                vendor_id: vid.map(|v| v as u16),
                product_id: pid.map(|v| v as u16),
                vendor: None,
                name,
                kinds,
                speed_mbps: None,
                usb_version: None,
                max_power_ma: None,
                driver: field("DRIVER="),
            });
        }
        list
    }

    /// Baterias de mouse, teclado, fone e controle (escopo "Device" no power_supply).
    fn batteries() -> Vec<Battery> {
        entries("/sys/class/power_supply")
            .into_iter()
            .filter(|(_, path)| sysfs(path.join("scope")).as_deref() == Some("Device"))
            .map(|(name, path)| Battery {
                name: sysfs(path.join("model_name")).unwrap_or(name),
                capacity: sysfs(path.join("capacity")).and_then(|c| c.parse().ok()),
                status: sysfs(path.join("status")),
            })
            .collect()
    }

    fn audio() -> Vec<AudioDevice> {
        let Some(cards) = sysfs("/proc/asound/cards") else { return Vec::new() };
        let mut list = Vec::new();
        for line in cards.lines() {
            // " 1 [Generic_1      ]: HDA-Intel - HD-Audio Generic"
            let Some((index, rest)) = line.trim_start().split_once(' ') else { continue };
            let Ok(index) = index.parse::<u32>() else { continue };
            let short = rest.split_once(" - ").map(|(_, s)| s.trim().to_string());
            let dir = format!("/proc/asound/card{index}");
            let codec = sysfs(format!("{dir}/codec#0"))
                .and_then(|c| c.lines().next().and_then(|l| l.strip_prefix("Codec: ")).map(str::to_string));
            let usb = Path::new(&format!("{dir}/usbid")).exists();
            for (pcm, ppath) in entries(&dir) {
                let direction = match pcm.strip_prefix("pcm").and_then(|p| p.chars().last()) {
                    Some('p') => "output",
                    Some('c') => "input",
                    _ => continue,
                };
                let info = sysfs(ppath.join("info")).unwrap_or_default();
                let name = info
                    .lines()
                    .find_map(|l| l.strip_prefix("name: "))
                    .map(str::to_string)
                    .unwrap_or_else(|| pcm.clone());
                let hdmi = name.contains("HDMI") || name.contains("DP");
                let params = sysfs(ppath.join("sub0/hw_params")).filter(|p| p != "closed");
                let param = |key: &str| {
                    params.as_deref().and_then(|p| {
                        p.lines()
                            .find_map(|l| l.strip_prefix(key))
                            .map(|v| v.trim().to_string())
                    })
                };
                list.push(AudioDevice {
                    name,
                    card: codec.clone().or(short.clone()),
                    direction,
                    bus: Some(if usb {
                        "usb"
                    } else if hdmi {
                        "hdmi"
                    } else {
                        "internal"
                    }),
                    form: hdmi.then_some("hdmi"),
                    default: false,
                    sample_rate: param("rate:").and_then(|r| r.split_whitespace().next()?.parse().ok()),
                    bits: param("format:").and_then(|f| format_bits(&f)),
                    channels: param("channels:").and_then(|c| c.parse().ok()),
                });
            }
        }
        list
    }

    /// Bits por amostra a partir do formato do ALSA ("S16_LE", "S24_3LE", "FLOAT_LE").
    fn format_bits(format: &str) -> Option<u16> {
        if format.starts_with("FLOAT64") {
            return Some(64);
        }
        if format.starts_with("FLOAT") {
            return Some(32);
        }
        format
            .trim_start_matches(['S', 'U'])
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    }

    /// O UVC repete o fabricante no fim ("Theia Camera: Positivo"); tira a repetição.
    fn camera_name(name: String) -> String {
        match name.rsplit_once(": ") {
            Some((head, tail)) if head.contains(tail) => head.to_string(),
            _ => name,
        }
    }

    fn cameras() -> Vec<Camera> {
        let mut list = Vec::new();
        for (name, path) in entries("/sys/class/video4linux") {
            let label = sysfs(path.join("name")).unwrap_or(name.clone());
            match v4l2::query(&format!("/dev/{name}")) {
                Some((card, bus, capture, formats)) => {
                    if capture {
                        list.push(Camera {
                            name: camera_name(card.unwrap_or(label)),
                            bus,
                            formats,
                        });
                    }
                }
                // Sem permissão para abrir o nó: lista só o primeiro nó de cada câmera.
                None if sysfs(path.join("index")).as_deref() == Some("0") => list.push(Camera {
                    name: label,
                    bus: None,
                    formats: Vec::new(),
                }),
                None => {}
            }
        }
        list
    }

    /// Consultas V4L2 que só leem capacidades: não ligam a câmera nem o LED.
    mod v4l2 {
        use super::super::{CameraFormat, CameraMode};
        use std::fs::OpenOptions;
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;

        const QUERYCAP: u64 = 0x8068_5600;
        const ENUM_FMT: u64 = 0xC040_5602;
        const ENUM_FRAMESIZES: u64 = 0xC02C_564A;
        const ENUM_FRAMEINTERVALS: u64 = 0xC034_564B;
        const CAP_VIDEO_CAPTURE: u32 = 0x1;
        const CAP_DEVICE_CAPS: u32 = 0x8000_0000;
        const BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
        const FRMSIZE_DISCRETE: u32 = 1;
        const FRMIVAL_DISCRETE: u32 = 1;

        #[repr(C)]
        struct Capability {
            driver: [u8; 16],
            card: [u8; 32],
            bus_info: [u8; 32],
            version: u32,
            capabilities: u32,
            device_caps: u32,
            reserved: [u32; 3],
        }

        #[repr(C)]
        struct FmtDesc {
            index: u32,
            kind: u32,
            flags: u32,
            description: [u8; 32],
            pixelformat: u32,
            mbus_code: u32,
            reserved: [u32; 3],
        }

        #[repr(C)]
        struct FrmSizeEnum {
            index: u32,
            pixel_format: u32,
            kind: u32,
            data: [u32; 6],
            reserved: [u32; 2],
        }

        #[repr(C)]
        struct FrmIvalEnum {
            index: u32,
            pixel_format: u32,
            width: u32,
            height: u32,
            kind: u32,
            data: [u32; 6],
            reserved: [u32; 2],
        }

        fn ioctl<T>(fd: i32, request: u64, arg: &mut T) -> bool {
            unsafe { libc::ioctl(fd, request as _, arg as *mut T) == 0 }
        }

        fn c_str(bytes: &[u8]) -> String {
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).trim().to_string()
        }

        type Query = (Option<String>, Option<&'static str>, bool, Vec<CameraFormat>);

        pub fn query(path: &str) -> Option<Query> {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)
                .ok()?;
            let fd = file.as_raw_fd();
            let mut cap: Capability = unsafe { std::mem::zeroed() };
            if !ioctl(fd, QUERYCAP, &mut cap) {
                return None;
            }
            let caps = if cap.capabilities & CAP_DEVICE_CAPS != 0 {
                cap.device_caps
            } else {
                cap.capabilities
            };
            let capture = caps & CAP_VIDEO_CAPTURE != 0;
            let bus = c_str(&cap.bus_info).starts_with("usb").then_some("usb");
            let formats = if capture { formats(fd) } else { Vec::new() };
            Some((Some(c_str(&cap.card)).filter(|s| !s.is_empty()), bus, capture, formats))
        }

        fn formats(fd: i32) -> Vec<CameraFormat> {
            let mut list = Vec::new();
            for index in 0..32 {
                let mut desc: FmtDesc = unsafe { std::mem::zeroed() };
                desc.index = index;
                desc.kind = BUF_TYPE_VIDEO_CAPTURE;
                if !ioctl(fd, ENUM_FMT, &mut desc) {
                    break;
                }
                let fourcc: String = desc
                    .pixelformat
                    .to_le_bytes()
                    .iter()
                    .map(|&b| b as char)
                    .collect::<String>()
                    .trim()
                    .to_string();
                list.push(CameraFormat {
                    fourcc,
                    description: c_str(&desc.description),
                    modes: modes(fd, desc.pixelformat),
                });
            }
            list
        }

        fn modes(fd: i32, pixel_format: u32) -> Vec<CameraMode> {
            let mut list: Vec<CameraMode> = Vec::new();
            for index in 0..64 {
                let mut size: FrmSizeEnum = unsafe { std::mem::zeroed() };
                size.index = index;
                size.pixel_format = pixel_format;
                if !ioctl(fd, ENUM_FRAMESIZES, &mut size) {
                    break;
                }
                let (width, height) = if size.kind == FRMSIZE_DISCRETE {
                    (size.data[0], size.data[1])
                } else {
                    // Faixa contínua: guarda só o tamanho máximo.
                    (size.data[1], size.data[4])
                };
                list.push(CameraMode {
                    width,
                    height,
                    max_fps: max_fps(fd, pixel_format, width, height),
                });
                if size.kind != FRMSIZE_DISCRETE {
                    break;
                }
            }
            list.sort_by_key(|m| std::cmp::Reverse(m.width * m.height));
            list
        }

        fn max_fps(fd: i32, pixel_format: u32, width: u32, height: u32) -> Option<f64> {
            let mut best: Option<f64> = None;
            for index in 0..32 {
                let mut ival: FrmIvalEnum = unsafe { std::mem::zeroed() };
                ival.index = index;
                ival.pixel_format = pixel_format;
                ival.width = width;
                ival.height = height;
                if !ioctl(fd, ENUM_FRAMEINTERVALS, &mut ival) {
                    break;
                }
                // Intervalo em segundos (numerador/denominador): o mínimo dá o FPS máximo.
                let (num, den) = (ival.data[0], ival.data[1]);
                if num > 0 {
                    let fps = den as f64 / num as f64;
                    best = Some(best.map_or(fps, |b: f64| b.max(fps)));
                }
                if ival.kind != FRMIVAL_DISCRETE {
                    break;
                }
            }
            best.map(|f| (f * 100.0).round() / 100.0)
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::hw::devnode::{id_field, locate, present_ids, text};
    use std::collections::HashMap;
    use windows::Win32::Devices::Properties::{
        DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_Class, DEVPKEY_Device_DeviceDesc,
        DEVPKEY_Device_FriendlyName, DEVPKEY_Device_Manufacturer, DEVPKEY_Device_Service,
    };
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_BINARY, RegGetValueW};
    use windows::core::{HSTRING, w};

    struct Node {
        id: String,
        class: Option<String>,
        name: Option<String>,
        devinst: u32,
    }

    pub fn read() -> Peripherals {
        let nodes: Vec<Node> = present_ids(None)
            .into_iter()
            .filter_map(|id| {
                let devinst = locate(&id)?;
                Some(Node {
                    class: text(devinst, &DEVPKEY_Device_Class),
                    name: text(devinst, &DEVPKEY_Device_FriendlyName)
                        .or_else(|| text(devinst, &DEVPKEY_Device_DeviceDesc)),
                    id,
                    devinst,
                })
            })
            .collect();
        Peripherals {
            displays: displays(&nodes),
            devices: devices(&nodes),
            batteries: Vec::new(),
            audio: audio::list(),
            cameras: cameras(&nodes),
        }
    }

    fn displays(nodes: &[Node]) -> Vec<Display> {
        nodes
            .iter()
            .filter(|n| n.id.to_uppercase().starts_with("DISPLAY\\"))
            .filter_map(|n| {
                let key = HSTRING::from(format!(r"SYSTEM\CurrentControlSet\Enum\{}\Device Parameters", n.id));
                let mut size = 0u32;
                let probe = unsafe {
                    RegGetValueW(HKEY_LOCAL_MACHINE, &key, w!("EDID"), RRF_RT_REG_BINARY, None, None, Some(&mut size))
                };
                if probe != ERROR_SUCCESS || size == 0 {
                    return None;
                }
                let mut data = vec![0u8; size as usize];
                let result = unsafe {
                    RegGetValueW(
                        HKEY_LOCAL_MACHINE,
                        &key,
                        w!("EDID"),
                        RRF_RT_REG_BINARY,
                        None,
                        Some(data.as_mut_ptr().cast()),
                        Some(&mut size),
                    )
                };
                if result != ERROR_SUCCESS {
                    return None;
                }
                Some(Display {
                    connector: None,
                    internal: None,
                    edid: edid::parse(&data[..size as usize])?,
                })
            })
            .collect()
    }

    fn class_kind(class: &str) -> Option<&'static str> {
        Some(match class {
            "Keyboard" => "keyboard",
            "Mouse" => "mouse",
            "Camera" | "Image" => "camera",
            "MEDIA" | "AudioEndpoint" => "audio",
            "DiskDrive" => "storage",
            "Bluetooth" => "bluetooth",
            "HIDClass" => "hid",
            "XnaComposite" | "XboxComposite" => "gamepad",
            "Net" => "network",
            "SmartCardReader" => "smartcard",
            _ => return None,
        })
    }

    fn devices(nodes: &[Node]) -> Vec<Device> {
        // Teclados, mouses e câmeras aparecem como nós filhos (HID\, USB\..&MI_xx) com o mesmo VID/PID.
        let mut kinds_by_id: HashMap<(u32, u32), Vec<&'static str>> = HashMap::new();
        for n in nodes {
            let (Some(vid), Some(pid)) = (id_field(&n.id, "VID_"), id_field(&n.id, "PID_")) else { continue };
            if let Some(kind) = n.class.as_deref().and_then(class_kind) {
                push_kind(kinds_by_id.entry((vid, pid)).or_default(), kind);
            }
        }

        let mut list = Vec::new();
        for n in nodes {
            let upper = n.id.to_uppercase();
            if upper.starts_with("USB\\") && !upper.contains("&MI_") {
                let (Some(vid), Some(pid)) = (id_field(&n.id, "VID_"), id_field(&n.id, "PID_")) else { continue };
                let mut kinds = kinds_by_id.get(&(vid, pid)).cloned().unwrap_or_default();
                let name = text(n.devinst, &DEVPKEY_Device_BusReportedDeviceDesc).or(n.name.clone());
                let label = name.clone().unwrap_or_default();
                if kinds.is_empty() && label.to_lowercase().contains("hub") {
                    continue;
                }
                kinds_from_name(&label, &mut kinds);
                list.push(Device {
                    bus: "usb",
                    vendor_id: Some(vid as u16),
                    product_id: Some(pid as u16),
                    vendor: text(n.devinst, &DEVPKEY_Device_Manufacturer).filter(|m| !m.starts_with('(')),
                    name,
                    kinds,
                    speed_mbps: None,
                    usb_version: None,
                    max_power_ma: None,
                    driver: text(n.devinst, &DEVPKEY_Device_Service),
                });
            } else if upper.starts_with("BTHENUM\\DEV_") || upper.starts_with("BTHLE\\DEV_") {
                let mut kinds = vec!["hid"];
                kinds_from_name(n.name.as_deref().unwrap_or_default(), &mut kinds);
                if kinds == ["hid"] {
                    kinds.clear();
                }
                list.push(Device {
                    bus: "bluetooth",
                    vendor_id: None,
                    product_id: None,
                    vendor: None,
                    name: n.name.clone(),
                    kinds,
                    speed_mbps: None,
                    usb_version: None,
                    max_power_ma: None,
                    driver: None,
                });
            }
        }
        list
    }

    fn cameras(nodes: &[Node]) -> Vec<Camera> {
        nodes
            .iter()
            .filter(|n| matches!(n.class.as_deref(), Some("Camera" | "Image")))
            .filter_map(|n| {
                Some(Camera {
                    name: n.name.clone()?,
                    bus: n.id.to_uppercase().starts_with("USB\\").then_some("usb"),
                    formats: Vec::new(),
                })
            })
            .collect()
    }

    /// Dispositivos de áudio pelo MMDevice (a mesma lista do painel de som do Windows).
    mod audio {
        use super::super::AudioDevice;
        use windows::Win32::Devices::FunctionDiscovery::{
            PKEY_Device_DeviceDesc, PKEY_Device_EnumeratorName, PKEY_Device_FriendlyName,
            PKEY_DeviceInterface_FriendlyName,
        };
        use windows::Win32::Foundation::PROPERTYKEY;
        use windows::Win32::Media::Audio::{
            DEVICE_STATE_ACTIVE, EDataFlow, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
            PKEY_AudioEndpoint_FormFactor, PKEY_AudioEngine_DeviceFormat, eCapture, eConsole, eRender,
        };
        use windows::Win32::System::Com::StructuredStorage::{
            PropVariantClear, PropVariantToStringAlloc, PropVariantToUInt32,
        };
        use windows::Win32::System::Com::{
            CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
            CoUninitialize, STGM_READ,
        };
        use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;

        pub fn list() -> Vec<AudioDevice> {
            let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let list = read().unwrap_or_default();
            if init.is_ok() {
                unsafe { CoUninitialize() };
            }
            list
        }

        fn read() -> windows::core::Result<Vec<AudioDevice>> {
            let enumerator: IMMDeviceEnumerator =
                unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
            let mut list = Vec::new();
            for (flow, direction) in [(eRender, "output"), (eCapture, "input")] {
                list.extend(endpoints(&enumerator, flow, direction)?);
            }
            Ok(list)
        }

        fn id(device: &IMMDevice) -> Option<String> {
            let raw = unsafe { device.GetId().ok()? };
            let id = unsafe { raw.to_string().ok() };
            unsafe { CoTaskMemFree(Some(raw.0 as _)) };
            id
        }

        fn endpoints(
            enumerator: &IMMDeviceEnumerator,
            flow: EDataFlow,
            direction: &'static str,
        ) -> windows::core::Result<Vec<AudioDevice>> {
            let default = unsafe { enumerator.GetDefaultAudioEndpoint(flow, eConsole) }
                .ok()
                .and_then(|d| id(&d));
            let collection = unsafe { enumerator.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)? };
            let mut list = Vec::new();
            for i in 0..unsafe { collection.GetCount()? } {
                let Ok(device) = (unsafe { collection.Item(i) }) else { continue };
                let Ok(store) = (unsafe { device.OpenPropertyStore(STGM_READ) }) else { continue };
                let form = number(&store, &PKEY_AudioEndpoint_FormFactor).and_then(form_factor);
                let enumerator_name = string(&store, &PKEY_Device_EnumeratorName).unwrap_or_default().to_uppercase();
                let bus = if enumerator_name.starts_with("USB") {
                    Some("usb")
                } else if enumerator_name.starts_with("BTH") {
                    Some("bluetooth")
                } else if form == Some("hdmi") {
                    Some("hdmi")
                } else if enumerator_name.starts_with("HDAUDIO") {
                    Some("internal")
                } else {
                    None
                };
                let format = wave_format(&store);
                list.push(AudioDevice {
                    name: string(&store, &PKEY_Device_DeviceDesc)
                        .or_else(|| string(&store, &PKEY_Device_FriendlyName))
                        .unwrap_or_default(),
                    card: string(&store, &PKEY_DeviceInterface_FriendlyName),
                    direction,
                    bus,
                    form,
                    default: default.is_some() && id(&device) == default,
                    sample_rate: format.map(|f| f.0),
                    bits: format.map(|f| f.1),
                    channels: format.map(|f| f.2),
                });
            }
            Ok(list)
        }

        fn form_factor(value: u32) -> Option<&'static str> {
            Some(match value {
                1 => "speakers",
                2 => "line",
                3 => "headphones",
                4 => "microphone",
                5 => "headset",
                6 => "handset",
                8 => "spdif",
                9 => "hdmi",
                _ => return None,
            })
        }

        fn string(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
            let mut value = unsafe { store.GetValue(key).ok()? };
            let text = unsafe { PropVariantToStringAlloc(&value) }.ok().and_then(|raw| {
                let s = unsafe { raw.to_string().ok() };
                unsafe { CoTaskMemFree(Some(raw.0 as _)) };
                s
            });
            let _ = unsafe { PropVariantClear(&mut value) };
            text.filter(|s| !s.is_empty())
        }

        fn number(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<u32> {
            let mut value = unsafe { store.GetValue(key).ok()? };
            let n = unsafe { PropVariantToUInt32(&value) }.ok();
            let _ = unsafe { PropVariantClear(&mut value) };
            n
        }

        /// Formato do mixer (WAVEFORMATEX/EXTENSIBLE): taxa, bits válidos e canais.
        fn wave_format(store: &IPropertyStore) -> Option<(u32, u16, u16)> {
            let mut value = unsafe { store.GetValue(&PKEY_AudioEngine_DeviceFormat).ok()? };
            let blob = unsafe { value.Anonymous.Anonymous.Anonymous.blob };
            let bytes = if blob.pBlobData.is_null() || blob.cbSize < 16 {
                None
            } else {
                Some(unsafe { std::slice::from_raw_parts(blob.pBlobData, blob.cbSize as usize) }.to_vec())
            };
            let _ = unsafe { PropVariantClear(&mut value) };
            let b = bytes?;
            let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
            let tag = u16_at(0);
            let channels = u16_at(2);
            let rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
            let mut bits = u16_at(14);
            // WAVE_FORMAT_EXTENSIBLE traz os bits válidos logo após o cabeçalho.
            if tag == 0xFFFE && b.len() >= 20 && u16_at(18) > 0 {
                bits = u16_at(18);
            }
            Some((rate, bits, channels))
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::Peripherals;

    pub fn read() -> Peripherals {
        Peripherals::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifica_interfaces_usb() {
        assert_eq!(interface_kind(0x03, 0x01, 0x01), Some("keyboard"));
        assert_eq!(interface_kind(0x03, 0x01, 0x02), Some("mouse"));
        assert_eq!(interface_kind(0x0E, 0x01, 0x00), Some("camera"));
        assert_eq!(interface_kind(0x09, 0x00, 0x00), None);
    }

    #[test]
    fn completa_funcao_pelo_nome() {
        let mut kinds = vec!["mouse", "keyboard"];
        kinds_from_name("CORSAIR KATAR PRO Gaming Mouse", &mut kinds);
        assert_eq!(kinds, vec!["mouse"]);
        let mut kinds = vec!["hid"];
        kinds_from_name("MX Keys", &mut kinds);
        assert_eq!(kinds, vec!["hid"]);
        let mut kinds = vec!["keyboard", "mouse"];
        kinds_from_name("USB Receiver", &mut kinds);
        assert_eq!(kinds, vec!["keyboard", "mouse", "receiver"]);
    }
}
