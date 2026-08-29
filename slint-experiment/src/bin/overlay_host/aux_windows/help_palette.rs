use super::*;

/// V0.8.4 — 🆘 Help (F1 / 🆘 chip): a read-only reference window (bar icons,
/// hotkeys, record gestures). Created on demand like open_text_ask —
/// scheme-themed, stealth-aware, Esc / "X" to close. Re-opening re-focuses it.
pub(in super::super) fn open_help(
    slot_ref: &Rc<RefCell<Option<HelpWindow>>>,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match HelpWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] HelpWindow::new failed: {e}");
            return;
        }
    };
    win.global::<ui::Platform>().set_is_macos(true);
    win.global::<ui::Theme>()
        .set_scheme(clamp_scheme(global_scheme()));
    // Light up the bar's 🆘 chip while help is open (same as ⚙ for Settings).
    if let Some(o) = overlay_weak.upgrade() {
        o.set_help_open(true);
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        let ow = overlay_weak.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
            if let Some(o) = ow.upgrade() {
                o.set_help_open(false);
            }
        });
    }
    // Frameless drag (cursor-delta, same as Settings) — the header is the handle.
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
    present_window_stealth_aware(&win, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window can't get them from
        // an inner border-radius) — same as the archive window.
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

/// Open (or reuse) the KB palette window. Auto-spawn a tile when
/// the user activates a result, mimicking the React palette flow.
pub(in super::super) fn open_palette(
    palette_ref: &Rc<RefCell<Option<PaletteWindow>>>,
    tiles_ref: &TileWindows,
    state: &slint_replay::app_state::SharedState,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    let mut slot = palette_ref.borrow_mut();
    if let Some(existing) = slot.as_ref() {
        let _ = existing.show();
        return;
    }
    let win = match PaletteWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] PaletteWindow::new failed: {e}");
            return;
        }
    };
    // Seed the palette's Theme global from the live scheme (the palette is
    // ephemeral — spawned per F4 — so it just reads at construction).
    apply_scheme_palette(&win, global_scheme());

    // Phase C — wire palette to real overlay_backend::kb::search.
    // Initial load: show top 20 entries (popular/first in cache).
    let initial = kb_to_palette_results(&kb::search("", 20));
    win.set_results(slint::ModelRc::new(slint::VecModel::from(initial)));

    let weak_self_q = win.as_weak();
    win.on_query_changed(move |q| {
        let Some(p) = weak_self_q.upgrade() else {
            return;
        };
        let hits = kb::search(q.as_str(), 20);
        let model = kb_to_palette_results(&hits);
        p.set_results(slint::ModelRc::new(slint::VecModel::from(model)));
    });

    let weak_close = win.as_weak();
    let palette_close = palette_ref.clone();
    win.on_close_requested(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *palette_close.borrow_mut() = None;
    });

    // Frameless drag — the new header owns the grab target; the close button
    // remains a sibling so pointer events never conflict.
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

    let s_ref = state.clone();
    let tiles_ref2 = tiles_ref.clone();
    let weak_overlay2 = weak_overlay.clone();
    let palette_after = palette_ref.clone();
    let weak_self = win.as_weak();
    win.on_result_activated(move |idx| {
        let Some(p) = weak_self.upgrade() else { return };
        let results = p.get_results();
        let Some(result) = results_index(&results, idx) else {
            return;
        };

        // Spawn a read-only tile with the result content via the shared helper
        // (also used by the session archive). Phase C — wire to real kb::get for
        // the full body; fall back to the preview if the key isn't found
        // (defensive — the result came from kb::search).
        let body = kb::get(result.key.as_str())
            .map_or_else(|| result.preview.to_string(), |e| e.body.clone());
        let md = format!("# {}\n\n{body}\n", result.heading_or_key());
        spawn_content_tile(
            result.title.as_str(),
            &format!("kb · {}", result.source),
            &md,
            &tiles_ref2,
            &s_ref,
            &weak_overlay2,
        );
        // Close palette after activation.
        if let Some(p) = weak_self.upgrade() {
            let _ = p.hide();
        }
        *palette_after.borrow_mut() = None;
    });

    // #111 + review M1 — exclude the palette from capture WITHOUT a flash:
    // park off-screen before show, apply WDA, then reveal centred. No extra
    // HWND decoration for the palette (it's an opaque window).
    present_window_stealth_aware(&win, |hwnd| {
        // Keep the palette out of the taskbar/Alt-Tab too (stealth existence
        // leak — same as help/text-ask/wizard above).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window) — same as archive.
        slint_replay::win32::set_round_corners(hwnd);
    });
    *slot = Some(win);
}

fn results_index(model: &slint::ModelRc<PaletteResult>, idx: i32) -> Option<PaletteResult> {
    use slint::Model;
    if idx < 0 {
        return None;
    }
    model.row_data(idx as usize)
}

/// Convert overlay_backend::kb::KBEntry rows into the Slint PaletteResult
/// struct that the .slint UI consumes.
fn kb_to_palette_results(entries: &[kb::KBEntry]) -> Vec<PaletteResult> {
    entries
        .iter()
        .map(|e| {
            // First sentence (or first 160 chars) of body for preview.
            let preview = e
                .body
                .split_terminator(['.', '\n'])
                .next()
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>();
            PaletteResult {
                key: SharedString::from(e.key.clone()),
                title: SharedString::from(e.heading.clone()),
                preview: SharedString::from(preview),
                source: SharedString::from(e.source),
            }
        })
        .collect()
}

/// PaletteResult ergonomic extension — `heading_or_key` returns the
/// .heading if non-empty, else falls back to the .key. Stops the
/// tile title from being blank when an entry has just a key.
trait PaletteResultExt {
    fn heading_or_key(&self) -> String;
}

impl PaletteResultExt for PaletteResult {
    fn heading_or_key(&self) -> String {
        if self.title.is_empty() {
            self.key.to_string()
        } else {
            self.title.to_string()
        }
    }
}
