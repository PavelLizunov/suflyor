//! Real compositor window identities and geometry, never synthetic HWNDs.

use super::desktop::{dispatch, query};
use slint::winit_030::winit::window::WindowId;
use slint::winit_030::WinitWindowAccessor;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

struct Identity {
    original: String,
    title: String,
    address: Option<isize>,
}
static WINDOWS: std::sync::LazyLock<Mutex<HashMap<WindowId, Identity>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

type Error = Box<dyn std::error::Error>;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
static DRAG: Mutex<Option<(isize, i32, i32, i32, i32)>> = Mutex::new(None);
static HIDDEN_ALPHA: std::sync::LazyLock<Mutex<HashMap<isize, Vec<(String, serde_json::Value)>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn clients() -> Result<Vec<serde_json::Value>, Error> {
    let value = query("clients")?;
    Ok(value
        .as_array()
        .ok_or("invalid Hyprland window response")?
        .iter()
        .filter(|v| v["pid"].as_u64() == Some(u64::from(std::process::id())))
        .cloned()
        .collect())
}
fn client(id: isize) -> Result<serde_json::Value, Error> {
    clients()?
        .into_iter()
        .find(|v| address(v) == Some(id))
        .ok_or_else(|| "native Wayland window unavailable".into())
}
fn address(v: &serde_json::Value) -> Option<isize> {
    isize::from_str_radix(v["address"].as_str()?.strip_prefix("0x")?, 16).ok()
}
fn selector(id: isize) -> Result<String, Error> {
    client(id)?;
    Ok(format!("address:0x{id:x}"))
}

pub fn view_id(window: &slint::Window) -> Result<isize, Error> {
    let all = clients()?;
    let result = window
        .with_winit_window(|w| -> Result<isize, Error> {
            let mut ids = WINDOWS.lock().map_err(|_| "native identity lock failed")?;
            // Remove resolved windows no longer owned by this process.
            ids.retain(|_, v| {
                v.address
                    .is_none_or(|id| all.iter().any(|c| address(c) == Some(id)))
            });
            if ids.len() > 128 {
                ids.retain(|_, v| v.address.is_some());
            }
            if let Some(address) = ids.get(&w.id()).and_then(|v| v.address) {
                return Ok(address);
            }
            let title = ids
                .get(&w.id())
                .map_or_else(|| w.title(), |v| v.original.clone());
            let claimed: Vec<isize> = ids.values().filter_map(|v| v.address).collect();
            let candidates: Vec<isize> = all
                .iter()
                .filter(|c| {
                    c["title"].as_str() == Some(&title)
                        || c["initialTitle"].as_str() == Some(&title)
                })
                .filter_map(address)
                .filter(|id| !claimed.contains(id))
                .collect();
            if candidates.len() == 1 {
                let address = candidates[0];
                ids.insert(
                    w.id(),
                    Identity {
                        original: title.clone(),
                        title: title.clone(),
                        address: Some(address),
                    },
                );
                if w.title() != title {
                    w.set_title(&title);
                }
                return Ok(address);
            }
            if all.is_empty() {
                return Err("compositor has not mapped this window yet".into());
            }
            let id = ids.entry(w.id()).or_insert_with(|| Identity {
                original: w.title(),
                title: format!(
                    "{} [suflyor-{}-{}]",
                    w.title(),
                    std::process::id(),
                    SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ),
                address: None,
            });
            if let Some(address) = id.address {
                return Ok(address);
            }
            if let Some(found) = all
                .iter()
                .find(|c| c["title"].as_str() == Some(&id.title))
                .and_then(address)
            {
                id.address = Some(found);
                w.set_title(&id.original);
                return Ok(found);
            }
            if candidates.len() > 1 {
                w.set_title(&id.title);
                w.request_redraw();
            }
            // The caller's normal realization retry gives Wayland its next roundtrip.
            Err("compositor has not mapped this window yet".into())
        })
        .ok_or("Wayland window is not realized")?;
    if let Err(error) = &result {
        log::debug!(
            "native identity unresolved: {error}; owned clients={}",
            all.len()
        );
    }
    result
}

pub fn rect(id: isize) -> Result<(i32, i32, i32, i32), Error> {
    let c = client(id)?;
    let n = |key: &str, i: usize| -> Result<i32, Error> {
        c[key][i]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| "invalid native window geometry".into())
    };
    Ok((n("at", 0)?, n("at", 1)?, n("size", 0)?, n("size", 1)?))
}
pub fn move_to(id: isize, x: i32, y: i32) -> Result<(), Error> {
    let s = selector(id)?;
    dispatch("movewindowpixel", &format!("exact {x} {y},{s}"))
}
pub fn resize_move(id: isize, x: i32, y: i32, w: i32, h: i32) -> Result<(), Error> {
    let s = selector(id)?;
    if !client(id)?["floating"].as_bool().unwrap_or(false) {
        dispatch("setfloating", &s)?;
    }
    dispatch(
        "resizewindowpixel",
        &format!("exact {} {},{s}", w.max(1), h.max(1)),
    )?;
    move_to(id, x, y)
}
pub fn focus(id: isize) -> Result<(), Error> {
    dispatch("focuswindow", &selector(id)?)
}
pub fn foreground(id: isize) -> bool {
    query("activewindow").ok().and_then(|v| address(&v)) == Some(id)
}
pub fn pin(id: isize, on: bool) -> Result<(), Error> {
    let c = client(id)?;
    if c["pinned"].as_bool().unwrap_or(false) != on {
        dispatch("pin", &selector(id)?)?;
    }
    Ok(())
}
pub fn set_exclusion(id: isize, on: bool) -> Result<(), Error> {
    let selector = selector(id)?;
    super::desktop::set_property(&selector, "no_screen_share", if on { "1" } else { "0" })?;
    if super::desktop::property(&selector, "no_screen_share")?.as_bool() != Some(on) {
        return Err("capture exclusion readback failed".into());
    }
    Ok(())
}
pub fn exclusion(id: isize) -> Result<bool, Error> {
    super::desktop::property(&selector(id)?, "no_screen_share")?
        .as_bool()
        .ok_or_else(|| "capture exclusion unavailable".into())
}
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
pub fn hide_own() -> Vec<(isize, bool)> {
    let mut hidden = Vec::new();
    for c in clients().unwrap_or_default() {
        if let Some(id) = address(&c) {
            if c["hidden"].as_bool() == Some(true) {
                continue;
            }
            let selector = format!("address:0x{id:x}");
            let saved: Vec<_> = [
                "opacity",
                "opacity_inactive",
                "opacity_fullscreen",
                "no_screen_share",
                "no_anim",
            ]
            .iter()
            .filter_map(|name| {
                super::desktop::property(&selector, name)
                    .ok()
                    .map(|v| (name.to_string(), v))
            })
            .collect();
            if saved.len() == 5 {
                let _ = super::desktop::set_property(&selector, "no_anim", "1");
                let mut transparent = true;
                for name in ["opacity", "opacity_inactive", "opacity_fullscreen"] {
                    transparent &= super::desktop::set_property(&selector, name, "0").is_ok();
                }
                HIDDEN_ALPHA
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(id, saved);
                if transparent {
                    // Exclusion paints black even at zero opacity. Suspend it
                    // only after the surface itself has become transparent.
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let _ = super::desktop::set_property(&selector, "no_screen_share", "0");
                }
                hidden.push((id, true));
            }
        }
    }
    hidden
}
pub fn show_own(hidden: &[(isize, bool)]) {
    for (id, _) in hidden {
        let saved = HIDDEN_ALPHA
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(id);
        if let (Some(mut saved), Ok(s)) = (saved, selector(*id)) {
            // Restore capture protection before any content becomes visible.
            saved.sort_by_key(|(name, _)| {
                if name == "no_screen_share" {
                    0
                } else if name == "no_anim" {
                    2
                } else {
                    1
                }
            });
            for (name, value) in saved {
                let value = if let Some(flag) = value.as_bool() {
                    Some(if flag {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    })
                } else {
                    value.as_f64().map(|alpha| alpha.to_string())
                };
                if let Some(value) = value {
                    let _ = super::desktop::set_property(&s, &name, &value);
                }
            }
        }
    }
}
pub fn drag_begin(id: isize) {
    if let (Ok((cx, cy)), Ok((x, y, _, _))) = (cursor(), rect(id)) {
        *DRAG
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((id, cx, cy, x, y));
    }
}
pub fn drag_update(id: isize) {
    let saved = *DRAG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let (Some((active, cx, cy, x, y)), Ok((nx, ny))) = (saved, cursor()) {
        if id == active {
            let _ = move_to(id, x + nx - cx, y + ny - cy);
        }
    }
}
pub fn drag_end() {
    *DRAG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}
