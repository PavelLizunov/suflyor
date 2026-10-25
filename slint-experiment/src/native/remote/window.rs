//! Window placement for the Omarchy plugin build: every window is a remote
//! window drawn as a shell layer, so ids are remote window numbers and moves
//! go to the shell. Same functions as `linux/window.rs`.

use crate::remote_ui;
use std::sync::Mutex;

type Error = Box<dyn std::error::Error>;

pub use super::displays::{cursor, monitors};

static DRAG: Mutex<Option<(isize, i32, i32, i32, i32)>> = Mutex::new(None);

/// Last stealth state Hyprland accepted; `None` until a rule request succeeded,
/// so `exclusion` never claims protection the compositor did not confirm.
static EXCLUDED: Mutex<Option<bool>> = Mutex::new(None);

/// One named layer rule hides every Suflyor surface (`suflyor-*` namespaces)
/// from screen sharing; stealth enables or disables it. If a config reload drops
/// the eval-made rule, the next placement or stealth toggle creates it again.
fn layer_rule_script(on: bool) -> String {
    format!(
        "suflyor_stealth = suflyor_stealth or hl.layer_rule({{name = \"suflyor-stealth\", \
         match = {{namespace = \"^suflyor-\"}}, no_screen_share = true}}); \
         suflyor_stealth:set_enabled({on})"
    )
}

fn apply_layer_rule(on: bool) -> Result<(), Error> {
    let out = super::desktop::hypr(&["eval", &layer_rule_script(on)])?;
    if out.to_ascii_lowercase().contains("error") {
        return Err(format!("Hyprland rejected the stealth layer rule: {}", out.trim()).into());
    }
    *EXCLUDED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(on);
    Ok(())
}

pub fn view_id(window: &slint::Window) -> Result<isize, Error> {
    remote_ui::native_id(window).ok_or_else(|| "remote window is not registered".into())
}
pub fn rect(id: isize) -> Result<(i32, i32, i32, i32), Error> {
    remote_ui::native_rect(id).ok_or_else(|| "remote window size is not known yet".into())
}
pub fn move_to(id: isize, x: i32, y: i32) -> Result<(), Error> {
    if remote_ui::native_move(id, x, y) {
        Ok(())
    } else {
        Err("remote window unavailable".into())
    }
}
pub fn resize_move(id: isize, x: i32, y: i32, w: i32, h: i32) -> Result<(), Error> {
    if remote_ui::native_resize_move(id, x, y, w, h) {
        Ok(())
    } else {
        Err("remote window unavailable".into())
    }
}
/// Layers show on every workspace, so placing one needs no pin.
pub fn arrange(id: isize, x: i32, y: i32, excluded: bool) -> Result<(), Error> {
    move_to(id, x, y)?;
    set_exclusion(id, excluded)
}
pub fn focus(id: isize) -> Result<(), Error> {
    if remote_ui::native_request(id, "focus", serde_json::Value::Null) {
        Ok(())
    } else {
        Err("remote window unavailable".into())
    }
}
/// The shell does not report layer focus back; callers treat this as "not known".
pub fn foreground(_id: isize) -> bool {
    false
}
pub fn pin(_id: isize, _on: bool) -> Result<(), Error> {
    Ok(())
}
pub fn set_exclusion(_id: isize, on: bool) -> Result<(), Error> {
    apply_layer_rule(on)
}
pub fn exclusion(_id: isize) -> Result<bool, Error> {
    EXCLUDED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .ok_or_else(|| "the stealth layer rule has not been applied".into())
}
/// Takes every Suflyor layer off screen before a capture; the short wait lets
/// the shell commit the change before grim reads the frame.
pub fn hide_own() -> Vec<(isize, bool)> {
    remote_ui::conceal(true);
    std::thread::sleep(std::time::Duration::from_millis(120));
    vec![(0, true)]
}
pub fn show_own(hidden: &[(isize, bool)]) {
    if !hidden.is_empty() {
        remote_ui::conceal(false);
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

#[cfg(test)]
mod tests {
    use super::layer_rule_script;

    #[test]
    fn stealth_rule_is_one_named_rule_toggled_in_place() {
        let on = layer_rule_script(true);
        assert!(on.starts_with("suflyor_stealth = suflyor_stealth or hl.layer_rule("));
        assert!(on.contains("namespace = \"^suflyor-\""));
        assert!(on.contains("no_screen_share = true"));
        assert!(on.ends_with("suflyor_stealth:set_enabled(true)"));
        assert!(layer_rule_script(false).ends_with("set_enabled(false)"));
    }
}
