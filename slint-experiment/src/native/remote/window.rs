//! Window placement for the Omarchy plugin build: every window is a remote
//! window drawn as a shell layer, so ids are remote window numbers and moves
//! go to the shell. Same functions as `linux/window.rs`.

use crate::remote_ui;
use std::sync::Mutex;

type Error = Box<dyn std::error::Error>;

pub use super::displays::{cursor, monitors};

static DRAG: Mutex<Option<(isize, i32, i32, i32, i32)>> = Mutex::new(None);

// RE2 FullMatch needs the entire namespace, not a bare prefix. Input-only
// dismissal layers must not match: Hyprland blacks out the whole layer bounds.
fn layer_rule_script(on: bool) -> String {
    format!(
        "suflyor_stealth = hl.layer_rule({{name = \"suflyor-stealth\", \
         match = {{namespace = \"^suflyor-(bar-widget|bar-panel|tile|settings|[a-z]+view)$\"}}, \
         no_screen_share = true}}); suflyor_stealth:set_enabled({on}); \
         assert(suflyor_stealth:is_enabled() == {on}, \"Suflyor capture rule unavailable\")"
    )
}

fn apply_layer_rule(on: bool) -> Result<(), Error> {
    remote_ui::send_global("Suflyor", "stealth-requested", on.into());
    let result = super::desktop::hypr(&["eval", &layer_rule_script(on)]).and_then(|out| {
        if out.trim() == "ok" { Ok(()) } else {
            Err(format!("Hyprland rejected the stealth layer rule: {}", out.trim()).into())
        }
    });
    remote_ui::send_global("Suflyor", "stealth-rule-applied", result.is_ok().into());
    result
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
    match super::desktop::hypr(&["repl", "suflyor_stealth and suflyor_stealth:is_enabled()"])?.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("the stealth layer rule is unavailable".into()),
    }
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
            remote_ui::native_request(id, "move", serde_json::json!([nx - cx, ny - cy]));
            *DRAG.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((id, nx, ny, x, y));
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
        assert!(on.starts_with("suflyor_stealth = hl.layer_rule("));
        assert!(on.contains("namespace = \"^suflyor-(bar-widget|bar-panel|tile|settings|[a-z]+view)$\""));
        assert!(on.contains("no_screen_share = true"));
        assert!(on.contains("suflyor_stealth:set_enabled(true)"));
        assert!(layer_rule_script(false).contains("set_enabled(false)"));
    }
}
