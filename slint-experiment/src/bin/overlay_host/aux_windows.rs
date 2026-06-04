//! Auxiliary on-demand overlay windows split out of the `overlay_host.rs`
//! composition root (P2 of `docs/overlay-host-gaps-and-next-checks.md`): the
//! "✏ Написать" text-ask window (`open_text_ask`), the 🆘 Help window
//! (`open_help`), and the F4 KB palette (`open_palette`) plus the palette's pure
//! helpers (`results_index`, `kb_to_palette_results`, `PaletteResultExt`). These
//! were the last large blocks of host-side window wiring still inlined in the
//! binary's file; `overlay_host.rs` now reaches them through the
//! `use aux_windows::*;` re-export, so the call sites (F1 / F4 / ✏ dispatch +
//! the 🆘 chip in `main`) resolve unchanged.
//!
//! SECURITY (unchanged by this mechanical move): every window is parked
//! off-screen + WDA-stealthed via `present_window_stealth_aware` before its
//! first on-screen frame, and skipped from the taskbar / Alt-Tab, so it never
//! leaks onto a screen-share while open. The palette renders only KB text — no
//! bearer / base_url / transcript ever reaches its scope.
//!
//! NOTE (§7): the parent crate-root symbols are imported explicitly below;
//! `diag!` is reached by textual macro scope (defined before the `mod` decl).
use super::{
    apply_scheme_palette, apply_scheme_text_ask, apply_tile_hwnd_with_monitor, clamp_scheme,
    drag_begin, drag_update, fire_f9_ask, focus_window, global_scheme, grab_hwnd, kb, markdown,
    present_tile_window, present_window_stealth_aware, refresh_open_tiles, toggle_tile_maximize,
    ui, wire_tile_drag, Arc, AskRoute, ComponentHandle, HelpWindow, MarkdownBlock, ModelRc,
    OverlayBarBridge, OverlayBarWindow, PaletteResult, PaletteWindow, Rc, RefCell, RuntimeEvents,
    SharedSlintRuntime, SharedString, TextAskWindow, TileWindow, TileWindows, VecModel,
};

/// V0.8.3 — "Написать": open (or re-focus) the small text-input window. On
/// submit it routes the typed text through `fire_f9_ask(.., Some(text))`, so the
/// whole tile-create + stream + cost + journal + follow-up pipeline is reused →
/// the answer lands in a standard tile. Stealth (WDA) + on-screen placement come
/// from `present_window_stealth_aware`; the decorate closure also grabs keyboard
/// focus so the user can type immediately. Esc (or submit) hides + drops it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_text_ask(
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
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    present_window_stealth_aware(&win, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

/// V0.8.4 — 🆘 Help (F1 / 🆘 chip): a read-only reference window (bar icons,
/// hotkeys, record gestures). Created on demand like open_text_ask —
/// scheme-themed, stealth-aware, Esc / "X" to close. Re-opening re-focuses it.
pub(crate) fn open_help(
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
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}

/// Open (or reuse) the KB palette window. Auto-spawn a tile when
/// the user activates a result, mimicking the React palette flow.
pub(crate) fn open_palette(
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

        // Spawn a tile with the result content (re-uses Phase 4 plumbing).
        let seq = {
            let mut st = match s_ref.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            st.tiles_spawned += 1;
            st.tiles_spawned
        };
        if let Some(o) = weak_overlay2.upgrade() {
            o.set_tiles_spawned(seq as i32);
        }
        if let Ok(tile) = TileWindow::new() {
            tile.set_sequence(seq as i32);
            tile.set_tile_title(SharedString::from(result.title.to_string()));
            tile.set_source_label(SharedString::from(format!("kb · {}", result.source)));
            wire_tile_drag(&tile);
            // Phase C — wire to real kb::get for the full body. Falls
            // back to the preview if the key isn't found (defensive;
            // shouldn't happen since result came from kb::search).
            let body = kb::get(result.key.as_str())
                .map_or_else(|| result.preview.to_string(), |e| e.body.clone());
            let md = format!("# {}\n\n{body}\n", result.heading_or_key());
            let blocks: Vec<MarkdownBlock> = markdown::parse(&md)
                .into_iter()
                .map(|b| MarkdownBlock {
                    kind: b.kind,
                    text: SharedString::from(b.text),
                    lang: SharedString::from(b.lang),
                })
                .collect();
            tile.set_blocks(ModelRc::new(VecModel::from(blocks)));

            let weak_tile = tile.as_weak();
            let vec_for_close = tiles_ref2.clone();
            let weak_overlay_close = weak_overlay2.clone();
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (KB-palette) close_clicked fired");
                if let Some(t) = weak_tile.upgrade() {
                    let close_hwnd = grab_hwnd(t.window()).ok();
                    let _ = t.hide();
                    if let Some(target) = close_hwnd {
                        vec_for_close
                            .borrow_mut()
                            .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
                        refresh_open_tiles(&weak_overlay_close, &vec_for_close);
                    }
                }
            });
            // Pin toggles visual state (cycle 17 stub upgraded v17).
            let weak_pin = tile.as_weak();
            tile.on_pin_clicked(move || {
                if let Some(t) = weak_pin.upgrade() {
                    let new = !t.get_pinned();
                    t.set_pinned(new);
                }
            });
            let weak_max = tile.as_weak();
            tile.on_maximize_clicked(move || {
                if let Some(t) = weak_max.upgrade() {
                    let Ok(hwnd) = grab_hwnd(t.window()) else {
                        return;
                    };
                    toggle_tile_maximize(hwnd, &t);
                }
            });

            present_tile_window(&tile);
            apply_tile_hwnd_with_monitor(&tile);
            tiles_ref2.borrow_mut().push(tile);
            refresh_open_tiles(&weak_overlay2, &tiles_ref2);
        }
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
