//! Minimal NumPy `.npy` reader for the voice style tensors
//! (`style_ttl.npy` [1,50,256] f32, `style_dp.npy` [1,8,16] f32).
//! Supports v1.x/v2.x headers, C-order, little-endian f4/f8 — the exact
//! shapes the reference loader validates.

use std::path::Path;

use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct NpyArray {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

pub fn load_f32(path: &Path) -> Result<NpyArray> {
    let bytes = std::fs::read(path).map_err(|e| anyhow!("read npy {}: {e}", path.display()))?;
    parse_f32(&bytes).map_err(|e| anyhow!("parse npy {}: {e}", path.display()))
}

pub fn parse_f32(bytes: &[u8]) -> Result<NpyArray> {
    const MAGIC: &[u8; 6] = b"\x93NUMPY";
    if bytes.len() < 10 || &bytes[0..6] != MAGIC {
        return Err(anyhow!("missing NPY magic"));
    }
    let major = bytes[6];
    let header_len: usize = match major {
        1 => u16::from_le_bytes([bytes[8], bytes[9]]) as usize,
        2 | 3 => {
            if bytes.len() < 12 {
                return Err(anyhow!("truncated v{major} header"));
            }
            u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize
        }
        _ => return Err(anyhow!("unsupported NPY version {major}")),
    };
    let header_start: usize = if major == 1 { 10 } else { 12 };
    let header_end = header_start
        .checked_add(header_len)
        .ok_or_else(|| anyhow!("header length overflow"))?;
    if bytes.len() < header_end {
        return Err(anyhow!("truncated header"));
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end])
        .map_err(|_| anyhow!("header is not utf-8"))?;

    let descr = dict_string_field(header, "descr")?;
    if descr != "<f4" && descr != "<f8" {
        return Err(anyhow!("unsupported dtype '{descr}'"));
    }
    if dict_bool_field(header, "fortran_order")? {
        return Err(anyhow!("fortran-order arrays are not supported"));
    }
    let shape = dict_shape_field(header)?;
    let elements: usize = shape.iter().product();

    let raw = &bytes[header_end..];
    let mut data = Vec::with_capacity(elements);
    match descr.as_str() {
        "<f4" => {
            if raw.len() < elements * 4 {
                return Err(anyhow!("data shorter than shape"));
            }
            for chunk in raw[..elements * 4].chunks_exact(4) {
                data.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
        }
        "<f8" => {
            if raw.len() < elements * 8 {
                return Err(anyhow!("data shorter than shape"));
            }
            for chunk in raw[..elements * 8].chunks_exact(8) {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(chunk);
                data.push(f64::from_le_bytes(buf) as f32);
            }
        }
        _ => return Err(anyhow!("unsupported dtype '{descr}'")),
    }
    Ok(NpyArray { shape, data })
}

fn dict_string_field(header: &str, key: &str) -> Result<String> {
    let needle = format!("'{key}':");
    let rest = header
        .find(&needle)
        .or_else(|| header.find(&format!("\"{key}\":")))
        .map(|i| &header[i + needle.len()..])
        .ok_or_else(|| anyhow!("missing '{key}' in header"))?;
    let rest = rest.trim_start();
    let quote = rest
        .chars()
        .next()
        .filter(|&c| c == '\'' || c == '"')
        .ok_or_else(|| anyhow!("unquoted '{key}'"))?;
    let value_end = rest[1..]
        .find(quote)
        .ok_or_else(|| anyhow!("unterminated '{key}'"))?;
    Ok(rest[1..1 + value_end].to_string())
}

fn dict_bool_field(header: &str, key: &str) -> Result<bool> {
    let needle = format!("'{key}':");
    let rest = header
        .find(&needle)
        .map(|i| &header[i + needle.len()..])
        .ok_or_else(|| anyhow!("missing '{key}' in header"))?;
    let rest = rest.trim_start();
    if rest.starts_with("True") {
        Ok(true)
    } else if rest.starts_with("False") {
        Ok(false)
    } else {
        Err(anyhow!("bad bool for '{key}'"))
    }
}

fn dict_shape_field(header: &str) -> Result<Vec<usize>> {
    let key = "'shape':";
    let rest = header
        .find(key)
        .map(|i| &header[i + key.len()..])
        .ok_or_else(|| anyhow!("missing 'shape' in header"))?;
    let open = rest.find('(').ok_or_else(|| anyhow!("bad shape"))?;
    let close = rest.find(')').ok_or_else(|| anyhow!("bad shape"))?;
    if close <= open {
        return Err(anyhow!("bad shape"));
    }
    let inner = &rest[open + 1..close];
    let mut shape = Vec::new();
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        shape.push(
            part.parse::<usize>()
                .map_err(|_| anyhow!("bad shape dim '{part}'"))?,
        );
    }
    if shape.is_empty() {
        return Err(anyhow!("empty shape"));
    }
    Ok(shape)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn build_npy(descr: &str, shape: &str, payload: &[u8]) -> Vec<u8> {
        let header = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': ({shape},)}}");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(1);
        bytes.push(0);
        let padded = header.len() + 1; // + newline
        bytes.extend_from_slice(&(padded as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn parses_f4_array() {
        let mut payload = Vec::new();
        for v in [1.0_f32, -2.5, 3.25] {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        let bytes = build_npy("<f4", "1, 3", &payload);
        let arr = parse_f32(&bytes).unwrap();
        assert_eq!(arr.shape, vec![1, 3]);
        assert_eq!(arr.data, vec![1.0, -2.5, 3.25]);
    }

    #[test]
    fn parses_f8_array_into_f32() {
        let mut payload = Vec::new();
        for v in [1.5_f64, 2.25] {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        let bytes = build_npy("<f8", "2,", &payload);
        let arr = parse_f32(&bytes).unwrap();
        assert_eq!(arr.shape, vec![2]);
        assert_eq!(arr.data, vec![1.5, 2.25]);
    }

    #[test]
    fn rejects_wrong_dtype_and_order() {
        let bytes = build_npy("<i8", "1,", &[0u8; 8]);
        assert!(parse_f32(&bytes).unwrap_err().to_string().contains("dtype"));
        let header = "{'descr': '<f4', 'fortran_order': True, 'shape': (1,)}\n";
        let mut raw = Vec::new();
        raw.extend_from_slice(b"\x93NUMPY");
        raw.push(1);
        raw.push(0);
        raw.extend_from_slice(&(header.len() as u16).to_le_bytes());
        raw.extend_from_slice(header.as_bytes());
        raw.extend_from_slice(&[0u8; 4]);
        assert!(parse_f32(&raw).unwrap_err().to_string().contains("fortran"));
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(parse_f32(b"NOTNPYDATA").is_err());
        assert!(parse_f32(&[]).is_err());
    }
}
