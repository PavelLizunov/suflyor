//! AI-ask INITIAL ENTRYPOINTS — the F3 / F6 / F9 hotkey actions that START a
//! fresh ask. Originally carved out of `tile_controller.rs` (Wave 2 of the
//! `tile_controller` split) and since slimmed by the `tile_ask` split (see
//! `docs/overlay-host-modular-structure-current.md`): the route model, the cost
//! helpers, the PTT flow, and the follow-up / escalate flow each moved to a
//! sibling module, leaving this file the INITIAL-ask side only.
//!
//! This module owns:
//!
//! - `fire_f3_reask` — F3 reask of the last Q&A;
//! - `fire_f6_manual_spawn` — F6 manual tile from the recent transcript;
//! - `fire_f9_ask` — F9 / Shift+F9 / typed "✏ Написать" ask (the primary ask).
//!
//! Sibling split modules it leans on (reached through the crate root):
//!
//! - `tile_routes` — `AskRoute` (Text / Vision / Cloud) + its `impl`
//!   (`endpoint` / `max_tokens` / `attaches_screenshot`), the per-tile mutable
//!   `LiveRoute` (sticky-cloud after 🧠 / Shift+F9) + `live_route`;
//! - `tile_followup` — `fire_followup_ask` (in-tile continue-dialog),
//!   `fire_regenerate` (🔄 / 🧠), `wire_escalate` (🧠 cloud), `wire_voice_followup`
//!   (🎤 + `VFU_TX`), and the send-time `reframe_for_send` (the escalate→
//!   follow-up root-cause fix, commit `6ffbc40`);
//! - `tile_ptt` — `fire_ptt_ask` (push-to-talk), `spawn_ptt_watchdog` (the 30 s
//!   lost-pointer-up backstop), `ptt_tile_error`;
//! - `tile_cost` — `warn_if_over_cost_cap`, `cost_cap_reason`,
//!   `select_recent_labeled`.
//!
//! What STAYS in `tile_controller.rs` (the STREAM-WRITE side — which messages
//! get STORED — reached through the crate root): the `OverlayBarBridge` (the
//! SOLE `handle_ai_event` conversation-writer, plus `store_conversation` /
//! `drop_conversation` and the in-flight counters); `StreamingTile` /
//! `ConvoState` / `SpawnTileRequest`; `install_streaming_tile` with
//! `GenGatedEvents` / `gated_events` (the wrong-tile-race generation guard); and
//! `PttStreamSink` / `PttSinkState` (the per-tile PTT stream SINK).
//!
//! SECURITY (unchanged): AI error tiles route the raw error chain through
//! `classify_ai_error` → a generic message, so the local AI server's LAN IP:port
//! can never leak into a screen-shared tile.
//!
//! Imports: explicit `use super::{…}` (narrowed from the original extraction
//! glob).
use super::{
    ai, apply_tile_hwnd_with_monitor, cost_cap_reason, fire_followup_ask, fire_regenerate,
    gated_events, grab_hwnd, install_streaming_tile, journal, live_route, markdown,
    present_tile_window, refresh_open_tiles, select_recent_labeled, toggle_tile_maximize,
    wire_copy, wire_escalate, wire_speak, wire_tile_drag, wire_voice_followup, Arc, AskRoute,
    ComponentHandle, MarkdownBlock, ModelRc, Ordering, OverlayBarBridge, OverlayBarWindow,
    RuntimeEvents, SharedSlintRuntime, SharedString, StreamingTile, TileWindow, TileWindows,
    VecModel, AI_STREAM_MAX_TOKENS, CONVO_SEQ, TILE_DISPLAY_SEQ,
};

/// Phase E3 slice 3 — F3 reask handler.
///
/// Snapshots SlintRuntime into ReaskInputs, spawns the ported
/// `reask_last` async fn, applies the outcome writeback
/// (session_cost plus last_qa) under the rt lock, then emits
/// `cost:update` so the bar updates. Wire-for-wire equivalent of
/// src-tauri's reask_last shim but using SlintEvents and
/// SharedSlintRuntime instead of TauriEvents and SharedRuntime.
pub(crate) fn fire_f3_reask(
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
) {
    let inputs = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        overlay_backend::runtime::ReaskInputs {
            last_question: s.last_question.clone(),
            last_answer: s.last_answer.clone(),
            recent_transcript_iconized: s
                .transcript
                .iter()
                .rev()
                .take(10)
                .rev()
                .map(|l| {
                    let icon = match l.source {
                        overlay_backend::audio::AudioSource::System => "sys",
                        overlay_backend::audio::AudioSource::Mic => "mic",
                    };
                    format!("{icon} {}", l.text)
                })
                .collect(),
            journal: s.journal.clone(),
            health: s.health.clone(),
        }
    };
    let events_c = events.clone();
    let cfg_c = cfg.clone();
    let rt_c = slint_rt.clone();
    rt_handle.spawn(async move {
        let outcome = overlay_backend::runtime::reask_last(events_c.clone(), cfg_c, inputs).await;
        if let Some(out) = outcome {
            let total = {
                let mut s = slint_replay::runtime_state::lock(&rt_c);
                s.session_cost_microcents = s
                    .session_cost_microcents
                    .saturating_add(out.cost_microcents_delta);
                s.last_question = Some(out.display_question);
                s.last_answer = Some(out.answer_trimmed);
                (s.session_cost_microcents as f64) / 100_000_000.0
            };
            events_c.emit("cost:update", serde_json::json!({ "session_usd": total }));
        }
    });
}

/// Phase E3 slice 3 — F6 manual spawn handler.
///
/// Snapshots rt into ManualSpawnInputs (recent 8 labeled lines +
/// last line + cost cap), spawns the ported `manual_spawn_tile`
/// async fn, applies outcome writeback. Same shape as F3 reask.
pub(crate) fn fire_f6_manual_spawn(
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
) {
    let inputs = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        let recent = select_recent_labeled(&s.transcript, 8);
        let last_line = s.transcript.back().cloned();
        let cap_usd = cfg.read().max_session_cost_usd;
        let cost_cap = cost_cap_reason(cap_usd, s.session_cost_microcents);
        overlay_backend::runtime::ManualSpawnInputs {
            recent_transcript_labeled: recent,
            last_line,
            cost_cap_reason: cost_cap,
            journal: s.journal.clone(),
            health: s.health.clone(),
        }
    };
    let events_c = events.clone();
    let cfg_c = cfg.clone();
    let rt_c = slint_rt.clone();
    rt_handle.spawn(async move {
        let outcome =
            overlay_backend::runtime::manual_spawn_tile(events_c.clone(), cfg_c, inputs).await;
        if let Some(out) = outcome {
            let total = {
                let mut s = slint_replay::runtime_state::lock(&rt_c);
                s.session_cost_microcents = s
                    .session_cost_microcents
                    .saturating_add(out.cost_microcents_delta);
                s.last_question = Some(out.display_question);
                s.last_answer = Some(out.answer_trimmed);
                (s.session_cost_microcents as f64) / 100_000_000.0
            };
            events_c.emit("cost:update", serde_json::json!({ "session_usd": total }));
        }
    });
}

/// Phase E3 slice 2 — F9 ask handler.
///
/// Runs on the Slint UI thread (called from the hotkey poll Timer
/// closure). Synchronously creates a placeholder TileWindow, registers
/// its Weak in the bridge's `current_streaming` slot so subsequent
/// `ai:event` payloads from `ask_stream_loop` land in this tile, then
/// spawns a tokio task that:
///   1. Snapshots cfg + transcript + screenshot under brief rt locks.
///   2. Builds messages via `ai::build_request` (same prompt builder
///      the src-tauri ask shim uses).
///   3. Writes `JournalEvent::AiRequest`.
///   4. Aborts any in-flight `ai_task` (rapid-F9 protection — matches
///      src-tauri behavior).
///   5. Starts `ai::stream_chat` → gets the receiver.
///   6. Builds the `cost_apply` closure (locks rt, accumulates
///      session_cost, returns new USD total).
///   7. Spawns `overlay_backend::runtime::ask_stream_loop` which drives
///      the stream + emits per-Delta `ai:event` payloads back through
///      `SlintEvents` → `OverlayBarBridge::handle_ai_event` → tile
///      body re-renders live.
///
/// Wire-parity invariants matched to src-tauri:
/// - F9 always proceeds even when over budget (cost:cap-hit is a warn
///   chip, not a gate).
/// - last_screenshot is CONSUMED (taken) on F9 so it ships once.
/// - In-flight ai_task aborted BEFORE the new spawn.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_f9_ask(
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
    // V0.8.0 (Поток D) — Text for a normal F9, Cloud for a Shift+F9 one-shot
    // escalation to the smart cloud model. (Vision isn't used here — F8 has its
    // own path.)
    route: AskRoute,
    // V0.8.3 — Some(text) = a typed "✏ Написать" question: answer it DIRECTLY
    // (no live-transcript / screenshot context). None = a normal F9/Shift+F9 ask
    // built from the transcript. Lets the text-input window reuse this whole
    // tile-create + stream + cost + journal + follow-up pipeline.
    typed_question: Option<String>,
) {
    let is_text = typed_question.is_some();
    // ===== 1. Sync placeholder tile creation =====
    let tile = match TileWindow::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[overlay-host] F9: TileWindow::new failed: {e}");
            return;
        }
    };
    // Phase E6 fix — share the same display-sequence counter so F9
    // tiles get a unique #N label in line with auto-tiles + manual_
    // spawn tiles (previously stuck at #0).
    let seq = TILE_DISPLAY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    tile.set_sequence(seq as i32);
    tile.set_tile_title(SharedString::from(if is_text {
        "text · live"
    } else {
        "F9 ask · live"
    }));
    tile.set_source_label(SharedString::from("ai · asking…"));
    // Phase E6 v12 — purple trigger badge for manual F9 ask so user
    // sees which tile came from a hotkey vs auto-detector. V0.8.0 (Поток D):
    // a CLOUD-escalated ask (Shift+F9) gets a distinct 🧠 cloud badge so the
    // user sees THIS answer came from the cloud model (egress is visible).
    // V0.8.3 — a typed "✏ Написать" ask gets its own green badge.
    if is_text {
        tile.set_trigger_label(SharedString::from("text ask"));
        tile.set_trigger_color(slint::Color::from_rgb_u8(0x34, 0xd3, 0x99));
    } else if route == AskRoute::Cloud {
        tile.set_trigger_label(SharedString::from("cloud (Shift+F9)"));
        tile.set_trigger_color(slint::Color::from_rgb_u8(0x38, 0xbd, 0xf8));
    } else {
        tile.set_trigger_label(SharedString::from("F9 manual ask"));
        tile.set_trigger_color(slint::Color::from_rgb_u8(0xa7, 0x8b, 0xfa));
    }
    // Phase E6 v45 — this tile carries a conversation, so it shows the
    // continue-dialog input. busy=true until the first answer completes.
    let convo_id = CONVO_SEQ.fetch_add(1, Ordering::Relaxed) as i32;
    tile.set_convo_id(convo_id);
    tile.set_followup_busy(true);
    wire_tile_drag(&tile);
    let placeholder = vec![MarkdownBlock {
        kind: markdown::kind::PARAGRAPH,
        text: SharedString::from("⏳ Asking AI…"),
        lang: SharedString::from(""),
    }];
    tile.set_blocks(ModelRc::new(VecModel::from(placeholder)));
    let weak_close = tile.as_weak();
    let vec_for_close = tiles.clone();
    let weak_overlay_close = weak_overlay.clone();
    let bridge_for_close = bridge.clone();
    tile.on_close_clicked(move || {
        eprintln!("[overlay-host] tile (F9) close_clicked fired");
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
        eprintln!("[overlay-host] tile (F9) pin_clicked fired");
        if let Some(t) = weak_pin.upgrade() {
            let new = !t.get_pinned();
            t.set_pinned(new);
        }
    });
    let weak_max = tile.as_weak();
    tile.on_maximize_clicked(move || {
        eprintln!("[overlay-host] tile (F9) maximize_clicked fired");
        if let Some(t) = weak_max.upgrade() {
            let Ok(hwnd) = grab_hwnd(t.window()) else {
                return;
            };
            toggle_tile_maximize(hwnd, &t);
        }
    });
    // V0.8.1 — shared per-tile live route (Text/Cloud). Shift+F9 seeds Cloud;
    // 🧠-escalate flips it to Cloud; the continuation surfaces below read it at
    // click time so the conversation stays sticky-cloud after one escalation.
    let live = live_route(route);
    // Phase E6 v45 — continue-dialog: a follow-up question reuses this
    // tile's conversation + streams the reply below the thread.
    {
        let weak_fu = tile.as_weak();
        let bridge_fu = bridge.clone();
        let events_fu = events.clone();
        let cfg_fu = cfg.clone();
        let slint_rt_fu = slint_rt.clone();
        let rt_handle_fu = rt_handle.clone();
        let live_fu = live.clone();
        tile.on_followup_submitted(move |q| {
            // V0.8.1 — read the LIVE route at click time (Cloud after 🧠 or
            // Shift+F9, else Text).
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
    // V5 — 🔄 regenerate, available on every answer tile (re-runs via the text
    // endpoint for F9/PTT tiles, vision endpoint for F8 tiles).
    tile.set_can_regenerate(true);
    {
        let weak_re = tile.as_weak();
        let bridge_re = bridge.clone();
        let events_re = events.clone();
        let cfg_re = cfg.clone();
        let slint_rt_re = slint_rt.clone();
        let rt_handle_re = rt_handle.clone();
        let live_re = live.clone();
        tile.on_regenerate_clicked(move || {
            fire_regenerate(
                convo_id,
                weak_re.clone(),
                &bridge_re,
                &events_re,
                &cfg_re,
                &slint_rt_re,
                &rt_handle_re,
                live_re.get(),
            );
        });
    }
    // V5 — 🎤 voice follow-up. Reads the live route (sticky-cloud aware).
    wire_voice_followup(&tile, convo_id, live.clone(), cfg);
    wire_copy(&tile, convo_id, bridge);
    wire_speak(&tile, convo_id, bridge);
    // V0.8.0 (Поток D) — 🧠 escalate to cloud (only shown if the answer is local).
    // V0.8.1 — also flips `live` to Cloud so the rest of the dialog stays cloud.
    wire_escalate(
        &tile, convo_id, &live, bridge, events, cfg, slint_rt, rt_handle,
    );
    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    let weak_for_stream = tile.as_weak();
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);

    // ===== 2. Register the tile in the bridge's streaming slot =====
    // request_messages is filled once `messages` is built below (before
    // the stream task spawns, so no event can fold an empty history).
    let generation = install_streaming_tile(
        bridge,
        StreamingTile {
            weak: weak_for_stream,
            accumulated: String::new(),
            prefix: String::new(),
            convo_id,
            request_messages: Vec::new(),
        },
    );

    // ===== 3. Snapshot cfg + cost-cap + transcript + screenshot =====
    let (
        base_url,
        bearer,
        model,
        meeting_context,
        response_language,
        cap_usd,
        is_local,
        local_vision,
    ) = {
        let c = cfg.read();
        // V0.8.0 (Поток D) — route picks the endpoint: normal F9 = Text (local
        // or cloud per provider), Shift+F9 = Cloud (smart model, one-shot).
        let ep = route.endpoint(&c);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            c.meeting_context.clone(),
            c.response_language.clone(),
            c.max_session_cost_usd,
            ep.is_local,
            c.ai_local_vision,
        )
    };
    // Audit (prompt-context): the F9 / typed-ask LLM prompt must carry the user's
    // APPROVED memory + profile, the same as the auto/F6/re-ask paths —
    // context_for_meeting folds the bounded memory block in (no-op when nothing
    // is approved, so the request is byte-identical for users without memory).
    let meeting_context = overlay_backend::memory::context_for_meeting(&meeting_context);
    let current_micro = slint_replay::runtime_state::lock(slint_rt).session_cost_microcents;
    if current_micro > 0 && cap_usd > 0.0 {
        let usd = (current_micro as f64) / 100_000_000.0;
        if usd >= cap_usd {
            events.emit(
                "cost:cap-hit",
                serde_json::json!({
                    "reason": format!(
                        "over budget: ${usd:.4} spent ≥ ${cap_usd:.2} (Settings → Max cost per session)"
                    ),
                    "source": "live_ask",
                    "blocking": false,
                }),
            );
        }
    }

    let (transcript_lines, screenshot) = {
        let mut s = slint_replay::runtime_state::lock(slint_rt);
        let lines: Vec<String> = s
            .transcript
            .iter()
            .map(|l| format!("[{:?}] {}", l.source, l.text))
            .collect();
        let shot = s.last_screenshot.take();
        (lines, shot)
    };
    // A local TEXT model can't accept an image_url part — drop the
    // screenshot unless the user flagged the local model as vision-capable.
    let screenshot = if is_local && !local_vision {
        None
    } else {
        screenshot
    };

    // V0.8.3 — a typed "✏ Написать" question is answered DIRECTLY (the typed
    // text IS the question; no live-transcript / screenshot noise). A normal F9
    // ask is built from the live transcript as before.
    let messages = match typed_question.as_deref() {
        Some(q) => ai::build_request(&meeting_context, &response_language, &[], None, Some(q)),
        None => ai::build_request(
            &meeting_context,
            &response_language,
            &transcript_lines,
            screenshot.as_deref(),
            None,
        ),
    };

    // Phase E6 v45 — record the sent messages in the streaming slot so
    // AiEvent::Done can fold this turn into the tile's conversation for
    // follow-ups. Done before the stream task spawns → no race.
    {
        let mut slot = match bridge.current_streaming.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(s) = slot.as_mut() {
            s.request_messages = messages.clone();
        }
    }

    let (journal_for_request, journal_for_loop, health_for_stream) = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        let j = s.journal.clone();
        (j.clone(), j, s.health.clone())
    };
    let sys_full = messages
        .first()
        .and_then(|m| match &m.content {
            ai::MessageContent::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let usr_full = match messages.get(1).map(|m| &m.content) {
        Some(ai::MessageContent::Text(t)) => t.clone(),
        Some(ai::MessageContent::Parts(parts)) => parts
            .iter()
            .find_map(|p| match p {
                ai::ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        None => String::new(),
    };
    let input_tokens_est = ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4;
    if let Some(j) = journal_for_request.as_ref() {
        j.write(&journal::JournalEvent::AiRequest {
            unix_ms: journal::now_unix_ms(),
            purpose: if is_text { "text_ask" } else { "live_ask" },
            model: &model,
            system_prompt: &sys_full,
            user_prompt: &usr_full,
            attached_screenshot: screenshot.is_some(),
            input_tokens_est,
        });
    }

    // ===== 4. Cancel in-flight + build cost_apply closure =====
    {
        let mut s = slint_replay::runtime_state::lock(slint_rt);
        if let Some(h) = s.ai_task.take() {
            h.abort();
        }
    }

    let rt_for_cost = slint_rt.clone();
    let cost_apply: overlay_backend::runtime::CostApplyFn = Box::new(move |micro| {
        // Local inference is free — don't bill it (and don't trip the cap).
        let micro = if is_local { 0 } else { micro };
        let mut s = slint_replay::runtime_state::lock(&rt_for_cost);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    });

    let t0 = std::time::Instant::now();
    let events_for_task = gated_events(bridge, events.clone(), generation);
    // CRITICAL: `ai::stream_chat` internally calls `tokio::spawn`,
    // which panics with "there is no reactor running" when called
    // from a non-tokio thread (Slint UI / hotkey poll Timer closure).
    // The same trap is documented in src-tauri/src/runtime.rs:1804
    // ("must be tauri::async_runtime::spawn, NOT tokio::spawn").
    // We move stream_chat INSIDE the rt_handle.spawn future so the
    // tokio runtime context exists when it runs.
    let task = rt_handle.spawn(async move {
        let ai_rx = ai::stream_chat(
            base_url,
            bearer,
            model.clone(),
            messages,
            AI_STREAM_MAX_TOKENS,
        );
        overlay_backend::runtime::ask_stream_loop(
            events_for_task,
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
    slint_replay::runtime_state::lock(slint_rt).ai_task = Some(task);
}
