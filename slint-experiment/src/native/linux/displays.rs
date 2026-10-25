//! Hyprland cursor and monitor geometry, shared by the native window module and
//! the Omarchy plugin build's remote one.

use super::desktop::query;

type Error = Box<dyn std::error::Error>;

pub fn cursor() -> Result<(i32, i32), Error> {
    let v = query("cursorpos")?;
    Ok((
        v["x"].as_f64().ok_or("cursor x unavailable")? as i32,
        v["y"].as_f64().ok_or("cursor y unavailable")? as i32,
    ))
}
pub fn monitors() -> Result<Vec<crate::win32::MonitorRect>, Error> {
    let v = query("monitors")?;
    Ok(v.as_array()
        .ok_or("native displays unavailable")?
        .iter()
        .filter_map(|m| {
            let scale = m["scale"].as_f64()?;
            if scale <= 0.0 {
                return None;
            }
            let (mut w, mut h) = (m["width"].as_f64()?, m["height"].as_f64()?);
            if m["transform"].as_u64().unwrap_or(0) % 2 == 1 {
                std::mem::swap(&mut w, &mut h);
            }
            let (x, y) = (m["x"].as_i64()? as i32, m["y"].as_i64()? as i32);
            Some(crate::win32::MonitorRect {
                left: x,
                top: y,
                right: x + (w / scale).round() as i32,
                bottom: y + (h / scale).round() as i32,
                is_primary: m["focused"].as_bool().unwrap_or(false),
            })
        })
        .collect())
}
