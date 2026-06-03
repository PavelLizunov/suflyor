//! AI-ask / follow-up / escalate ENTRYPOINTS carved out of
//! `tile_controller.rs` (Wave 2 of the `tile_controller` split — see
//! `docs/overlay-host-modularization-plan.md` §5.10 and
//! `docs/overlay-host-current-review.md` §"tile_controller.rs стал новым
//! мини-монолитом"). This is the ASK-INITIATION side: the code that decides
//! WHICH messages get sent. The STREAM-WRITE side (which messages get STORED) —
//! `OverlayBarBridge`, `handle_ai_event`, the conversation map, the streaming-
//! tile install + generation gating, and the per-PTT `PttStreamSink` — stays in
//! `tile_controller.rs` and is reached here through the crate-root glob below.
//!
//! This module owns:
//!
//! - the route model — `AskRoute` (Text / Vision / Cloud) + its `impl`
//!   (`endpoint` / `max_tokens` / `attaches_screenshot`), the per-tile mutable
//!   `LiveRoute` (sticky-cloud after 🧠 / Shift+F9) + `live_route`;
//! - the ask entrypoints — `fire_f3_reask` (F3 reask), `fire_f6_manual_spawn`
//!   (F6 manual tile), `fire_f9_ask` (F9 / Shift+F9 / typed "✏ Написать"),
//!   `fire_ptt_ask` (push-to-talk);
//! - the follow-up / escalate flow — `fire_followup_ask` (in-tile continue-
//!   dialog), `fire_regenerate` (🔄 / 🧠), `wire_escalate` (the 🧠 cloud button),
//!   `wire_voice_followup` (the 🎤 voice button + its `VFU_TX` drain sender);
//! - PTT ask helpers — `spawn_ptt_watchdog` (the 30 s lost-pointer-up backstop,
//!   shared with the Settings dictation toggle), `ptt_tile_error`;
//! - cost / transcript helpers used by the above — `warn_if_over_cost_cap`,
//!   `cost_cap_reason`, `select_recent_labeled`.
//!
//! What STAYS in `tile_controller.rs` (reached here through the glob): the
//! `OverlayBarBridge` (the SOLE `handle_ai_event` conversation-writer, plus
//! `store_conversation` / `drop_conversation` and the in-flight counters);
//! `StreamingTile` / `ConvoState` / `SpawnTileRequest`; `install_streaming_tile`
//! with `GenGatedEvents` / `gated_events` (the wrong-tile-race generation
//! guard); and `PttStreamSink` / `PttSinkState` (the per-tile PTT stream SINK
//! that `fire_ptt_ask` constructs).
//!
//! SECURITY (unchanged by this move): AI error tiles route the raw error chain
//! through `classify_ai_error` → a generic message, so the local AI server's LAN
//! IP:port can never leak into a screen-shared tile.
//!
//! NOTE (§7): this mechanical move imports the parent crate-root via
//! `use super::*;` (the moved code reaches `TileWindow`/`OverlayBarWindow`, the
//! win32 helpers, the runtime/journal/health glue, `to_md_blocks`, the mic
//! guard, `classify_ai_error`, the tile-window / tile-copy leaf helpers, and the
//! `OverlayBarBridge` / `install_streaming_tile` / `gated_events` /
//! `PttStreamSink` that stay in `tile_controller.rs` through it). That is
//! intentional for the extraction; the imports get narrowed in a later pass.
use super::*;

// ============================================================================
// Follow-up reframe — root cause of the escalate→follow-up bug.
// ============================================================================
// The stored conversation carries the F9 "answer the last question FROM THE
// TRANSCRIPT" frame in the system prompt AND the first transcript user turn, so a
// follow-up keeps getting answered as the ORIGINAL question. v1 of this fix only
// reframed the system + demoted the transcript turn but KEPT the multi-turn array
// — and a live test (journal-confirmed) showed the model STILL re-answered the
// original even with a neutral system + the new question last. So the model (or
// the LAN bridge) anchors on the multi-turn DIALOG itself, not just the wording.
//
// v2 — COLLAPSE: for a TEXT dialog, fold the prior turns into labelled REFERENCE
// context inside ONE system message and send the new question as the SINGLE user
// turn. With no prior "conversation" in the array there is nothing for the model
// to continue, so the latest question is unambiguously THE task. A VISION dialog
// (image in a `Parts` turn) is left multi-turn + unchanged (collapsing would drop
// the screenshot; the reported bug is the text F9/escalate path). SEND-TIME only:
// the STORED history stays original, so copy/display are unaffected and the
// reframe is recomputed fresh each turn. If a single-user-turn send STILL returns
// the prior topic, the fault is the bridge ignoring the user turn (not the client).

/// Neutral system prompt for a follow-up / re-ask SEND. The new question is sent
/// as a SINGLE user turn; `prior_context` (the folded prior dialog) is reference
/// only. Keeps the user's background + format/language rules.
fn followup_system_prompt(
    response_language: &str,
    meeting_context: &str,
    prior_context: &str,
) -> String {
    let lang = match response_language {
        "ru" => {
            "Отвечай ИСКЛЮЧИТЕЛЬНО на русском (английский только для названий \
             технологий/команд)."
        }
        "en" => "Respond exclusively in English.",
        _ => "Отвечай на языке вопроса пользователя.",
    };
    let ctx_block = if meeting_context.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\nБэкграунд пользователя (фон для уровня детализации — НЕ ограничивай \
             ответ этой темой):\n{}",
            meeting_context.trim()
        )
    };
    let prior_block = if prior_context.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n\n=== Контекст прошлого диалога (СПРАВКА, НЕ задание) ===\n{}\n\
             === Конец контекста ===",
            prior_context.trim()
        )
    };
    format!(
        "Ты — техничный AI-ассистент. Ответь ПРЯМО и по сути на ВОПРОС ПОЛЬЗОВАТЕЛЯ \
         (он придёт отдельным сообщением «user»). Контекст прошлого диалога ниже — \
         это ТОЛЬКО справка: опирайся на него, если новый вопрос явно его \
         продолжает; если новый вопрос про ДРУГОЕ — отвечай на него и НЕ возвращайся \
         к прошлой теме.{ctx_block}\n\n\
         === Формат ===\n\
         - БЕЗ преамбулы, сразу по сути.\n\
         - Маркдаун: **жирное** для важного, `code` для команд, списки.\n\
         - Конкретные команды/числа, не общие фразы.\n\
         - {lang}{prior_block}"
    )
}

/// Strip the F9 transcript scaffolding from a prior user turn so, folded into the
/// reference context, it reads as a plain past question — not a live "answer the
/// transcript" instruction.
fn strip_transcript_scaffold(s: &str) -> String {
    let mut out = s.to_string();
    // Drop the "answer the last transcript question" trailer.
    if let Some(p) = out.find("На основе последнего вопроса в транскрипте")
    {
        out.truncate(p);
    }
    // Unwrap the "Помоги ответить: <q>" prefix → keep <q>.
    if let Some(p) = out.find("Помоги ответить:") {
        out = out[p + "Помоги ответить:".len()..].to_string();
    }
    // Drop the transcript header line.
    out = out.replace("Транскрипт последних реплик (внизу — самые свежие):", "");
    out.trim().to_string()
}

/// Build the SEND messages for a follow-up / re-ask (v2 — collapse; see the
/// section above). TEXT dialog → `[system(neutral + prior dialog as reference),
/// user(new question)]` (a single user turn). VISION dialog (any `Parts` turn) →
/// the original multi-turn history, unchanged. Pure → unit-tested.
fn reframe_for_send(
    history: &[ai::ChatMessage],
    response_language: &str,
    meeting_context: &str,
) -> Vec<ai::ChatMessage> {
    let Some(last_user) = history.iter().rposition(|m| m.role == "user") else {
        return history.to_vec();
    };
    // Vision: keep the multi-turn array (the screenshot lives in a Parts turn).
    if history
        .iter()
        .any(|m| matches!(&m.content, ai::MessageContent::Parts(_)))
    {
        return history.to_vec();
    }
    let question = match &history[last_user].content {
        ai::MessageContent::Text(t) => t.clone(),
        _ => String::new(),
    };
    // Fold every prior turn (skip the F9 system) into labelled reference context.
    let mut prior = String::new();
    for (i, m) in history.iter().enumerate() {
        if i == last_user {
            break;
        }
        match m.role.as_str() {
            "user" => {
                let q = strip_transcript_scaffold(&message_text(&m.content));
                if !q.is_empty() {
                    prior.push_str("Пользователь ранее спросил: ");
                    prior.push_str(&q);
                    prior.push('\n');
                }
            }
            "assistant" => {
                let a = message_text(&m.content);
                if !a.trim().is_empty() {
                    prior.push_str("Ты ответил: ");
                    prior.push_str(a.trim());
                    prior.push('\n');
                }
            }
            _ => {}
        }
    }
    vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(followup_system_prompt(
                response_language,
                meeting_context,
                &prior,
            )),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(question),
        },
    ]
}

/// V0.8.0 (Поток D) — which AI endpoint an ask/follow-up/regenerate routes to.
/// Replaces the old `use_vision: bool` so the three routes are explicit and the
/// compiler enforces exhaustive handling (no silent bool transposition across
/// the ~9 call sites of the central ask fns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AskRoute {
    /// Default text model (local or cloud per `ai_provider`).
    Text,
    /// Vision endpoint — the stored conversation carries the screenshot (F8).
    Vision,
    /// One-shot CLOUD escalation: the smart `prep_model` on the cloud bridge,
    /// IGNORING `ai_provider`. For a single hard question without flipping the
    /// persistent provider. Stronger reasoning, NOT live web.
    Cloud,
}

impl AskRoute {
    /// Resolve the endpoint for this route from config.
    fn endpoint(self, c: &overlay_backend::config::Config) -> overlay_backend::config::AiEndpoint {
        match self {
            AskRoute::Text => c.ai_endpoint(false),
            AskRoute::Vision => c.vision_endpoint().unwrap_or_else(|| c.ai_endpoint(false)),
            AskRoute::Cloud => c.ai_endpoint_cloud(),
        }
    }
    /// Max output tokens for this route (vision is capped tighter).
    fn max_tokens(self) -> u32 {
        match self {
            AskRoute::Vision => vision::VISION_MAX_TOKENS,
            AskRoute::Text | AskRoute::Cloud => AI_STREAM_MAX_TOKENS,
        }
    }
    /// True when the request carries a screenshot (journal flag).
    fn attaches_screenshot(self) -> bool {
        matches!(self, AskRoute::Vision)
    }
}

/// V0.8.1 — a per-tile MUTABLE route, shared by a tile's continuation surfaces
/// (text follow-up, 🔄 regenerate, 🎤 voice). They read it at CLICK time (not at
/// wire time), so when the 🧠 escalate button flips it to Cloud the rest of that
/// tile's conversation stays in the cloud — matching the sticky-cloud behaviour
/// Shift+F9 already has. UI-thread-only, so a Cell (no lock) is sufficient.
pub(crate) type LiveRoute = Rc<std::cell::Cell<AskRoute>>;

pub(crate) fn live_route(initial: AskRoute) -> LiveRoute {
    Rc::new(std::cell::Cell::new(initial))
}

/// V5 — voice follow-up: a tile's 🎤 button records + transcribes a question
/// off the UI thread, then ships `(convo_id, use_vision, text)` here. A
/// UI-thread drain (sibling to the PTT drain) routes it into the tile's
/// conversation by convo_id. Process-global so `wire_voice_followup` reaches it
/// without threading a sender through every tile-creation fn. Set once at start.
pub(crate) static VFU_TX: std::sync::OnceLock<
    tokio_mpsc::UnboundedSender<(i32, AskRoute, String)>,
> = std::sync::OnceLock::new();

/// V5 — wire the 🎤 voice button on a conversation tile. Toggle: first click
/// records the mic, second click (⏹) stops + transcribes off-thread and ships
/// the recognized text to the voice follow-up drain, which streams the answer
/// into THIS tile (text endpoint when `use_vision` is false, vision endpoint —
/// keeping the dialog about the screenshot — when true). Mirrors the Settings
/// dictation toggle; reuses the PTT 30 s watchdog so a forgotten stop can't
/// leak a recording thread.
pub(crate) fn wire_voice_followup(
    tile: &TileWindow,
    convo_id: i32,
    route: LiveRoute,
    cfg: &overlay_backend::config::SharedConfig,
) {
    let voice_stop: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::new(RefCell::new(None));
    let weak = tile.as_weak();
    let cfg_c = cfg.clone();
    tile.on_followup_voice_toggled(move || {
        let Some(t) = weak.upgrade() else {
            return;
        };
        // Toggle OFF — stop the in-flight recording; the thread finishes,
        // transcribes, and ships the text to the drain.
        if let Some(stop) = voice_stop.borrow_mut().take() {
            // Normal toggle-OFF: we set the stop flag; the record thread finishes,
            // transcribes, and ships the text to the drain.
            if !stop.swap(true, Ordering::AcqRel) {
                t.set_voice_recording(false);
                t.set_followup_busy(true);
                t.set_source_label(SharedString::from("stt · расшифровка…"));
                return;
            }
            // The 30 s watchdog already fired (stop was already true): the prior
            // recording has ended + shipped, so this is NOT a real toggle-off —
            // fall through to start a FRESH recording instead of swallowing the
            // click (audit #23: the first 🎤 click after a watchdog timeout was
            // a dead-click that the user had to repeat).
        }
        // Toggle ON — snapshot STT config, then record on a thread.
        let (
            mic_dev,
            stt_backend,
            stt_is_local,
            groq_key,
            stt_language,
            trigger_keywords,
            meeting_context,
        ) = {
            let c = cfg_c.read();
            (
                c.mic_device.clone(),
                c.stt_backend(),
                c.stt_is_local(),
                c.groq_api_key.clone(),
                c.stt_language.clone(),
                c.trigger_keywords.clone(),
                c.meeting_context.clone(),
            )
        };
        if !stt_is_local && groq_key.is_empty() {
            // No STT key — generic message (never leak endpoint/secret).
            t.set_source_label(SharedString::from("stt · ключ не задан (Settings → STT)"));
            return;
        }
        let Some(tx) = VFU_TX.get().cloned() else {
            return;
        };
        // M2 — only one mic capture at a time across all recorders.
        if !try_acquire_mic() {
            t.set_source_label(SharedString::from("stt · микрофон занят"));
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        *voice_stop.borrow_mut() = Some(stop.clone());
        spawn_ptt_watchdog(stop.clone());
        t.set_voice_recording(true);
        t.set_source_label(SharedString::from("🎤 запись… (нажми ⏹)"));
        // V0.8.1 — snapshot the LIVE route NOW (UI thread, click time) into a
        // plain Copy value; the worker thread can't hold the !Send Rc<Cell>. So
        // a 🎤 follow-up after 🧠-escalation correctly routes to Cloud.
        let route_now = route.get();
        std::thread::spawn(move || {
            let pcm = audio::record_source_until_stop(audio::AudioSource::Mic, mic_dev, None, stop)
                .unwrap_or_else(|e| {
                    eprintln!("[overlay-host] voice follow-up record failed: {e:#}");
                    Vec::new()
                });
            // M2 — free the mic the instant recording ends (before STT, which
            // doesn't touch the device) so the next recorder can start.
            release_mic();
            let text = if pcm.len() < 4800 {
                String::new()
            } else {
                let whisper_prompt = stt::build_whisper_prompt(&trigger_keywords, &meeting_context);
                match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt
                        .block_on(stt::transcribe_once(
                            &stt_backend,
                            &pcm,
                            stt_language.as_deref(),
                            whisper_prompt.as_deref(),
                        ))
                        .unwrap_or_default(),
                    Err(_) => String::new(),
                }
            };
            let _ = tx.send((convo_id, route_now, text));
        });
    });
}

/// V0.8.0 (Поток D, item B) — wire the per-tile 🧠 "ask the smart cloud model"
/// button. Shown ONLY when this tile's answer came from the LOCAL model (cloud→
/// cloud escalation is pointless); clicking re-runs the SAME question on the
/// cloud bridge via `fire_regenerate(.., Cloud)`, replacing the answer in place.
/// One-shot — does not change the persistent provider.
///
/// V0.8.1 — also flips the tile's shared `route` cell to Cloud, so the rest of
/// the conversation (text follow-up / 🔄 / 🎤) stays in the cloud after one
/// escalation — sticky-cloud, matching Shift+F9. (The user noticed continuing
/// the dialog fell back to the local model.)
#[allow(clippy::too_many_arguments)]
pub(crate) fn wire_escalate(
    tile: &TileWindow,
    convo_id: i32,
    route: &LiveRoute,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
) {
    // Only offer escalation when the live answer endpoint is local (otherwise the
    // answer is already cloud and 🧠 is a no-op upgrade) AND a cloud bearer is
    // configured: escalation routes to the cloud bridge (`ai_endpoint_cloud`), so
    // for a local-only user who never set `ai_bearer` the button would fail with a
    // generic error every time — don't offer a dead affordance to that cohort.
    {
        let c = cfg.read();
        if !c.ai_endpoint(false).is_local || c.ai_bearer.trim().is_empty() {
            return;
        }
    }
    tile.set_can_escalate(true);
    let weak = tile.as_weak();
    let route_c = route.clone();
    let bridge_c = bridge.clone();
    let events_c = events.clone();
    let cfg_c = cfg.clone();
    let slint_rt_c = slint_rt.clone();
    let rt_handle_c = rt_handle.clone();
    tile.on_escalate_clicked(move || {
        // V0.8.1 — make the WHOLE conversation sticky-cloud from here on.
        route_c.set(AskRoute::Cloud);
        // Mark the tile as cloud-escalated (review NIT-1) so it's visible the
        // answer now came off-box — parity with the Shift+F9 🧠 badge. Egress is
        // a conscious action (the user clicked 🧠); this just makes it legible.
        if let Some(t) = weak.upgrade() {
            t.set_trigger_label(SharedString::from("🧠 cloud (escalated)"));
            t.set_trigger_color(slint::Color::from_rgb_u8(0x38, 0xbd, 0xf8));
        }
        fire_regenerate(
            convo_id,
            weak.clone(),
            &bridge_c,
            &events_c,
            &cfg_c,
            &slint_rt_c,
            &rt_handle_c,
            AskRoute::Cloud,
        );
    });
}

/// Local helper: compute `Some(reason)` if session_cost is over the
/// configured cap, else `None`. Duplicated from src-tauri's
/// `over_cost_budget` — small enough to inline rather than promote
/// to overlay-backend.
fn cost_cap_reason(cap_usd: f64, current_microcents: u64) -> Option<String> {
    if cap_usd <= 0.0 {
        return None;
    }
    let current_usd = (current_microcents as f64) / 100_000_000.0;
    if current_usd >= cap_usd {
        Some(format!(
            "over budget: ${current_usd:.4} spent ≥ ${cap_usd:.2} (Settings → Max cost per session)"
        ))
    } else {
        None
    }
}

/// Local helper: last `max` transcript lines labeled with speaker
/// tags `[ПОЛЬЗОВАТЕЛЬ]` / `[СОБЕСЕДНИК]`. Mirrors src-tauri's
/// `select_recent_lines_labeled` — kept local to avoid promoting a
/// tiny helper to overlay-backend.
pub(crate) fn select_recent_labeled(
    transcript: &std::collections::VecDeque<overlay_backend::audio::TranscriptLine>,
    max: usize,
) -> Vec<String> {
    let n = transcript.len();
    let start = n.saturating_sub(max);
    transcript
        .iter()
        .skip(start)
        .map(|l| {
            let src = match l.source {
                overlay_backend::audio::AudioSource::System => "[СОБЕСЕДНИК]",
                overlay_backend::audio::AudioSource::Mic => "[ПОЛЬЗОВАТЕЛЬ]",
            };
            format!("{src} {}", l.text)
        })
        .collect()
}

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
                        overlay_backend::audio::AudioSource::System => "🗣",
                        overlay_backend::audio::AudioSource::Mic => "🎤",
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
        "✏ текст · live"
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
        tile.set_trigger_label(SharedString::from("✏ Написать"));
        tile.set_trigger_color(slint::Color::from_rgb_u8(0x34, 0xd3, 0x99));
    } else if route == AskRoute::Cloud {
        tile.set_trigger_label(SharedString::from("🧠 cloud (Shift+F9)"));
        tile.set_trigger_color(slint::Color::from_rgb_u8(0x38, 0xbd, 0xf8));
    } else {
        tile.set_trigger_label(SharedString::from("✋ F9 manual ask"));
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

/// v0.8.2 (MAJOR-2) — sticky-cloud cost-cap warning. After a 🧠 / Shift+F9
/// escalation a tile's `live` route stays `Cloud`, so EVERY subsequent text
/// follow-up + 🔄 regenerate + 🎤 voice follow-up is now a BILLABLE cloud call.
/// `fire_f9_ask` already emits `cost:cap-hit` when over budget, but the
/// continuation paths did not — so the per-session cap was silently ignored
/// mid-conversation (the regression sticky-cloud introduced). This emits the
/// SAME non-blocking warning (warn only — never block a continuation the user
/// is mid-thread on). No-op for local ($0) calls, when no cap is set, or before
/// any spend has accrued.
fn warn_if_over_cost_cap(
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    is_local: bool,
    source: &str,
) {
    if is_local {
        return;
    }
    let cap_usd = cfg.read().max_session_cost_usd;
    if cap_usd <= 0.0 {
        return;
    }
    let current_micro = slint_replay::runtime_state::lock(slint_rt).session_cost_microcents;
    if current_micro == 0 {
        return;
    }
    let usd = (current_micro as f64) / 100_000_000.0;
    if usd >= cap_usd {
        events.emit(
            "cost:cap-hit",
            serde_json::json!({
                "reason": format!(
                    "over budget: ${usd:.4} spent ≥ ${cap_usd:.2} (Settings → Max cost per session)"
                ),
                "source": source,
                "blocking": false,
            }),
        );
    }
}

/// Phase E6 v45 — continue the dialog inside a tile. Reads the tile's
/// stored conversation (seeded when the previous answer completed),
/// appends the new user question, and streams the reply BELOW the
/// existing thread via the SAME `current_streaming` slot + `ai:event`
/// path as F9. `turn` = (convo_id, question).
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_followup_ask(
    turn: (i32, String),
    tile_weak: slint::Weak<TileWindow>,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    // V0.8.0 (Поток D) — which endpoint to route to: Text (default), Vision
    // (F8 tile keeps the dialog about the screenshot), or Cloud (one-shot smart
    // escalation). Was the V5 `use_vision: bool`.
    route: AskRoute,
) {
    let (convo_id, question) = turn;
    let question = question.trim().to_string();
    if question.is_empty() {
        return;
    }

    // Pull the prior conversation (history + rendered thread). Absent only
    // if the input was used before the first answer folded in — the input
    // is disabled until then, so this is a defensive bail.
    let (history, prior_rendered) = {
        let convos = match bridge.conversations.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match convos.get(&convo_id) {
            Some(c) => (c.messages.clone(), c.rendered.clone()),
            None => {
                diag!("followup: no conversation for convo_id={convo_id}");
                // M1 — clear busy so a follow-up fired before the first answer
                // seeded the conversation can't wedge this tile's inputs dead
                // (button/LineEdit are gated on followup-busy / voice-recording).
                if let Some(t) = tile_weak.upgrade() {
                    t.set_followup_busy(false);
                    t.set_voice_recording(false);
                }
                return;
            }
        }
    };

    // New request = full history + this user turn. V0.8.3 — wrap the question as
    // an explicit DIRECT question (FOLLOWUP_DIRECTIVE) so the transcript-framed
    // system prompt doesn't make the model ignore it / re-answer the original.
    // Only the model sees the wrapper — the prefix below + the journal use the
    // clean question, and copy strips the marker.
    let mut messages = history;
    // Clean any legacy FOLLOWUP_DIRECTIVE wrappers older builds left on prior
    // turns (the directive is no longer added — `reframe_for_send` below is what
    // redirects the model now).
    strip_followup_directives(&mut messages);
    // Append the new question BARE. The old verbose FOLLOWUP_DIRECTIVE wrapper
    // made the model META-reply ("это продолжение? звучит как 'Д.'"); the
    // send-time reframe (neutral system + demoted transcript turns) is the real
    // redirect, so the user turn stays clean (and so does the STORED history +
    // the copied transcript).
    messages.push(ai::ChatMessage {
        role: "user".into(),
        content: ai::MessageContent::Text(question.clone()),
    });

    // Visible thread = prior thread + the new question header; the streamed
    // answer renders after this prefix.
    let prefix = format!("{prior_rendered}\n\n---\n\n**🧑 {question}**\n\n");

    // Show the question immediately + mark busy; register the slot so the
    // ai:event deltas land in this tile.
    if let Some(tile) = tile_weak.upgrade() {
        tile.set_followup_busy(true);
        tile.set_source_label(SharedString::from("ai · asking…"));
        let shown = format!("{prefix}⏳ …");
        tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&shown))));
    }
    let generation = install_streaming_tile(
        bridge,
        StreamingTile {
            weak: tile_weak,
            accumulated: String::new(),
            prefix,
            convo_id,
            request_messages: messages.clone(),
        },
    );

    // Snapshot config + journal/health, abort any in-flight task, then
    // spawn the stream (mirrors fire_f9_ask's tail).
    let (base_url, bearer, model, is_local, max_tokens, response_language, meeting_context) = {
        let c = cfg.read();
        let ep = route.endpoint(&c);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            ep.is_local,
            route.max_tokens(),
            c.response_language.clone(),
            c.meeting_context.clone(),
        )
    };
    // THE FIX — send a reframed copy (neutral continuation system + demoted
    // transcript turns) so the model answers THIS question, not the original
    // transcript question. The STORED history (request_messages installed above)
    // stays original — the reframe is recomputed fresh on every turn.
    let send_messages = reframe_for_send(&messages, &response_language, &meeting_context);
    // v0.8.2 (MAJOR-2) — a sticky-cloud follow-up is billable; warn if the
    // session cost cap is already exceeded (mirrors fire_f9_ask).
    warn_if_over_cost_cap(events, cfg, slint_rt, is_local, "followup_ask");
    let (journal_for_loop, health_for_stream) = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        (s.journal.clone(), s.health.clone())
    };
    {
        let mut s = slint_replay::runtime_state::lock(slint_rt);
        if let Some(h) = s.ai_task.take() {
            h.abort();
        }
    }
    let sys_full = send_messages
        .first()
        .and_then(|m| match &m.content {
            ai::MessageContent::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let usr_full = question.clone();
    // Journal the follow-up request so it pairs with the AiResponse that
    // ask_stream_loop writes on completion (F9 + PTT already do this;
    // without it every follow-up turn leaves an orphaned response).
    if let Some(j) = journal_for_loop.as_ref() {
        j.write(&journal::JournalEvent::AiRequest {
            unix_ms: journal::now_unix_ms(),
            purpose: "followup_ask",
            model: &model,
            system_prompt: &sys_full,
            user_prompt: &usr_full,
            attached_screenshot: route.attaches_screenshot(),
            input_tokens_est: ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4,
        });
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
    let task = rt_handle.spawn(async move {
        let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), send_messages, max_tokens);
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
    diag!(
        "followup sent (convo_id={convo_id}, {} chars)",
        question.len()
    );
}

/// V5 — regenerate: re-run the last request (dropping the trailing assistant
/// turn) and replace the tile's answer. For vision tiles (`use_vision`) the
/// screenshot is still in the stored history, so a short first answer can be
/// expanded with one click. V0.8.3: routes through the shared `current_streaming`
/// slot + generation gating (same path as fire_followup_ask) so handle_ai_event
/// is the SOLE conversation-writer — fixes the escalate→follow-up corruption
/// where a detached PttStreamSink left divergent, ungated state.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fire_regenerate(
    convo_id: i32,
    tile_weak: slint::Weak<TileWindow>,
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    route: AskRoute,
) {
    let mut messages = {
        let convos = match bridge.conversations.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match convos.get(&convo_id) {
            Some(c) => c.messages.clone(),
            None => {
                diag!("regenerate: no conversation for convo_id={convo_id}");
                return;
            }
        }
    };
    // Drop the trailing assistant turn(s) so we re-ask the same question.
    while matches!(messages.last(), Some(m) if m.role == "assistant") {
        messages.pop();
    }
    if messages.is_empty() {
        return;
    }
    // Clean stale FOLLOWUP_DIRECTIVE wrappers off the PRIOR turns (all but the
    // last — the turn being re-asked keeps whatever framing it had: a follow-up
    // stays a direct question, an original F9 ask stays unwrapped). Mirrors
    // fire_followup_ask so a regenerate doesn't re-accumulate directives.
    if messages.len() > 1 {
        let n = messages.len() - 1;
        strip_followup_directives(&mut messages[..n]);
    }
    let (base_url, bearer, model, is_local, max_tokens, response_language, meeting_context) = {
        let c = cfg.read();
        let ep = route.endpoint(&c);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            ep.is_local,
            route.max_tokens(),
            c.response_language.clone(),
            c.meeting_context.clone(),
        )
    };
    // Re-asking a FOLLOW-UP turn (≥2 user turns) inherits the same transcript
    // anchors as fire_followup_ask → reframe at send time so the model answers
    // THAT turn, not the original. A plain 🧠/🔄 of the ORIGINAL answer (1 user
    // turn) keeps the rich F9 system so the cloud re-answer has full meeting
    // context. Send-time only — the STORED history stays original.
    let send_messages = if messages.iter().filter(|m| m.role == "user").count() >= 2 {
        reframe_for_send(&messages, &response_language, &meeting_context)
    } else {
        messages.clone()
    };
    // v0.8.2 (MAJOR-2) — a sticky-cloud regenerate is billable; warn over cap.
    warn_if_over_cost_cap(events, cfg, slint_rt, is_local, "regenerate");
    if let Some(t) = tile_weak.upgrade() {
        t.set_followup_busy(true);
        t.set_source_label(SharedString::from("ai · перегенерация…"));
        t.set_blocks(ModelRc::new(VecModel::from(to_md_blocks("⏳ …"))));
    }
    // V0.8.3 (escalate→followup bug) — route the regenerate through the SAME
    // `current_streaming` slot + generation gating as fire_followup_ask (was a
    // detached, ungated PttStreamSink). `prefix = ""` because a regenerate
    // REPLACES the tile body with the fresh answer (matches the old display),
    // but now `handle_ai_event` is the SOLE writer of conversations[convo_id]:
    // the generation is bumped (so an in-flight stream is superseded/gated) and
    // the task is abortable. Before this, 🧠-escalate (which calls here) left the
    // conversation in a divergent, ungated state, so the 2nd follow-up after an
    // escalation re-sent stale history and re-emitted the escalation answer
    // verbatim.
    let generation = install_streaming_tile(
        bridge,
        StreamingTile {
            weak: tile_weak,
            accumulated: String::new(),
            prefix: String::new(),
            convo_id,
            request_messages: messages.clone(),
        },
    );
    let (journal_for_loop, health_for_stream) = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        (s.journal.clone(), s.health.clone())
    };
    {
        let mut s = slint_replay::runtime_state::lock(slint_rt);
        if let Some(h) = s.ai_task.take() {
            h.abort();
        }
    }
    let sys_full = send_messages
        .first()
        .and_then(|m| match &m.content {
            ai::MessageContent::Text(t) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default();
    // The re-asked question = the last user turn in the (assistant-trimmed)
    // history. Journal the request so it pairs with the AiResponse (parity with
    // F9/follow-up; regenerate previously left an orphan response).
    let usr_full = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| message_text(&m.content))
        .unwrap_or_default();
    if let Some(j) = journal_for_loop.as_ref() {
        j.write(&journal::JournalEvent::AiRequest {
            unix_ms: journal::now_unix_ms(),
            purpose: "regenerate",
            model: &model,
            system_prompt: &sys_full,
            user_prompt: &usr_full,
            attached_screenshot: route.attaches_screenshot(),
            input_tokens_est: ((sys_full.chars().count() + usr_full.chars().count()) as u64) / 4,
        });
    }
    let rt_for_cost = slint_rt.clone();
    let cost_apply: overlay_backend::runtime::CostApplyFn = Box::new(move |micro| {
        let micro = if is_local { 0 } else { micro };
        let mut s = slint_replay::runtime_state::lock(&rt_for_cost);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    });
    let t0 = std::time::Instant::now();
    let events_for_task = gated_events(bridge, events.clone(), generation);
    let task = rt_handle.spawn(async move {
        let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), send_messages, max_tokens);
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
    diag!("regenerate sent (convo_id={convo_id}, route={route:?})");
}

#[cfg(test)]
mod tests {
    //! The escalate→follow-up fix (v2 — collapse). A multi-turn array let the
    //! model (or the LAN bridge) keep answering the ORIGINAL topic even with a
    //! neutral system + the new question last (live, journal-confirmed). So a TEXT
    //! follow-up is COLLAPSED to `[system(neutral + prior dialog as reference),
    //! user(new question)]` — one user turn, no prior "conversation" to continue.
    //! Vision dialogs stay multi-turn. Pure: no UI, no network.
    use super::*;

    fn msg(role: &str, text: &str) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.into(),
            content: ai::MessageContent::Text(text.into()),
        }
    }
    fn text_of(m: &ai::ChatMessage) -> String {
        match &m.content {
            ai::MessageContent::Text(t) => t.clone(),
            _ => String::new(),
        }
    }

    // A text follow-up collapses to EXACTLY [system(+prior context), user(new q)]:
    // the new question is the single user turn, the prior Q&A is reference context,
    // and the F9 transcript framing is gone.
    #[test]
    fn reframe_collapses_text_followup_to_single_user_turn_with_context() {
        let history = vec![
            msg("system", "Ты — техничный AI-ассистент … из транскрипта."),
            msg("user", "Помоги ответить: что такое ChatGPT"),
            msg("assistant", "ChatGPT — это большая языковая модель."),
            msg("user", "1+1?"),
        ];
        let out = reframe_for_send(&history, "ru", "");
        // Collapsed to exactly system + one user turn.
        assert_eq!(
            out.len(),
            2,
            "expected [system, user], got {} msgs",
            out.len()
        );
        assert_eq!(out[0].role, "system");
        assert_eq!(out[1].role, "user");
        // The single user turn IS the new question.
        assert_eq!(text_of(&out[1]), "1+1?");
        // F9 transcript framing gone; the prior Q + A folded in as reference.
        assert!(!text_of(&out[0]).contains("из транскрипта"));
        assert!(
            text_of(&out[0]).contains("что такое ChatGPT"),
            "prior question not in context"
        );
        assert!(
            text_of(&out[0]).contains("большая языковая модель"),
            "prior answer not in context"
        );
        assert!(
            text_of(&out[0]).contains("СПРАВКА"),
            "context not labelled as reference"
        );
    }

    // A vision dialog (image in a Parts turn) is left multi-turn + unchanged so the
    // screenshot is not dropped.
    #[test]
    fn reframe_leaves_vision_dialog_multiturn() {
        let history = vec![
            msg("system", "vision sys"),
            ai::ChatMessage {
                role: "user".into(),
                content: ai::MessageContent::Parts(vec![]),
            },
            msg("assistant", "screenshot reading"),
            msg("user", "и что дальше?"),
        ];
        let out = reframe_for_send(&history, "ru", "");
        assert_eq!(
            out.len(),
            history.len(),
            "vision dialog must stay multi-turn"
        );
    }

    // strip_transcript_scaffold unwraps "Помоги ответить:" + drops the transcript
    // header + the "answer the transcript" trailer → a plain past question.
    #[test]
    fn strip_transcript_scaffold_yields_plain_question() {
        assert_eq!(
            strip_transcript_scaffold("Помоги ответить: что такое DNS"),
            "что такое DNS"
        );
        let framed = "Транскрипт последних реплик (внизу — самые свежие):\n\
                      [Mic] что такое DNS\n\
                      На основе последнего вопроса в транскрипте предложи краткий ответ.";
        let s = strip_transcript_scaffold(framed);
        assert!(s.contains("что такое DNS"));
        assert!(!s.contains("Транскрипт последних реплик"));
        assert!(!s.contains("предложи краткий ответ"));
    }

    // The neutral system carries language + meeting-context + the folded prior.
    #[test]
    fn followup_system_prompt_carries_language_context_and_prior() {
        let s = followup_system_prompt("ru", "Senior SRE", "Пользователь ранее спросил: X");
        assert!(s.contains("русском"));
        assert!(s.contains("Senior SRE"));
        assert!(s.contains("Пользователь ранее спросил: X"));
        assert!(followup_system_prompt("en", "", "").contains("English"));
        // Empty meeting-context → no background block.
        assert!(!followup_system_prompt("ru", "", "").contains("Бэкграунд"));
    }
}
