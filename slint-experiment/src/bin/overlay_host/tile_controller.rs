//! Tile controller: the AI-ask / tile-streaming / conversation machinery
//! (Phase 7a of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.10).
//!
//! This module owns the host-side conversational-tile pipeline:
//!
//! - the `OverlayBarBridge` (the `SlintUiBridge`/`RuntimeEvents` sink that routes
//!   `ai:event`/`transcript`/`cost`/`health`/`session` updates to the bar and
//!   spawns tile windows) plus its conversation map, with `handle_ai_event` the
//!   SOLE conversation-writer (`store_conversation`, `drop_conversation`);
//! - the streaming-tile install + generation gating
//!   (`install_streaming_tile` + `GenGatedEvents`/`gated_events` +
//!   `StreamingTile`), the wrong-tile-race guard: a superseded stream's emits
//!   are DROPPED so a buffered delta from a torn-down stream can't fold into the
//!   now-current tile;
//! - the per-PTT independent sink (`PttStreamSink`/`PttSinkState`) so two rapid
//!   push-to-talk asks don't clobber each other;
//! - the ask/stream entrypoints — `fire_f3_reask`, `fire_f6_manual_spawn`,
//!   `fire_f9_ask` (F9 / Shift+F9 / typed "✏ Написать"), `fire_ptt_ask`,
//!   `fire_followup_ask`, `fire_regenerate` — plus the route model
//!   (`AskRoute`/`LiveRoute`/`live_route`) and cost-cap helpers
//!   (`warn_if_over_cost_cap` / `cost_cap_reason` / `select_recent_labeled`);
//! - per-tile wiring + placement: `wire_copy`, `wire_voice_followup`,
//!   `wire_escalate`, `wire_tile_drag`, `present_tile_window`,
//!   `apply_tile_hwnd_with_monitor`, `toggle_tile_maximize`, `ptt_tile_error`,
//!   `spawn_ptt_watchdog`;
//! - the 📋-copy / conversation-format helpers (`message_text`,
//!   `user_question_for_copy`, `strip_followup_directives`, `convo_copy_text`,
//!   `format_convo_copy`, `conversations_evict_keys`) and their pure unit tests.
//!
//! What STAYS in `overlay_host.rs` (reached here through the glob below):
//! - `fn main` (constructs the `OverlayBarBridge`, owns the hotkey DISPATCH +
//!   bar-chip wiring + the spawn-tile / voice-follow-up drain timers — all of
//!   which call the moved `fire_*` / `wire_*` / `install_streaming_tile` via the
//!   `use tile_controller::*;` re-export);
//! - `to_md_blocks` (markdown→blocks, also used by `vision_capture.rs`);
//! - the mic guard (`try_acquire_mic` / `release_mic` / `MIC_BUSY`, shared with
//!   the wizard + Settings dictation);
//! - the shared tuning constants (`AI_STREAM_MAX_TOKENS`, `HWND_GRAB_DELAY_MS`,
//!   `TILE_DEFAULT_W` / `TILE_DEFAULT_H`) and window/help/palette/Settings glue.
//!
//! SECURITY (unchanged by this move): AI error tiles route the raw error chain
//! through `classify_ai_error` → a generic message, so the local AI server's LAN
//! IP:port can never leak into a screen-shared tile.
//!
//! NOTE (§7): this mechanical move imports the parent crate-root via
//! `use super::*;` (the moved code reaches `TileWindow`/`OverlayBarWindow`, the
//! win32 helpers, the runtime/journal/health glue, `to_md_blocks`, the mic
//! guard, and `classify_ai_error` through it). That is intentional for the
//! extraction; the imports get narrowed in a later pass.
use super::*;

/// Phase E6 v45 — monotonic conversation id for the in-tile continue-dialog
/// feature. Each F9/PTT tile that supports follow-ups gets a unique id.
pub(crate) static CONVO_SEQ: AtomicU64 = AtomicU64::new(0);

/// FIX #8 — hard cap on the per-tile `conversations` map (a ConvoState holds
/// the full chat history plus rendered markdown). The map is pruned on tile
/// close and at the MAX_LIVE_TILES eviction, so in normal use it tracks roughly
/// the live tiles only; this cap is a backstop against any path that drops a
/// tile without a reachable convo_id. Mirrors `qa_cache`'s bounded eviction
/// (256, half-evicted when full). Because `convo_id` is monotonic (CONVO_SEQ),
/// the LOWEST ids are the OLDEST, so evicting those first naturally keeps the
/// currently-open tiles' state.
const CONVERSATIONS_MAX_ENTRIES: usize = 256;

/// FIX #8 — given the current conversation keys, pick the oldest half to evict
/// (lowest `convo_id` = oldest, since the ids are monotonic). Pure + testable;
/// mirrors `qa_cache`'s `take(MAX/2)` half-eviction. Returns the ids to remove.
fn conversations_evict_keys(keys: &[i32], max: usize) -> Vec<i32> {
    let mut ids = keys.to_vec();
    ids.sort_unstable();
    ids.truncate(max / 2);
    ids
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
/// Plain text of one chat message — the `Text` body, or for a vision turn the
/// concatenated text Part(s) only (NEVER the base64 image).
fn message_text(content: &ai::MessageContent) -> String {
    match content {
        ai::MessageContent::Text(t) => t.clone(),
        ai::MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ai::ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Strip the `build_request` wrapper from a user turn for the 📋 copy, leaving
/// the actual question. The F9/auto ask bundles the live transcript as AI
/// context ("Транскрипт последних реплик…\n\nПомоги ответить: <q>"), so the real
/// question is the bit after "Помоги ответить:" — without that we'd copy the
/// raw Mic/System transcript lines into the chat copy. A transcript-only F9 ask
/// (no explicit question) → empty, so the noisy transcript is dropped; a typed
/// follow-up is already clean and passes through unchanged.
/// V0.8.3 — prepended to a follow-up's user message sent to the model. The
/// conversation's system prompt frames the assistant as "answer the last
/// question FROM THE TRANSCRIPT", so a bare follow-up was treated as transcript
/// noise and the model re-answered the original (user saw Sonnet reply "Два" to
/// "what is arc raider"). This marker makes the follow-up an explicit DIRECT
/// question. The UI + 📋 copy still show the clean question (it's stripped in
/// user_question_for_copy); the journal logs the raw question.
const FOLLOWUP_DIRECTIVE: &str =
    "Это прямой вопрос пользователя к тебе (НЕ из транскрипта, НЕ предыдущий вопрос). \
     Ответь именно на него: ";

fn user_question_for_copy(raw: &str) -> String {
    let raw = raw.strip_prefix(FOLLOWUP_DIRECTIVE).unwrap_or(raw);
    const MARK: &str = "Помоги ответить:";
    if let Some(i) = raw.rfind(MARK) {
        return raw[i + MARK.len()..].trim().to_string();
    }
    if raw.trim_start().starts_with("Транскрипт последних реплик") {
        return String::new();
    }
    // A vision tile's first user turn is the canned screenshot prompt, not text
    // the user typed — drop it so a multi-turn vision copy doesn't render
    // "🧑 Что на этом скриншоте?…" as if the user had asked it.
    if raw.trim() == vision::DEFAULT_VISION_PROMPT
        || raw.trim().starts_with(vision::TRANSLATE_VISION_PROMPT)
    {
        return String::new();
    }
    raw.trim().to_string()
}

/// Remove the [`FOLLOWUP_DIRECTIVE`] wrapper from the given user turns. Used when
/// building a follow-up / regenerate request so that only the CURRENT question
/// carries the directive. The wrapper is stored verbatim in `conversations`
/// (`handle_ai_event` Done folds `request_messages`), so without this a 3-turn
/// thread would send the model TWO "this is THE direct question" instructions on
/// two different historical turns — and a weak local model then anchors on the
/// wrong one. Non-user turns are left untouched.
fn strip_followup_directives(messages: &mut [ai::ChatMessage]) {
    for m in messages.iter_mut() {
        if m.role != "user" {
            continue;
        }
        let cleaned = match &m.content {
            ai::MessageContent::Text(t) => t.strip_prefix(FOLLOWUP_DIRECTIVE).map(str::to_string),
            _ => None,
        };
        if let Some(c) = cleaned {
            m.content = ai::MessageContent::Text(c);
        }
    }
}

/// V0.8.3 — text for the 📋 copy button. Adaptive so it fits both uses:
///
/// - a single Q→A tile → just the answer (clean paste — the "screenshot →
///   answer → paste it" case);
/// - a multi-turn dialog (a branch) → the WHOLE thread, every question +
///   answer, labelled 🧑 / 🤖 — so a conversation isn't truncated to its last
///   reply (user: "копируется только последнее сообщение, а не весь чат").
///
/// System prompts are skipped; vision turns contribute their text only. Empty
/// if the tile has no (or an unknown / not-yet-seeded) conversation.
fn convo_copy_text(bridge: &OverlayBarBridge, convo_id: i32) -> String {
    let convos = bridge
        .conversations
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    match convos.get(&convo_id) {
        Some(c) => format_convo_copy(&c.messages, &c.rendered),
        None => String::new(),
    }
}

/// Pure formatter behind [`convo_copy_text`] — split out (no bridge / no lock)
/// so the adaptive single-vs-thread logic and the user-turn cleaning are
/// unit-testable. `rendered` is the mid-stream fallback (used when there is no
/// recorded assistant turn yet, or when every turn cleans to empty).
fn format_convo_copy(messages: &[ai::ChatMessage], rendered: &str) -> String {
    let turns: Vec<(&str, String)> = messages
        .iter()
        .filter(|m| m.role != "system")
        .filter_map(|m| {
            let t = message_text(&m.content).trim().to_string();
            (!t.is_empty()).then_some((m.role.as_str(), t))
        })
        .collect();
    if turns.is_empty() {
        return rendered.to_string();
    }
    let assistant_turns = turns.iter().filter(|(r, _)| *r == "assistant").count();
    if assistant_turns <= 1 {
        // Single answer: copy just it (or the rendered body if, mid-stream, no
        // assistant turn is recorded yet).
        return turns
            .iter()
            .rev()
            .find(|(r, _)| *r == "assistant")
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| rendered.to_string());
    }
    let mut out = String::new();
    for (role, text) in &turns {
        // User turns carry the build_request wrapper (transcript + "Помоги
        // ответить:") — copy only the real question, never the Mic/System dump.
        let display = if *role == "assistant" {
            (*text).clone()
        } else {
            user_question_for_copy(text)
        };
        if display.trim().is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(if *role == "assistant" {
            "🤖 "
        } else {
            "🧑 "
        });
        out.push_str(display.trim());
    }
    if out.is_empty() {
        return rendered.to_string();
    }
    out
}

/// V0.8.3 — wire a tile's 📋 copy button: write the answer text to the Windows
/// clipboard and flash the ✅ feedback glyph for ~1.5 s. Called for every
/// conversational tile (those with a `convo_id`). Copy is purely local — no
/// network egress — so it stays safe under screen-share / stealth.
pub(crate) fn wire_copy(tile: &TileWindow, convo_id: i32, bridge: &Arc<OverlayBarBridge>) {
    tile.set_can_copy(true);
    let weak = tile.as_weak();
    let bridge_c = bridge.clone();
    tile.on_copy_clicked(move || {
        let text = convo_copy_text(&bridge_c, convo_id);
        if text.is_empty() {
            return;
        }
        match clipboard_win::set_clipboard_string(&text) {
            Ok(()) => {
                let Some(t) = weak.upgrade() else {
                    return;
                };
                t.set_copied(true);
                let w = t.as_weak();
                Timer::single_shot(Duration::from_millis(1500), move || {
                    if let Some(t) = w.upgrade() {
                        t.set_copied(false);
                    }
                });
            }
            Err(e) => eprintln!("[overlay-host] clipboard copy failed: {e}"),
        }
    });
}

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

/// Install `new_tile` as the active streaming tile, FIRST clearing the
/// slot's previous occupant. The single `current_streaming` slot is shared
/// across F9/PTT/follow-up; starting a new stream aborts the prior task,
/// which then emits no Done/Error — so without this the superseded tile
/// would keep `followup-busy = true` (a permanently dead input). Must run
/// on the UI thread (every ask path registers from a UI-thread callback or
/// timer), so the direct `upgrade()` + setter is safe.
fn install_streaming_tile(bridge: &Arc<OverlayBarBridge>, new_tile: StreamingTile) -> u64 {
    // A new stream supersedes any prior one; reset the in-flight pulse
    // count so an aborted prior stream (which never emits Done/Error)
    // can't leak its Start increment and pin the bar pulse ON forever.
    bridge.reset_ai_in_flight();
    // Bump the stream generation: any still-running prior stream is now
    // "stale" and its GenGatedEvents wrapper will drop further emits.
    let generation = bridge.stream_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let new_convo = new_tile.convo_id;
    let mut slot = match bridge.current_streaming.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(old) = slot.take() {
        // Re-enable only a DIFFERENT tile — the new one is intentionally
        // busy until its own answer completes.
        if old.convo_id != new_convo {
            if let Some(t) = old.weak.upgrade() {
                t.set_followup_busy(false);
                t.set_source_label(SharedString::from("ai · superseded"));
            }
        }
    }
    *slot = Some(new_tile);
    generation
}

/// RuntimeEvents wrapper that drops a SUPERSEDED AI stream's emits. Each
/// ask captures `my_gen` (the generation current when it installed its
/// tile); the next ask bumps `current` via `install_streaming_tile`. Once
/// `my_gen != current` this stream is stale, so its emits — including a
/// buffered `ai:event` Delta delivered after `JoinHandle::abort` but before
/// the loop's next `.await` — are discarded instead of folding into the
/// now-current tile. Closes the wrong-tile race the bare abort leaves open.
struct GenGatedEvents {
    inner: Arc<dyn RuntimeEvents>,
    my_gen: u64,
    current: Arc<AtomicU64>,
}

impl RuntimeEvents for GenGatedEvents {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        if self.my_gen == self.current.load(Ordering::SeqCst) {
            self.inner.emit(channel, payload);
        }
        // else: stale stream — drop the event.
    }
    fn spawn_tile(&self, spec: TileSpec) -> String {
        self.inner.spawn_tile(spec)
    }
    fn spawn_tile_full(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        self.inner.spawn_tile_full(spec, monitor, stealth, kind)
    }
}

/// Wrap `inner` so a stream spawned at `generation` stops emitting once a
/// newer ask supersedes it.
fn gated_events(
    bridge: &Arc<OverlayBarBridge>,
    inner: Arc<dyn RuntimeEvents>,
    generation: u64,
) -> Arc<dyn RuntimeEvents> {
    Arc::new(GenGatedEvents {
        inner,
        my_gen: generation,
        current: bridge.stream_gen.clone(),
    })
}

/// Per-tile streaming sink for push-to-talk asks. F9 shares the single
/// `current_streaming` slot and SUPERSEDES the prior stream (latest-wins, which
/// is correct for a re-ask). PTT is different: each push is a distinct question
/// whose answer must survive — two rapid PTTs must NOT clobber each other. So a
/// PTT streams its answer straight into ONE specific tile (no shared slot, no
/// abort), reusing the bridge's conversation map (for follow-ups) and in-flight
/// pulse counter. Mirrors `OverlayBarBridge::handle_ai_event` but bound to a
/// fixed tile instead of "whatever is in the slot".
pub(crate) struct PttStreamSink {
    bridge: Arc<OverlayBarBridge>,
    inner: Arc<dyn RuntimeEvents>,
    tile: slint::Weak<TileWindow>,
    convo_id: i32,
    state: std::sync::Mutex<PttSinkState>,
    last_render: std::sync::Mutex<std::time::Instant>,
}

struct PttSinkState {
    accumulated: String,
    request_messages: Vec<ai::ChatMessage>,
}

impl PttStreamSink {
    pub(crate) fn new(
        bridge: Arc<OverlayBarBridge>,
        inner: Arc<dyn RuntimeEvents>,
        tile: slint::Weak<TileWindow>,
        convo_id: i32,
        request_messages: Vec<ai::ChatMessage>,
    ) -> Self {
        // Seed last_render in the past so the first delta paints immediately.
        let seeded = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .unwrap_or_else(std::time::Instant::now);
        Self {
            bridge,
            inner,
            tile,
            convo_id,
            state: std::sync::Mutex::new(PttSinkState {
                accumulated: String::new(),
                request_messages,
            }),
            last_render: std::sync::Mutex::new(seeded),
        }
    }
}

impl RuntimeEvents for PttStreamSink {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        if channel != "ai:event" {
            self.inner.emit(channel, payload);
            return;
        }
        let Ok(evt) = serde_json::from_value::<ai::AiEvent>(payload) else {
            return;
        };
        match evt {
            ai::AiEvent::Start { .. } => self.bridge.inc_ai_in_flight(),
            ai::AiEvent::Delta { text } => {
                let body = {
                    let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
                    st.accumulated.push_str(&text);
                    st.accumulated.clone()
                };
                {
                    let now = std::time::Instant::now();
                    let mut last = self.last_render.lock().unwrap_or_else(|p| p.into_inner());
                    if now.duration_since(*last) < std::time::Duration::from_millis(50) {
                        return;
                    }
                    *last = now;
                }
                let weak = self.tile.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(t) = weak.upgrade() {
                        t.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&body))));
                    }
                });
            }
            ai::AiEvent::Done { reason } => {
                self.bridge.dec_ai_in_flight();
                let (final_body, messages) = {
                    let st = self.state.lock().unwrap_or_else(|p| p.into_inner());
                    (st.accumulated.clone(), st.request_messages.clone())
                };
                if self.convo_id >= 0 {
                    let mut messages = messages;
                    messages.push(ai::ChatMessage {
                        role: "assistant".into(),
                        content: ai::MessageContent::Text(final_body.clone()),
                    });
                    // FIX #8 — bounded insert (caps + half-evicts the map).
                    self.bridge.store_conversation(
                        self.convo_id,
                        ConvoState {
                            messages,
                            rendered: final_body.clone(),
                        },
                    );
                }
                // A zero-token Done (some vision servers do this on an
                // unsupported image, or a content filter) would replace the
                // placeholder with an empty body — show a note, not a blank tile.
                let final_body = if final_body.trim().is_empty() {
                    "_(модель не вернула ответ)_".to_string()
                } else {
                    final_body
                };
                let weak = self.tile.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(t) = weak.upgrade() {
                        t.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&final_body))));
                        t.set_source_label(SharedString::from(format!("ai · done ({reason})")));
                        t.set_followup_busy(false);
                    }
                });
            }
            ai::AiEvent::Error { message } => {
                self.bridge.dec_ai_in_flight();
                // SECURITY (review C1) — the raw error chain embeds the AI
                // base_url (the LOCAL server's LAN IP:port), which would leak
                // into the screen-shared tile. classify_ai_error maps it to a
                // generic &'static str (no URL/IP). Same sanitiser the non-
                // streaming path uses; this streaming path is reached by 🔄
                // regenerate + push-to-talk asks.
                let safe = classify_ai_error(&message);
                let weak = self.tile.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(t) = weak.upgrade() {
                        t.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&format!(
                            "⚠ AI error: {safe}"
                        )))));
                        t.set_source_label(SharedString::from("⚠ error"));
                        t.set_followup_busy(false);
                    }
                });
            }
        }
    }
    fn spawn_tile(&self, spec: TileSpec) -> String {
        self.inner.spawn_tile(spec)
    }
    fn spawn_tile_full(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        self.inner.spawn_tile_full(spec, monitor, stealth, kind)
    }
}

// ===== Phase E3 — OverlayBarBridge =====
//
// Implements SlintUiBridge so the ported overlay-backend fns (called
// via SlintEvents) can update the overlay bar UI + spawn tile windows.
// Tile spawning routes through an mpsc channel because slint::invoke_
// from_event_loop requires Send + 'static closures and TileWindow is
// not Send (Rc inside) — a Timer on the UI thread polls the channel
// and creates real TileWindows.
pub(crate) struct OverlayBarBridge {
    pub(crate) overlay_weak: slint::Weak<OverlayBarWindow>,
    pub(crate) spawn_tx: tokio_mpsc::UnboundedSender<SpawnTileRequest>,
    pub(crate) tile_seq: AtomicU64,
    /// Phase E6 v18 — last time we pushed a transcript:line update
    /// to the bar UI. Throttle to MIN_TRANSCRIPT_PUSH_INTERVAL so
    /// fast STT chunks (one every ~200ms in aggressive Whisper mode)
    /// don't flood invoke_from_event_loop and saturate the UI
    /// thread. Drops the IN-BETWEEN updates — user only ever cares
    /// about the LATEST transcript text anyway.
    pub(crate) last_transcript_push: std::sync::Mutex<std::time::Instant>,
    /// Phase E3 slice 2 — weak handle to the in-flight streaming
    /// tile plus per-tile accumulator. F9 ask handler synchronously
    /// creates a placeholder TileWindow, registers its weak here,
    /// then spawns `ask_stream_loop` which streams `ai:event`
    /// payloads back through `forward_event` and these updates land
    /// in THIS tile. Cleared on `AiEvent::Done` or `AiEvent::Error`.
    /// Mutex (not RwLock) because only one streaming tile at a time
    /// (rapid-F9 aborts the prior task).
    pub(crate) current_streaming: std::sync::Mutex<Option<StreamingTile>>,
    /// Phase E6 v11 — count of in-flight AI streams (auto-tiles run
    /// in parallel even though F9 is exclusive). Bar's ai-streaming
    /// flag mirrors `counter > 0`. Incremented on AiEvent::Start,
    /// decremented on AiEvent::Done/Error.
    pub(crate) ai_in_flight: std::sync::atomic::AtomicI32,
    /// Phase E6 v45 — per-tile conversations for the in-tile "continue
    /// dialog" feature, keyed by the tile's `convo-id`. Seeded when an
    /// F9/PTT answer completes; read+extended on each follow-up.
    pub(crate) conversations: std::sync::Mutex<std::collections::HashMap<i32, ConvoState>>,
    /// E9 — monotonic stream generation. `install_streaming_tile` bumps it
    /// per new ask; each spawned `ask_stream_loop` runs behind a
    /// `GenGatedEvents` wrapper carrying the generation it was spawned at.
    /// A superseded stream (older generation) has its emits DROPPED, so a
    /// buffered `ai:event` from a torn-down stream can't fold into the new
    /// tile (closes the wrong-tile race that `JoinHandle::abort` alone
    /// leaves open until the next .await).
    pub(crate) stream_gen: Arc<AtomicU64>,
    /// E9 — throttle for the streaming tile re-render. The Delta handler
    /// re-parses the WHOLE answer markdown per token; gating it to ~50ms
    /// bounds that cost independent of token speed. The terminal Done/Error
    /// render is never throttled, so the final answer always shows in full.
    pub(crate) last_tile_render: std::sync::Mutex<std::time::Instant>,
}

/// Per-streaming-tile state: weak handle + accumulated answer text.
/// Bridge re-renders the full markdown tree on every Delta — cheap
/// at <500 tokens, can be windowed later if needed.
pub(crate) struct StreamingTile {
    weak: slint::Weak<TileWindow>,
    accumulated: String,
    /// Phase E6 v45 (continue-dialog) — rendered markdown of the prior
    /// conversation turns. Each Delta re-renders `prefix + accumulated`
    /// so a follow-up answer appends BELOW the existing thread instead of
    /// replacing it. Empty for the first answer in a tile.
    prefix: String,
    /// Conversation key (mirrors the tile's `convo-id` property). On
    /// `AiEvent::Done` the finished answer is folded into
    /// `OverlayBarBridge::conversations[convo_id]` so the next follow-up
    /// carries full context. `-1` = this stream is not part of a
    /// continuable dialog (nothing is folded).
    convo_id: i32,
    /// The messages SENT for this turn (system + history + this user
    /// turn). On Done we append the assistant answer → the new history.
    request_messages: Vec<ai::ChatMessage>,
}

/// Phase E6 v45 — per-tile conversation, keyed by `convo-id`. Lets the
/// user keep asking inside one tile with full context. `messages` is the
/// running chat history (system + alternating user/assistant); `rendered`
/// is the markdown of the whole thread shown so far (used as the next
/// follow-up's `prefix`).
pub(crate) struct ConvoState {
    pub(crate) messages: Vec<ai::ChatMessage>,
    pub(crate) rendered: String,
}

/// Tile-spawn request sent from the bridge (any thread) to the UI
/// poll-Timer running on the Slint main thread. Carries everything
/// needed to construct a TileWindow + render the markdown body.
pub(crate) struct SpawnTileRequest {
    pub(crate) label: String,
    pub(crate) spec: TileSpec,
    /// Reserved for Phase E3 follow-up — pass through to a tile-
    /// placement helper that honors MonitorHint::Named (cfg.tile_
    /// monitor_name pin). Today apply_tile_hwnd_with_monitor reads
    /// config directly, so the hint is dropped on this Slint
    /// trajectory. TauriEvents adapter uses it for the React side.
    #[allow(dead_code, reason = "reserved for monitor-name routing")]
    monitor: MonitorHint,
    pub(crate) stealth: bool,
    pub(crate) kind: TileKind,
}

impl OverlayBarBridge {
    /// FIX #8 — store a tile's conversation, keeping the map bounded. When at
    /// the cap, evict the oldest half by `convo_id` (monotonic → lowest = oldest)
    /// BEFORE inserting, mirroring `qa_cache`'s half-eviction. The eviction is a
    /// backstop only — the primary prune is `drop_conversation` on tile close /
    /// MAX_LIVE_TILES eviction, so an open tile (which has one of the highest
    /// ids) is never the one dropped here.
    pub(crate) fn store_conversation(&self, convo_id: i32, state: ConvoState) {
        let mut convos = match self.conversations.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if convos.len() >= CONVERSATIONS_MAX_ENTRIES && !convos.contains_key(&convo_id) {
            let keys: Vec<i32> = convos.keys().copied().collect();
            for id in conversations_evict_keys(&keys, CONVERSATIONS_MAX_ENTRIES) {
                convos.remove(&id);
            }
        }
        convos.insert(convo_id, state);
    }

    /// FIX #8 — drop a tile's conversation when the tile is closed or evicted.
    /// No-op for a non-conversational tile (`convo_id < 0`, the tile.slint
    /// default) or one that never had an answer folded in. Keeps the map from
    /// growing one entry per completed answer for the life of the session.
    pub(crate) fn drop_conversation(&self, convo_id: i32) {
        if convo_id < 0 {
            return;
        }
        let mut convos = match self.conversations.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        convos.remove(&convo_id);
    }

    /// Handle `ai:event` separately because it needs to look up the
    /// `current_streaming` slot (per-call mutable state) before
    /// scheduling the UI mutation. Mutex is released BEFORE the
    /// invoke_from_event_loop call so the UI thread isn't blocked
    /// on the lock if the same code path re-enters.
    fn handle_ai_event(&self, payload: serde_json::Value) {
        let Ok(evt) = serde_json::from_value::<ai::AiEvent>(payload) else {
            return;
        };
        match evt {
            ai::AiEvent::Delta { text } => {
                let (weak, body) = {
                    let mut slot = match self.current_streaming.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    let Some(stream) = slot.as_mut() else {
                        return; // No active stream; drop the delta.
                    };
                    stream.accumulated.push_str(&text);
                    // Render prior thread + the live answer (continue-dialog):
                    // `prefix` is empty for the first answer, non-empty after
                    // a follow-up so the new answer appends below the thread.
                    let body = format!("{}{}", stream.prefix, stream.accumulated);
                    (stream.weak.clone(), body)
                };
                // Throttle the full-answer re-parse to ~50ms. The text is
                // already accumulated above; a skipped delta just defers its
                // repaint, and the terminal Done render shows the full answer.
                {
                    let now = std::time::Instant::now();
                    let mut last = match self.last_tile_render.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    if now.duration_since(*last) < std::time::Duration::from_millis(50) {
                        return;
                    }
                    *last = now;
                }
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(tile) = weak.upgrade() else {
                        return;
                    };
                    tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&body))));
                });
            }
            ai::AiEvent::Done { reason } => {
                self.dec_ai_in_flight();
                // Take the finished stream out of the slot, then fold its
                // answer into the tile's conversation so the next follow-up
                // carries full context.
                let finished = {
                    let mut slot = match self.current_streaming.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    slot.take()
                };
                if let Some(stream) = finished {
                    // Final body — used for the conversation snapshot AND the
                    // terminal render below (which is never throttled).
                    let final_body = format!("{}{}", stream.prefix, stream.accumulated);
                    if stream.convo_id >= 0 {
                        let mut messages = stream.request_messages;
                        messages.push(ai::ChatMessage {
                            role: "assistant".into(),
                            content: ai::MessageContent::Text(stream.accumulated.clone()),
                        });
                        // FIX #8 — bounded insert (caps + half-evicts the map).
                        self.store_conversation(
                            stream.convo_id,
                            ConvoState {
                                messages,
                                rendered: final_body.clone(),
                            },
                        );
                    }
                    let weak = stream.weak;
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(tile) = weak.upgrade() {
                            // Terminal render — NOT throttled, so the complete
                            // answer always shows even if the throttle skipped
                            // the last Delta repaint.
                            tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(
                                &final_body,
                            ))));
                            tile.set_source_label(SharedString::from(format!(
                                "ai · done ({reason})"
                            )));
                            tile.set_followup_busy(false);
                        }
                    });
                }
            }
            ai::AiEvent::Error { message } => {
                self.dec_ai_in_flight();
                let captured = {
                    let mut slot = match self.current_streaming.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    slot.take()
                };
                if let Some(stream) = captured {
                    let weak = stream.weak;
                    // SECURITY (review C1) — sanitise: the raw error chain embeds
                    // the LOCAL AI server's LAN IP:port, which would leak into the
                    // screen-shared tile. This streaming path is reached by F9 +
                    // voice follow-up (🎤). classify_ai_error → generic message.
                    let safe = classify_ai_error(&message);
                    // Keep any prior thread; append the error below it so a
                    // follow-up failure doesn't wipe the conversation.
                    let body = if stream.prefix.is_empty() {
                        format!("⚠ AI error: {safe}")
                    } else {
                        format!("{}\n\n⚠ AI error: {safe}", stream.prefix)
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(tile) = weak.upgrade() {
                            tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&body))));
                            tile.set_source_label(SharedString::from("⚠ error"));
                            tile.set_followup_busy(false);
                        }
                    });
                }
            }
            ai::AiEvent::Start { .. } => {
                // Phase E6 v11 — Start fires once per AI call (F9 +
                // each auto-tile). Bump the in-flight counter and
                // light the bar's ai-streaming pulse.
                self.inc_ai_in_flight();
            }
        }
    }

    /// Increment in-flight AI stream count and push the new state to
    /// the bar's ai-streaming flag (true if > 0).
    fn inc_ai_in_flight(&self) {
        let prev = self
            .ai_in_flight
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if prev == 0 {
            let weak = self.overlay_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(o) = weak.upgrade() {
                    o.set_ai_streaming(true);
                }
            });
        }
    }

    /// Decrement in-flight AI stream count. Clears the pulse when it
    /// reaches 0. Saturates at 0 in case Done fires without a paired
    /// Start (shouldn't happen but defensive).
    fn dec_ai_in_flight(&self) {
        let prev = self
            .ai_in_flight
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if prev <= 1 {
            // Clamp to 0 to recover from any unpaired Done/Error.
            self.ai_in_flight
                .store(0, std::sync::atomic::Ordering::SeqCst);
            let weak = self.overlay_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(o) = weak.upgrade() {
                    o.set_ai_streaming(false);
                }
            });
        }
    }

    /// Force the in-flight AI stream count to 0 and clear the bar pulse.
    /// Called from `install_streaming_tile` (each new ask supersedes the
    /// prior stream) and on `session:stopped`. An aborted `ask_stream_loop`
    /// never emits Done/Error, so its earlier Start increment would
    /// otherwise leak and pin the "AI streaming" pulse ON permanently after
    /// any rapid re-ask. Single-slot model makes reset-to-0 correct; a rare
    /// concurrent auto-tile at worst clears the pulse a beat early (cosmetic,
    /// never stuck).
    fn reset_ai_in_flight(&self) {
        self.ai_in_flight
            .store(0, std::sync::atomic::Ordering::SeqCst);
        let weak = self.overlay_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(o) = weak.upgrade() {
                o.set_ai_streaming(false);
            }
        });
    }
}

impl SlintUiBridge for OverlayBarBridge {
    fn forward_event(&self, channel: String, payload: serde_json::Value) {
        // ai:event has its own path because it needs mutable access
        // to current_streaming before scheduling the UI update.
        if channel == "ai:event" {
            self.handle_ai_event(payload);
            return;
        }
        // Phase E6 v18 — transcript:line throttle. STT in aggressive
        // Whisper mode produces ~5 events/sec; each schedules an
        // invoke_from_event_loop which the UI thread has to drain.
        // After 30s of streaming the queue is hundreds deep and the
        // bar (+ chip click handlers) becomes unresponsive. Drop
        // events that arrive within 200ms of the previous push —
        // user only ever sees the LATEST line anyway.
        if channel == "transcript:line" {
            const MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);
            let now = std::time::Instant::now();
            let mut last = match self.last_transcript_push.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if now.duration_since(*last) < MIN_INTERVAL {
                return;
            }
            *last = now;
        }
        // A stopped session leaves no live AI stream — clear any leaked
        // in-flight count so the "AI streaming" pulse can't stick ON
        // (an aborted stream emits no Done/Error to decrement it).
        if channel == "session:stopped" {
            self.reset_ai_in_flight();
            // M2 — finalize a tile that was still streaming when the session
            // stopped. stop_session aborts the ai_task, so NO Done/Error ever
            // arrives to take the slot or re-enable the tile: the tile would
            // freeze forever on its partial answer with a disabled follow-up,
            // until some LATER F9 happened to supersede the slot. Take the slot
            // here and mark the tile interrupted, preserving whatever streamed
            // so far. We deliberately do NOT fold the partial answer into the
            // conversation — a later follow-up should build on the last COMPLETE
            // turn, not a truncated one (and a never-completed first answer keeps
            // no convo entry, so its follow-up bails cleanly).
            let interrupted = {
                let mut slot = match self.current_streaming.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                slot.take()
            };
            if let Some(stream) = interrupted {
                let body = format!("{}{}", stream.prefix, stream.accumulated);
                let weak = stream.weak;
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(tile) = weak.upgrade() {
                        if !body.is_empty() {
                            tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&body))));
                        }
                        tile.set_source_label(SharedString::from(
                            "ai · прервано (сессия остановлена)",
                        ));
                        tile.set_followup_busy(false);
                    }
                });
            }
        }
        let weak = self.overlay_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(o) = weak.upgrade() else {
                return;
            };
            match channel.as_str() {
                "cost:update" => {
                    if let Some(usd) = payload.get("session_usd").and_then(|v| v.as_f64()) {
                        o.set_cost_label(SharedString::from(format!("${usd:.3}")));
                    }
                }
                "session:started" => {
                    o.set_timer_active(true);
                    o.set_status_text(SharedString::from("recording"));
                    o.set_status_color(slint::Color::from_rgb_u8(0x2a, 0xc7, 0x60));
                }
                "session:stopped" => {
                    o.set_timer_active(false);
                    o.set_status_text(SharedString::from("idle"));
                    o.set_status_color(slint::Color::from_rgb_u8(0x88, 0x88, 0x8c));
                }
                "health:update" => {
                    // Crude: collapse 3-subsystem state to single
                    // status color until the bar gets dedicated dots.
                    let st = |k: &str| -> Option<&str> {
                        payload.get(k).and_then(serde_json::Value::as_str)
                    };
                    let ai_down = matches!(st("ai"), Some("down"));
                    let any_down = matches!(st("audio"), Some("down"))
                        || matches!(st("stt"), Some("down"))
                        || ai_down;
                    let any_degraded = matches!(st("audio"), Some("degraded"))
                        || matches!(st("stt"), Some("degraded"))
                        || matches!(st("ai"), Some("degraded"));
                    // v0.8.2 (C1 fix, cont.) — gate the down/degraded COLOR on an
                    // active session too (mirrors the TEXT guard below). Else a
                    // stale post-stop {ai:down} tick (queued before the emitter was
                    // aborted) repaints the idle bar red until the next
                    // session:started, leaving "idle" text inside a red pill.
                    if o.get_timer_active() {
                        if any_down {
                            o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0x4b, 0x4b));
                        } else if any_degraded {
                            o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0xb4, 0x4b));
                        } else {
                            // All-clear during a session → restore the green
                            // recording pill. A degraded→ok episode only ever set
                            // the COLOR (never the AI_DOWN_MARK text), so the
                            // text-recovery branch below can't restore it and the
                            // pill would otherwise stay amber until the next
                            // session start/stop.
                            o.set_status_color(slint::Color::from_rgb_u8(0x2a, 0xc7, 0x60));
                        }
                    }
                    // V0.8.0 (Поток A) — surface AI-down in the bar TEXT, not just
                    // color, so the user knows WHY auto-tiles stopped (the
                    // reported pain). The marker is set/cleared only by this arm,
                    // so we restore the session pill on recovery without
                    // clobbering session:started/stopped's own text.
                    const AI_DOWN_MARK: &str = "⚠ AI недоступен";
                    let cur = o.get_status_text();
                    // v0.8.2 (C1 fix) — only SET the mark while a session is
                    // active (timer_active). Without this guard a stale
                    // health:update{ai:down} that the aborted emitter queued just
                    // before stop_session could land AFTER session:stopped set
                    // "idle"; with the emitter now dead nothing would ever clear
                    // it, stranding the bar on "⚠ AI недоступен" over an idle
                    // session — exactly when the user stops to go fix the bridge.
                    // The clear branch stays unguarded so it can still tidy up.
                    if ai_down && o.get_timer_active() {
                        if cur != AI_DOWN_MARK {
                            o.set_status_text(SharedString::from(AI_DOWN_MARK));
                        }
                    } else if cur == AI_DOWN_MARK {
                        // Recovered — restore the session pill we overwrote.
                        if o.get_timer_active() {
                            o.set_status_text(SharedString::from("recording"));
                            o.set_status_color(slint::Color::from_rgb_u8(0x2a, 0xc7, 0x60));
                        } else {
                            o.set_status_text(SharedString::from("idle"));
                            o.set_status_color(slint::Color::from_rgb_u8(0x88, 0x88, 0x8c));
                        }
                    }
                    // ok / idle leaves the prior color alone
                    // (set by session:started / session:stopped).
                }
                "meeting:ending" => {
                    o.set_status_text(SharedString::from("🏁 wrapping up"));
                }
                "transcript:line" => {
                    // Phase E6 v11 — surface latest STT on bar.
                    // (Throttle handled UPSTREAM in forward_event
                    // before invoke_from_event_loop is scheduled —
                    // see the early return in forward_event.)
                    let text = payload
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    let source = payload
                        .get("source")
                        .and_then(|v| v.as_str())
                        .map(|s| match s {
                            "Mic" => "mic",
                            "System" => "sys",
                            _ => "",
                        })
                        .unwrap_or("");
                    let truncated: String = text.chars().take(120).collect();
                    o.set_last_transcript_line(SharedString::from(truncated));
                    o.set_last_transcript_source(SharedString::from(source));
                }
                "tile:error" | "tile:rate-limited" | "cost:cap-hit" | "speech:coach" => {
                    // No toast UI yet — log so developer sees these
                    // during testing without losing them.
                    eprintln!("[overlay-bridge] {channel}: {payload}");
                }
                other => {
                    eprintln!("[overlay-bridge] unknown channel '{other}'");
                }
            }
        });
    }

    fn schedule_spawn_tile(
        &self,
        spec: TileSpec,
        monitor: MonitorHint,
        stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        let n = self.tile_seq.fetch_add(1, Ordering::Relaxed);
        let label = format!("slint-tile-{n}");
        let req = SpawnTileRequest {
            label: label.clone(),
            spec,
            monitor,
            stealth,
            kind,
        };
        self.spawn_tx
            .send(req)
            .map_err(|e| format!("tile-spawn channel send failed: {e}"))?;
        Ok(label)
    }
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
    // Strip any FOLLOWUP_DIRECTIVE left on PRIOR user turns before wrapping this
    // one — the directive is stored in history, so a multi-turn thread would
    // otherwise carry several of them and confuse a weak local model. Only the
    // turn pushed just below should carry it.
    strip_followup_directives(&mut messages);
    messages.push(ai::ChatMessage {
        role: "user".into(),
        content: ai::MessageContent::Text(format!("{FOLLOWUP_DIRECTIVE}{question}")),
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
    let (base_url, bearer, model, is_local, max_tokens) = {
        let c = cfg.read();
        let ep = route.endpoint(&c);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            ep.is_local,
            route.max_tokens(),
        )
    };
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
    let sys_full = messages
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
        let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), messages, max_tokens);
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
    let (base_url, bearer, model, is_local, max_tokens) = {
        let c = cfg.read();
        let ep = route.endpoint(&c);
        (
            ep.base_url,
            ep.bearer,
            ep.model,
            ep.is_local,
            route.max_tokens(),
        )
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
    let sys_full = messages
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
        let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), messages, max_tokens);
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

/// Atomic counter for tile-slot index — increments per spawn so
/// successive tiles distribute across a 2-column grid on the right
/// half of the chosen monitor.
static TILE_SLOT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Monotonic counter for the tile-title #N badge. Increments per
/// spawn (never wraps) so the user can tell tiles apart in a busy
/// session. Reset only at process restart.
pub(crate) static TILE_DISPLAY_SEQ: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Phase E6 v17 — maximize toggle helper. User: "нет функционала
/// развернуть, нужно отдельной кнопкой или даб-кликом". Maximized
/// tile is 800×600 (~1.7× default); restored back to 460×360. Uses
/// Win32 SetWindowPos with current position so the tile expands in
/// place from its top-left corner. Flips tile.maximized so the
/// button glyph updates.
pub(crate) fn toggle_tile_maximize(hwnd: windows::Win32::Foundation::HWND, tile: &TileWindow) {
    // Phase E6 v18 fix — use Slint's window().set_size() not raw
    // Win32 SetWindowPos. SetWindowPos resized the OS window but
    // left Slint's layout pass thinking the size was still 460×360
    // → chrome buttons (pin/max/X) stayed at old logical positions
    // → user clicks hit dead space. set_size goes through the Slint
    // engine which both updates the OS window AND re-runs layout.
    // Fixes: "когда я развернул окно, другой его функционал завис".
    let new = !tile.get_maximized();
    let (w, h): (f32, f32) = if new { (800.0, 600.0) } else { (460.0, 360.0) };
    tile.window().set_size(slint::LogicalSize::new(w, h));
    tile.set_maximized(new);

    // Phase E6 v45 — keep the resized tile fully on-screen. Growing in
    // place from the top-left pushed tiles near a screen edge/corner off
    // the monitor (user: "тайл у угла раскрывается за экран"). Work in
    // PHYSICAL pixels (logical × DPI scale) since Win32 rects/positions
    // are physical, then nudge the origin back inside the tile's monitor.
    let scale = tile.window().scale_factor();
    let pw = (w * scale) as i32;
    let ph = (h * scale) as i32;
    // Clamp against the WORK AREA (monitor minus taskbar) of the tile's
    // own monitor so a maximized tile near an edge/corner stays fully
    // visible AND its bottom row (the follow-up input) clears the taskbar.
    if let (Ok((x, y, _r, _b)), Some(m)) = (get_window_rect(hwnd), work_area_for_window(hwnd)) {
        let mut nx = x;
        let mut ny = y;
        // Pull the right/bottom edges inside first, then guarantee the
        // top-left stays visible (matters if the tile is wider/taller
        // than the work area — keep the top-left corner reachable).
        if nx + pw > m.right {
            nx = m.right - pw;
        }
        if ny + ph > m.bottom {
            ny = m.bottom - ph;
        }
        if nx < m.left {
            nx = m.left;
        }
        if ny < m.top {
            ny = m.top;
        }
        if nx != x || ny != y {
            let _ = move_window_pos_only(hwnd, nx, ny);
        }
    }
    diag!("tile maximized -> {new} (logical {w}x{h}, phys {pw}x{ph})");
}

/// Wire the chrome-row drag callbacks on a tile so the user can move
/// it by pressing+dragging the title area. Phase E6 v22 — manual
/// cursor-delta drag (drag_begin on down, drag_update on move-while-
/// pressed). REPLACES the old WM_NCLBUTTONDOWN modal system-drag
/// which consumed the mouse-up before Slint saw it, leaving the
/// TouchArea stuck "pressed" → tile became undraggable/unclickable.
/// User: "вызванный тайл завис, двигается но ничего не прожимается".
pub(crate) fn wire_tile_drag(tile: &TileWindow) {
    // Seed this tile's Theme global from the process-global scheme. Called on
    // every tile-creation path, so newly-spawned tiles inherit the live choice
    // without threading the value through 5 call sites.
    apply_scheme_tile(tile, global_scheme());
    let weak = tile.as_weak();
    tile.on_drag_start_requested(move || {
        if let Some(t) = weak.upgrade() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                drag_begin(hwnd);
            }
        }
    });
    let weak_move = tile.as_weak();
    tile.on_drag_moved(move || {
        if let Some(t) = weak_move.upgrade() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                drag_update(hwnd);
            }
        }
    });
}

/// Apply transparency + position tile on the appropriate monitor.
///
/// Show a freshly-built tile WITHOUT a stealth capture-flash.
///
/// Bug: under stealth, every tile used to be `show()`n on-screen at winit's
/// default position and only stealthed ~200 ms later (WDA_EXCLUDEFROMCAPTURE
/// needs a realized HWND — see `apply_tile_hwnd_with_monitor`). For that gap the
/// tile was fully capturable, so a screen-share saw a ~0.1 s flash of the tile.
///
/// Fix: when stealth is on, park the window OFF the virtual desktop BEFORE its
/// first frame, so winit realizes the HWND off-screen and the tile is never
/// composited onto a real monitor. `apply_tile_hwnd_with_monitor` then applies
/// WDA *before* it moves the tile on-screen, so the first on-screen frame is
/// already excluded from capture. Same pattern the persistent capture overlay
/// uses. When stealth is off there's nothing to hide, so show normally.
pub(crate) fn present_tile_window(tile: &TileWindow) {
    if global_stealth() {
        tile.window()
            .set_position(slint::PhysicalPosition::new(-32000, -32000));
    }
    let _ = tile.show();
}

/// Phase E6 fix v2 (2026-05-27): previous "right-edge stack" math
/// overflowed monitor.bottom after ~slot 2 (tile_h+12 × N > screen
/// height) → user complaint "тайлы уходят за экран". Now uses a
/// 2-column × dynamic-rows grid with hard clamps to monitor bounds.
/// Pre-port React/Tauri used src-tauri's tile.rs::grid_position
/// (~80 LOC of layered math); this is a simpler 2-col wrap that
/// fits on any landscape monitor without overflow.
pub(crate) fn apply_tile_hwnd_with_monitor(tile: &TileWindow) {
    // Phase E6 v36 — every spawn path funnels through here, so this is
    // the one place to apply the saved tile body opacity. Without this,
    // only tiles that existed when the Settings slider moved went
    // transparent; freshly spawned tiles reset to opaque (user bug
    // report). Set synchronously on the passed handle so it takes
    // effect on the first painted frame.
    tile.set_body_opacity(global_tile_opacity());

    let weak = tile.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
        let Some(t) = weak.upgrade() else { return };
        let Ok(hwnd) = grab_hwnd(t.window()) else {
            return;
        };

        // Phase E6 fix v4 — use make_transparent_tile (no WS_EX_
        // TRANSPARENT) so tiles accept clicks for buttons + drag.
        // Previous make_transparent_overlay set WS_EX_TRANSPARENT
        // which made every click pass through to underlying windows
        // (Explorer/desktop), silently swallowing every chrome-row
        // press → drag-to-move never fired. Same root cause as user
        // complaint "тайлы нельзя двигать".
        let _ = make_transparent_tile(hwnd);

        // Phase E6 v5 — Slint's `always-on-top: true` declaration is
        // applied at window creation but doesn't reliably translate
        // to HWND_TOPMOST on Windows + winit + skia. Explicitly set
        // HWND_TOPMOST so tile windows sit above Explorer / desktop
        // / browser windows and the user can interact with them.
        // Without this, clicks land on whatever non-topmost window
        // is at the pixel under the tile.
        let _ = set_always_on_top(hwnd, true);

        // #111 — inherit stealth: a tile spawned while stealth is on must
        // also be excluded from screen capture (the toggle only covered tiles
        // that already existed). No-op when stealth is off.
        if global_stealth() {
            let _ = set_stealth(hwnd, true);
        }

        // Phase E6 fix v3 — read the ACTUAL physical window size that
        // Slint produced (HiDPI-aware), then place using that real
        // width so the right-edge alignment is accurate. Previous
        // version forced TILE_DEFAULT_W (460 raw pixels) which
        // overrode Slint's logical-to-physical scaling and made
        // tile content overflow the dark fill area on 125% scaling.
        let (_cur_x, _cur_y, real_w, real_h) =
            get_window_rect(hwnd).unwrap_or((0, 0, TILE_DEFAULT_W, TILE_DEFAULT_H));

        let monitors = enum_monitors();
        if let Some(mon) = pick_monitor(&monitors) {
            let gap_x: i32 = 12;
            let gap_y: i32 = 12;
            let top_margin: i32 = 80;
            let right_margin: i32 = 20;

            let usable_h = mon.height().saturating_sub(top_margin + 20);
            let rows = ((usable_h + gap_y) / (real_h + gap_y)).max(1) as usize;
            let cols: usize = 2;
            let total_slots = (rows * cols).max(1);

            // Phase E6 v9 — cascade-offset on wrap. Previously
            // `slot = COUNTER % total_slots` made the 5th+ tile land
            // ON TOP of the 1st tile, etc. User complaint: "потом
            // они начали друг на друга прыгать". Now: track which
            // cycle (wraparound generation) we're on, and offset
            // every wrapped tile by (cascade_dx, cascade_dy) per
            // cycle — visually a stagger like macOS cascade-windows.
            // Hard clamps still prevent off-screen.
            let raw_seq = TILE_SLOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let slot = raw_seq % total_slots;
            let cycle = raw_seq / total_slots; // 0 for first batch, 1 for second, etc.
            let cascade_dx: i32 = 32;
            let cascade_dy: i32 = 24;
            let row = slot / cols;
            let col = slot % cols;

            let x_outer = mon.left + mon.width() - real_w - right_margin;
            let x_inner = x_outer - real_w - gap_x;
            let x_base = if col == 0 { x_inner } else { x_outer };
            let y_base = mon.top + top_margin + (row as i32) * (real_h + gap_y);

            // Cascade offset grows leftward + downward so wrapped tiles
            // peek out from under their first-cycle siblings. Negative
            // dx on x because the right-cluster is already at right edge.
            let x = x_base - (cycle as i32) * cascade_dx;
            let y = y_base + (cycle as i32) * cascade_dy;

            // Hard clamp so a tile can never land off-screen even if
            // monitor enum returned weird coordinates (portrait
            // secondary at negative x). The max bound is `.max()`'d with the
            // min so a tile WIDER/TALLER than the monitor (possible on the
            // 1200px portrait secondary, or under heavy DPI) can't make
            // max < min and panic `i32::clamp` — it just pins to the top-left
            // margin instead of crashing.
            let x_min = mon.left + 8;
            let x_max = (mon.right - real_w - 8).max(x_min);
            let y_min = mon.top + 8;
            let y_max = (mon.bottom - real_h - 8).max(y_min);
            let x_clamped = x.clamp(x_min, x_max);
            let y_clamped = y.clamp(y_min, y_max);

            eprintln!(
                "[overlay-host] tile placement: monitor=({},{},{},{}) real_size=({},{}) slot={} cycle={} row={} col={} pos=({},{})",
                mon.left, mon.top, mon.right, mon.bottom,
                real_w, real_h, slot, cycle, row, col, x_clamped, y_clamped,
            );
            // Move-only — preserve Slint's natural size so HiDPI
            // rendering stays correct (text fills the dark fill area
            // instead of overflowing).
            let _ = move_window_pos_only(hwnd, x_clamped, y_clamped);
        } else {
            // No monitor from pick_monitor (degenerate — no primary display).
            // A stealth-parked tile would otherwise stay off the virtual desktop
            // (permanently invisible), so bring it back to a safe on-screen spot.
            let _ = move_window_pos_only(hwnd, 100, 100);
            eprintln!("[overlay-host] tile placement: no monitor from pick_monitor — fallback to (100, 100)");
        }
    });
}

#[cfg(test)]
mod copy_tests {
    //! Locks the 📋-copy text derivation — the exact area the user hit live:
    //! copy pulling in the raw Mic/System transcript, and follow-ups being
    //! re-answered as the original question. Pure: no bridge, no UI, no network.
    use super::*;

    fn msg(role: &str, text: &str) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Text(text.to_string()),
        }
    }
    fn parts_msg(role: &str, texts: &[&str]) -> ai::ChatMessage {
        ai::ChatMessage {
            role: role.to_string(),
            content: ai::MessageContent::Parts(
                texts
                    .iter()
                    .map(|t| ai::ContentPart::Text {
                        text: (*t).to_string(),
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn message_text_text_and_parts() {
        assert_eq!(
            message_text(&ai::MessageContent::Text("plain".into())),
            "plain"
        );
        // Parts: text parts are joined (image parts, when present, contribute
        // nothing — exercised here with two text parts).
        let m = parts_msg("user", &["hello", "world"]);
        assert_eq!(message_text(&m.content), "hello\nworld");
    }

    #[test]
    fn copy_question_strips_transcript_wrapper() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] arc raiders?\n\n\
                   Помоги ответить: что такое arc raiders";
        assert_eq!(user_question_for_copy(raw), "что такое arc raiders");
    }

    #[test]
    fn conversations_evict_keys_drops_oldest_half_keeps_newest() {
        // FIX #8 — at the cap, the lowest-id half (oldest tiles) is evicted,
        // and the highest ids (newest / currently-open tiles) are kept.
        let keys: Vec<i32> = (0..256).collect();
        let evicted = conversations_evict_keys(&keys, 256);
        assert_eq!(evicted.len(), 128, "evicts exactly half the cap");
        assert_eq!(evicted.first(), Some(&0), "oldest id is evicted");
        assert_eq!(evicted.last(), Some(&127), "eviction stops at the midpoint");
        assert!(
            !evicted.contains(&255),
            "the newest id (an open tile) is never evicted"
        );
        // Unsorted input is handled (HashMap key order is arbitrary).
        let shuffled = [50, 3, 200, 7, 99];
        let mut e = conversations_evict_keys(&shuffled, 4); // max/2 = 2 → drop 2 lowest
        e.sort_unstable();
        assert_eq!(
            e,
            vec![3, 7],
            "drops the two lowest ids regardless of order"
        );
    }

    #[test]
    fn copy_question_drops_transcript_only_ask() {
        let raw = "Транскрипт последних реплик:\n[СОБЕСЕДНИК] что-то сказал";
        assert_eq!(user_question_for_copy(raw), "");
    }

    #[test]
    fn copy_question_strips_followup_directive() {
        let raw = format!("{FOLLOWUP_DIRECTIVE}а что дальше?");
        assert_eq!(user_question_for_copy(&raw), "а что дальше?");
    }

    #[test]
    fn copy_question_drops_canned_vision_prompt() {
        assert_eq!(user_question_for_copy(vision::DEFAULT_VISION_PROMPT), "");
    }

    #[test]
    fn copy_question_drops_translate_vision_prompt() {
        // Feature #3 — a translate tile's first turn is the canned translate
        // prompt, not user-typed text → drop it (both phonetics states; the ON
        // variant is base+suffix, so starts_with the base still matches).
        assert_eq!(user_question_for_copy(vision::TRANSLATE_VISION_PROMPT), "");
        assert_eq!(user_question_for_copy(&vision::translate_prompt(true)), "");
    }

    #[test]
    fn copy_question_passes_plain_text_trimmed() {
        assert_eq!(user_question_for_copy("  привет  "), "привет");
    }

    #[test]
    fn single_turn_copies_only_the_answer() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg("user", "Помоги ответить: что такое Rust"),
            msg("assistant", "Rust — системный язык."),
        ];
        assert_eq!(
            format_convo_copy(&msgs, "RENDERED"),
            "Rust — системный язык."
        );
    }

    #[test]
    fn multi_turn_copies_labelled_thread_without_transcript() {
        let msgs = vec![
            msg("system", "ты ассистент"),
            msg(
                "user",
                "Транскрипт последних реплик:\n[СОБЕСЕДНИК] x\n\nПомоги ответить: вопрос 1",
            ),
            msg("assistant", "ответ 1"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос 2")),
            msg("assistant", "ответ 2"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "🧑 вопрос 1\n\n🤖 ответ 1\n\n🧑 вопрос 2\n\n🤖 ответ 2"
        );
        // The raw Mic/System transcript must never reach the clipboard.
        assert!(!out.contains("СОБЕСЕДНИК"));
    }

    #[test]
    fn multi_turn_vision_skips_canned_prompt() {
        let msgs = vec![
            parts_msg("user", &[vision::DEFAULT_VISION_PROMPT]),
            msg("assistant", "на экране код"),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}а на каком языке?")),
            msg("assistant", "на Rust"),
        ];
        let out = format_convo_copy(&msgs, "RENDERED");
        assert_eq!(
            out,
            "🤖 на экране код\n\n🧑 а на каком языке?\n\n🤖 на Rust"
        );
    }

    #[test]
    fn empty_conversation_falls_back_to_rendered() {
        assert_eq!(format_convo_copy(&[], "RENDERED"), "RENDERED");
    }

    #[test]
    fn strip_directives_cleans_user_turns_only() {
        let mut msgs = [
            msg("system", &format!("{FOLLOWUP_DIRECTIVE}sys")),
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}вопрос")),
            msg("assistant", &format!("{FOLLOWUP_DIRECTIVE}ответ")),
            msg("user", "уже чистый"),
        ];
        strip_followup_directives(&mut msgs);
        // system + assistant turns are untouched (only user turns get cleaned).
        assert_eq!(
            message_text(&msgs[0].content),
            format!("{FOLLOWUP_DIRECTIVE}sys")
        );
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}ответ")
        );
        // user turns are stripped; an already-clean one is unchanged.
        assert_eq!(message_text(&msgs[1].content), "вопрос");
        assert_eq!(message_text(&msgs[3].content), "уже чистый");
    }

    #[test]
    fn strip_all_but_last_preserves_reasked_turn() {
        // Mirrors fire_regenerate's `&mut messages[..len-1]`: prior turns are
        // cleaned, but the last (re-asked) turn keeps whatever framing it had.
        let mut msgs = [
            msg("user", &format!("{FOLLOWUP_DIRECTIVE}старый вопрос")),
            msg("assistant", "старый ответ"),
            msg(
                "user",
                &format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос"),
            ),
        ];
        let n = msgs.len() - 1;
        strip_followup_directives(&mut msgs[..n]);
        // Prior user turn is cleaned…
        assert_eq!(message_text(&msgs[0].content), "старый вопрос");
        // …but the last (re-asked) turn keeps its direct-question framing.
        assert_eq!(
            message_text(&msgs[2].content),
            format!("{FOLLOWUP_DIRECTIVE}перезапрашиваемый вопрос")
        );
    }
}
