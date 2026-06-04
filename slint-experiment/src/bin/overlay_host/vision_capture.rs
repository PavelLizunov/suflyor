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
    set_always_on_top, toggle_tile_maximize, vision, wire_copy, wire_tile_drag,
    wire_voice_followup, Arc, AskRoute, CaptureOverlay, ComponentHandle, MarkdownBlock, ModelRc,
    Ordering, OverlayBarBridge, OverlayBarWindow, PttStreamSink, Rc, RefCell, RuntimeEvents,
    SharedSlintRuntime, SharedString, TileWindow, TileWindows, VecModel, CONVO_SEQ,
    TILE_DISPLAY_SEQ,
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
    translate: bool,
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
    let Some(ep) = cfg.read().vision_endpoint() else {
        diag!("[overlay-host] F8: vision provider is 'off' (Settings -> Vision) — skipping");
        return;
    };

    // Freeze the WHOLE virtual desktop (ALL monitors) so the user can select on
    // either screen. The earlier "shrunk" bug was NOT DPI — it was the geometry
    // never applying (grab_hwnd failed right after show()); fixed below by
    // sizing via Slint's own set_size/set_position. The origin can be NEGATIVE
    // (the portrait secondary sits at x=-1200).
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
        "[overlay-host] F8 capture virtual=({vx},{vy}) {fw}x{fh} monitors={:?}",
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
                             // Feature #3 — seed the capture overlay's mode (Shift+F8 → translate). The
                             // user can still flip the on-overlay Describe/Translate toggle before drag.
    win.set_translate_mode(translate);
    // PHYSICAL units = monitor pixels (1:1 with the captured frame). Geometry is
    // set on the still-hidden window, then show() lands it there (Slint's
    // set_size/set_position apply reliably).
    win.window()
        .set_size(slint::PhysicalSize::new(fw.max(1), fh.max(1)));
    win.window()
        .set_position(slint::PhysicalPosition::new(vx, vy));
    let _ = win.show();
    let scale = win.window().scale_factor().max(0.1);
    diag!("[overlay-host] F8 overlay {fw}x{fh} at ({vx},{vy}) scale={scale}");

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
            let translate = if let Some(w) = weak_self.upgrade() {
                let t = w.get_translate_mode();
                w.set_shown(false);
                let _ = w.hide();
                t
            } else {
                false
            };
            // logical px × scale = image px.
            let to_px = |v: f32| (v * scale).round().max(0.0) as u32;
            let (px1, py1) = (to_px(x1), to_px(y1));
            let (px2, py2) = (to_px(x2), to_px(y2));
            let cropped = slint_replay::capture::crop_bgra(
                &frozen_c,
                px1,
                py1,
                px2.saturating_sub(px1),
                py2.saturating_sub(py1),
            );
            launch_vision_for_bgra(
                cropped,
                ep_c.clone(),
                translate,
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
    // The persistent HWND exists, so grab_hwnd works synchronously here. WDA
    // stealth was set at pre-create and PERSISTS across hide/show — but winit
    // re-applies the window's ex-style on show(), dropping WS_EX_TOOLWINDOW, so
    // the taskbar button reappears. Re-apply it now: synchronous + lands before
    // the shell creates the button = flash-free. (The overlay is WDA-hidden from
    // any screen-share regardless.)
    if let Ok(hwnd) = grab_hwnd(win.window()) {
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        let _ = set_always_on_top(hwnd, true);
        slint_replay::win32::focus_window(hwnd);
    }
}

/// Spawn a vision tile for a captured BGRA frame and stream the answer into it
/// via the SEPARATE vision endpoint. Shared entry for the F8 region capture.
/// No follow-up (a follow-up would route to the text endpoint), so the tile
/// uses `convo_id = -1`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn launch_vision_for_bgra(
    shot: slint_replay::capture::CapturedBgra,
    ep: overlay_backend::config::AiEndpoint,
    translate: bool,
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
    tile.set_tile_title(SharedString::from(if translate {
        "🌐 Перевод"
    } else {
        "📷 Скриншот"
    }));
    tile.set_source_label(SharedString::from(if translate {
        "vision · перевод…"
    } else {
        "vision · анализ…"
    }));
    tile.set_trigger_label(SharedString::from(if translate {
        "🌐 Shift+F8 перевод"
    } else {
        "📷 F8 vision"
    }));
    tile.set_trigger_color(slint::Color::from_rgb_u8(0x22, 0xd3, 0xee)); // cyan
    tile.set_convo_id(convo_id);
    tile.set_followup_busy(true);
    wire_tile_drag(&tile);
    tile.set_blocks(ModelRc::new(VecModel::from(vec![MarkdownBlock {
        kind: markdown::kind::PARAGRAPH,
        text: SharedString::from(if translate {
            "⏳ Перевожу…"
        } else {
            "⏳ Распознаю экран…"
        }),
        lang: SharedString::from(""),
    }])));
    let weak_close = tile.as_weak();
    let vec_for_close = tiles.clone();
    let weak_overlay_close = weak_overlay.clone();
    let bridge_for_close = bridge.clone();
    tile.on_close_clicked(move || {
        if let Some(t) = weak_close.upgrade() {
            // FIX #8 — prune this tile's conversation (no-op if none).
            bridge_for_close.drop_conversation(t.get_convo_id());
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
    // V5 — 🎤 voice follow-up (record → STT → ask via the VISION endpoint, so
    // the spoken question stays about the screenshot). Vision tiles aren't
    // escalatable (wire_escalate isn't called), so this route stays Vision.
    wire_voice_followup(&tile, convo_id, live_route(AskRoute::Vision), cfg);
    wire_copy(&tile, convo_id, bridge);
    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    let weak_for_stream = tile.as_weak();
    let weak_for_title = tile.as_weak();
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);

    // ===== 4. Snapshot what the streaming task needs =====
    let model = ep.model.clone();
    let is_local = ep.is_local;
    // Feature #3/#4 — describe vs translate prompt (translate appends the IPA
    // phonetics suffix when the user enabled it). Computed sync (UI thread) so the
    // async task below just sends the finished string.
    let prompt = if translate {
        vision::translate_prompt(cfg.read().vision_phonetics)
    } else {
        vision::DEFAULT_VISION_PROMPT.to_string()
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
        (s.session_cost_microcents as f64) / 100_000_000.0
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
                ptt_tile_error(weak_for_title.clone(), "Не удалось обработать кадр экрана.");
                return;
            }
            Err(e) => {
                diag!("[overlay-host] F8 encode task failed: {e}");
                ptt_tile_error(weak_for_title.clone(), "Сбой кодирования кадра.");
                return;
            }
        };
        let messages = vision::build_vision_request(&data_url, &prompt);
        let usr_full = prompt;
        let sys_full = String::new();
        // Dedicated per-tile sink (convo_id = -1 → no conversation fold) so a
        // vision answer streams independently of any live text answer.
        let sink: Arc<dyn RuntimeEvents> = Arc::new(PttStreamSink::new(
            bridge_for_task.clone(),
            events_inner.clone(),
            weak_for_stream,
            convo_id,
            messages.clone(),
        ));
        if let Some(j) = journal_for_loop.as_ref() {
            j.write(&journal::JournalEvent::AiRequest {
                unix_ms: journal::now_unix_ms(),
                purpose: "vision_ask",
                model: &model,
                system_prompt: &sys_full,
                user_prompt: &usr_full,
                attached_screenshot: true,
                input_tokens_est: (usr_full.chars().count() as u64) / 4,
            });
        }
        let t0 = std::time::Instant::now();
        let ai_rx = ai::stream_chat(
            ep.base_url,
            ep.bearer,
            model.clone(),
            messages,
            vision::VISION_MAX_TOKENS,
        );
        overlay_backend::runtime::ask_stream_loop(
            sink,
            ai_rx,
            model,
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
