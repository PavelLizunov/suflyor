//! Tile controller: the tile-streaming / conversation-WRITE machinery
//! (Phase 7a of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.10). Wave 2 of the
//! `tile_controller` split carved the ASK-INITIATION side (the route model + the
//! `fire_*`/`wire_*` ask/follow-up/escalate entrypoints + the cost-cap helpers)
//! out into `tile_ask.rs`; what's left here is the STREAM-WRITE side — which
//! messages get STORED into a tile's conversation.
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
//!   now-current tile (the `tile_ask.rs` ask entrypoints register through these);
//! - the per-PTT independent sink (`PttStreamSink`/`PttSinkState`) so two rapid
//!   push-to-talk asks don't clobber each other (`fire_ptt_ask`, in `tile_ask.rs`,
//!   constructs it via the crate-root glob);
//! - `StreamingTile`/`ConvoState`/`SpawnTileRequest` + the conversation map
//!   eviction backstop (`conversations_evict_keys`, `CONVO_SEQ`,
//!   `CONVERSATIONS_MAX_ENTRIES`).
//!
//! What MOVED to `tile_ask.rs` (reached here / there through the crate-root
//! glob): the route model (`AskRoute`/`LiveRoute`/`live_route`), the ask
//! entrypoints (`fire_f3_reask`/`fire_f6_manual_spawn`/`fire_f9_ask`/
//! `fire_ptt_ask`), the follow-up/escalate flow (`fire_followup_ask`/
//! `fire_regenerate`/`wire_escalate`/`wire_voice_followup` + its `VFU_TX`
//! drain sender), the PTT helpers (`spawn_ptt_watchdog`/`ptt_tile_error`), and
//! the cost/transcript helpers (`warn_if_over_cost_cap`/`cost_cap_reason`/
//! `select_recent_labeled`).
//!
//! What STAYS in `overlay_host.rs` (reached here through the glob below):
//! - `fn main` (constructs the `OverlayBarBridge`, owns the hotkey DISPATCH +
//!   bar-chip wiring + the spawn-tile / voice-follow-up drain timers — all of
//!   which call the moved `fire_*` / `wire_*` / `install_streaming_tile` via the
//!   `use tile_controller::*;` / `use tile_ask::*;` re-exports);
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
use super::{
    ai, classify_ai_error, to_md_blocks, tokio_mpsc, Arc, AtomicU64, ModelRc, MonitorHint,
    Ordering, OverlayBarWindow, RuntimeEvents, SharedString, SlintUiBridge, TileKind, TileSpec,
    TileWindow, VecModel,
};

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
pub(crate) fn conversations_evict_keys(keys: &[i32], max: usize) -> Vec<i32> {
    let mut ids = keys.to_vec();
    ids.sort_unstable();
    ids.truncate(max / 2);
    ids
}

/// Install `new_tile` as the active streaming tile, FIRST clearing the
/// slot's previous occupant. The single `current_streaming` slot is shared
/// across F9/PTT/follow-up; starting a new stream aborts the prior task,
/// which then emits no Done/Error — so without this the superseded tile
/// would keep `followup-busy = true` (a permanently dead input). Must run
/// on the UI thread (every ask path registers from a UI-thread callback or
/// timer), so the direct `upgrade()` + setter is safe.
pub(crate) fn install_streaming_tile(
    bridge: &Arc<OverlayBarBridge>,
    new_tile: StreamingTile,
) -> u64 {
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
pub(crate) fn gated_events(
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
                            "AI error: {safe}"
                        )))));
                        t.set_source_label(SharedString::from("error"));
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
    pub(crate) weak: slint::Weak<TileWindow>,
    pub(crate) accumulated: String,
    /// Phase E6 v45 (continue-dialog) — rendered markdown of the prior
    /// conversation turns. Each Delta re-renders `prefix + accumulated`
    /// so a follow-up answer appends BELOW the existing thread instead of
    /// replacing it. Empty for the first answer in a tile.
    pub(crate) prefix: String,
    /// Conversation key (mirrors the tile's `convo-id` property). On
    /// `AiEvent::Done` the finished answer is folded into
    /// `OverlayBarBridge::conversations[convo_id]` so the next follow-up
    /// carries full context. `-1` = this stream is not part of a
    /// continuable dialog (nothing is folded).
    pub(crate) convo_id: i32,
    /// The messages SENT for this turn (system + history + this user
    /// turn). On Done we append the assistant answer → the new history.
    pub(crate) request_messages: Vec<ai::ChatMessage>,
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
                    // An EMPTY answer means the model emitted a tool call (e.g. a
                    // web-search tool the cloud bridge offers) instead of text —
                    // which this text-stream client can't execute, so the tile
                    // would otherwise render blank ("найди в интернете…" → nothing).
                    // Show a generic note (no endpoint/tool internals) and do NOT
                    // fold an empty assistant turn into the conversation.
                    let answer_empty = stream.accumulated.trim().is_empty();
                    // Final body — used for the conversation snapshot AND the
                    // terminal render below (which is never throttled).
                    let final_body = if answer_empty {
                        let note = "_(Модель не вернула текст — вероятно, запросила \
                            инструмент вроде веб-поиска, который в этом режиме не \
                            поддерживается. Переформулируй вопрос без «найди/загугли».)_";
                        if stream.prefix.is_empty() {
                            note.to_string()
                        } else {
                            format!("{}\n\n{note}", stream.prefix)
                        }
                    } else {
                        format!("{}{}", stream.prefix, stream.accumulated)
                    };
                    if stream.convo_id >= 0 && !answer_empty {
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
                        format!("AI error: {safe}")
                    } else {
                        format!("{}\n\nAI error: {safe}", stream.prefix)
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(tile) = weak.upgrade() {
                            tile.set_blocks(ModelRc::new(VecModel::from(to_md_blocks(&body))));
                            tile.set_source_label(SharedString::from("error"));
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
                    const AI_DOWN_MARK: &str = "AI недоступен";
                    let cur = o.get_status_text();
                    // v0.8.2 (C1 fix) — only SET the mark while a session is
                    // active (timer_active). Without this guard a stale
                    // health:update{ai:down} that the aborted emitter queued just
                    // before stop_session could land AFTER session:stopped set
                    // "idle"; with the emitter now dead nothing would ever clear
                    // it, stranding the bar on "AI недоступен" over an idle
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
                    // UI-audit 2026-06-13: dropped the 🏁 flag emoji — the rest
                    // of the chrome is SVG/ASCII; the status pill is English-only
                    // by design (idle/recording/…), so this matches it.
                    o.set_status_text(SharedString::from("wrapping up"));
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
