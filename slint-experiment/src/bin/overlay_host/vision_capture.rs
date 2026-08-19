//! Vision capture: the F8 / Shift+F8 screenshot → vision → tile ORCHESTRATION
//! (Phase 5 of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.6).
//!
//! This module owns the host-side glue that turns a frozen virtual-desktop
//! snapshot + a user-selected region into a streaming Vision tile:
//!
//! - `fire_f8_vision_capture` — the F8 (describe) / Shift+F8 (translate) handler.
//!   It freezes the WHOLE virtual desktop, reuses the PERSISTENT, pre-stealthed
//!   capture overlay (constructed + WDA-stealthed in `main` — §5.1, NOT touched
//!   here), wires its `on_region_selected` / `on_cancelled` callbacks, and on
//!   release crops the frozen frame and hands it to `launch_vision_for_bgra`.
//! - `launch_vision_for_bgra` — spawns the placeholder Vision tile, wires its
//!   follow-up / regenerate / voice / copy / close affordances, then encodes the
//!   frame off-thread and streams the answer in via the SEPARATE vision endpoint.
//! - `bgra_to_slint_image` — the BGRA→Slint-RGBA bridge used solely to display
//!   the frozen snapshot in the capture overlay (vision-only helper).
//!
//! What STAYS in `overlay_host.rs` (reached here through the glob below):
//! - the PERSISTENT capture-overlay CONSTRUCTION + its pre-stealth (WDA before
//!   the first frame) in `main` — §5.1 special case, untouched;
//! - the hotkey DISPATCH (F8 / Shift+F8) and the 📷 capture-chip wiring in
//!   `main` — they call `fire_f8_vision_capture` via the `use vision_capture::*;`
//!   re-export at crate root;
//! - the shared tile/ask machinery (`OverlayBarBridge`, `PttStreamSink`,
//!   `AskRoute`/`live_route`, `wire_tile_drag`, `present_tile_window`,
//!   `apply_tile_hwnd_with_monitor`, `toggle_tile_maximize`, `wire_copy`,
//!   `wire_voice_followup`, `fire_followup_ask`, `fire_regenerate`,
//!   `ptt_tile_error`, `refresh_open_tiles`, `CONVO_SEQ`, `TILE_DISPLAY_SEQ`),
//!   which is used by the F9/PTT tiles too — left in place, reached via glob.
//!
//! The low-level BGRA capture (`slint_replay::capture`) and the Win32 helpers
//! (`slint_replay::win32`) are already separate modules and are NOT touched.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    ai, apply_tile_hwnd_with_monitor, fire_followup_ask, fire_regenerate, grab_hwnd, journal,
    live_route, markdown, present_tile_window, ptt_tile_error, refresh_open_tiles,
    set_always_on_top, set_stealth, surface_stealth_unavailable, toggle_tile_maximize, vision,
    wire_copy, wire_speak, wire_tile_drag, wire_voice_followup, Arc, AskRoute, CaptureOverlay,
    ComponentHandle, MarkdownBlock, ModelRc, MonitorHint, Ordering, OverlayBarBridge,
    OverlayBarWindow, PttStreamSink, Rc, RefCell, RuntimeEvents, SharedSlintRuntime, SharedString,
    TileKind, TileSpec, TileWindow, TileWindows, VecModel, CONVO_SEQ, TILE_DISPLAY_SEQ,
};

/// Build a Slint RGBA image from a top-down BGRA capture. Alpha is forced
/// opaque — GDI BitBlt leaves garbage in the alpha byte. Used by the V3 capture
/// overlay to display the frozen virtual-desktop snapshot.
pub(crate) fn bgra_to_slint_image(bgra: &[u8], w: u32, h: u32) -> slint::Image {
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let dst = buf.make_mut_bytes();
    for (i, px) in bgra.chunks_exact(4).enumerate() {
        let o = i * 4;
        if let Some(slot) = dst.get_mut(o..o + 4) {
            slot[0] = px[2]; // R
            slot[1] = px[1]; // G
            slot[2] = px[0]; // B
            slot[3] = 255; // A
        }
    }
    slint::Image::from_rgba8(buf)
}

#[must_use]
fn local_ocr_available() -> bool {
    #[cfg(windows)]
    {
        overlay_backend::ocr::is_available()
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        false
    }
}

fn run_local_ocr(bgra: &[u8], width: u32, height: u32) -> Result<String, String> {
    #[cfg(windows)]
    {
        overlay_backend::ocr::run_ocr(bgra, width, height, overlay_backend::ocr::DEFAULT_OCR_LANG)
            .map_err(|error| format!("{error:#}"))
    }
    #[cfg(target_os = "macos")]
    {
        slint_replay::native::screen::recognize_text_from_bgra(bgra, width, height)
            .map_err(|error| error.to_string())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (bgra, width, height);
        Err("local OCR is unsupported".into())
    }
}

/// V3 — F8 screenshot. Freezes the whole virtual desktop, shows a Lightshot-
/// style selection overlay, and on release crops the frozen frame to the chosen
/// region and hands it to `launch_vision_for_bgra`. Esc / right-click / a tiny
/// drag cancel. The capture goes to the SEPARATE vision endpoint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_f8_vision_capture(
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
    capture_overlay: &Rc<RefCell<Option<CaptureOverlay>>>,
    mode: vision::VisionMode,
) {
    // Second F8 while an overlay is up → dismiss it FIRST (before resolving the
    // provider), so a stuck overlay can ALWAYS be cleared — even if Vision was
    // since switched to "off" in Settings. Escape hatch for a drag that lost its
    // pointer-up.
    {
        let b = capture_overlay.borrow();
        if let Some(win) = b.as_ref() {
            if win.get_shown() {
                win.set_shown(false);
                let _ = win.hide();
                diag!("[overlay-host] F8: capture overlay dismissed");
                return;
            }
        }
    }
    // The VLM modes need a configured vision endpoint. OCR uses the platform's
    // local engine (Tesseract on Windows, Apple Vision on macOS), so it remains
    // available when the VLM route is off.
    let ocr_ready = matches!(mode, vision::VisionMode::Ocr) && local_ocr_available();
    let ep = cfg.read().vision_endpoint();
    if ep.is_none() && !ocr_ready {
        let (is_ru, preferred_monitor, stealth) = {
            let c = cfg.read();
            (c.ui_is_ru(), c.tile_monitor_name.clone(), c.stealth_enabled)
        };
        let answer = if is_ru {
            "Vision выключен. Выберите маршрут в Настройки → AI мост → Vision."
        } else {
            "Vision is off. Choose a route in Settings → AI bridge → Vision."
        };
        let monitor = match preferred_monitor.as_deref() {
            Some(name) if !name.is_empty() => MonitorHint::Named(name.to_string()),
            _ => MonitorHint::Auto,
        };
        if let Err(e) = events.spawn_tile_full(
            TileSpec {
                question: "Vision (F8)".into(),
                answer: answer.into(),
                source: "vision_off".into(),
                is_translation: false,
                highlights: vec![],
                summary_session: None,
            },
            monitor,
            stealth,
            TileKind::Error,
        ) {
            eprintln!("[overlay-host] F8 vision-off notice tile spawn failed: {e}");
        }
        diag!("[overlay-host] F8: vision off and no local OCR — notice shown");
        return;
    }

    // Freeze the virtual desktop for region selection. Windows composes all
    // monitors; macOS currently captures the display under the cursor. In both
    // cases the returned global origin can be negative and must position the
    // overlay rather than assuming (0, 0).
    let hidden = slint_replay::win32::hide_own_windows();
    let frozen = slint_replay::capture::capture_virtual_desktop();
    slint_replay::win32::show_windows(&hidden);
    let (frozen, vx, vy) = match frozen {
        Ok(x) => x,
        Err(e) => {
            diag!("[overlay-host] F8: virtual capture failed: {e}");
            return;
        }
    };
    let (fw, fh) = (frozen.width, frozen.height);
    diag!(
        "[overlay-host] F8 capture origin=({vx},{vy}) {fw}x{fh} monitors={:?}",
        slint_replay::win32::enum_monitors()
            .iter()
            .map(|m| (m.left, m.top, m.right, m.bottom))
            .collect::<Vec<_>>()
    );
    let img = bgra_to_slint_image(&frozen.bgra, fw, fh);

    // Reuse the PERSISTENT, pre-stealthed overlay (created at startup). Its
    // WDA_EXCLUDEFROMCAPTURE + WS_EX_TOOLWINDOW persist across hide/show, so it
    // shows flash-free: never visible on a screen-share, never in the taskbar.
    let b = capture_overlay.borrow();
    let Some(win) = b.as_ref() else {
        eprintln!("[overlay-host] F8: capture overlay not initialised");
        return;
    };
    win.set_frozen(img);
    win.set_dragging(false); // clear any stale selection rect from a prior capture
                             // Seed the capture overlay's mode: Shift+F8 → translate; plain F8 →
                             // describe OR test-practice (per the Settings toggle, resolved by the
                             // caller). The on-overlay tap can still flip to translate before drag.
    win.set_translate_mode(mode == vision::VisionMode::Translate);
    win.set_practice_mode(mode == vision::VisionMode::TestPractice);
    // Geometry is set on the still-hidden window, then show() lands it there.
    // GDI frames use physical pixels; ScreenCaptureKit is configured in macOS
    // screen points, matching Slint's logical coordinates on Retina displays.
    #[cfg(windows)]
    {
        win.window()
            .set_size(slint::PhysicalSize::new(fw.max(1), fh.max(1)));
        win.window()
            .set_position(slint::PhysicalPosition::new(vx, vy));
    }
    #[cfg(target_os = "macos")]
    {
        win.window()
            .set_size(slint::LogicalSize::new(fw.max(1) as f32, fh.max(1) as f32));
        win.window()
            .set_position(slint::LogicalPosition::new(vx as f32, vy as f32));
    }
    let _ = win.show();
    let window_scale = win.window().scale_factor().max(0.1);
    #[cfg(windows)]
    let capture_scale = window_scale;
    #[cfg(target_os = "macos")]
    let capture_scale = 1.0_f32;
    diag!(
        "[overlay-host] F8 overlay {fw}x{fh} at ({vx},{vy}) \
         window-scale={window_scale} capture-scale={capture_scale}"
    );

    // Share the frozen frame into the region callback (UI thread only → Rc ok).
    let frozen_rc = Rc::new(frozen);
    {
        let weak_self = win.as_weak();
        let frozen_c = frozen_rc.clone();
        let bridge_c = bridge.clone();
        let events_c = events.clone();
        let rt_c = slint_rt.clone();
        let h_c = rt_handle.clone();
        let tiles_c = tiles.clone();
        let wo_c = weak_overlay.clone();
        let ep_c = ep.clone();
        let cfg_c = cfg.clone();
        win.on_region_selected(move |x1, y1, x2, y2| {
            // Read the overlay's mode BEFORE hiding it (Shift+F8 seeds it; the
            // on-overlay Describe/Translate toggle can override before drag).
            let mode = if let Some(w) = weak_self.upgrade() {
                // OCR (Ctrl+F8 read-aloud) is NOT an on-overlay toggle — the
                // requested mode wins so a tap can't collapse it to Describe.
                // Otherwise translate-mode takes precedence (a tap toggles it).
                let m = if matches!(mode, vision::VisionMode::Ocr) {
                    vision::VisionMode::Ocr
                } else if w.get_translate_mode() {
                    vision::VisionMode::Translate
                } else if w.get_practice_mode() {
                    vision::VisionMode::TestPractice
                } else {
                    vision::VisionMode::Describe
                };
                w.set_shown(false);
                let _ = w.hide();
                m
            } else {
                mode
            };
            // Windows GDI frames use physical pixels; the macOS frame is
            // deliberately one image pixel per logical ScreenCaptureKit point.
            let to_px = |v: f32| (v * capture_scale).round().max(0.0) as u32;
            let (px1, py1) = (to_px(x1), to_px(y1));
            let (px2, py2) = (to_px(x2), to_px(y2));
            let (cw, ch) = (px2.saturating_sub(px1), py2.saturating_sub(py1));
            // Audit (F8 #4): reject a tiny/degenerate region BEFORE crop_bgra — its
            // `.max(1)` would otherwise coerce it into a 1×1 buffer and launch a
            // spurious vision request on noise. The .slint already guards with a
            // 16px-logical minimum + a fresh-drag check; this is the image-pixel
            // backstop (covers DPI rounding + any future caller of this path).
            const MIN_CAPTURE_PX: u32 = 8;
            if cw < MIN_CAPTURE_PX || ch < MIN_CAPTURE_PX {
                diag!(
                    "[overlay-host] F8 region rejected: {cw}x{ch} image px \
                     (logical {x1:.0},{y1:.0}-{x2:.0},{y2:.0} \
                     capture-scale={capture_scale:.2}) — too small, no request"
                );
                return;
            }
            let cropped = slint_replay::capture::crop_bgra(&frozen_c, px1, py1, cw, ch);
            launch_vision_for_bgra(
                cropped,
                ep_c.clone(),
                mode,
                &bridge_c,
                &events_c,
                &cfg_c,
                &rt_c,
                &h_c,
                &tiles_c,
                &wo_c,
            );
        });
    }
    {
        let weak_self = win.as_weak();
        win.on_cancelled(move || {
            if let Some(w) = weak_self.upgrade() {
                w.set_shown(false);
                let _ = w.hide();
            }
        });
    }

    win.set_shown(true);
    // The persistent HWND exists, so grab_hwnd works synchronously here. winit
    // re-applies the window's ex-style on show() (it drops WS_EX_TOOLWINDOW, so
    // the taskbar button would reappear) — and the pre-create WDA affinity must
    // NEVER be assumed to have survived: I4 re-applies + READS BACK stealth on
    // EVERY show. `set_stealth` verifies via GetWindowDisplayAffinity; on a
    // failed exclusion we surface "stealth unavailable" instead of claiming it.
    // Synchronous, so both land before the shell creates a taskbar button / the
    // first composited frame = flash-free.
    match grab_hwnd(win.window()) {
        Ok(hwnd) => {
            let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
            let _ = set_always_on_top(hwnd, true);
            match set_stealth(hwnd, true) {
                Ok(()) => diag!("[overlay-host] F8 overlay stealth re-applied + verified"),
                Err(e) => {
                    diag!(
                        "[overlay-host] F8 overlay stealth FAILED — capture overlay is \
                         capturable: {e}"
                    );
                    if let Some(o) = weak_overlay.upgrade() {
                        surface_stealth_unavailable(&o);
                    }
                }
            }
            slint_replay::win32::focus_window(hwnd);
        }
        Err(e) => {
            diag!("[overlay-host] F8 overlay HWND grab failed — stealth NOT verified: {e}");
            // I4 — without an HWND the exclusion could be neither applied nor
            // verified; surface the SAME generic failure as a failed WDA apply
            // above (a log-only miss would leave the user believing the capture
            // overlay is hidden).
            if let Some(o) = weak_overlay.upgrade() {
                surface_stealth_unavailable(&o);
            }
        }
    }
}

/// Spawn a vision tile for a captured BGRA frame and stream the answer into it
/// via the SEPARATE vision endpoint. Shared entry for the F8 region capture.
///
/// `ep` is `Some` for the VLM modes (Describe / Translate / TestPractice) and
/// for an OCR request that may need to fall back to the VLM; it is `None` only
/// for a local-OCR request made while Vision is "off" (the platform engine needs
/// no endpoint). If a non-OCR path is reached with `ep == None` — only possible
/// if the OCR engine vanished between the `is_available()` checks (TOCTOU) — a
/// generic "OCR недоступен" tile is shown instead of a network call.
fn vision_tile_copy(
    mode: vision::VisionMode,
    is_ru: bool,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match (mode, is_ru) {
        (vision::VisionMode::Translate, true) => (
            "Перевод",
            "vision · перевод…",
            "Shift+F8 перевод",
            "Перевожу…",
        ),
        (vision::VisionMode::Translate, false) => (
            "Translation",
            "vision · translating…",
            "Shift+F8 translate",
            "Translating…",
        ),
        (vision::VisionMode::TestPractice, true) => (
            "Тренировка",
            "vision · тренировка…",
            "Practice",
            "Решаю вопрос…",
        ),
        (vision::VisionMode::TestPractice, false) => (
            "Practice",
            "vision · practice…",
            "Practice",
            "Solving question…",
        ),
        (vision::VisionMode::Describe, true) => (
            "Скриншот",
            "vision · анализ…",
            "F8 vision",
            "Распознаю экран…",
        ),
        (vision::VisionMode::Describe, false) => (
            "Screenshot",
            "vision · analyzing…",
            "F8 vision",
            "Analyzing screen…",
        ),
        (vision::VisionMode::Ocr, true) => (
            "Текст с экрана",
            "vision · текст…",
            "Ctrl+F8 текст",
            "Распознаю текст…",
        ),
        (vision::VisionMode::Ocr, false) => (
            "Screen text",
            "vision · text…",
            "Ctrl+F8 text",
            "Recognizing text…",
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn launch_vision_for_bgra(
    shot: slint_replay::capture::CapturedBgra,
    ep: Option<overlay_backend::config::AiEndpoint>,
    mode: vision::VisionMode,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    // ===== Placeholder vision tile (mirrors the PTT tile setup) =====
    let tile = match TileWindow::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[overlay-host] F8: TileWindow::new failed: {e}");
            return;
        }
    };
    let seq = TILE_DISPLAY_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    // V5 — give the vision tile a real conversation id so its follow-up input
    // appears + PttStreamSink seeds the conversation (incl. the screenshot) on
    // done; follow-ups then route to the VISION endpoint (use_vision = true).
    let convo_id = CONVO_SEQ.fetch_add(1, Ordering::Relaxed) as i32;
    tile.set_sequence(seq as i32);
    // Per-mode tile chrome. The trigger_label doubles as the "Practice" badge
    // so a test-practice answer is always visibly marked as self-check.
    // Placeholders are plain text — no hourglass glyph (tofu square on the
    // skia font fallback; project no-tofu rule).
    let ui_is_ru = cfg.read().ui_is_ru();
    let (title_s, source_s, trigger_s, placeholder_s) = vision_tile_copy(mode, ui_is_ru);
    tile.set_tile_title(SharedString::from(title_s));
    tile.set_source_label(SharedString::from(source_s));
    tile.set_trigger_label(SharedString::from(trigger_s));
    tile.set_trigger_color(slint::Color::from_rgb_u8(0x22, 0xd3, 0xee)); // cyan
    tile.set_convo_id(convo_id);
    tile.set_followup_busy(true);
    wire_tile_drag(&tile);
    tile.set_blocks(ModelRc::new(VecModel::from(vec![MarkdownBlock {
        kind: markdown::kind::PARAGRAPH,
        text: SharedString::from(placeholder_s),
        display_text: SharedString::from(placeholder_s),
        lang: SharedString::from(""),
        marked: false,
    }])));
    let weak_close = tile.as_weak();
    let vec_for_close = tiles.clone();
    let weak_overlay_close = weak_overlay.clone();
    let bridge_for_close = bridge.clone();
    tile.on_close_clicked(move || {
        if let Some(t) = weak_close.upgrade() {
            // Closing the tile that's being read aloud must silence it.
            super::stop_if_speaking(t.get_convo_id());
            // FIX #8 — prune this tile's conversation (no-op if none).
            bridge_for_close.drop_conversation(t.get_convo_id());
            let close_hwnd = grab_hwnd(t.window()).ok();
            let _ = t.hide();
            slint_replay::win32::force_hide(t.window());
            if let Some(target) = close_hwnd {
                vec_for_close
                    .borrow_mut()
                    .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
                refresh_open_tiles(&weak_overlay_close, &vec_for_close);
            }
        }
    });
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
    // V5 — follow-up: a question typed in the tile continues the dialog ABOUT the
    // screenshot via the VISION endpoint (use_vision = true). The conversation
    // PttStreamSink seeds on done already carries the image.
    {
        let weak_fu = tile.as_weak();
        let bridge_fu = bridge.clone();
        let events_fu = events.clone();
        let cfg_fu = cfg.clone();
        let slint_rt_fu = slint_rt.clone();
        let rt_handle_fu = rt_handle.clone();
        tile.on_followup_submitted(move |q| {
            fire_followup_ask(
                (convo_id, q.to_string()),
                weak_fu.clone(),
                &bridge_fu,
                &events_fu,
                &cfg_fu,
                &slint_rt_fu,
                &rt_handle_fu,
                AskRoute::Vision,
            );
        });
    }
    // V5 — 🔄 regenerate: re-run the screenshot query (vision endpoint) for a
    // longer / different answer when the first one was too short.
    tile.set_can_regenerate(true);
    {
        let weak_re = tile.as_weak();
        let bridge_re = bridge.clone();
        let events_re = events.clone();
        let cfg_re = cfg.clone();
        let slint_rt_re = slint_rt.clone();
        let rt_handle_re = rt_handle.clone();
        tile.on_regenerate_clicked(move || {
            fire_regenerate(
                convo_id,
                weak_re.clone(),
                &bridge_re,
                &events_re,
                &cfg_re,
                &slint_rt_re,
                &rt_handle_re,
                AskRoute::Vision,
            );
        });
    }
    // ТЗ 2026-07-06 (B) — 🧠 escalate: re-send the SAME screenshot to the smart
    // cloud. No stashing needed: the conversation already holds the base64 image
    // (PttStreamSink seeds it on Done) and `fire_regenerate` re-sends a 1-turn
    // convo verbatim. One-shot: vision tiles have no shared LiveRoute, so later
    // follow-ups return to the Vision endpoint. Gate mirrors `wire_escalate` but
    // on the VISION endpoint: only offer when the answer was local (cloud→cloud
    // is a no-op) AND a cloud bearer exists (no dead affordance), and never on
    // the OCR path (platform-local text has no cloud upgrade; also covers the
    // OCR→VLM fallback, which still enters this fn with mode == Ocr).
    if ep.as_ref().is_some_and(|e| e.is_local)
        && !cfg.read().ai_bearer.trim().is_empty()
        && !matches!(mode, vision::VisionMode::Ocr)
    {
        tile.set_can_escalate(true);
        let weak_es = tile.as_weak();
        let bridge_es = bridge.clone();
        let events_es = events.clone();
        let cfg_es = cfg.clone();
        let slint_rt_es = slint_rt.clone();
        let rt_handle_es = rt_handle.clone();
        tile.on_escalate_clicked(move || {
            // Cloud badge — parity with the text tile's 🧠 (egress stays legible).
            if let Some(t) = weak_es.upgrade() {
                t.set_trigger_label(SharedString::from("cloud (escalated)"));
                t.set_trigger_color(slint::Color::from_rgb_u8(0x38, 0xbd, 0xf8));
            }
            fire_regenerate(
                convo_id,
                weak_es.clone(),
                &bridge_es,
                &events_es,
                &cfg_es,
                &slint_rt_es,
                &rt_handle_es,
                AskRoute::Cloud,
            );
        });
    }
    // V5 — 🎤 voice follow-up (record → STT → ask via the VISION endpoint, so
    // the spoken question stays about the screenshot; escalate above is one-shot
    // and does not re-route this).
    wire_voice_followup(&tile, convo_id, live_route(AskRoute::Vision), cfg);
    wire_copy(&tile, convo_id, bridge);
    wire_speak(&tile, convo_id, bridge);
    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    let weak_for_stream = tile.as_weak();
    let weak_for_title = tile.as_weak();
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);

    // ===== OCR (read-aloud) — platform-local engine, NOT the VLM =====
    // Keep both native engines off the UI thread. Windows still falls through
    // to the configured VLM when Tesseract is absent; Apple Vision is built in.
    if matches!(mode, vision::VisionMode::Ocr) && local_ocr_available() {
        let weak_ocr = weak_for_stream.clone();
        let bridge_ocr = bridge.clone();
        let (bgra, w, h) = (shot.bgra, shot.width, shot.height);
        rt_handle.spawn(async move {
            let res = tokio::task::spawn_blocking(move || run_local_ocr(&bgra, w, h)).await;
            let text = match res {
                Ok(Ok(text)) => Some(text),
                Ok(Err(e)) => {
                    // Detail to the local log only; the tile stays generic.
                    diag!("[overlay-host] OCR failed: {e:#}");
                    None
                }
                Err(e) => {
                    diag!("[overlay-host] OCR task join error: {e}");
                    None
                }
            };
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(text) = text {
                    super::fill_ocr_tile(weak_ocr, convo_id, &bridge_ocr, &text, ui_is_ru);
                } else {
                    super::fill_ocr_error_tile(weak_ocr, ui_is_ru);
                }
            });
        });
        return;
    }

    // Past the OCR branch we need a real endpoint. It is `Some` for every VLM
    // mode; it is `None` only for an OCR request whose engine vanished after the
    // `fire_f8` check (TOCTOU — e.g. an AV quarantine of the user-writable
    // engine dir mid-drag). Show a generic tile instead of calling the VLM with
    // empty credentials.
    let Some(ep) = ep else {
        ptt_tile_error(
            weak_for_title.clone(),
            if ui_is_ru {
                "OCR недоступен."
            } else {
                "OCR is unavailable."
            },
            ui_is_ru,
        );
        return;
    };
    if !ep.accepts_images() {
        ptt_tile_error(
            weak_for_title.clone(),
            if ui_is_ru {
                "Выбранный AI-провайдер не принимает снимки экрана."
            } else {
                "The selected AI provider does not accept screenshots."
            },
            ui_is_ru,
        );
        return;
    }

    // ===== 4. Snapshot what the streaming task needs =====
    let model = ep.model.clone();
    let is_local = ep.is_unmetered();
    // Feature #3/#4 — describe vs translate prompt (translate appends the IPA
    // phonetics suffix when the user enabled it). Computed sync (UI thread) so the
    // async task below just sends the finished string.
    // Per-mode prompt (sync on the UI thread; the async task just sends it):
    //  - Translate → the RU rewrite prompt (+ IPA suffix if enabled).
    //  - TestPractice → built below from `response_language` (answer + short why).
    //  - Describe → the default capture prompt.
    let response_language = cfg.read().response_language.clone();
    let prompt = match mode {
        vision::VisionMode::Translate => vision::translate_prompt(cfg.read().vision_phonetics),
        vision::VisionMode::TestPractice => String::new(),
        vision::VisionMode::Describe => vision::DEFAULT_VISION_PROMPT.to_string(),
        vision::VisionMode::Ocr => vision::OCR_VISION_PROMPT.to_string(),
    };
    // Profile/persona applies ONLY to Describe (v0.10.5). Translate is a pure
    // translation task; TestPractice is a factual answer — a persona would
    // distort both, so they stay profile-free.
    // F8 Describe also folds in approved memory; only the cheap base-context clone
    // happens here — the blocking catalog read is deferred into the stream task
    // below so it never freezes the UI. ТЗ 2026-07-06 (A) — no user question on a
    // screenshot Describe → recency block (None).
    let describe_base = if matches!(mode, vision::VisionMode::Describe) {
        Some(cfg.read().meeting_context.clone())
    } else {
        None
    };
    let (journal_for_loop, health_for_stream) = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        (s.journal.clone(), s.health.clone())
    };
    let rt_for_cost = slint_rt.clone();
    let cost_apply: overlay_backend::runtime::CostApplyFn = Box::new(move |micro| {
        // Local vision is free; cloud vision bills (image tokens under-counted
        // by the text pricing table — acceptable for the MVP).
        let micro = if is_local { 0 } else { micro };
        let mut s = slint_replay::runtime_state::lock(&rt_for_cost);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        overlay_backend::ai::microcents_to_usd(s.session_cost_microcents)
    });
    let bridge_for_task = bridge.clone();
    let events_inner = events.clone();

    // ===== 5. Encode the frame off-thread, then stream the vision answer =====
    rt_handle.spawn(async move {
        let (bgra, w, h) = (shot.bgra, shot.width, shot.height);
        let data_url = match tokio::task::spawn_blocking(move || {
            // Stringify the error inside the closure: Box<dyn Error> isn't Send,
            // but spawn_blocking requires a Send return.
            slint_replay::capture::bgra_to_jpeg_data_url(&bgra, w, h).map_err(|e| e.to_string())
        })
        .await
        {
            Ok(Ok(u)) => u,
            Ok(Err(e)) => {
                // Detail to the local log only; the tile message stays generic
                // for consistency with classify_ai_error (the encode error is
                // local image data, but this is the one streaming path that
                // didn't route through a sanitizer).
                diag!("[overlay-host] F8 encode failed: {e}");
                ptt_tile_error(
                    weak_for_title.clone(),
                    if ui_is_ru {
                        "Не удалось обработать кадр экрана."
                    } else {
                        "Couldn't process the screen capture."
                    },
                    ui_is_ru,
                );
                return;
            }
            Err(e) => {
                diag!("[overlay-host] F8 encode task failed: {e}");
                ptt_tile_error(
                    weak_for_title.clone(),
                    if ui_is_ru {
                        "Сбой кодирования кадра."
                    } else {
                        "Screen capture encoding failed."
                    },
                    ui_is_ru,
                );
                return;
            }
        };
        // Blocking catalog read runs here (off the event loop) — see the
        // describe_base snapshot above (audit C2/G2).
        let vision_context = match describe_base {
            Some(raw) => overlay_backend::memory::context_for_meeting(&raw, None),
            None => String::new(),
        };
        let (messages, usr_full, sys_full) = match mode {
            vision::VisionMode::TestPractice => (
                vision::build_test_practice_request(&data_url, &response_language),
                "Вопрос со скриншота — разбери для самопроверки.".to_string(),
                vision::test_practice_prompt(&response_language),
            ),
            _ => (
                vision::build_vision_request_with_context(&data_url, &prompt, &vision_context),
                prompt,
                vision_context,
            ),
        };
        // Dedicated per-tile sink (convo_id = -1 → no conversation fold) so a
        // vision answer streams independently of any live text answer.
        let sink: Arc<dyn RuntimeEvents> = Arc::new(PttStreamSink::new(
            bridge_for_task.clone(),
            events_inner.clone(),
            weak_for_stream,
            convo_id,
            messages.clone(),
        ));
        // Audit D1 — the SAME purpose must tag the paired AiResponse that
        // ask_stream_loop journals (previously hardcoded "live_ask" there).
        let purpose = "vision_ask";
        if let Some(j) = journal_for_loop.as_ref() {
            j.write(&journal::JournalEvent::AiRequest {
                unix_ms: journal::now_unix_ms(),
                purpose,
                model: &model,
                system_prompt: &sys_full,
                user_prompt: &usr_full,
                attached_screenshot: true,
                input_tokens_est: (usr_full.chars().count() as u64) / 4,
            });
        }
        let t0 = std::time::Instant::now();
        let ai_rx = ai::stream_chat_endpoint(ep, messages, vision::VISION_MAX_TOKENS);
        overlay_backend::runtime::ask_stream_loop(
            sink,
            ai_rx,
            model,
            purpose,
            is_local,
            sys_full,
            usr_full,
            journal_for_loop,
            health_for_stream,
            t0,
            cost_apply,
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::vision_tile_copy;
    use overlay_backend::vision::VisionMode;

    #[test]
    fn deterministic_vision_copy_follows_ui_language() {
        assert_eq!(
            vision_tile_copy(VisionMode::Describe, false).0,
            "Screenshot"
        );
        assert_eq!(
            vision_tile_copy(VisionMode::Ocr, false).3,
            "Recognizing text…"
        );
        assert_eq!(vision_tile_copy(VisionMode::Translate, true).0, "Перевод");
        assert_eq!(
            vision_tile_copy(VisionMode::TestPractice, true).3,
            "Решаю вопрос…"
        );
    }
}
