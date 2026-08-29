use super::*;

/// v0.10.1 — format the active profile/persona for the text-ask header so the
/// user sees which profile will shape the typed answer. The profile applies to
/// a typed question the SAME as to a voice one (both go through `fire_f9_ask` →
/// `cfg.read().meeting_context` → `ai::build_request`); this label just makes it
/// visible. Read LIVE so a profile switch in Settings is reflected even on a
/// reused window.
fn text_ask_profile_label(cfg: &overlay_backend::config::SharedConfig) -> String {
    let c = cfg.read();
    text_ask_profile_label_for(
        c.active_profile.as_deref(),
        !c.meeting_context.trim().is_empty(),
        c.ui_language == "ru",
    )
}

fn text_ask_profile_label_for(
    active_profile: Option<&str>,
    has_custom_context: bool,
    is_ru: bool,
) -> String {
    match (
        active_profile.filter(|name| !name.trim().is_empty()),
        has_custom_context,
        is_ru,
    ) {
        (Some(name), _, true) => format!("Профиль: {name}"),
        (Some(name), _, false) => format!("Profile: {name}"),
        (None, true, true) => "Профиль: свой контекст".to_string(),
        (None, true, false) => "Profile: custom context".to_string(),
        (None, false, true) => "Профиль: не задан".to_string(),
        (None, false, false) => "Profile: not set".to_string(),
    }
}

/// ТЗ 2026-07-06 (C) — persist the text-ask window's current top-left so the
/// next open restores it (`present_window_stealth_aware_at`). Called on submit
/// and cancel (the window is dropped right after, so this is the last chance to
/// read the rect). No-op when the position is unchanged — avoids a config
/// rewrite on every plain open/close.
fn persist_text_ask_pos(win: &TextAskWindow, cfg: &overlay_backend::config::SharedConfig) {
    let Ok(hwnd) = grab_hwnd(win.window()) else {
        return;
    };
    let Ok((x, y, _, _)) = slint_replay::win32::get_window_rect(hwnd) else {
        return;
    };
    let mut c = cfg.write();
    if c.text_ask_pos != Some((x, y)) {
        c.text_ask_pos = Some((x, y));
        let _ = overlay_backend::config::save(&c);
    }
}

/// V0.8.3 — "Написать": open (or re-focus) the small text-input window. On
/// submit it routes the typed text through `fire_f9_ask(.., Some(text))`, so the
/// whole tile-create + stream + cost + journal + follow-up pipeline is reused →
/// the answer lands in a standard tile. Stealth (WDA) + on-screen placement come
/// from `present_window_stealth_aware_at` (restoring the last dragged position,
/// ТЗ 2026-07-06 C); the decorate closure also grabs keyboard focus so the user
/// can type immediately. Esc (or submit) persists the position + hides + drops.
#[allow(clippy::too_many_arguments)]
pub(in super::super) fn open_text_ask(
    slot_ref: &Rc<RefCell<Option<TextAskWindow>>>,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            // Refresh the profile label in case it changed since this window was
            // first opened (reused windows keep their original handlers).
            existing.set_active_profile(SharedString::from(text_ask_profile_label(cfg)));
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match TextAskWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] TextAskWindow::new failed: {e}");
            return;
        }
    };
    apply_scheme_text_ask(&win, global_scheme());
    win.set_active_profile(SharedString::from(text_ask_profile_label(cfg)));
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let bridge_c = bridge.clone();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        let rt_c = slint_rt.clone();
        let rth = rt_handle.clone();
        let tiles_c = tiles.clone();
        let wov = weak_overlay.clone();
        win.on_submitted(move |q| {
            let q = q.trim().to_string();
            if !q.is_empty() {
                fire_f9_ask(
                    &bridge_c,
                    &events_c,
                    &cfg_c,
                    &rt_c,
                    &rth,
                    &tiles_c,
                    &wov,
                    AskRoute::Text,
                    Some(q),
                );
            }
            if let Some(w) = weak.upgrade() {
                persist_text_ask_pos(&w, &cfg_c);
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let cfg_c = cfg.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                persist_text_ask_pos(&w, &cfg_c);
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    // ТЗ 2026-07-06 (C) — frameless drag (cursor-delta, same as help/wizard);
    // the header row is the handle.
    {
        let weak = win.as_weak();
        win.on_drag_start_requested(move || {
            if let Some(w) = weak.upgrade() {
                #[cfg(target_os = "macos")]
                let _ = slint_replay::native::window::begin_drag(w.window());
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_begin(hwnd);
                }
            }
        });
        let weak_move = win.as_weak();
        win.on_drag_moved(move || {
            if let Some(w) = weak_move.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_update(hwnd);
                }
            }
        });
    }
    // Restore the last dragged position (validated against visible monitors
    // inside; stale/None → centered as before).
    let saved_pos = cfg.read().text_ask_pos;
    present_window_stealth_aware_at(&win, saved_pos, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window) — same as archive.
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn text_ask_profile_label_follows_ui_language() {
        assert_eq!(
            text_ask_profile_label_for(Some("Interview"), false, false),
            "Profile: Interview"
        );
        assert_eq!(
            text_ask_profile_label_for(None, true, false),
            "Profile: custom context"
        );
        assert_eq!(
            text_ask_profile_label_for(None, false, true),
            "Профиль: не задан"
        );
    }
}
