//! Consultas WMI no Windows, devolvendo linhas genéricas.
//!
//! O WMI entrega inteiros de 64 bits como texto, então os valores são lidos
//! como `Variant` e convertidos aqui em vez de deserializar structs fixas.

use ::wmi::{Variant, WMIConnection};
use std::collections::HashMap;

pub type Row = HashMap<String, Variant>;

pub fn query(sql: &str) -> Vec<Row> {
    WMIConnection::new()
        .and_then(|con| con.raw_query(sql))
        .unwrap_or_default()
}

pub fn text(row: &Row, key: &str) -> Option<String> {
    match row.get(key)? {
        Variant::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        _ => None,
    }
}

pub fn number(row: &Row, key: &str) -> Option<u64> {
    Some(match row.get(key)? {
        Variant::String(s) => s.trim().parse().ok()?,
        Variant::UI1(v) => *v as u64,
        Variant::UI2(v) => *v as u64,
        Variant::UI4(v) => *v as u64,
        Variant::UI8(v) => *v,
        Variant::I1(v) => u64::try_from(*v).ok()?,
        Variant::I2(v) => u64::try_from(*v).ok()?,
        Variant::I4(v) => u64::try_from(*v).ok()?,
        Variant::I8(v) => u64::try_from(*v).ok()?,
        _ => return None,
    })
}
