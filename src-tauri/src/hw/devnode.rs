//! Leitura de dispositivos pelo Configuration Manager do Windows (CM_*),
//! que funciona sem privilégio de administrador.

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_GETIDLIST_FILTER_ENUMERATOR, CM_GETIDLIST_FILTER_NONE, CM_GETIDLIST_FILTER_PRESENT,
    CM_Get_DevNode_PropertyW, CM_Get_Device_ID_List_SizeW, CM_Get_Device_ID_ListW,
    CM_LOCATE_DEVNODE_NORMAL, CM_Locate_DevNodeW, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::DEVPROPTYPE;
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::{HSTRING, PCWSTR};

/// IDs de instância dos dispositivos presentes; `enumerator` filtra pelo barramento ("PCI", "USB").
pub fn present_ids(enumerator: Option<&str>) -> Vec<String> {
    let filter = enumerator.map(HSTRING::from);
    let pcwstr = filter.as_ref().map_or(PCWSTR::null(), |h| PCWSTR(h.as_ptr()));
    let flags = CM_GETIDLIST_FILTER_PRESENT
        | if filter.is_some() {
            CM_GETIDLIST_FILTER_ENUMERATOR
        } else {
            CM_GETIDLIST_FILTER_NONE
        };
    let mut len = 0u32;
    if unsafe { CM_Get_Device_ID_List_SizeW(&mut len, pcwstr, flags) } != CR_SUCCESS || len == 0 {
        return Vec::new();
    }
    let mut buf = vec![0u16; len as usize];
    if unsafe { CM_Get_Device_ID_ListW(pcwstr, &mut buf, flags) } != CR_SUCCESS {
        return Vec::new();
    }
    buf.split(|c| *c == 0)
        .filter(|id| !id.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

pub fn locate(id: &str) -> Option<u32> {
    let wide = HSTRING::from(id);
    let mut devinst = 0u32;
    let result =
        unsafe { CM_Locate_DevNodeW(&mut devinst, PCWSTR(wide.as_ptr()), CM_LOCATE_DEVNODE_NORMAL) };
    (result == CR_SUCCESS).then_some(devinst)
}

pub fn property(devinst: u32, key: &DEVPROPKEY) -> Option<Vec<u8>> {
    let mut kind = DEVPROPTYPE(0);
    let mut size = 0u32;
    // A primeira chamada só informa o tamanho necessário.
    unsafe { CM_Get_DevNode_PropertyW(devinst, key, &mut kind, None, &mut size, 0) };
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    let result = unsafe {
        CM_Get_DevNode_PropertyW(devinst, key, &mut kind, Some(buf.as_mut_ptr()), &mut size, 0)
    };
    (result == CR_SUCCESS).then_some(buf)
}

pub fn number(devinst: u32, key: &DEVPROPKEY) -> Option<u32> {
    let b = property(devinst, key)?;
    Some(match b.len() {
        1 => b[0] as u32,
        2 | 3 => u16::from_le_bytes([b[0], b[1]]) as u32,
        _ => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
    })
}

pub fn text(devinst: u32, key: &DEVPROPKEY) -> Option<String> {
    let b = property(devinst, key)?;
    let wide: Vec<u16> = b
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let s = String::from_utf16_lossy(&wide)
        .trim_end_matches('\0')
        .trim()
        .to_string();
    (!s.is_empty()).then_some(s)
}

/// Valor hexadecimal de um campo do ID de instância ("VID_046D", "DEV_1630").
pub fn id_field(instance: &str, tag: &str) -> Option<u32> {
    instance
        .to_uppercase()
        .split(['\\', '&'])
        .find_map(|part| part.strip_prefix(tag).map(str::to_string))
        .and_then(|v| u32::from_str_radix(&v, 16).ok())
}
