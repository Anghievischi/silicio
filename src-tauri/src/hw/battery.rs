//! Bateria do notebook: desgaste (capacidade atual × original), ciclos, carga
//! e consumo. As baterias de mouse e fone ficam em `peripherals`.
//!
//! Linux: `/sys/class/power_supply`. Windows: IOCTLs do driver de bateria
//! (`IOCTL_BATTERY_QUERY_*`), os mesmos do `powercfg /batteryreport`.

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Battery {
    pub model: Option<String>,
    pub manufacturer: Option<String>,
    /// "Li-ion", "LiP"…
    pub technology: Option<String>,
    /// Capacidades em Wh: de fábrica, a que ela segura hoje e a atual.
    pub design_wh: Option<f64>,
    pub full_wh: Option<f64>,
    pub now_wh: Option<f64>,
    /// Carga atual (%).
    pub percent: Option<f64>,
    pub cycles: Option<u32>,
    /// "charging", "discharging", "full", "not_charging" ou "unknown".
    pub status: &'static str,
    /// Potência entrando ou saindo da bateria, sempre positiva (W).
    pub power_w: Option<f64>,
    pub voltage_v: Option<f64>,
    /// Ligado na tomada.
    pub ac_online: Option<bool>,
}

impl Battery {
    /// Fecha os campos derivados: carga e potência coerentes com o estado.
    fn finish(mut self) -> Self {
        if self.percent.is_none()
            && let (Some(now), Some(full)) = (self.now_wh, self.full_wh)
            && full > 0.0
        {
            self.percent = Some((now / full * 100.0).min(100.0));
        }
        // Alguns firmwares mandam potência mesmo parado; vale só carregando ou descarregando.
        if !matches!(self.status, "charging" | "discharging") {
            self.power_w = None;
        }
        self.power_w = self.power_w.filter(|w| *w > 0.05 && *w < 500.0);
        self
    }
}

pub fn read() -> Vec<Battery> {
    platform::read().into_iter().map(Battery::finish).collect()
}

#[cfg(target_os = "linux")]
mod platform {
    use super::Battery;
    use std::fs;
    use std::path::Path;

    fn sysfs(path: &Path, file: &str) -> Option<String> {
        let s = fs::read_to_string(path.join(file)).ok()?;
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    fn micro(path: &Path, file: &str) -> Option<f64> {
        sysfs(path, file)?.parse::<f64>().ok().map(|v| v / 1e6)
    }

    pub fn read() -> Vec<Battery> {
        let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
            return Vec::new();
        };
        let supplies: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        let ac_online = supplies
            .iter()
            .filter(|p| sysfs(p, "type").as_deref() == Some("Mains"))
            .filter_map(|p| sysfs(p, "online"))
            .map(|o| o == "1")
            .reduce(|a, b| a || b);
        let mut list: Vec<Battery> = supplies
            .iter()
            .filter(|p| sysfs(p, "type").as_deref() == Some("Battery"))
            // Mouse, teclado e fone têm escopo "Device".
            .filter(|p| sysfs(p, "scope").as_deref() != Some("Device"))
            .filter(|p| sysfs(p, "present").as_deref() != Some("0"))
            .map(|p| battery(p, ac_online))
            .collect();
        list.sort_by(|a, b| a.model.cmp(&b.model));
        list
    }

    fn battery(p: &Path, ac_online: Option<bool>) -> Battery {
        // Uns drivers informam energia (µWh); outros, carga (µAh) e a tensão
        // de projeto para converter.
        let volts = micro(p, "voltage_min_design").or_else(|| micro(p, "voltage_now"));
        let wh = |energy: &str, charge: &str| {
            micro(p, energy)
                .or_else(|| Some(micro(p, charge)? * volts?))
                .filter(|w| *w > 0.0)
        };
        let power_w = micro(p, "power_now")
            .filter(|w| *w > 0.0)
            .or_else(|| Some((micro(p, "current_now")? * micro(p, "voltage_now")?).abs()));
        Battery {
            model: sysfs(p, "model_name"),
            manufacturer: sysfs(p, "manufacturer"),
            technology: sysfs(p, "technology").filter(|t| t != "Unknown"),
            design_wh: wh("energy_full_design", "charge_full_design"),
            full_wh: wh("energy_full", "charge_full"),
            now_wh: wh("energy_now", "charge_now"),
            percent: sysfs(p, "capacity").and_then(|c| c.parse().ok()),
            cycles: sysfs(p, "cycle_count")
                .and_then(|c| c.parse().ok())
                .filter(|c| *c > 0),
            status: match sysfs(p, "status").as_deref() {
                Some("Charging") => "charging",
                Some("Discharging") => "discharging",
                Some("Full") => "full",
                Some("Not charging") => "not_charging",
                _ => "unknown",
            },
            power_w,
            voltage_v: micro(p, "voltage_now"),
            ac_online,
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::Battery;
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CM_Get_Device_Interface_List_SizeW,
        CM_Get_Device_Interface_ListW, CR_SUCCESS,
    };
    use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::System::Power::{
        BATTERY_CHARGING, BATTERY_DISCHARGING, BATTERY_INFORMATION, BATTERY_POWER_ON_LINE,
        BATTERY_QUERY_INFORMATION, BATTERY_QUERY_INFORMATION_LEVEL, BATTERY_STATUS,
        BATTERY_WAIT_STATUS, BatteryDeviceName, BatteryInformation, BatteryManufactureName,
        GUID_DEVICE_BATTERY, GetSystemPowerStatus, IOCTL_BATTERY_QUERY_INFORMATION,
        IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG, SYSTEM_POWER_STATUS,
    };
    use windows::core::{HSTRING, PCWSTR};

    /// Capacidades em mWh; com esta flag são só relativas (sem unidade).
    const CAPACITY_RELATIVE: u32 = 0x4000_0000;
    const SYSTEM_BATTERY: u32 = 0x8000_0000;
    const UNKNOWN: u32 = 0xFFFF_FFFF;

    struct Handle(HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    impl Handle {
        fn ioctl<I, O: Default>(&self, code: u32, input: &I) -> Option<O> {
            let mut out = O::default();
            self.raw(
                code,
                input,
                &mut out as *mut O as *mut c_void,
                size_of::<O>(),
            )?;
            Some(out)
        }

        fn raw<I>(&self, code: u32, input: &I, out: *mut c_void, size: usize) -> Option<u32> {
            let mut returned = 0u32;
            unsafe {
                DeviceIoControl(
                    self.0,
                    code,
                    Some(input as *const I as *const c_void),
                    size_of::<I>() as u32,
                    Some(out),
                    size as u32,
                    Some(&mut returned),
                    None,
                )
            }
            .ok()?;
            Some(returned)
        }

        fn text(&self, tag: u32, level: BATTERY_QUERY_INFORMATION_LEVEL) -> Option<String> {
            let query = BATTERY_QUERY_INFORMATION {
                BatteryTag: tag,
                InformationLevel: level,
                AtRate: 0,
            };
            let mut buf = [0u16; 128];
            let n = self.raw(
                IOCTL_BATTERY_QUERY_INFORMATION,
                &query,
                buf.as_mut_ptr() as *mut c_void,
                size_of::<[u16; 128]>(),
            )? as usize;
            let s = String::from_utf16_lossy(&buf[..(n / 2).min(buf.len())]);
            let s = s.trim_end_matches('\0').trim();
            (!s.is_empty()).then(|| s.to_string())
        }
    }

    fn interfaces() -> Vec<String> {
        let flags = CM_GET_DEVICE_INTERFACE_LIST_PRESENT;
        let mut len = 0u32;
        if unsafe {
            CM_Get_Device_Interface_List_SizeW(
                &mut len,
                &GUID_DEVICE_BATTERY,
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
            CM_Get_Device_Interface_ListW(&GUID_DEVICE_BATTERY, PCWSTR::null(), &mut buf, flags)
        } != CR_SUCCESS
        {
            return Vec::new();
        }
        buf.split(|c| *c == 0)
            .filter(|p| !p.is_empty())
            .map(String::from_utf16_lossy)
            .collect()
    }

    pub fn read() -> Vec<Battery> {
        let mut power = SYSTEM_POWER_STATUS::default();
        let ac_online =
            match unsafe { GetSystemPowerStatus(&mut power) }.map(|_| power.ACLineStatus) {
                Ok(0) => Some(false),
                Ok(1) => Some(true),
                _ => None,
            };
        interfaces()
            .iter()
            .filter_map(|i| battery(i, ac_online))
            .collect()
    }

    fn battery(interface: &str, ac_online: Option<bool>) -> Option<Battery> {
        let wide = HSTRING::from(interface);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .ok()?;
        let h = Handle(handle);
        let tag: u32 = h.ioctl(IOCTL_BATTERY_QUERY_TAG, &0u32)?;
        if tag == 0 {
            return None; // compartimento vazio
        }
        let info: BATTERY_INFORMATION = h.ioctl(
            IOCTL_BATTERY_QUERY_INFORMATION,
            &BATTERY_QUERY_INFORMATION {
                BatteryTag: tag,
                InformationLevel: BatteryInformation,
                AtRate: 0,
            },
        )?;
        // No-breaks USB também aparecem aqui, sem a flag de bateria do sistema.
        if info.Capabilities & SYSTEM_BATTERY == 0 {
            return None;
        }
        let status: Option<BATTERY_STATUS> = h.ioctl(
            IOCTL_BATTERY_QUERY_STATUS,
            &BATTERY_WAIT_STATUS {
                BatteryTag: tag,
                ..Default::default()
            },
        );
        let absolute = info.Capabilities & CAPACITY_RELATIVE == 0;
        let wh = |mwh: u32| (absolute && mwh != 0 && mwh != UNKNOWN).then(|| mwh as f64 / 1000.0);
        let chemistry = String::from_utf8_lossy(&info.Chemistry)
            .trim_end_matches('\0')
            .trim()
            .to_string();
        let state = status.as_ref().map_or(0, |s| s.PowerState);
        Some(Battery {
            model: h.text(tag, BatteryDeviceName),
            manufacturer: h.text(tag, BatteryManufactureName),
            technology: (!chemistry.is_empty()).then_some(chemistry),
            design_wh: wh(info.DesignedCapacity),
            full_wh: wh(info.FullChargedCapacity),
            now_wh: status.as_ref().and_then(|s| wh(s.Capacity)),
            percent: (!absolute)
                .then(|| status.as_ref().map(|s| s.Capacity as f64))
                .flatten()
                .filter(|p| *p <= 100.0),
            cycles: Some(info.CycleCount).filter(|c| *c > 0),
            status: if state & BATTERY_CHARGING != 0 {
                "charging"
            } else if state & BATTERY_DISCHARGING != 0 {
                "discharging"
            } else if state & BATTERY_POWER_ON_LINE != 0 {
                if status
                    .as_ref()
                    .is_some_and(|s| s.Capacity >= info.FullChargedCapacity)
                {
                    "full"
                } else {
                    "not_charging"
                }
            } else {
                "unknown"
            },
            power_w: status
                .as_ref()
                .map(|s| s.Rate)
                .filter(|r| *r != i32::MIN && absolute)
                .map(|r| r.unsigned_abs() as f64 / 1000.0),
            voltage_v: status
                .as_ref()
                .map(|s| s.Voltage)
                .filter(|v| *v != 0 && *v != UNKNOWN)
                .map(|v| v as f64 / 1000.0),
            ac_online,
        })
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    pub fn read() -> Vec<super::Battery> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deriva_carga_e_ignora_potencia_parada() {
        let b = Battery {
            full_wh: Some(40.0),
            now_wh: Some(30.0),
            power_w: Some(3.0),
            status: "not_charging",
            ..Battery::default()
        }
        .finish();
        assert_eq!(b.percent, Some(75.0));
        assert_eq!(b.power_w, None);
    }

    #[test]
    fn le_baterias() {
        let _ = read();
    }
}
