use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use overlay_backend::ai;
use overlay_backend::events::RuntimeEvents;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, VecModel};
use slint_replay::runtime_state::SharedSlintRuntime;
use slint_replay::win32::grab_hwnd;
use crate::ui::{OverlayBarWindow, TileWindow};

use super::{
    apply_tile_hwnd_with_monitor, fire_followup_ask, live_route, present_tile_window,
    refresh_open_tiles, speak_explicit, stop_if_speaking, to_md_blocks, toggle_tile_maximize,
    user_turn_markdown, wire_copy, wire_speak, wire_tile_drag, wire_voice_followup, AskRoute,
    ConvoState, OverlayBarBridge, TILE_DISPLAY_SEQ, CONVO_SEQ, TileWindows,
};

thread_local! {
    /// One bounded strong handle: closing a read-aloud tile hides it instead of
    /// destroying the only copy of a long browser selection. Replacing the slot
    /// drops the previous conversation, so this cannot grow without bound.
    pub(super) static LAST_CLOSED_READ_TILE: RefCell<Option<TileWindow>> = const { RefCell::new(None) };
}

pub(super) fn restore_text_clipboard(saved: &Option<String>) {
    // ponytail: SA1 preserves only UTF-8 text; snapshot every pasteboard item
    // and format if non-text clipboard parity becomes a product requirement.
    match saved {
        Some(text) => slint_replay::win32::clipboard_write_text(text),
        None => slint_replay::win32::clipboard_clear(),
    }
}

/// Spawn a conversational tile that DISPLAYS `text` and immediately reads it
/// aloud — no AI/vision call. Used by the Shift+Alt+1 "read selection" path so
/// the user SEES what's being read and gets the 🔊/⏯/📋/✕ controls. The
/// conversation map is seeded directly (there is no `AiEvent::Done` to seed it),
/// so `wire_speak`/`convo_speak_text` find the text.
pub(super) fn spawn_text_tile(
    text: &str,
    title: &str,
    trigger: &str,
    bridge: &Arc<OverlayBarBridge>,
    runtime: (
        &Arc<dyn RuntimeEvents>,
        &overlay_backend::config::SharedConfig,
        &SharedSlintRuntime,
        &tokio::runtime::Handle,
    ),
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    let (events, cfg, slint_rt, rt_handle) = runtime;
    let tile = match TileWindow::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[overlay-host] spawn_text_tile: TileWindow::new failed: {e}");
            return;
        }
    };
    let seq = TILE_DISPLAY_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let convo_id = CONVO_SEQ.fetch_add(1, Ordering::Relaxed) as i32;
    tile.set_sequence(seq as i32);
    tile.set_tile_title(SharedString::from(title));
    tile.set_source_label(SharedString::from("read-aloud"));
    tile.set_trigger_label(SharedString::from(trigger));
    tile.set_trigger_color(slint::Color::from_rgb_u8(0x22, 0xd3, 0xee));
    tile.set_convo_id(convo_id);
    let rendered = user_turn_markdown(text);
    tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&rendered))));
    wire_tile_drag(&tile);

    // Seed the conversation directly — there's no AI call to do it, and
    // wire_speak/wire_copy read the text from here (NOT from tile.blocks).
    bridge.store_conversation(
        convo_id,
        ConvoState {
            messages: vec![ai::ChatMessage {
                role: "user".to_string(),
                content: ai::MessageContent::Text(text.to_string()),
            }],
            rendered,
        },
    );

    {
        let weak_close = tile.as_weak();
        let vec_for_close = tiles.clone();
        let weak_overlay_close = weak_overlay.clone();
        let bridge_for_close = bridge.clone();
        tile.on_close_clicked(move || {
            if let Some(t) = weak_close.upgrade() {
                stop_if_speaking(t.get_convo_id());
                let close_hwnd = grab_hwnd(t.window()).ok();
                let _ = t.hide();
                slint_replay::win32::force_hide(t.window());
                if let Some(target) = close_hwnd {
                    vec_for_close
                        .borrow_mut()
                        .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
                    refresh_open_tiles(&weak_overlay_close, &vec_for_close);
                }
                let replaced = LAST_CLOSED_READ_TILE.with(|slot| slot.borrow_mut().replace(t));
                if let Some(old) = replaced {
                    bridge_for_close.drop_conversation(old.get_convo_id());
                }
                if let Some(o) = weak_overlay_close.upgrade() {
                    o.set_can_restore_tile(true);
                }
            }
        });
    }
    {
        let weak_pin = tile.as_weak();
        tile.on_pin_clicked(move || {
            if let Some(t) = weak_pin.upgrade() {
                let new = !t.get_pinned();
                t.set_pinned(new);
            }
        });
    }
    {
        let weak_max = tile.as_weak();
        tile.on_maximize_clicked(move || {
            if let Some(t) = weak_max.upgrade() {
                if let Ok(hwnd) = grab_hwnd(t.window()) {
                    toggle_tile_maximize(hwnd, &t);
                }
            }
        });
    }
    // A selected-text tile is a real Text conversation: the selected passage
    // becomes reference context and the next typed/voice turn asks about it.
    let live = live_route(AskRoute::Text);
    {
        let weak_fu = tile.as_weak();
        let bridge_fu = bridge.clone();
        let events_fu = events.clone();
        let cfg_fu = cfg.clone();
        let slint_rt_fu = slint_rt.clone();
        let rt_handle_fu = rt_handle.clone();
        let live_fu = live.clone();
        tile.on_followup_submitted(move |q| {
            fire_followup_ask(
                (convo_id, q.to_string()),
                weak_fu.clone(),
                &bridge_fu,
                &events_fu,
                &cfg_fu,
                &slint_rt_fu,
                &rt_handle_fu,
                live_fu.get(),
            );
        });
    }
    wire_voice_followup(&tile, convo_id, live, cfg);
    wire_copy(&tile, convo_id, bridge);
    wire_speak(&tile, convo_id, bridge);
    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);

    // Auto-start the read (mirror wire_speak's click handler). Mark the tile as
    // speaking ONLY when playback is accepted — a missing sidecar/voice must not
    // show as speaking nor falsely suppress STT (F2).
    speak_explicit(text, convo_id);
}

pub(super) fn after_read_aloud_hotkey_release(attempts_left: u8, action: Rc<dyn Fn()>) {
    if slint_replay::win32::read_aloud_hotkey_modifiers_released() {
        action();
    } else if attempts_left > 0 {
        Timer::single_shot(std::time::Duration::from_millis(25), move || {
            after_read_aloud_hotkey_release(attempts_left - 1, action);
        });
    } else {
        diag!("[overlay-host] sa1: modifier release timed out");
    }
}

#[must_use]
fn ocr_source_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "OCR · Apple Vision"
    } else {
        "OCR · Tesseract"
    }
}

/// Finish an already-spawned OCR placeholder tile with recognized text and
/// read it aloud. The platform-local engine runs off-thread, then marshals the
/// result here on the Slint UI thread. Mirrors the tail of `spawn_text_tile`
/// (seed conversation -> auto-read), but the tile already exists.
pub(super) fn fill_ocr_tile(
    weak: slint::Weak<TileWindow>,
    convo_id: i32,
    bridge: &Arc<OverlayBarBridge>,
    text: &str,
    ui_is_ru: bool,
) {
    let Some(tile) = weak.upgrade() else {
        return;
    };
    tile.set_followup_busy(false);
    // OCR output isn't regeneratable (no model call to vary) — hide 🔄.
    tile.set_can_regenerate(false);
    let trimmed = text.trim();
    let source = ocr_source_label();
    if trimmed.is_empty() {
        let empty = if ui_is_ru {
            "*(текст не распознан)*"
        } else {
            "*(no text recognized)*"
        };
        tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(empty))));
        tile.set_source_label(SharedString::from(source));
        // Nothing to read or copy — don't present no-op 🔊/📋 controls (the
        // conversation is never seeded on this path, so they'd be dead anyway).
        tile.set_can_speak(false);
        tile.set_can_copy(false);
        return;
    }
    tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(trimmed))));
    tile.set_source_label(SharedString::from(source));
    // Seed the conversation so wire_speak / wire_copy read THIS text (they read
    // from the conversation store, NOT tile.blocks).
    bridge.store_conversation(
        convo_id,
        ConvoState {
            messages: vec![ai::ChatMessage {
                role: "assistant".to_string(),
                content: ai::MessageContent::Text(trimmed.to_string()),
            }],
            rendered: trimmed.to_string(),
        },
    );
    // Auto-read (mirror spawn_text_tile's tail). Mark the tile as speaking ONLY
    // when playback is accepted — a missing sidecar/voice must not show as
    // speaking nor falsely suppress STT (F2).
    speak_explicit(trimmed, convo_id);
}

/// Finish an existing OCR tile after the platform-local engine fails. Called
/// on the Slint UI thread; diagnostic detail stays in the worker-side log.
pub(super) fn fill_ocr_error_tile(weak: slint::Weak<TileWindow>, ui_is_ru: bool) {
    let Some(tile) = weak.upgrade() else {
        return;
    };
    let message = if ui_is_ru {
        "Не удалось распознать текст."
    } else {
        "Text recognition failed."
    };
    tile.set_followup_busy(false);
    tile.set_can_regenerate(false);
    tile.set_can_speak(false);
    tile.set_can_copy(false);
    tile.set_source_label(SharedString::from(ocr_source_label()));
    tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(message))));
}
