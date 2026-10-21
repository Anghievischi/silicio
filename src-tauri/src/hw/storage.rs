//! Discos: modelo, interface, capacidade, saúde (SMART) e volumes.
//!
//! No Linux a lista vem do sysfs (`/sys/block`) e a saúde do udisks2 pelo
//! D-Bus: ele lê o SMART como root e o entrega a qualquer usuário. No Windows
//! tudo sai de consultas `IOCTL_STORAGE_QUERY_PROPERTY`, que não exigem
//! administrador. Nada aqui escreve no disco nem envia comandos ao firmware
//! além das leituras de log que o próprio sistema já faz.

use crate::hw::pci::PcieLink;
use serde::Serialize;
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize)]
pub struct Disk {
    /// Identificador do sistema: "nvme0n1" no Linux, o número do disco no Windows.
    pub id: String,
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub firmware: Option<String>,
    pub size_bytes: u64,
    /// "nvme", "sata", "usb", "mmc", "scsi" ou "virtual".
    pub bus: Option<&'static str>,
    /// "ssd" ou "hdd".
    pub media: Option<&'static str>,
    pub removable: bool,
    /// Link PCIe do controlador NVMe.
    pub link: Option<PcieLink>,
    /// Velocidade negociada do link SATA ("6.0 Gbps").
    pub sata_speed: Option<String>,
    pub temperature_c: Option<f32>,
    pub health: Option<Health>,
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Health {
    /// "good", "warning" ou "bad".
    pub status: &'static str,
    /// Vida útil restante estimada pelo próprio disco (100 = novo).
    pub life_left: Option<u8>,
    pub written_bytes: Option<u64>,
    pub read_bytes: Option<u64>,
    pub power_on_hours: Option<u64>,
    pub power_cycles: Option<u64>,
    pub unsafe_shutdowns: Option<u64>,
    pub media_errors: Option<u64>,
    /// Setores realocados ou pendentes (discos SATA).
    pub bad_sectors: Option<u64>,
    /// Bloco reserva disponível (%), só NVMe.
    pub available_spare: Option<u8>,
    /// Bits de alerta crítico do NVMe (0 = nenhum).
    pub critical_warning: Option<u8>,
    /// O SMART do disco SATA diz que ele está falhando.
    pub failing: bool,
}

impl Health {
    /// Classifica com os mesmos critérios que o próprio disco usa para alertar.
    fn classify(mut self) -> Self {
        let warning = self.critical_warning.unwrap_or(0);
        // Reserva abaixo do limite, confiabilidade degradada ou só leitura.
        let bad = self.failing || warning & 0b1101 != 0 || self.life_left == Some(0);
        // Temperatura acima do limite (bit 1) passa; setores ruins não voltam.
        let caution = warning != 0
            || self.life_left.is_some_and(|l| l <= 10)
            || self.bad_sectors.is_some_and(|n| n > 0);
        self.status = if bad {
            "bad"
        } else if caution {
            "warning"
        } else {
            "good"
        };
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Volume {
    pub mount_point: String,
    pub label: Option<String>,
    pub file_system: Option<String>,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

pub fn read() -> Vec<Disk> {
    let mut disks = platform::disks();
    let mounted = Disks::new_with_refreshed_list();
    // Btrfs e bind mounts repetem o mesmo volume em vários pontos; fica o
    // primeiro na ordem do sistema, que é o principal ("/" antes de "/home").
    let mut seen: Vec<(String, String)> = Vec::new();
    for volume in mounted.list() {
        let Some((disk, source)) = platform::disk_of(volume) else {
            continue;
        };
        if seen.iter().any(|(d, s)| *d == disk && *s == source) {
            continue;
        }
        seen.push((disk.clone(), source));
        let Some(target) = disks.iter_mut().find(|d| d.id == disk) else {
            continue;
        };
        let label = volume.name().to_string_lossy().trim().to_string();
        target.volumes.push(Volume {
            mount_point: volume.mount_point().to_string_lossy().into_owned(),
            label: (cfg!(windows) && !label.is_empty()).then_some(label),
            file_system: Some(volume.file_system().to_string_lossy().into_owned())
                .filter(|f| !f.is_empty()),
            total_bytes: volume.total_space(),
            available_bytes: volume.available_space(),
        });
    }
    disks.sort_by(|a, b| a.removable.cmp(&b.removable).then_with(|| a.id.cmp(&b.id)));
    disks
}

/// Log de saúde do NVMe (Get Log Page 02h, 512 bytes), como o Windows entrega.
#[cfg_attr(not(windows), allow(dead_code))]
fn nvme_health(log: &[u8]) -> Option<Health> {
    if log.len() < 192 {
        return None;
    }
    // Contadores de 128 bits; os 64 de baixo bastam.
    let counter = |at: usize| u64::from_le_bytes(log[at..at + 8].try_into().unwrap());
    // "Data units" são milhares de setores de 512 bytes.
    let units = |at: usize| counter(at).saturating_mul(512_000);
    Some(
        Health {
            critical_warning: Some(log[0]),
            available_spare: Some(log[3]),
            life_left: Some(100u8.saturating_sub(log[5])),
            read_bytes: Some(units(32)),
            written_bytes: Some(units(48)),
            power_cycles: Some(counter(112)),
            power_on_hours: Some(counter(128)),
            unsafe_shutdowns: Some(counter(144)),
            media_errors: Some(counter(160)),
            ..Health::default()
        }
        .classify(),
    )
}

/// Temperatura composta do log NVMe, em kelvin.
#[cfg_attr(not(windows), allow(dead_code))]
fn kelvin(k: f32) -> Option<f32> {
    (k > 200.0).then_some(k - 273.15)
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{Disk, udisks};
    use crate::hw::pci;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn sysfs(path: impl AsRef<Path>) -> Option<String> {
        let s = fs::read_to_string(path).ok()?;
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    pub fn disks() -> Vec<Disk> {
        let drives = udisks::read();
        let Ok(entries) = fs::read_dir("/sys/block") else {
            return Vec::new();
        };
        let mut list: Vec<Disk> = entries
            .flatten()
            .filter_map(|e| disk(&e.file_name().to_string_lossy(), &drives))
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    fn disk(name: &str, drives: &[udisks::Drive]) -> Option<Disk> {
        let virtual_ = ["loop", "ram", "zram", "dm-", "md", "nbd", "fd", "sr"];
        if virtual_.iter().any(|p| name.starts_with(p)) {
            return None;
        }
        let base = Path::new("/sys/block").join(name);
        let device = base.join("device");
        if !device.exists() {
            return None;
        }
        let size_bytes = sysfs(base.join("size"))?.parse::<u64>().ok()? * 512;
        if size_bytes == 0 {
            return None;
        }
        let real = fs::canonicalize(&base).unwrap_or_default();
        let path = real.to_string_lossy();
        let bus = if path.contains("/usb") {
            "usb"
        } else if name.starts_with("nvme") {
            "nvme"
        } else if name.starts_with("mmcblk") {
            "mmc"
        } else if path.contains("/virtio") {
            "virtual"
        } else if path.contains("/ata") {
            "sata"
        } else {
            "scsi"
        };
        let drive = sysfs(base.join("dev")).and_then(|dev| {
            let (major, minor) = dev.split_once(':')?;
            let (major, minor): (u32, u32) = (major.parse().ok()?, minor.parse().ok()?);
            drives.iter().find(|d| d.major == major && d.minor == minor)
        });
        let rotational = sysfs(base.join("queue/rotational")).as_deref() == Some("1");
        // Pontes USB costumam se dizer rotativas; o udisks sabe melhor.
        let media = match drive.and_then(|d| d.rotation_rpm) {
            Some(0) => Some("ssd"),
            Some(rpm) if rpm > 0 => Some("hdd"),
            _ if bus == "usb" => None,
            _ => Some(if rotational { "hdd" } else { "ssd" }),
        };
        let link = (bus == "nvme")
            .then(|| pci::read_link(&device.join("device")))
            .flatten();
        let temperature_c = hwmon_celsius(&device).or_else(|| drive.and_then(|d| d.temperature_c));
        Some(Disk {
            id: name.to_string(),
            model: sysfs(device.join("model")),
            vendor: sysfs(device.join("vendor")).filter(|v| v != "ATA" && bus != "nvme"),
            firmware: sysfs(device.join("firmware_rev")).or_else(|| sysfs(device.join("rev"))),
            size_bytes,
            bus: Some(bus),
            media,
            removable: bus == "usb" || sysfs(base.join("removable")).as_deref() == Some("1"),
            link,
            sata_speed: (bus == "sata").then(|| sata_speed(&path)).flatten(),
            temperature_c,
            health: drive.and_then(|d| d.health.clone()),
            volumes: Vec::new(),
        })
    }

    /// Sensor do próprio disco: `nvme` cria um hwmon no controlador e o
    /// `drivetemp` faz o mesmo para discos SATA.
    fn hwmon_celsius(device: &Path) -> Option<f32> {
        fs::read_dir(device)
            .ok()?
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("hwmon"))
            .find_map(|e| sysfs(e.path().join("temp1_input")))
            .and_then(|t| t.parse::<f32>().ok())
            .map(|m| m / 1000.0)
            .filter(|c| *c > 0.0 && *c < 150.0)
    }

    /// ".../ata3/host2/..." -> "/sys/class/ata_link/link3/sata_spd".
    fn sata_speed(path: &str) -> Option<String> {
        let port = path.split('/').find_map(|part| {
            part.strip_prefix("ata")
                .filter(|n| n.parse::<u32>().is_ok())
        })?;
        sysfs(format!("/sys/class/ata_link/link{port}/sata_spd")).filter(|s| s != "<unknown>")
    }

    /// Disco físico de um volume montado e o dispositivo que o identifica.
    /// LUKS e LVM são seguidos pelos `slaves` até a partição de verdade.
    pub fn disk_of(volume: &sysinfo::Disk) -> Option<(String, String)> {
        let dev = PathBuf::from(volume.name());
        if !dev.starts_with("/dev") {
            return None;
        }
        let name = fs::canonicalize(&dev)
            .ok()?
            .file_name()?
            .to_string_lossy()
            .into_owned();
        Some((whole_disk(&name, 0)?, name))
    }

    fn whole_disk(name: &str, depth: u8) -> Option<String> {
        let class = Path::new("/sys/class/block").join(name);
        if let Some(slave) = fs::read_dir(class.join("slaves"))
            .ok()
            .and_then(|mut e| e.next())
            .and_then(Result::ok)
        {
            return (depth < 4)
                .then(|| whole_disk(&slave.file_name().to_string_lossy(), depth + 1))
                .flatten();
        }
        if class.join("partition").exists() {
            let real = fs::canonicalize(&class).ok()?;
            return Some(real.parent()?.file_name()?.to_string_lossy().into_owned());
        }
        Path::new("/sys/block")
            .join(name)
            .exists()
            .then(|| name.to_string())
    }
}

/// Saúde dos discos pelo udisks2 (D-Bus de sistema). Sem o serviço, o resto
/// da aba continua funcionando, só sem SMART.
#[cfg(target_os = "linux")]
mod udisks {
    use super::Health;
    use std::collections::HashMap;
    use zbus::blocking::Connection;
    use zbus::blocking::fdo::ObjectManagerProxy;
    use zbus::zvariant::{OwnedValue, Value};

    const SERVICE: &str = "org.freedesktop.UDisks2";
    const NVME: &str = "org.freedesktop.UDisks2.NVMe.Controller";
    const ATA: &str = "org.freedesktop.UDisks2.Drive.Ata";

    type Props = HashMap<String, OwnedValue>;

    pub struct Drive {
        pub major: u32,
        pub minor: u32,
        pub rotation_rpm: Option<i32>,
        pub temperature_c: Option<f32>,
        pub health: Option<Health>,
    }

    fn get<'a, T>(props: &'a Props, key: &str) -> Option<T>
    where
        T: TryFrom<&'a Value<'a>>,
        <T as TryFrom<&'a Value<'a>>>::Error: Into<zbus::zvariant::Error>,
    {
        props.get(key)?.downcast_ref::<T>().ok()
    }

    pub fn read() -> Vec<Drive> {
        try_read().unwrap_or_default()
    }

    fn try_read() -> zbus::Result<Vec<Drive>> {
        let conn = Connection::system()?;
        let manager = ObjectManagerProxy::builder(&conn)
            .destination(SERVICE)?
            .path("/org/freedesktop/UDisks2")?
            .build()?;
        let objects = manager.get_managed_objects()?;
        let mut list = Vec::new();
        for interfaces in objects.values() {
            let Some(block) = interfaces.get("org.freedesktop.UDisks2.Block") else {
                continue;
            };
            if interfaces.contains_key("org.freedesktop.UDisks2.Partition") {
                continue;
            }
            let Some(drive_path) = get::<zbus::zvariant::ObjectPath>(block, "Drive") else {
                continue;
            };
            let Some(number) = get::<u64>(block, "DeviceNumber") else {
                continue;
            };
            let Some(drive) = objects
                .iter()
                .find(|(p, _)| p.as_str() == drive_path.as_str())
            else {
                continue;
            };
            let (path, ifaces) = drive;
            let rotation_rpm = ifaces
                .get("org.freedesktop.UDisks2.Drive")
                .and_then(|d| get::<i32>(d, "RotationRate"))
                .filter(|r| *r >= 0);
            let (health, temperature_c) = if let Some(nvme) = ifaces.get(NVME) {
                nvme_health(&conn, path.as_str(), nvme)
            } else if let Some(ata) = ifaces.get(ATA) {
                ata_health(&conn, path.as_str(), ata)
            } else {
                (None, None)
            };
            list.push(Drive {
                major: libc::major(number as libc::dev_t),
                minor: libc::minor(number as libc::dev_t),
                rotation_rpm,
                temperature_c,
                health,
            });
        }
        Ok(list)
    }

    /// Opções que impedem o udisks de pedir senha pelo polkit.
    fn no_auth() -> HashMap<&'static str, Value<'static>> {
        HashMap::from([("auth.no_user_interaction", Value::from(true))])
    }

    fn call<R>(conn: &Connection, path: &str, iface: &str) -> Option<R>
    where
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let reply = conn
            .call_method(
                Some(SERVICE),
                path,
                Some(iface),
                "SmartGetAttributes",
                &(no_auth(),),
            )
            .ok()?;
        reply.body().deserialize::<R>().ok()
    }

    fn nvme_health(conn: &Connection, path: &str, props: &Props) -> (Option<Health>, Option<f32>) {
        // Nunca lido ainda (o udisks atualiza a cada poucos minutos).
        if get::<u64>(props, "SmartUpdated").unwrap_or(0) == 0 {
            return (None, None);
        }
        let temperature =
            get::<u16>(props, "SmartTemperature").and_then(|k| super::kelvin(k as f32));
        let attrs: Props = call(conn, path, NVME).unwrap_or_default();
        // O udisks devolve os alertas como nomes; voltam a ser os bits do NVMe.
        let warnings: Vec<String> = props
            .get("SmartCriticalWarning")
            .and_then(|v| v.try_clone().ok())
            .and_then(|v| Vec::<String>::try_from(v).ok())
            .unwrap_or_default();
        let bit = |name: &str, b: u8| {
            if warnings.iter().any(|w| w == name) {
                b
            } else {
                0
            }
        };
        let critical = bit("spare", 1)
            | bit("temperature", 2)
            | bit("degraded", 4)
            | bit("readonly", 8)
            | bit("volatile_mem", 16);
        let health = Health {
            critical_warning: Some(critical),
            available_spare: get::<u8>(&attrs, "avail_spare"),
            life_left: get::<u8>(&attrs, "percent_used").map(|u| 100u8.saturating_sub(u)),
            read_bytes: get::<u64>(&attrs, "total_data_read"),
            written_bytes: get::<u64>(&attrs, "total_data_written"),
            power_cycles: get::<u64>(&attrs, "power_cycles"),
            power_on_hours: get::<u64>(props, "SmartPowerOnHours"),
            unsafe_shutdowns: get::<u64>(&attrs, "unsafe_shutdowns"),
            media_errors: get::<u64>(&attrs, "media_errors"),
            ..Health::default()
        };
        (Some(health.classify()), temperature)
    }

    /// Atributo SMART do ATA: id, nome, flags, valor, pior, limite, valor bruto
    /// interpretado, unidade dele e dados extras.
    type Attribute = (
        u8,
        String,
        u16,
        i32,
        i32,
        i32,
        i64,
        i32,
        HashMap<String, OwnedValue>,
    );

    fn ata_health(conn: &Connection, path: &str, props: &Props) -> (Option<Health>, Option<f32>) {
        if !get::<bool>(props, "SmartEnabled").unwrap_or(false)
            || get::<u64>(props, "SmartUpdated").unwrap_or(0) == 0
        {
            return (None, None);
        }
        let temperature =
            get::<f64>(props, "SmartTemperature").and_then(|k| super::kelvin(k as f32));
        let attrs: Vec<Attribute> = call::<(Vec<Attribute>,)>(conn, path, ATA)
            .map(|(a,)| a)
            .unwrap_or_default();
        let raw = |id: u8| attrs.iter().find(|a| a.0 == id).map(|a| a.6);
        // Vida restante: cada fabricante usa um atributo, todos normalizados em 0–100.
        let life_left = [231u8, 202, 169, 233, 177]
            .iter()
            .find_map(|id| attrs.iter().find(|a| a.0 == *id))
            .map(|a| a.3)
            .filter(|v| (0..=100).contains(v))
            .map(|v| v as u8);
        // Total gravado só quando o udisks sabe a unidade (setores).
        let written = attrs
            .iter()
            .find(|a| a.0 == 241 && a.7 == 3)
            .map(|a| a.6.max(0) as u64 * 512);
        let health = Health {
            life_left,
            written_bytes: written,
            power_on_hours: get::<u64>(props, "SmartPowerOnSeconds").map(|s| s / 3600),
            power_cycles: raw(12).map(|v| v.max(0) as u64),
            unsafe_shutdowns: raw(192).map(|v| v.max(0) as u64),
            bad_sectors: get::<i64>(props, "SmartNumBadSectors").map(|n| n.max(0) as u64),
            failing: get::<bool>(props, "SmartFailing").unwrap_or(false),
            ..Health::default()
        };
        (Some(health.classify()), temperature)
    }
}

#[cfg(windows)]
mod platform {
    use super::{Disk, Health, kelvin, nvme_health};
    use crate::hw::devnode::locate;
    use crate::hw::pci;
    use std::ffi::c_void;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CM_Get_Device_IDW,
        CM_Get_Device_Interface_List_SizeW, CM_Get_Device_Interface_ListW,
        CM_Get_Device_Interface_PropertyW, CM_Get_Parent, CR_SUCCESS,
    };
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_InstanceId, DEVPROPTYPE};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        BusTypeAta, BusTypeMmc, BusTypeNvme, BusTypeSata, BusTypeSd, BusTypeUsb, BusTypeVirtual,
        CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
        IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::System::Ioctl::{
        AtaDataTypeLogPage, GUID_DEVINTERFACE_DISK, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
        IOCTL_STORAGE_GET_DEVICE_NUMBER, IOCTL_STORAGE_QUERY_PROPERTY, NVMeDataTypeLogPage,
        PropertyStandardQuery, ProtocolTypeAta, ProtocolTypeNvme, StorageDeviceProperty,
        StorageDeviceProtocolSpecificProperty, StorageDeviceSeekPenaltyProperty,
        StorageDeviceTemperatureProperty,
    };
    use windows::core::{HSTRING, PCWSTR};

    /// Handle sem direito de leitura nem escrita: basta para as consultas
    /// (todas `FILE_ANY_ACCESS`) e não exige administrador.
    struct Handle(HANDLE);

    impl Handle {
        fn open(path: &str) -> Option<Self> {
            let wide = HSTRING::from(path);
            unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                )
            }
            .ok()
            .map(Self)
        }

        fn ioctl(&self, code: u32, input: &[u8], size: usize) -> Option<Vec<u8>> {
            let mut out = vec![0u8; size];
            let mut returned = 0u32;
            unsafe {
                DeviceIoControl(
                    self.0,
                    code,
                    (!input.is_empty()).then_some(input.as_ptr() as *const c_void),
                    input.len() as u32,
                    Some(out.as_mut_ptr() as *mut c_void),
                    size as u32,
                    Some(&mut returned),
                    None,
                )
            }
            .ok()?;
            out.truncate(returned as usize);
            Some(out)
        }

        /// `STORAGE_PROPERTY_QUERY` padrão para uma propriedade, sem parâmetros.
        fn property(&self, id: i32, size: usize) -> Option<Vec<u8>> {
            let mut query = [0u8; 12];
            query[..4].copy_from_slice(&id.to_le_bytes());
            query[4..8].copy_from_slice(&PropertyStandardQuery.0.to_le_bytes());
            self.ioctl(IOCTL_STORAGE_QUERY_PROPERTY, &query, size)
        }

        /// Página de log do protocolo (NVMe ou ATA) pelo driver do Windows.
        fn protocol_log(
            &self,
            protocol: i32,
            data_type: i32,
            value: u32,
            sub: u32,
        ) -> Option<Vec<u8>> {
            const SPECIFIC: usize = 40; // sizeof(STORAGE_PROTOCOL_SPECIFIC_DATA)
            const LOG: usize = 512;
            let mut query = vec![0u8; 8 + SPECIFIC + LOG];
            let mut put = |at: usize, v: u32| query[at..at + 4].copy_from_slice(&v.to_le_bytes());
            put(0, StorageDeviceProtocolSpecificProperty.0 as u32);
            put(4, PropertyStandardQuery.0 as u32);
            put(8, protocol as u32);
            put(12, data_type as u32);
            put(16, value);
            put(20, sub);
            put(24, SPECIFIC as u32);
            put(28, LOG as u32);
            let out = self.ioctl(IOCTL_STORAGE_QUERY_PROPERTY, &query, query.len())?;
            // STORAGE_PROTOCOL_DATA_DESCRIPTOR: os dados começam no offset
            // informado, contado a partir da estrutura de protocolo (byte 8).
            let offset = u32::from_le_bytes(out.get(24..28)?.try_into().ok()?) as usize;
            let length = u32::from_le_bytes(out.get(28..32)?.try_into().ok()?) as usize;
            out.get(8 + offset..8 + offset + length.min(LOG))
                .map(<[u8]>::to_vec)
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    fn u32_at(b: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
    }

    /// Texto ASCII terminado em zero a partir de um offset do descritor.
    fn ascii(b: &[u8], offset: Option<u32>) -> Option<String> {
        let start = offset.filter(|o| *o != 0)? as usize;
        let bytes = b.get(start..)?;
        let end = bytes.iter().position(|c| *c == 0).unwrap_or(bytes.len());
        let s = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    fn interfaces() -> Vec<String> {
        let mut len = 0u32;
        let flags = CM_GET_DEVICE_INTERFACE_LIST_PRESENT;
        if unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut len,
                &GUID_DEVINTERFACE_DISK,
                PCWSTR::null(),
                flags,
            )
        } != CR_SUCCESS
            || len == 0
        {
            return Vec::new();
        }
        let mut buf = vec![0u16; len as usize];
        if unsafe {
            CM_Get_Device_Interface_ListW(&GUID_DEVINTERFACE_DISK, PCWSTR::null(), &mut buf, flags)
        } != CR_SUCCESS
        {
            return Vec::new();
        }
        buf.split(|c| *c == 0)
            .filter(|p| !p.is_empty())
            .map(String::from_utf16_lossy)
            .collect()
    }

    /// Sobe da interface do disco até o controlador PCI para ler o link.
    fn pcie_link(interface: &str) -> Option<pci::PcieLink> {
        let wide = HSTRING::from(interface);
        let mut kind = DEVPROPTYPE(0);
        let mut buf = vec![0u8; 1024];
        let mut size = buf.len() as u32;
        let ok = unsafe {
            CM_Get_Device_Interface_PropertyW(
                PCWSTR(wide.as_ptr()),
                &DEVPKEY_Device_InstanceId,
                &mut kind,
                Some(buf.as_mut_ptr()),
                &mut size,
                0,
            )
        };
        if ok != CR_SUCCESS {
            return None;
        }
        let units: Vec<u16> = buf[..size as usize]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        let mut devinst = locate(String::from_utf16_lossy(&units).trim_end_matches('\0'))?;
        for _ in 0..6 {
            let mut parent = 0u32;
            if unsafe { CM_Get_Parent(&mut parent, devinst, 0) } != CR_SUCCESS {
                return None;
            }
            devinst = parent;
            let mut id = [0u16; 512];
            if unsafe { CM_Get_Device_IDW(devinst, &mut id, 0) } != CR_SUCCESS {
                return None;
            }
            if String::from_utf16_lossy(&id).starts_with("PCI\\") {
                return pci::windows_link(devinst);
            }
        }
        None
    }

    pub fn disks() -> Vec<Disk> {
        let mut list: Vec<Disk> = interfaces().iter().filter_map(|i| disk(i)).collect();
        list.sort_by_key(|d| d.id.parse::<u32>().unwrap_or(u32::MAX));
        list.dedup_by(|a, b| a.id == b.id);
        list
    }

    fn disk(interface: &str) -> Option<Disk> {
        let h = Handle::open(interface)?;
        let number = u32_at(&h.ioctl(IOCTL_STORAGE_GET_DEVICE_NUMBER, &[], 12)?, 4)?;
        let desc = h.property(StorageDeviceProperty.0, 1024)?;
        let bus_type = u32_at(&desc, 28)? as i32;
        let bus = match bus_type {
            t if t == BusTypeNvme.0 => "nvme",
            t if t == BusTypeSata.0 || t == BusTypeAta.0 => "sata",
            t if t == BusTypeUsb.0 => "usb",
            t if t == BusTypeSd.0 || t == BusTypeMmc.0 => "mmc",
            t if t == BusTypeVirtual.0 || t == 15 || t == 16 => "virtual",
            _ => "scsi",
        };
        let geometry = h.ioctl(IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, &[], 256)?;
        let size_bytes = u64::from_le_bytes(geometry.get(24..32)?.try_into().ok()?);
        let media = h
            .property(StorageDeviceSeekPenaltyProperty.0, 12)
            .and_then(|b| b.get(8).copied())
            .map(|penalty| if penalty != 0 { "hdd" } else { "ssd" });
        // STORAGE_TEMPERATURE_DATA_DESCRIPTOR: primeira leitura no byte 26.
        let temperature_c = h
            .property(StorageDeviceTemperatureProperty.0, 64)
            .and_then(|b| Some(i16::from_le_bytes(b.get(26..28)?.try_into().ok()?)))
            .map(f32::from)
            .filter(|c| *c > 0.0 && *c < 150.0);
        let (health, log_temperature) = match bus {
            "nvme" => nvme(&h),
            "sata" => ata(&h),
            _ => (None, None),
        };
        let vendor = ascii(&desc, u32_at(&desc, 12));
        Some(Disk {
            id: number.to_string(),
            model: ascii(&desc, u32_at(&desc, 16)),
            vendor: vendor
                .filter(|v| !v.eq_ignore_ascii_case("ATA") && !v.eq_ignore_ascii_case("NVMe")),
            firmware: ascii(&desc, u32_at(&desc, 20)),
            size_bytes,
            bus: Some(bus),
            media,
            removable: desc.get(10).is_some_and(|r| *r != 0) || bus == "usb",
            link: (bus == "nvme").then(|| pcie_link(interface)).flatten(),
            sata_speed: None,
            temperature_c: temperature_c.or(log_temperature),
            health,
            volumes: Vec::new(),
        })
    }

    fn nvme(h: &Handle) -> (Option<Health>, Option<f32>) {
        // Log 02h (SMART / Health Information) de todo o controlador.
        let Some(log) = h.protocol_log(ProtocolTypeNvme.0, NVMeDataTypeLogPage.0, 2, 0) else {
            return (None, None);
        };
        let temperature = kelvin(u16::from_le_bytes([log[1], log[2]]) as f32);
        (nvme_health(&log), temperature)
    }

    /// Log de estatísticas do dispositivo ATA (04h): cada valor é um qword
    /// com o bit 63 = suportado e o bit 62 = válido.
    fn ata(h: &Handle) -> (Option<Health>, Option<f32>) {
        let page = |n: u32| h.protocol_log(ProtocolTypeAta.0, AtaDataTypeLogPage.0, 0x04, n);
        let stat = |p: &Option<Vec<u8>>, at: usize| -> Option<u64> {
            let q = u64::from_le_bytes(p.as_ref()?.get(at..at + 8)?.try_into().ok()?);
            (q >> 62 == 0b11).then_some(q & 0x0000_FFFF_FFFF_FFFF)
        };
        let general = page(1);
        let temperature = page(5);
        let ssd = page(7);
        if general.is_none() && ssd.is_none() {
            return (None, None);
        }
        let health = Health {
            power_cycles: stat(&general, 8),
            power_on_hours: stat(&general, 16).map(|h| h & 0xFFFF_FFFF),
            written_bytes: stat(&general, 24).map(|s| s * 512),
            read_bytes: stat(&general, 40).map(|s| s * 512),
            // "Percentage Used Endurance Indicator": 0 = novo, pode passar de 100.
            life_left: stat(&ssd, 8).map(|u| 100u8.saturating_sub((u & 0xFF).min(255) as u8)),
            ..Health::default()
        };
        let celsius = stat(&temperature, 8)
            .map(|t| (t & 0xFF) as i8 as f32)
            .filter(|c| *c > 0.0);
        (Some(health.classify()), celsius)
    }

    /// Disco físico de um volume ("C:\\") pelas extensões que ele ocupa.
    pub fn disk_of(volume: &sysinfo::Disk) -> Option<(String, String)> {
        let mount = volume.mount_point().to_string_lossy().into_owned();
        let letter = mount.trim_end_matches('\\');
        let h = Handle::open(&format!(r"\\.\{letter}"))?;
        let extents = h.ioctl(IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, &[], 256)?;
        Some((u32_at(&extents, 8)?.to_string(), mount))
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    use super::Disk;

    pub fn disks() -> Vec<Disk> {
        Vec::new()
    }

    pub fn disk_of(_: &sysinfo::Disk) -> Option<(String, String)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_nvme() {
        let mut log = vec![0u8; 512];
        log[1..3].copy_from_slice(&310u16.to_le_bytes()); // 36,85 °C
        log[3] = 100;
        log[5] = 14;
        log[48..56].copy_from_slice(&38_936_512u64.to_le_bytes());
        log[128..136].copy_from_slice(&4642u64.to_le_bytes());
        let h = nvme_health(&log).unwrap();
        assert_eq!(h.status, "good");
        assert_eq!(h.life_left, Some(86));
        assert_eq!(h.written_bytes, Some(19_935_494_144_000));
        assert_eq!(h.power_on_hours, Some(4642));
        assert!((kelvin(310.0).unwrap() - 36.85).abs() < 0.01);

        log[0] = 0b100; // confiabilidade degradada
        assert_eq!(nvme_health(&log).unwrap().status, "bad");
        log[0] = 0b10; // só temperatura alta
        assert_eq!(nvme_health(&log).unwrap().status, "warning");
    }

    #[test]
    fn classifica_sata() {
        let h = |bad: u64, failing: bool| {
            Health {
                bad_sectors: Some(bad),
                failing,
                life_left: Some(95),
                ..Health::default()
            }
            .classify()
            .status
        };
        assert_eq!(h(0, false), "good");
        assert_eq!(h(3, false), "warning");
        assert_eq!(h(0, true), "bad");
    }

    #[test]
    fn lista_discos() {
        // Não falha sem discos nem sem udisks; só garante que a leitura roda.
        let _ = read();
    }
}
