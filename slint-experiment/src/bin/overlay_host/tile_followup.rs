//! The continuation surfaces of a tile — follow-up / re-ask / regenerate / voice /
//! 🧠 escalate — carved out of `tile_ask.rs` (the `tile_ask` split, see
//! `docs/overlay-host-modular-structure-current.md`). Owns the FOLLOW-UP REFRAME
//! (root-cause fix for the escalate→follow-up bug, commit `6ffbc40`):
//! `followup_system_prompt` / `strip_transcript_scaffold` / `reframe_for_send`
//! (+ its unit tests); the continuation entrypoints `fire_followup_ask` (in-tile
//! continue-dialog) + `fire_regenerate` (🔄 / 🧠); and the button wiring
//! `wire_escalate` (🧠 cloud) + `wire_voice_followup` (🎤) with its `VFU_TX` drain
//! sender. The F3/F6/F9 INITIAL-ask entrypoints stay in `tile_ask.rs`; cross-split
//! references resolve through the crate root.
//!
//! BEHAVIOUR UNCHANGED — pure relocation; the reframe logic + its 4 tests are
//! byte-identical (re-smoke the live follow-up after escalate to confirm).
//!
//! NOTE (§7): the crate-root symbols this module uses are imported below.
use super::{
    ai, audio, gated_events, install_streaming_tile, journal, message_text, release_mic,
    spawn_ptt_watchdog, strip_followup_directives, stt, to_md_blocks, tokio_mpsc, try_acquire_mic,
    warn_if_over_cost_cap, Arc, AskRoute, AtomicBool, ComponentHandle, LiveRoute, ModelRc,
    Ordering, OverlayBarBridge, Rc, RefCell, RuntimeEvents, SharedSlintRuntime, SharedString,
    StreamingTile, TileWindow, VecModel,
};
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
        // Audit Finding 2: mirror ai::build_request's profile semantics so a ROLE/
        // style profile is honored in follow-ups as strongly as in the first answer
        // (it used to be framed only as weak "background", causing persona drift).
        format!(
            "\n\nПрофиль/контекст пользователя — применяй его ОДИНАКОВО к этому \
             продолжению диалога. Если профиль задаёт РОЛЬ или стиль общения \
             (например «отвечай как психолог», «говори кратко») — следуй ему. Если \
             это бэкграунд/опыт — используй для уровня детализации, НЕ ограничивая \
             тему ответа этим:\n{}",
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
        t.set_source_label(SharedString::from("recording... (click stop)"));
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
            t.set_trigger_label(SharedString::from("cloud (escalated)"));
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
    let prefix = format!("{prior_rendered}\n\n---\n\n**You: {question}**\n\n");

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
    // Audit (prompt-context): the follow-up LLM prompt carries approved memory +
    // profile too (reframe builds a fresh neutral system prompt, so no double-add).
    let meeting_context = overlay_backend::memory::context_for_meeting(&meeting_context);
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
        // Audit (prompt-context): the reframed follow-up prompt carries approved
        // memory + profile. Computed ONLY on this branch — a 1-turn regenerate
        // sends the stored (already-effective) system as-is, no extra catalog read.
        let meeting_context = overlay_backend::memory::context_for_meeting(&meeting_context);
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
        assert!(!followup_system_prompt("ru", "", "").contains("Профиль/контекст"));
    }

    #[test]
    fn followup_system_prompt_carries_role_style_semantics() {
        // Audit Finding 2: a ROLE/style profile must be honored in follow-ups as
        // strongly as in the first answer (was previously framed as weak background).
        let s = followup_system_prompt("ru", "отвечай как психолог", "");
        assert!(
            s.contains("РОЛЬ") || s.contains("роль"),
            "follow-up must invoke role semantics"
        );
        assert!(s.contains("стиль"), "follow-up must invoke style semantics");
        assert!(
            s.contains("отвечай как психолог"),
            "follow-up must carry the profile text"
        );
    }
}
