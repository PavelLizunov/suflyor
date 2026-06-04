//! Push-to-talk (PTT) ask flow carved out of `tile_ask.rs` (the `tile_ask`
//! split — see `docs/overlay-host-modular-structure-current.md`). The 30s
//! lost-pointer-up watchdog (`spawn_ptt_watchdog`), the generic PTT tile-error
//! helper (`ptt_tile_error`), and the per-PTT ask entrypoint (`fire_ptt_ask`)
//! that records + transcribes off the UI thread then streams the answer into
//! its own tile via the independent `PttStreamSink`. Reached from `main`'s
//! hotkey dispatch through the `use tile_ptt::*;` re-export.
//!
//! NOTE (§7): the crate-root symbols this module uses are imported below.
use super::{
    ai, apply_tile_hwnd_with_monitor, audio, classify_ai_error, fire_followup_ask, fire_regenerate,
    grab_hwnd, journal, live_route, markdown, present_tile_window, refresh_open_tiles, stt,
    toggle_tile_maximize, wire_copy, wire_escalate, wire_tile_drag, wire_voice_followup, Arc,
    AskRoute, AtomicBool, ComponentHandle, Duration, MarkdownBlock, ModelRc, Ordering,
    OverlayBarBridge, OverlayBarWindow, PttStreamSink, RuntimeEvents, SharedSlintRuntime,
    SharedString, TileWindow, TileWindows, VecModel, AI_STREAM_MAX_TOKENS, CONVO_SEQ,
    TILE_DISPLAY_SEQ,
};
/// Phase E6 v42 — hard cap on a push-to-record hold (30 s). Backstop for a
/// lost pointer-up (alt-tab / focus loss mid-hold): without it the record
/// thread would loop forever on the stop flag and the PTT guard would stay
/// stuck, permanently blocking the feature. Forcing `stop` after the cap
/// makes the thread finish + ship its PCM, which the drain timer uses to
/// self-heal the guard.
pub(crate) fn spawn_ptt_watchdog(stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(30));
        stop.store(true, Ordering::Release);
    });
}

/// Phase E6 v42 — set a PTT tile's body to an error line. Called from the
/// transcribe task (off the UI thread) so it hops back via the event loop;
/// `slint::Weak` is Send, the strong handle is not.
pub(crate) fn ptt_tile_error(weak: slint::Weak<TileWindow>, msg: &str) {
    let msg = msg.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(t) = weak.upgrade() {
            t.set_source_label(SharedString::from("stt · error"));
            t.set_blocks(ModelRc::new(VecModel::from(vec![MarkdownBlock {
                kind: markdown::kind::PARAGRAPH,
                text: SharedString::from(msg),
                lang: SharedString::from(""),
            }])));
        }
    });
}

/// Phase E6 v42 — push-to-talk ask. Given PCM captured while a record
/// button was held, spawn a placeholder tile, then a tokio task that
/// (1) transcribes via Groq, (2) feeds the text as the explicit
/// `user_question` to `ai::build_request` (rolling transcript = context),
/// (3) streams the answer into the tile via the SAME `current_streaming`
/// slot + `ai:event` path as F9. Mirrors `fire_f9_ask` with a transcribe
/// step prepended; F9 itself is untouched.
// Wiring fn: bridge + events + cfg + runtime + tiles + overlay-weak are all
// distinct shared handles this path needs; bundling them into a struct would
// add indirection without clarifying anything. #B1 added the overlay weak.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_ptt_ask(
    recording: (audio::AudioSource, Vec<i16>),
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
    weak_overlay: &slint::Weak<OverlayBarWindow>,
) {
    let (source, pcm) = recording;
    let icon = match source {
        audio::AudioSource::Mic => "🎤",
        audio::AudioSource::System => "🔊",
    };

    // Ignore trivially short holds (<~0.3 s @ 16 kHz mono = 4800 samples).
    if pcm.len() < 4800 {
        eprintln!(
            "[overlay-host] PTT: hold too short ({} samples) — skipping",
            pcm.len()
        );
        return;
    }

    // ===== 1. Sync placeholder tile =====
    let tile = match TileWindow::new() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[overlay-host] PTT: TileWindow::new failed: {e}");
            return;
        }
    };
    let seq = TILE_DISPLAY_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    tile.set_sequence(seq as i32);
    tile.set_tile_title(SharedString::from(format!("{icon} Запись")));
    tile.set_source_label(SharedString::from("stt · расшифровка…"));
    tile.set_trigger_label(SharedString::from(format!("{icon} push-to-talk")));
    tile.set_trigger_color(slint::Color::from_rgb_u8(0xef, 0x44, 0x44));
    // Phase E6 v45 — PTT answers are continuable dialogs too.
    let convo_id = CONVO_SEQ.fetch_add(1, Ordering::Relaxed) as i32;
    tile.set_convo_id(convo_id);
    tile.set_followup_busy(true);
    wire_tile_drag(&tile);
    tile.set_blocks(ModelRc::new(VecModel::from(vec![MarkdownBlock {
        kind: markdown::kind::PARAGRAPH,
        text: SharedString::from("⏳ Расшифровка…"),
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
    // V0.8.1 — per-tile live route (sticky-cloud after 🧠). PTT starts Text.
    let live = live_route(AskRoute::Text);
    // Phase E6 v45 — continue-dialog follow-ups on PTT answer tiles.
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
    // V0.8.0 (Поток D) — 🧠 escalate to cloud; V0.8.1 — flips `live` to Cloud.
    wire_escalate(
        &tile, convo_id, &live, bridge, events, cfg, slint_rt, rt_handle,
    );
    present_tile_window(&tile);
    apply_tile_hwnd_with_monitor(&tile);
    let weak_for_stream = tile.as_weak();
    let weak_for_title = tile.as_weak();
    tiles.borrow_mut().push(tile);
    refresh_open_tiles(weak_overlay, tiles);

    // ===== 2. Independent per-tile streaming (NOT the shared F9 slot) =====
    // Each PTT is a distinct question whose answer must survive a second rapid
    // PTT, so we stream straight into THIS tile via a PttStreamSink (built in
    // the task once `messages` exist) instead of the single `current_streaming`
    // slot. No supersede, no abort — rapid PTTs no longer clobber each other.

    // ===== 3. Snapshot config + rolling transcript (context) =====
    let (base_url, bearer, model, meeting_context, response_language, is_local) = {
        let c = cfg.read();
        let ep = c.ai_endpoint(false);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            c.meeting_context.clone(),
            c.response_language.clone(),
            ep.is_local,
        )
    };
    let (stt_backend, stt_is_local, groq_key, stt_language, trigger_keywords) = {
        let c = cfg.read();
        (
            c.stt_backend(),
            c.stt_is_local(),
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            c.trigger_keywords.clone(),
        )
    };
    let transcript_lines: Vec<String> = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        s.transcript
            .iter()
            .map(|l| format!("[{:?}] {}", l.source, l.text))
            .collect()
    };
    let (journal_for_loop, health_for_stream) = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        (s.journal.clone(), s.health.clone())
    };

    // ===== 4. Cost closure (NO abort — PTT streams run independently, so a
    // second PTT must not cancel the first's in-flight answer) =====
    let rt_for_cost = slint_rt.clone();
    let cost_apply: overlay_backend::runtime::CostApplyFn = Box::new(move |micro| {
        // Local inference is free — don't bill it (and don't trip the cap).
        let micro = if is_local { 0 } else { micro };
        let mut s = slint_replay::runtime_state::lock(&rt_for_cost);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    });

    // ===== 5. Spawn transcribe → ask (detached: never stored in ai_task, so a
    // later F9/PTT/followup can't abort it) =====
    let bridge_for_task = bridge.clone();
    let events_inner = events.clone();
    rt_handle.spawn(async move {
        let whisper_prompt = stt::build_whisper_prompt(&trigger_keywords, &meeting_context);
        let result = if !stt_is_local && groq_key.is_empty() {
            Err(anyhow::anyhow!("Groq API key not set (Settings → STT)"))
        } else {
            stt::transcribe_once(
                &stt_backend,
                &pcm,
                stt_language.as_deref(),
                whisper_prompt.as_deref(),
            )
            .await
        };
        let question = match result {
            Ok(q) if !q.trim().is_empty() => q.trim().to_string(),
            Ok(_) => {
                ptt_tile_error(weak_for_title.clone(), "Речь не распознана (тишина?)");
                return;
            }
            Err(e) => {
                // SECURITY (review C1/nit) — a self-hosted STT backend's error
                // can carry its endpoint URL; show a generic message (the tile
                // is screen-shared). classify_ai_error covers timeout/refused/
                // auth/etc. without echoing any URL.
                ptt_tile_error(
                    weak_for_title.clone(),
                    &format!("Ошибка STT: {}", classify_ai_error(&format!("{e:#}"))),
                );
                return;
            }
        };
        // Reflect the recognised question in the tile chrome.
        {
            let q = question.clone();
            let w = weak_for_title.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(t) = w.upgrade() {
                    t.set_tile_title(SharedString::from(q));
                    t.set_source_label(SharedString::from("ai · asking…"));
                }
            });
        }
        let messages = ai::build_request(
            &meeting_context,
            &response_language,
            &transcript_lines,
            None,
            Some(&question),
        );
        // Per-tile sink: streams this answer into THIS PTT tile and, on Done,
        // folds the turn into its conversation (for follow-ups). Carries the
        // sent messages so the fold has full context. Replaces the shared-slot
        // registration — this is what makes rapid PTTs independent.
        let sink: Arc<dyn RuntimeEvents> = Arc::new(PttStreamSink::new(
            bridge_for_task.clone(),
            events_inner.clone(),
            weak_for_stream,
            convo_id,
            messages.clone(),
        ));
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
        if let Some(j) = journal_for_loop.as_ref() {
            j.write(&journal::JournalEvent::AiRequest {
                unix_ms: journal::now_unix_ms(),
                purpose: "ptt_ask",
                model: &model,
                system_prompt: &sys_full,
                user_prompt: &usr_full,
                attached_screenshot: false,
                input_tokens_est: ((sys_full.chars().count() + usr_full.chars().count()) as u64)
                    / 4,
            });
        }
        let t0 = std::time::Instant::now();
        let ai_rx = ai::stream_chat(
            base_url,
            bearer,
            model.clone(),
            messages,
            AI_STREAM_MAX_TOKENS,
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
