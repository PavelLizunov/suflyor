// Release builds run without a console window (no black cmd window on
// launch — user feedback). Debug builds KEEP the console so `eprintln!`
// tracing is visible during development. Diagnostics in release go to
// %APPDATA%\overlay-mvp\overlay-host.log via `slint_replay::logging`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Phase 1 Day 2 + Phase 3 — multi-window manager with real overlay bar.
//!
//! Spawns the overlay bar with a full chip set (status pill, mic/sys
//! capture chips, session timer, AI model selector, cost, tips,
//! bookmark, stealth, +Tile, ⚙ Settings, ✕ Quit).
//!
//! All callbacks update the shared AppState. Stealth toggle applies
//! WDA_EXCLUDEFROMCAPTURE to overlay + all open tiles via win32 helpers.
//! Tile spawn uses pick_monitor + move_window for proper multi-monitor
//! placement (respects user's portrait-secondary setup).
//!
//! Run: `cargo run --bin overlay-host` from `slint-experiment/`.

use overlay_backend::events::{MonitorHint, RuntimeEvents, TileKind, TileSpec};
use overlay_backend::{ai, audio, config, journal, kb, stt, vision};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_replay::app_state::{format_timer, new_shared_state};
use slint_replay::markdown;
use slint_replay::runtime_state::{shared_runtime, SharedSlintRuntime};
use slint_replay::slint_events::{SlintEvents, SlintUiBridge};
use slint_replay::slint_session;
use slint_replay::win32::{
    drag_begin, drag_update, enum_monitors, focus_window, get_window_rect, grab_hwnd,
    make_transparent_overlay, make_transparent_tile, move_window_pos_only, pick_monitor,
    set_always_on_top, set_skip_taskbar, set_stealth, work_area_for_window,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

/// Diagnostic log line → `%APPDATA%\overlay-mvp\overlay-host.log` AND
/// stderr (debug builds keep a console; release has none). Use for
/// lifecycle + error events worth keeping for tester debugging. NEVER
/// pass secrets (API keys) — log presence booleans, not values.
macro_rules! diag {
    ($($arg:tt)*) => {
        slint_replay::logging::line(&format!($($arg)*))
    };
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use ui::{
    CaptureOverlay, HelpWindow, MarkdownBlock, OverlayBarWindow, PaletteResult, PaletteWindow,
    RecoverOfferWindow, SettingsWindow, TextAskWindow, TileWindow, WizardWindow,
};

// Phase 1 of the modularization (docs/overlay-host-modularization-plan.md §5.1):
// window lifecycle + stealth/theme registry lives in its own file alongside the
// binary. `use window_lifecycle::*;` re-exports the moved globals/getters/setters
// (`set_global_stealth`/`global_stealth`/`set_global_scheme`/…),
// `present_window_stealth_aware`, the `apply_scheme_*` helpers, `refresh_open_tiles`,
// `clamp_scheme`, and the new `WindowRegistry` so existing call sites resolve
// unchanged.
#[path = "overlay_host/window_lifecycle.rs"]
mod window_lifecycle;
use window_lifecycle::*;

// Phase 2 of the modularization (docs/overlay-host-modularization-plan.md §5.2):
// diagnostics readiness population + the REDACTED clipboard report live in their
// own file alongside the binary. `use diagnostics::*;` re-exports the moved
// `populate_diagnostics`, `build_diag_report`, and the redaction helpers
// (`redact_ipv4`/`redact_urls`/`is_ipv4`) so existing call sites — and the
// Settings-tab `Check all` / `Copy report` closures that will move in Phase 7 —
// resolve unchanged. The shared `hotkey_diag_row` (Phase 3) + `active_stack_label`
// (also drives the bar) stay here and are reached from diagnostics via its glob.
#[path = "overlay_host/diagnostics.rs"]
mod diagnostics;
use diagnostics::*;

// Phase 3 of the modularization (docs/overlay-host-modularization-plan.md §5.3):
// the one-time global-hotkey REGISTRATION + the hotkey-registration diagnostics
// state live in their own file alongside the binary. `use hotkeys::*;` re-exports
// the moved `HotkeyDiag`, `hotkey_diag_row` (read by diagnostics.rs via its own
// glob), and the extracted `register_hotkeys` / `RegisteredHotkeys` so the inline
// block formerly in `main` is now one call. The hotkey EVENT-DISPATCH timer stays
// in `main` (it captures a dozen Rc-borrowed slots + closures) and matches on the
// ids `register_hotkeys` hands back.
#[path = "overlay_host/hotkeys.rs"]
mod hotkeys;
use hotkeys::*;

// Phase 4 of the modularization (docs/overlay-host-modularization-plan.md §5.5):
// crash-recovery — the recovered-context string composition (`build_recovery_block`
// / `strip_recovery_block` / `compose_recovery_context` / `seed_recovery_context`,
// with their `RECOVERY_CONTEXT_HEADER`/`_FOOTER` sentinels) + the on-demand
// `open_recover_offer` window live in their own file alongside the binary.
// `use recovery::*;` re-exports them so the ask/follow-up callers (which call
// `strip_recovery_block` / `compose_recovery_context`) and `main`'s delayed
// `open_recover_offer` Timer resolve unchanged. The recovery FEATURE stays gated
// off behind the `SLINT_OVERLAY_RECOVERY` env in `main` — the move is mechanical.
#[path = "overlay_host/recovery.rs"]
mod recovery;
use recovery::*;

// Phase 4 of the modularization (docs/overlay-host-modularization-plan.md §5.4):
// the first-run setup wizard — `open_wizard`, `wire_wizard_steps`, and the
// wizard-only `refill_wizard_summary` — lives in its own file alongside the
// binary. `use wizard::*;` re-exports them so `main`'s 2200 ms first-run Timer
// and `open_settings`' "Run setup wizard" button resolve unchanged. The shared
// mic guard (`try_acquire_mic`/`release_mic`) the step-4 check uses stays here
// (a dozen non-wizard sites need it) and is reached from wizard.rs via its glob.
#[path = "overlay_host/wizard.rs"]
mod wizard;
use wizard::*;

// Phase 5 of the modularization (docs/overlay-host-modularization-plan.md §5.6):
// the F8 / Shift+F8 screenshot → vision → tile ORCHESTRATION — `fire_f8_vision_capture`
// (the describe/translate handler), `launch_vision_for_bgra` (the per-frame vision
// tile spawn + stream), and the vision-only `bgra_to_slint_image` helper — lives in
// its own file alongside the binary. `use vision_capture::*;` re-exports them so the
// F8/Shift+F8 hotkey dispatch + the 📷 capture-chip wiring in `main` resolve unchanged.
// The PERSISTENT capture overlay's CONSTRUCTION + pre-stealth (WDA before first frame)
// stays in `main` (§5.1 special case); the shared tile/ask machinery
// (`OverlayBarBridge`, `PttStreamSink`, `AskRoute`/`live_route`, the `wire_*`/tile
// helpers, `CONVO_SEQ`/`TILE_DISPLAY_SEQ`) stays here and is reached from
// vision_capture via its glob.
#[path = "overlay_host/vision_capture.rs"]
mod vision_capture;
use vision_capture::*;

pub(crate) type TileWindows = Rc<RefCell<Vec<TileWindow>>>;

/// Parse markdown source into the Slint `MarkdownBlock` rows a tile body
/// renders. Shared by the streaming Delta/Error paths + follow-ups.
fn to_md_blocks(md: &str) -> Vec<MarkdownBlock> {
    markdown::parse(md)
        .into_iter()
        .map(|b| MarkdownBlock {
            kind: b.kind,
            text: SharedString::from(b.text),
            lang: SharedString::from(b.lang),
        })
        .collect()
}

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
type LiveRoute = Rc<std::cell::Cell<AskRoute>>;

pub(crate) fn live_route(initial: AskRoute) -> LiveRoute {
    Rc::new(std::cell::Cell::new(initial))
}

/// V5 — voice follow-up: a tile's 🎤 button records + transcribes a question
/// off the UI thread, then ships `(convo_id, use_vision, text)` here. A
/// UI-thread drain (sibling to the PTT drain) routes it into the tile's
/// conversation by convo_id. Process-global so `wire_voice_followup` reaches it
/// without threading a sender through every tile-creation fn. Set once at start.
static VFU_TX: std::sync::OnceLock<tokio_mpsc::UnboundedSender<(i32, AskRoute, String)>> =
    std::sync::OnceLock::new();

/// V5 (review M2) — process-global single-microphone guard. Exactly ONE mic
/// capture may run at a time across every recorder that opens the mic: PTT-mic,
/// the per-tile 🎤 voice follow-up, and the Settings dictation toggle. They all
/// open the same WASAPI capture endpoint; a second concurrent open yields
/// garbage audio or an error (and a misleading "ничего не распознано"). PTT
/// *system*-audio is a different device and is intentionally NOT gated here.
///
/// Contract: a recorder calls `try_acquire_mic()` on the UI thread before
/// spawning its record thread; on `false` it bails with a generic "занят"
/// message (no state change, no thread). The record thread MUST call
/// `release_mic()` the instant `record_source_until_stop` returns — the mic is
/// physically held until then, and releasing before transcription (which never
/// touches the device) frees it for the next recorder immediately. One acquire
/// pairs with exactly one release.
static MIC_BUSY: AtomicBool = AtomicBool::new(false);

pub(crate) fn try_acquire_mic() -> bool {
    MIC_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

pub(crate) fn release_mic() {
    MIC_BUSY.store(false, Ordering::Release);
}

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
fn wire_escalate(
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
    overlay_weak: slint::Weak<OverlayBarWindow>,
    spawn_tx: tokio_mpsc::UnboundedSender<SpawnTileRequest>,
    tile_seq: AtomicU64,
    /// Phase E6 v18 — last time we pushed a transcript:line update
    /// to the bar UI. Throttle to MIN_TRANSCRIPT_PUSH_INTERVAL so
    /// fast STT chunks (one every ~200ms in aggressive Whisper mode)
    /// don't flood invoke_from_event_loop and saturate the UI
    /// thread. Drops the IN-BETWEEN updates — user only ever cares
    /// about the LATEST transcript text anyway.
    last_transcript_push: std::sync::Mutex<std::time::Instant>,
    /// Phase E3 slice 2 — weak handle to the in-flight streaming
    /// tile plus per-tile accumulator. F9 ask handler synchronously
    /// creates a placeholder TileWindow, registers its weak here,
    /// then spawns `ask_stream_loop` which streams `ai:event`
    /// payloads back through `forward_event` and these updates land
    /// in THIS tile. Cleared on `AiEvent::Done` or `AiEvent::Error`.
    /// Mutex (not RwLock) because only one streaming tile at a time
    /// (rapid-F9 aborts the prior task).
    current_streaming: std::sync::Mutex<Option<StreamingTile>>,
    /// Phase E6 v11 — count of in-flight AI streams (auto-tiles run
    /// in parallel even though F9 is exclusive). Bar's ai-streaming
    /// flag mirrors `counter > 0`. Incremented on AiEvent::Start,
    /// decremented on AiEvent::Done/Error.
    ai_in_flight: std::sync::atomic::AtomicI32,
    /// Phase E6 v45 — per-tile conversations for the in-tile "continue
    /// dialog" feature, keyed by the tile's `convo-id`. Seeded when an
    /// F9/PTT answer completes; read+extended on each follow-up.
    conversations: std::sync::Mutex<std::collections::HashMap<i32, ConvoState>>,
    /// E9 — monotonic stream generation. `install_streaming_tile` bumps it
    /// per new ask; each spawned `ask_stream_loop` runs behind a
    /// `GenGatedEvents` wrapper carrying the generation it was spawned at.
    /// A superseded stream (older generation) has its emits DROPPED, so a
    /// buffered `ai:event` from a torn-down stream can't fold into the new
    /// tile (closes the wrong-tile race that `JoinHandle::abort` alone
    /// leaves open until the next .await).
    stream_gen: Arc<AtomicU64>,
    /// E9 — throttle for the streaming tile re-render. The Delta handler
    /// re-parses the WHOLE answer markdown per token; gating it to ~50ms
    /// bounds that cost independent of token speed. The terminal Done/Error
    /// render is never throttled, so the final answer always shows in full.
    last_tile_render: std::sync::Mutex<std::time::Instant>,
}

/// Per-streaming-tile state: weak handle + accumulated answer text.
/// Bridge re-renders the full markdown tree on every Delta — cheap
/// at <500 tokens, can be windowed later if needed.
struct StreamingTile {
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
struct ConvoState {
    messages: Vec<ai::ChatMessage>,
    rendered: String,
}

/// Tile-spawn request sent from the bridge (any thread) to the UI
/// poll-Timer running on the Slint main thread. Carries everything
/// needed to construct a TileWindow + render the markdown body.
struct SpawnTileRequest {
    label: String,
    spec: TileSpec,
    /// Reserved for Phase E3 follow-up — pass through to a tile-
    /// placement helper that honors MonitorHint::Named (cfg.tile_
    /// monitor_name pin). Today apply_tile_hwnd_with_monitor reads
    /// config directly, so the hint is dropped on this Slint
    /// trajectory. TauriEvents adapter uses it for the React side.
    #[allow(dead_code, reason = "reserved for monitor-name routing")]
    monitor: MonitorHint,
    stealth: bool,
    kind: TileKind,
}

impl OverlayBarBridge {
    /// FIX #8 — store a tile's conversation, keeping the map bounded. When at
    /// the cap, evict the oldest half by `convo_id` (monotonic → lowest = oldest)
    /// BEFORE inserting, mirroring `qa_cache`'s half-eviction. The eviction is a
    /// backstop only — the primary prune is `drop_conversation` on tile close /
    /// MAX_LIVE_TILES eviction, so an open tile (which has one of the highest
    /// ids) is never the one dropped here.
    fn store_conversation(&self, convo_id: i32, state: ConvoState) {
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

// ===== Tuning constants — extracted from inline literals 2026-05-27 =====
//
// Code-quality audit (top-3 priority) flagged 9 scattered bare-number
// sites: probe durations, status auto-revert, hotkey poll, HWND grab
// delay, tile dimensions. Grouped here so a future config-driven UI
// can wire each to a Settings tab without grepping the binary.

/// Mic/sys probe record duration (audio::record_*_blocking).
const PROBE_DURATION_MS: u64 = 3000;
/// Status pill auto-revert delay after a chip-action flash (mic/sys
/// test result, bookmark saved/failed, etc.).
const STATUS_REVERT_SECS: u64 = 5;
/// global-hotkey channel poll interval. 50 ms is the standard
/// responsiveness/CPU trade-off for desktop hotkeys.
const HOTKEY_POLL_MS: u64 = 50;
/// Delay after window.show() before grabbing the HWND. winit realizes
/// the native window lazily; calling earlier returns NotSupported. Used as the
/// conservative FALLBACK delay (the fast attempt below covers the common case)
/// and by the F8 capture-overlay pre-create.
const HWND_GRAB_DELAY_MS: u64 = 200;
/// V0.8.4 — fast first reveal attempt (~2 frames). winit usually realizes the
/// HWND within 1-2 frames, so grabbing at ~33ms lets on-demand windows pop
/// nearly instantly instead of waiting the full 200ms; if the HWND isn't ready
/// yet, present_window_stealth_aware falls back to HWND_GRAB_DELAY_MS.
const HWND_REVEAL_FAST_MS: u64 = 33;
/// SLINT_OVERLAY_AUTO_TILE auto-spawn delay (smoke-test convenience).
const AUTO_TILE_DELAY_MS: u64 = 500;
/// Periodic session-timer chip update interval.
const TIMER_TICK_SECS: u64 = 1;
/// Default tile window dimensions (match ui/tile.slint preferred-*
/// values so the spawned window isn't forcibly shrunk on first paint).
const TILE_DEFAULT_W: i32 = 460;
const TILE_DEFAULT_H: i32 = 360;
/// AI ask cap for the non-streaming auto-tile/reask `complete` path.
/// Sized to fit typical session-question answers without runaway cost.
const AI_MAX_TOKENS: u32 = 600;
/// Upper bound for the STREAMING F9/PTT/follow-up asks. Higher than
/// `AI_MAX_TOKENS` because these are interactive and may want a longer
/// answer; in streaming mode the cap does NOT affect time-to-first-token
/// (it only bounds the worst-case length). One source of truth for the
/// three `stream_chat` sites (was a bare `4096` literal repeated 3×).
const AI_STREAM_MAX_TOKENS: u32 = 4096;

fn main() -> Result<(), slint::PlatformError> {
    // Open the diagnostics log + install the panic hook FIRST so any
    // early failure (config, tokio, window create) is captured even in a
    // release build that has no console.
    slint_replay::logging::init();

    // V0.8.0 (Поток B) — single-instance guard for the emergency-restart (⟳)
    // flow. A `--relaunch` child was spawned by a quitting parent; it must wait
    // for the parent to release the named mutex (i.e. fully exit + free the
    // global hotkeys) before it registers its own hotkeys and shows a bar.
    // Otherwise two bars run at once — and under stealth the 2nd could flash on
    // the screen-share before WDA. A normal launch acquires immediately; if a
    // DIFFERENT instance is already alive (user double-clicked the exe), we bail
    // so we never run a competing bar.
    let is_relaunch = std::env::args().any(|a| a == "--relaunch");
    // Relaunch: give the parent up to 8s to exit. Normal: try-once (0ms).
    let wait_ms = if is_relaunch { 8_000 } else { 0 };
    let _singleton = match slint_replay::win32::acquire_singleton(wait_ms) {
        Ok(g) => {
            if is_relaunch {
                eprintln!("[overlay-host] relaunch: parent exited, singleton acquired");
            }
            Some(g)
        }
        Err(e) => {
            // Another instance holds the bar. Don't run a second one.
            eprintln!("[overlay-host] another instance is already running ({e}); exiting.");
            return Ok(());
        }
    };

    // Phase 6 — MCP server enablement hint.
    //
    // The mcp feature on i-slint-backend-selector auto-starts an HTTP MCP
    // server when SLINT_MCP_PORT is set (Phase 0.5 spike 2 result). For
    // operator visibility, log the value at startup.
    match std::env::var("SLINT_MCP_PORT") {
        Ok(p) => {
            eprintln!(
                "[overlay-host] MCP server: listening on http://127.0.0.1:{p}/mcp (SLINT_MCP_PORT={p})"
            );
            if std::env::var("SLINT_EMIT_DEBUG_INFO").is_err() {
                eprintln!(
                    "[overlay-host] MCP HINT: set SLINT_EMIT_DEBUG_INFO=1 for element introspection."
                );
            }
        }
        Err(_) => eprintln!(
            "[overlay-host] MCP server disabled. Enable with `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=8080`."
        ),
    }

    // Phase C — tokio runtime for async AI calls. Multi-threaded so
    // AI HTTP requests don't block the Slint UI event loop. Spawn
    // background tasks via `rt.handle().spawn(...)` from UI callbacks.
    let tokio_rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[overlay-host] tokio runtime init failed: {e}. AI calls disabled.");
            return Err(slint::PlatformError::Other(format!("tokio init: {e}")));
        }
    };
    let rt_handle = tokio_rt.handle().clone();

    // First-run detection — capture BEFORE config::shared() (load() may create
    // the file). Absent config.json == this is the user's first launch → we
    // auto-open the setup wizard once the overlay is up (see below, pre-run()).
    let first_run = overlay_backend::config::config_path()
        .map(|p| !p.exists())
        .unwrap_or(false);

    // Phase C — load config once at startup. SharedConfig (Arc<RwLock>)
    // because Settings tab will eventually mutate it.
    let cfg = config::shared();
    {
        // Log key PRESENCE only (never the values) so a tester can confirm
        // from the log file whether their AI/STT keys are configured.
        let c = cfg.read();
        diag!(
            "config loaded: ai_model={} base_url={} ai_bearer={} groq_key={}",
            c.ai_model,
            if c.ai_base_url.is_empty() {
                "unset"
            } else {
                "set"
            },
            if c.ai_bearer.is_empty() {
                "MISSING"
            } else {
                "set"
            },
            if c.groq_api_key.is_empty() {
                "MISSING"
            } else {
                "set"
            }
        );
        // E10.3 — log the resolved AI + STT stack (which engine + which
        // endpoint) so the log shows what is actually used. The tester could
        // not tell from logs whether AI was local/cloud or on which port.
        let ai_desc = if c.ai_provider == "local" {
            format!(
                "local {} model={}",
                c.ai_local_base_url,
                if c.ai_local_model.is_empty() {
                    "(unset)"
                } else {
                    c.ai_local_model.as_str()
                }
            )
        } else {
            format!("cloud {}", c.ai_model)
        };
        let stt_desc = match c.stt_provider.as_str() {
            "gigaam" => format!(
                "GigaAM in-process/{} dir={}",
                if c.stt_gigaam_gpu {
                    "GPU(DirectML)"
                } else {
                    "CPU"
                },
                if c.stt_gigaam_dir.is_empty() {
                    "(unset)"
                } else {
                    c.stt_gigaam_dir.as_str()
                }
            ),
            "whisper" => format!("Whisper {}", c.stt_whisper_url),
            _ => "cloud Groq".to_string(),
        };
        diag!("stack: AI={} STT={}", ai_desc, stt_desc);
    }

    // Phase E6 v36 — seed the process-global tile opacity from config so
    // the very first tile spawned (before the Settings panel is ever
    // opened) already honours the saved transparency.
    set_global_tile_opacity(cfg.read().tile_body_opacity);
    // E9 — seed the experimental prompt-cache toggle from config.
    ai::set_prompt_cache(cfg.read().ai_prompt_cache);
    // E10 — disable local-model "thinking" for fast answers unless the user
    // opted in. Only affects the local AI provider (cloud bodies unchanged).
    {
        let c = cfg.read();
        ai::set_local_no_think(c.ai_provider == "local" && !c.ai_local_thinking);
    }
    // E10.2 — restore persisted stealth (WDA_EXCLUDEFROMCAPTURE) so it survives
    // a restart (was previously lost → overlay launched visible to capture).
    set_global_stealth(cfg.read().stealth_enabled);

    let state = new_shared_state();
    if let Ok(mut st) = state.lock() {
        st.stealth = cfg.read().stealth_enabled;
    }
    // Choose the GigaAM ONNX Runtime accelerator (GPU via DirectML, or CPU) ONCE
    // at startup — the ORT session bakes its execution provider in at model load
    // time, so this must run before any transcription. Falls back to CPU when no
    // GPU / DirectML runtime is available.
    overlay_backend::stt::configure_gigaam_accelerator(cfg.read().stt_gigaam_gpu);

    // V0.8.4 — warm up LOCAL models shortly after boot so the user's FIRST real
    // request isn't penalised by cold-start (GigaAM lazy-loads its model on the
    // first transcribe; an llama-server's first inference fills caches). Fire-and-
    // forget on the tokio runtime after a short delay (lets an auto-started local
    // server finish booting first). Cloud is skipped — no cold-start + it would
    // spend API quota. Best-effort: any error is just logged (the real request
    // then loads the model the normal way). Reuses the diagnostics pings.
    {
        let cfg_w = cfg.clone();
        rt_handle.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
            let (ai_local, ai_ep, stt_local, stt_backend) = {
                let c = cfg_w.read();
                (
                    c.ai_provider == "local",
                    c.ai_endpoint(false),
                    c.stt_provider == "gigaam" || c.stt_provider == "whisper",
                    c.stt_backend(),
                )
            };
            if ai_local {
                let t = std::time::Instant::now();
                match overlay_backend::ai::test_connection(
                    ai_ep.base_url,
                    ai_ep.bearer,
                    ai_ep.model,
                )
                .await
                {
                    Ok(_) => diag!("local AI warmed in {:?}", t.elapsed()),
                    Err(e) => diag!("local AI warm-up skipped: {e}"),
                }
            }
            if stt_local {
                let t = std::time::Instant::now();
                match overlay_backend::stt::test_connection_backend(&stt_backend).await {
                    Ok(_) => diag!("local STT warmed in {:?}", t.elapsed()),
                    Err(e) => diag!("local STT warm-up skipped: {e}"),
                }
            }
        });
    }

    // E10.5 — auto-start the local AI servers the config points at but that
    // aren't already running (after a restart following an in-app install the
    // app's own servers are gone — it kills them on quit). Off the UI thread;
    // tracked in app_state for kill-on-quit.
    {
        let (want_llama, want_whisper) = {
            let c = cfg.read();
            (
                c.ai_provider == "local" && c.ai_local_base_url.contains(":8080"),
                c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081"),
            )
        };
        if want_llama || want_whisper {
            let state_auto = state.clone();
            std::thread::spawn(move || {
                let root = overlay_backend::local_ai::default_root();
                let started =
                    overlay_backend::local_ai::ensure_servers(&root, want_llama, want_whisper);
                if !started.is_empty() {
                    state_auto
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .local_ai_servers
                        .extend(started);
                }
            });
        }
    }
    let tiles: TileWindows = Rc::new(RefCell::new(Vec::new()));
    let settings: Rc<RefCell<Option<SettingsWindow>>> = Rc::new(RefCell::new(None));

    let overlay = OverlayBarWindow::new()?;
    // Seed the process-global colour scheme from config, then apply to the bar's
    // Theme global so the very first paint uses the user's choice (default
    // 0=Glacier). Every later-created window (tiles, palette, settings) reads
    // `global_scheme()` at construction.
    set_global_scheme(cfg.read().color_scheme);
    apply_scheme_bar(&overlay, global_scheme());

    // ===== Phase E3 — SlintRuntime + SlintEvents bridge =====
    //
    // SlintRuntime carries session state (transcript, journal, health,
    // last_qa, session_cost, task handles). SlintEvents wraps the
    // OverlayBarBridge which routes RuntimeEvents.emit() to UI property
    // setters via slint::invoke_from_event_loop + schedule_spawn_tile
    // posts SpawnTileRequest through an mpsc channel that the
    // spawn_poll_timer below drains on the UI thread.
    let slint_rt: SharedSlintRuntime = shared_runtime();
    let (spawn_tx, mut spawn_rx) = tokio_mpsc::unbounded_channel::<SpawnTileRequest>();
    let bridge = Arc::new(OverlayBarBridge {
        overlay_weak: overlay.as_weak(),
        spawn_tx,
        tile_seq: AtomicU64::new(0),
        current_streaming: std::sync::Mutex::new(None),
        ai_in_flight: std::sync::atomic::AtomicI32::new(0),
        conversations: std::sync::Mutex::new(std::collections::HashMap::new()),
        stream_gen: Arc::new(AtomicU64::new(0)),
        last_tile_render: std::sync::Mutex::new(std::time::Instant::now()),
        last_transcript_push: std::sync::Mutex::new(
            std::time::Instant::now() - std::time::Duration::from_secs(1),
        ),
    });
    let events: Arc<dyn RuntimeEvents> = Arc::new(SlintEvents::new(bridge.clone()));

    // Phase D1 — select bundled translation per config.ui_language.
    // MUST be called AFTER creating at least one component (Slint
    // requirement: the platform backend has to be initialized first,
    // and component creation triggers that). Default "ru" per
    // overlay_backend::config::default_ui_language().
    let lang = cfg.read().ui_language.clone();
    match slint::select_bundled_translation(&lang) {
        Ok(()) => eprintln!("[overlay-host] translation set to {lang}"),
        Err(e) => eprintln!("[overlay-host] translation {lang} not available: {e}"),
    }

    overlay.set_status_text(SharedString::from("idle"));
    overlay.set_status_color(slint::Color::from_rgb_u8(0x88, 0x88, 0x8c));
    overlay.set_active_stack(SharedString::from(active_stack_label(&cfg.read())));
    overlay.set_stealth_active(cfg.read().stealth_enabled);
    overlay.set_cost_label(SharedString::from("$0.000"));
    overlay.set_timer_label(SharedString::from("00:00"));

    apply_overlay_hwnd(&overlay);

    // ===== Mic chip (Phase C: real 3s mic level test via audio backend) =====
    //
    // Going-active toggle now runs `audio::record_mic_blocking(3000)` on
    // a tokio blocking task (WASAPI is synchronous), computes peak dBFS
    // from the i16 samples, and posts the result to the status pill via
    // slint::invoke_from_event_loop.
    //
    // Real continuous capture (start_capture + STT pipeline drain) is
    // Phase B2 work — needs the runtime::start_session port. For now
    // the chip click is a 3-second mic-health probe.
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        let cfg_mic = cfg.clone();
        let rt_mic = rt_handle.clone();
        overlay.on_mic_toggle_clicked(move || {
            // Re-entry guard: don't spawn a second probe while the
            // first is still running. Review-agent finding 2026-05-27.
            let (new_active, may_probe) = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.mic_active = !st.mic_active;
                let may = st.mic_active && !st.mic_probe_in_flight;
                if may {
                    st.mic_probe_in_flight = true;
                }
                (st.mic_active, may)
            };
            let Some(o) = weak.upgrade() else { return };
            o.set_mic_active(new_active);
            refresh_status(&o, new_active, get_sys_active(&s));

            if !new_active || !may_probe {
                // off-toggle OR a probe is already in flight; let the
                // current one finish and fire its own status update.
                return;
            }

            // Capture device name + spawn the blocking probe.
            let mic_device = cfg_mic.read().mic_device.clone();
            let weak_for_status = weak.clone();
            let s_for_status = s.clone();
            rt_mic.spawn_blocking(move || {
                let started_label = mic_device.clone().unwrap_or_else(|| "default".into());
                eprintln!("[overlay-host] mic test 3s — device={started_label}");
                // M-1: don't open a 2nd WASAPI capture if PTT / voice follow-up /
                // dictation already hold the mic (both get garbage). Clear the
                // in-flight flag + show "busy" instead of recording.
                if !try_acquire_mic() {
                    eprintln!("[overlay-host] mic test skipped — mic busy");
                    let s_busy = s_for_status.clone();
                    let weak_busy = weak_for_status.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        {
                            let mut st = match s_busy.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            st.mic_probe_in_flight = false;
                        }
                        if let Some(o) = weak_busy.upgrade() {
                            o.set_status_text(SharedString::from("mic busy"));
                            o.set_status_color(slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24));
                        }
                    });
                    return;
                }
                let result = audio::record_mic_blocking(PROBE_DURATION_MS, mic_device);
                release_mic();
                let peak_dbfs = match result {
                    Ok(samples) if samples.is_empty() => None,
                    Ok(samples) => {
                        let peak = samples
                            .iter()
                            .map(|s| s.unsigned_abs() as u32)
                            .max()
                            .unwrap_or(0);
                        if peak == 0 {
                            Some(f32::NEG_INFINITY)
                        } else {
                            let norm = peak as f32 / 32768.0;
                            Some(20.0 * norm.log10())
                        }
                    }
                    Err(e) => {
                        eprintln!("[overlay-host] mic test failed: {e:#}");
                        None
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    // Clear the in-flight flag whatever happens (success,
                    // silence, error, or user toggled off mid-test).
                    {
                        let mut st = match s_for_status.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        st.mic_probe_in_flight = false;
                    }
                    let Some(o) = weak_for_status.upgrade() else {
                        return;
                    };
                    // If user toggled mic OFF while the probe was running,
                    // don't overwrite the now-idle status with a "mic ok"
                    // flash. Review-agent finding 2026-05-27.
                    if !get_mic_active(&s_for_status) {
                        eprintln!(
                            "[overlay-host] mic test result ignored — user toggled off mid-probe"
                        );
                        return;
                    }
                    // 3-bucket label aligned with React's coloured-dot
                    // convention (silent / quiet / ok). Avoids leaking
                    // dev jargon ("-42.3 dBFS") to non-technical users.
                    let (label, color) = match peak_dbfs {
                        Some(db) if db.is_finite() && db >= -40.0 => {
                            ("mic ok", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99))
                        }
                        Some(db) if db.is_finite() => {
                            ("mic quiet", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24))
                        }
                        Some(_) => ("mic silent", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24)),
                        None => (
                            "mic test failed",
                            slint::Color::from_rgb_u8(0xf8, 0x71, 0x71),
                        ),
                    };
                    o.set_status_text(SharedString::from(label));
                    o.set_status_color(color);
                    eprintln!(
                        "[overlay-host] mic test result: {} dBFS ({label})",
                        peak_dbfs.map_or_else(|| "?".into(), |d| format!("{d:.2}"))
                    );
                    // Auto-revert status after 5s.
                    let weak_revert = weak_for_status.clone();
                    let s_revert = s_for_status.clone();
                    slint::Timer::single_shot(Duration::from_secs(STATUS_REVERT_SECS), move || {
                        if let Some(o) = weak_revert.upgrade() {
                            refresh_status(
                                &o,
                                get_mic_active(&s_revert),
                                get_sys_active(&s_revert),
                            );
                        }
                    });
                });
            });
        });
    }

    // ===== System (loopback) chip (Phase C: real 3s loopback probe) =====
    //
    // Mirror of the mic chip: runs `audio::record_sys_blocking(3000)`
    // on a tokio blocking task, computes peak dBFS from loopback PCM,
    // posts result to status pill. Same race-guard + ON-OFF mid-test
    // handling as the mic chip.
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        let cfg_sys = cfg.clone();
        let rt_sys = rt_handle.clone();
        overlay.on_sys_toggle_clicked(move || {
            let (new_active, may_probe) = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.sys_active = !st.sys_active;
                let may = st.sys_active && !st.sys_probe_in_flight;
                if may {
                    st.sys_probe_in_flight = true;
                }
                (st.sys_active, may)
            };
            let Some(o) = weak.upgrade() else { return };
            o.set_sys_active(new_active);
            refresh_status(&o, get_mic_active(&s), new_active);

            if !new_active || !may_probe {
                return;
            }

            // Phase C symmetry with mic — respect cfg.system_audio_device
            // when set so users with non-default loopback (e.g. A50
            // Stream Out) get their chosen device probed. Review-agent
            // 2026-05-27 (mirror of the mic chip's cfg.mic_device read).
            let sys_device = cfg_sys.read().system_audio_device.clone();
            let weak_for_status = weak.clone();
            let s_for_status = s.clone();
            rt_sys.spawn_blocking(move || {
                let device_label = sys_device.clone().unwrap_or_else(|| "default".into());
                eprintln!("[overlay-host] sys test 3s — device={device_label}");
                let result = audio::record_sys_blocking(PROBE_DURATION_MS, sys_device);
                let peak_dbfs = match result {
                    Ok(samples) if samples.is_empty() => None,
                    Ok(samples) => {
                        let peak = samples
                            .iter()
                            .map(|s| s.unsigned_abs() as u32)
                            .max()
                            .unwrap_or(0);
                        if peak == 0 {
                            Some(f32::NEG_INFINITY)
                        } else {
                            let norm = peak as f32 / 32768.0;
                            Some(20.0 * norm.log10())
                        }
                    }
                    Err(e) => {
                        eprintln!("[overlay-host] sys test failed: {e:#}");
                        None
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    {
                        let mut st = match s_for_status.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        st.sys_probe_in_flight = false;
                    }
                    let Some(o) = weak_for_status.upgrade() else {
                        return;
                    };
                    if !get_sys_active(&s_for_status) {
                        eprintln!(
                            "[overlay-host] sys test result ignored — user toggled off mid-probe"
                        );
                        return;
                    }
                    let (label, color) = match peak_dbfs {
                        Some(db) if db.is_finite() && db >= -40.0 => {
                            ("sys ok", slint::Color::from_rgb_u8(0x6c, 0xcf, 0xff))
                        }
                        Some(db) if db.is_finite() => {
                            ("sys quiet", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24))
                        }
                        Some(_) => ("sys silent", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24)),
                        None => (
                            "sys test failed",
                            slint::Color::from_rgb_u8(0xf8, 0x71, 0x71),
                        ),
                    };
                    o.set_status_text(SharedString::from(label));
                    o.set_status_color(color);
                    eprintln!(
                        "[overlay-host] sys test result: {} dBFS ({label})",
                        peak_dbfs.map_or_else(|| "?".into(), |d| format!("{d:.2}"))
                    );
                    let weak_revert = weak_for_status.clone();
                    let s_revert = s_for_status.clone();
                    slint::Timer::single_shot(Duration::from_secs(STATUS_REVERT_SECS), move || {
                        if let Some(o) = weak_revert.upgrade() {
                            refresh_status(
                                &o,
                                get_mic_active(&s_revert),
                                get_sys_active(&s_revert),
                            );
                        }
                    });
                });
            });
        });
    }

    // ===== Session timer (Phase E3: real session start/stop) =====
    //
    // Clicking the timer chip now starts or stops the real audio +
    // STT pipeline via slint_session::start_session/stop_session. On
    // start failure (e.g. groq_api_key empty), the chip stays off and
    // the diagnostic appears via the bridge's tile:error path
    // (currently logged; UI toast comes in a follow-up).
    //
    // The chip's local AppState.timer_active flag tracks the user's
    // INTENT (toggle on / toggle off). The real session lifecycle
    // (capture handle, tasks) lives in SlintRuntime — they're kept
    // in sync via this handler.
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        let events_for_timer = events.clone();
        let cfg_for_timer = cfg.clone();
        let rt_for_timer = slint_rt.clone();
        let rt_handle_for_timer = rt_handle.clone();
        overlay.on_timer_toggle_clicked(move || {
            let new_active = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.timer_active = !st.timer_active;
                if !st.timer_active {
                    st.session_secs = 0;
                }
                st.timer_active
            };
            if let Some(o) = weak.upgrade() {
                o.set_timer_active(new_active);
                if !new_active {
                    o.set_timer_label(SharedString::from("00:00"));
                }
            }

            if new_active {
                // Starting — kick off real capture/STT/forwarder via
                // the slint_session orchestrator. Must run within the
                // tokio runtime context (spawn_* calls inside).
                let events_c = events_for_timer.clone();
                let cfg_c = cfg_for_timer.clone();
                let rt_c = rt_for_timer.clone();
                let s_for_revert = s.clone();
                let weak_revert = weak.clone();
                rt_handle_for_timer.spawn(async move {
                    if let Err(e) = slint_session::start_session(events_c, cfg_c, rt_c) {
                        eprintln!("[overlay-host] start_session failed: {e:#}");
                        // Revert UI toggle since the pipeline didn't start.
                        let _ = slint::invoke_from_event_loop(move || {
                            let mut st = match s_for_revert.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            st.timer_active = false;
                            st.session_secs = 0;
                            drop(st);
                            if let Some(o) = weak_revert.upgrade() {
                                o.set_timer_active(false);
                                o.set_status_text(SharedString::from("start failed"));
                                o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0x4b, 0x4b));
                            }
                        });
                    }
                });
            } else {
                // Stopping — snapshot transcript + abort tasks + fire
                // Phase E5 post-meeting debrief if the gate allows.
                let rt_c = rt_for_timer.clone();
                let events_c = events_for_timer.clone();
                let cfg_c = cfg_for_timer.clone();
                let rt_handle_c = rt_handle_for_timer.clone();
                let session_secs_snapshot = {
                    let st = match s.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    st.session_secs
                };
                rt_handle_for_timer.spawn(async move {
                    let snapshot = slint_session::stop_session(rt_c);
                    eprintln!(
                        "[overlay-host] session stopped — {} transcript lines snapshotted",
                        snapshot.len()
                    );
                    events_c.emit("session:stopped", serde_json::Value::Null);
                    // Phase E5 — debrief (gated: opt-in + ≥30s +
                    // ≥5 mic lines + non-empty AI bearer).
                    slint_session::maybe_run_debrief(
                        events_c,
                        cfg_c,
                        snapshot,
                        session_secs_snapshot * 1000,
                        &rt_handle_c,
                    );
                });
            }
        });
    }

    // ===== Spawn-tile poll Timer (Phase E3) =====
    //
    // OverlayBarBridge sends SpawnTileRequest into spawn_rx from any
    // thread. This Timer (running on the Slint main thread) drains
    // the channel every 50ms and creates real TileWindows. Cannot
    // use invoke_from_event_loop directly because TileWindow holds
    // Rc internally and isn't Send.
    let tiles_for_poll = tiles.clone();
    let cfg_for_poll = cfg.clone();
    let weak_overlay_poll = overlay.as_weak();
    // V5 — auto-tiles carry a COMPLETE answer (not a stream); to give them the
    // same follow-up / 🔄 / 🎤 as F9 we seed the conversation here, which needs
    // the bridge (conversations map), events, runtime, and tokio handle.
    let bridge_for_poll = bridge.clone();
    let events_for_poll = events.clone();
    let slint_rt_for_poll = slint_rt.clone();
    let rt_handle_for_poll = rt_handle.clone();
    let spawn_poll_timer = Timer::default();
    spawn_poll_timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        // Phase E6 v19 — process at most 1 spawn request per 50 ms
        // tick (was 2 in v18). TileWindow::new + Slint layout +
        // apply_transparency + markdown::parse + on_*_clicked
        // wiring takes 20-50 ms per tile. Two-per-tick burned 40-
        // 100 ms of UI thread every 50 ms tick → still 80-200%
        // UI-thread saturation under aggressive flood. One-per-tick
        // = 20 tiles/sec max throughput which is plenty (aggressive
        // rate-limit is 10/min, see MAX_TILES_PER_MIN_AGGRESSIVE).
        // User reported (cycle 24): "баг с зависанием основной
        // панели не пропал".
        //
        // Also: cap the LIVE tiles Vec at MAX_LIVE_TILES — if the
        // user lets the session run wild, force-close the oldest
        // tile before spawning a new one. Bounds Slint internal
        // event dispatch cost (was O(N) per UI event).
        const MAX_SPAWNS_PER_TICK: usize = 1;
        const MAX_LIVE_TILES: usize = 16;
        let mut processed = 0;
        while processed < MAX_SPAWNS_PER_TICK {
            let Ok(req) = spawn_rx.try_recv() else { break };
            processed += 1;
            // Drop oldest tile if we're at the cap. Slint releases
            // the native window when the Strong refcount hits 0.
            while tiles_for_poll.borrow().len() >= MAX_LIVE_TILES {
                let dropped = tiles_for_poll.borrow_mut().remove(0);
                // FIX #8 — prune this tile's conversation too (no-op if it had
                // none), so the map doesn't outlive the force-evicted tile.
                bridge_for_poll.drop_conversation(dropped.get_convo_id());
                let _ = dropped.hide();
                eprintln!(
                    "[overlay-host] live tile cap hit (>= {MAX_LIVE_TILES}) — dropping oldest"
                );
            }
            // Keep the bar's open-tile count honest even if the new() below
            // fails after a cap eviction (review minor).
            refresh_open_tiles(&weak_overlay_poll, &tiles_for_poll);
            let tile = match TileWindow::new() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "[overlay-host] spawn poll: TileWindow::new failed for {}: {e}",
                        req.label
                    );
                    continue;
                }
            };
            tile.set_tile_title(SharedString::from(req.spec.question.clone()));
            // Phase E6 fix — auto-increment sequence so tile labels
            // show #1, #2, #3 instead of all #0. Use Relaxed because
            // poll-Timer is single-threaded (UI thread).
            let seq = TILE_DISPLAY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            tile.set_sequence(seq as i32);
            wire_tile_drag(&tile);
            tile.set_source_label(SharedString::from(format!(
                "{} · {}",
                req.kind.as_journal_tag(),
                if req.stealth { "stealth" } else { "" }
            )));
            // Phase E6 v12 — first highlight (if any) becomes the
            // trigger badge. Backend's trigger_highlights() already
            // formats it as "🔥 keyword" or "❓ question snippet".
            // Color: orange for keyword/aggressive, blue for question.
            if let Some(first) = req.spec.highlights.first() {
                tile.set_trigger_label(SharedString::from(first.clone()));
                let is_keyword = first.starts_with("🔥");
                tile.set_trigger_color(if is_keyword {
                    slint::Color::from_rgb_u8(0xfb, 0x92, 0x3c) // orange
                } else {
                    slint::Color::from_rgb_u8(0x6c, 0xcf, 0xff) // cyan
                });
            }
            // Render answer markdown via the spike adapter
            // (same pattern as on_spawn_tile_clicked at ~line 996).
            let blocks: Vec<MarkdownBlock> = markdown::parse(&req.spec.answer)
                .into_iter()
                .map(|b| MarkdownBlock {
                    kind: b.kind,
                    text: SharedString::from(b.text),
                    lang: SharedString::from(b.lang),
                })
                .collect();
            tile.set_blocks(ModelRc::new(VecModel::from(blocks)));
            // Phase E6 v20 — apply saved tile opacity from config so
            // new auto-tiles inherit the user's last slider setting.
            tile.set_body_opacity(cfg_for_poll.read().tile_body_opacity);
            let weak_tile = tile.as_weak();
            // Phase E6 v17 — capture the vec so close-handler can
            // REMOVE the tile (not just hide). Previous version
            // only called tile.hide() — TileWindow Strong stayed
            // in the Vec → Slint kept dispatching to dead windows
            // → UI thread saturated after 30+ tiles. User: "у
            // меня зависла основная панель".
            let vec_for_close = tiles_for_poll.clone();
            let weak_overlay_close = weak_overlay_poll.clone();
            let bridge_for_close = bridge_for_poll.clone();
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (poll/F3) close_clicked fired");
                if let Some(t) = weak_tile.upgrade() {
                    // FIX #8 — prune this tile's conversation (no-op if none).
                    bridge_for_close.drop_conversation(t.get_convo_id());
                    let close_hwnd = grab_hwnd(t.window()).ok();
                    let _ = t.hide();
                    if let Some(target) = close_hwnd {
                        let before = vec_for_close.borrow().len();
                        vec_for_close
                            .borrow_mut()
                            .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
                        let after = vec_for_close.borrow().len();
                        eprintln!(
                            "[overlay-host]   dropped from vec: before={before} after={after}"
                        );
                        refresh_open_tiles(&weak_overlay_close, &vec_for_close);
                    }
                }
            });
            // Phase E6 v17 — pin toggles visual state. Pinned tiles
            // stay around even when session stops (auto-hide skips
            // them). User: "кнопка pin не работает".
            let weak_pin = tile.as_weak();
            tile.on_pin_clicked(move || {
                eprintln!("[overlay-host] tile (poll/F3) pin_clicked fired");
                if let Some(t) = weak_pin.upgrade() {
                    let new = !t.get_pinned();
                    t.set_pinned(new);
                    eprintln!("[overlay-host]   pinned -> {new}");
                }
            });
            // Phase E6 v17 — maximize toggles tile size. User: "нет
            // функционала развернуть, нужно отдельной кнопкой или
            // даб-кликом". Win32 SetWindowPos honours new size; we
            // store the previous rect in app_state for restore.
            let weak_max = tile.as_weak();
            tile.on_maximize_clicked(move || {
                eprintln!("[overlay-host] tile (poll/F3) maximize_clicked fired");
                if let Some(t) = weak_max.upgrade() {
                    let Ok(hwnd) = grab_hwnd(t.window()) else {
                        return;
                    };
                    toggle_tile_maximize(hwnd, &t);
                }
            });
            // V5 — auto-tiles (auto-detector / F3 reask / F6 manual) carry a
            // COMPLETE answer, not a stream, so seed the conversation manually
            // so follow-up + 🔄 + 🎤 work exactly like F9. Only AI-answer kinds
            // get a dialog — KB / snippet / translate / reload aren't
            // conversational, and Vision goes through launch_vision_for_bgra.
            let is_conversational = matches!(
                req.kind,
                TileKind::Ai
                    | TileKind::Auto
                    | TileKind::Manual
                    | TileKind::System
                    | TileKind::Mic
                    | TileKind::Debrief
            );
            if is_conversational && !req.spec.answer.trim().is_empty() {
                let convo_id = CONVO_SEQ.fetch_add(1, Ordering::Relaxed) as i32;
                tile.set_convo_id(convo_id);
                tile.set_followup_busy(false); // answer already complete
                                               // Seed [system, user(question), assistant(answer)] the same way
                                               // F9 builds history, so regenerate re-asks the same question and
                                               // a follow-up carries full context.
                let (meeting_context, response_language) = {
                    let c = cfg_for_poll.read();
                    (c.meeting_context.clone(), c.response_language.clone())
                };
                let question = req.spec.question.clone();
                let mut messages = ai::build_request(
                    &meeting_context,
                    &response_language,
                    &[],
                    None,
                    Some(&question),
                );
                messages.push(ai::ChatMessage {
                    role: "assistant".into(),
                    content: ai::MessageContent::Text(req.spec.answer.clone()),
                });
                // FIX #8 — bounded insert (caps + half-evicts the map).
                bridge_for_poll.store_conversation(
                    convo_id,
                    ConvoState {
                        messages,
                        rendered: req.spec.answer.clone(),
                    },
                );
                // V0.8.1 — per-tile live route (sticky-cloud after 🧠).
                let live = live_route(AskRoute::Text);
                {
                    let weak_fu = tile.as_weak();
                    let bridge_fu = bridge_for_poll.clone();
                    let events_fu = events_for_poll.clone();
                    let cfg_fu = cfg_for_poll.clone();
                    let slint_rt_fu = slint_rt_for_poll.clone();
                    let rt_handle_fu = rt_handle_for_poll.clone();
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
                tile.set_can_regenerate(true);
                {
                    let weak_re = tile.as_weak();
                    let bridge_re = bridge_for_poll.clone();
                    let events_re = events_for_poll.clone();
                    let cfg_re = cfg_for_poll.clone();
                    let slint_rt_re = slint_rt_for_poll.clone();
                    let rt_handle_re = rt_handle_for_poll.clone();
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
                wire_voice_followup(&tile, convo_id, live.clone(), &cfg_for_poll);
                wire_copy(&tile, convo_id, &bridge_for_poll);
                wire_escalate(
                    &tile,
                    convo_id,
                    &live,
                    &bridge_for_poll,
                    &events_for_poll,
                    &cfg_for_poll,
                    &slint_rt_for_poll,
                    &rt_handle_for_poll,
                );
            }
            // (monitor placement applied via apply_tile_hwnd_with_monitor.)
            present_tile_window(&tile);
            apply_tile_hwnd_with_monitor(&tile);
            tiles_for_poll.borrow_mut().push(tile);
            refresh_open_tiles(&weak_overlay_poll, &tiles_for_poll);
        }
    });

    // Periodic timer (every 1 s) — updates the session-timer label
    // when active. Slint Timer::default() with `start(Repeated, ...)`
    // pattern.
    let tick_state = state.clone();
    let tick_weak = overlay.as_weak();
    let tick_timer = Timer::default();
    tick_timer.start(
        TimerMode::Repeated,
        Duration::from_secs(TIMER_TICK_SECS),
        move || {
            let (active, secs) = {
                let mut st = match tick_state.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                if st.timer_active {
                    st.session_secs += 1;
                }
                (st.timer_active, st.session_secs)
            };
            if active {
                if let Some(o) = tick_weak.upgrade() {
                    o.set_timer_label(SharedString::from(format_timer(secs)));
                }
            }
        },
    );

    // (#E10.2) The bar's brain-emoji cloud-model cycle chip was removed —
    // model choice now lives in Settings (the cloud + local model dropdowns)
    // and the bar's active-stack readout shows what's actually live.

    // (#E10.2) The ⭐ bookmark chip was removed (no use-case found).
    // journal::append_bookmark stays available for a future re-add.

    // KB palette — opened via the F4 global hotkey (registered below).
    // (The 💡 tips chip was removed; F4 is the sole entry point.)
    let palette: Rc<RefCell<Option<PaletteWindow>>> = Rc::new(RefCell::new(None));
    // V0.8.3 — "Написать" text-input window, created on demand like the palette.
    let text_ask: Rc<RefCell<Option<TextAskWindow>>> = Rc::new(RefCell::new(None));
    // First-run setup wizard, created on demand like text_ask / palette.
    let wizard: Rc<RefCell<Option<WizardWindow>>> = Rc::new(RefCell::new(None));
    // 🆘 Help window (F1 / 🆘 chip), created on demand.
    let help: Rc<RefCell<Option<HelpWindow>>> = Rc::new(RefCell::new(None));
    // Memory Phase 1 — crash-recovery offer, shown once a beat after startup if
    // the newest journal looks unfinished (see the delayed-open below).
    let recover_offer: Rc<RefCell<Option<RecoverOfferWindow>>> = Rc::new(RefCell::new(None));
    // Phase 1 (modularization §5.1): the ONE registry of on-demand overlay
    // windows whose stealth + theme must stay in lock-step. Built once here from
    // the slots above; cloned (cheap — all Rc) into every stealth/theme handler
    // so a single `registry.apply_stealth(on)` / `registry.apply_scheme(scheme)`
    // covers ALL open windows (incl. 🆘 Help + the recover-offer) instead of
    // three hand-maintained loops that each enumerated a different subset. The
    // bar + the persistent pre-stealthed capture overlay stay outside it.
    let registry = WindowRegistry {
        tiles: tiles.clone(),
        settings: settings.clone(),
        palette: palette.clone(),
        text_ask: text_ask.clone(),
        wizard: wizard.clone(),
        help: help.clone(),
        recover_offer: recover_offer.clone(),
    };
    // V3 — the Lightshot capture overlay. PERSISTENT + pre-stealthed so F8 shows
    // it flash-free: WDA_EXCLUDEFROMCAPTURE keeps it off any screen-share from the
    // first frame, WS_EX_TOOLWINDOW keeps it out of the taskbar. We realize the
    // HWND once (tiny + off-screen), apply both, then hide; F8 just re-shows it
    // (the affinity + ex-style persist across hide/show). Earlier the stealth was
    // applied via grab_hwnd RIGHT AFTER show(), which fails (HWND not realized) —
    // so the capture overlay used to be visible on screen-share + in the taskbar.
    let capture_overlay: Rc<RefCell<Option<CaptureOverlay>>> = Rc::new(RefCell::new(None));
    match CaptureOverlay::new() {
        Ok(co) => {
            co.window().set_size(slint::PhysicalSize::new(1, 1));
            co.window()
                .set_position(slint::PhysicalPosition::new(-32000, -32000));
            let _ = co.show();
            let weak = co.as_weak();
            Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
                if let Some(w) = weak.upgrade() {
                    match grab_hwnd(w.window()) {
                        Ok(hwnd) => {
                            let s = set_stealth(hwnd, true); // WDA_EXCLUDEFROMCAPTURE
                            let t = slint_replay::win32::set_skip_taskbar(hwnd, true);
                            eprintln!(
                                "[overlay-host] capture pre-stealth: stealth_ok={} taskbar_ok={}",
                                s.is_ok(),
                                t.is_ok()
                            );
                        }
                        Err(e) => {
                            eprintln!("[overlay-host] capture pre-stealth: grab_hwnd FAILED: {e}")
                        }
                    }
                    let _ = w.hide();
                } else {
                    eprintln!("[overlay-host] capture pre-stealth: weak upgrade failed");
                }
            });
            *capture_overlay.borrow_mut() = Some(co);
        }
        Err(e) => eprintln!("[overlay-host] F8 capture overlay pre-create failed: {e}"),
    }

    // ===== Global hotkeys (Phase D2 + B3 extra) =====
    //
    // Registration (manager + F3/F4/F6/F8/Shift+F8/F9/Shift+F9/F1, the per-key
    // log lines, and the Diagnostics-tab outcome) moved verbatim into
    // `hotkeys::register_hotkeys` (Phase 3, docs/overlay-host-modularization-plan
    // .md §5.3). `hotkey_manager` MUST stay bound here for the rest of `main` —
    // dropping the `GlobalHotKeyManager` unregisters every hotkey. The returned
    // ids are rebound to the same local names the dispatch loop below matches on,
    // so that loop is unchanged.
    // `_hotkey_manager`: bound (not `_`) so it lives to the end of `main` — its
    // Drop unregisters every hotkey. Leading underscore silences the unused warn
    // without changing the drop point (it was read inside the moved block before).
    let RegisteredHotkeys {
        manager: _hotkey_manager,
        f1_id,
        f3_id,
        f4_id,
        f6_id,
        f8_id,
        sf8_id,
        f9_id,
        sf9_id,
    } = register_hotkeys();

    let hotkey_poll = Timer::default();
    let hp_palette = palette.clone();
    let hp_help = help.clone();
    let hp_capture_overlay = capture_overlay.clone();
    let hp_tiles = tiles.clone();
    let hp_state = state.clone();
    let hp_weak_overlay = overlay.as_weak();
    let hp_bridge = bridge.clone();
    let hp_events = events.clone();
    let hp_cfg = cfg.clone();
    let hp_rt = slint_rt.clone();
    let hp_rt_handle = rt_handle.clone();
    hotkey_poll.start(
        TimerMode::Repeated,
        Duration::from_millis(HOTKEY_POLL_MS),
        move || {
            while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                if event.state != global_hotkey::HotKeyState::Pressed {
                    continue;
                }
                if event.id == f4_id {
                    // Phase E6 v37 — F4 is a TOGGLE, not open-only. User
                    // report: "при вызове f4 я не могу сразу закрыть его".
                    // Previously the second F4 press hit open_palette's
                    // reuse branch (just re-show) so F4 could never close
                    // the palette; and Esc inside the window doesn't fire
                    // because a hotkey-spawned always-on-top window has no
                    // keyboard focus yet. A toggle is focus-independent —
                    // the global hotkey always fires regardless of focus.
                    let palette_open = hp_palette.borrow().is_some();
                    if palette_open {
                        eprintln!("[overlay-host] F4 pressed — closing palette (toggle)");
                        if let Some(p) = hp_palette.borrow_mut().take() {
                            let _ = p.hide();
                        }
                    } else {
                        eprintln!("[overlay-host] F4 pressed — opening palette");
                        open_palette(&hp_palette, &hp_tiles, &hp_state, &hp_weak_overlay);
                    }
                } else if event.id == f1_id {
                    // V0.8.4 — F1 toggles the 🆘 help (focus-independent, like F4;
                    // a hotkey-spawned always-on-top window has no keyboard focus,
                    // so Esc inside it wouldn't fire reliably as the only closer).
                    let help_open = hp_help.borrow().is_some();
                    if help_open {
                        eprintln!("[overlay-host] F1 pressed — closing help (toggle)");
                        if let Some(h) = hp_help.borrow_mut().take() {
                            let _ = h.hide();
                        }
                        if let Some(o) = hp_weak_overlay.upgrade() {
                            o.set_help_open(false);
                        }
                    } else {
                        eprintln!("[overlay-host] F1 pressed — opening help");
                        open_help(&hp_help, &hp_weak_overlay);
                    }
                } else if event.id == f3_id {
                    // Phase E3 slice 3 — F3 reask via overlay-backend's
                    // ported reask_last. Refines the last AI answer using
                    // newest transcript context. Replaces the prior D2
                    // stub that re-invoked the +tile chip.
                    eprintln!("[overlay-host] F3 pressed — reask_last");
                    fire_f3_reask(&hp_events, &hp_cfg, &hp_rt, &hp_rt_handle);
                } else if event.id == f6_id {
                    // Phase E3 slice 3 — F6 manual spawn from last
                    // transcript line (bypasses auto-detector).
                    eprintln!("[overlay-host] F6 pressed — manual_spawn_tile");
                    fire_f6_manual_spawn(&hp_events, &hp_cfg, &hp_rt, &hp_rt_handle);
                } else if event.id == f9_id {
                    // Phase E3 slice 2 — F9 live AI ask via overlay-backend's
                    // `ask_stream_loop`. Synchronously creates a placeholder
                    // tile + registers it in the bridge's current_streaming
                    // slot, then spawns the streaming AI task. Deltas land
                    // back through the bridge's ai:event handler and update
                    // the tile body live.
                    eprintln!("[overlay-host] F9 pressed — live ask streaming");
                    fire_f9_ask(
                        &hp_bridge,
                        &hp_events,
                        &hp_cfg,
                        &hp_rt,
                        &hp_rt_handle,
                        &hp_tiles,
                        &hp_weak_overlay,
                        AskRoute::Text,
                        None,
                    );
                } else if event.id == sf9_id {
                    // V0.8.0 (Поток D) — Shift+F9 escalates ONE ask to the smart
                    // cloud model (deeper reasoning), without flipping the
                    // persistent provider. Egress is intentional + visible (the
                    // tile shows a 🧠 cloud badge).
                    eprintln!("[overlay-host] Shift+F9 — one-shot CLOUD escalation");
                    fire_f9_ask(
                        &hp_bridge,
                        &hp_events,
                        &hp_cfg,
                        &hp_rt,
                        &hp_rt_handle,
                        &hp_tiles,
                        &hp_weak_overlay,
                        AskRoute::Cloud,
                        None,
                    );
                } else if event.id == f8_id {
                    // V3 — F8 screenshot → Lightshot region select → vision (describe).
                    diag!("[overlay-host] F8 pressed — capture overlay");
                    fire_f8_vision_capture(
                        &hp_bridge,
                        &hp_events,
                        &hp_cfg,
                        &hp_rt,
                        &hp_rt_handle,
                        &hp_tiles,
                        &hp_weak_overlay,
                        &hp_capture_overlay,
                        false,
                    );
                } else if event.id == sf8_id {
                    // Feature #3 — Shift+F8: same region capture, TRANSLATE mode.
                    diag!("[overlay-host] Shift+F8 pressed — translate capture");
                    fire_f8_vision_capture(
                        &hp_bridge,
                        &hp_events,
                        &hp_cfg,
                        &hp_rt,
                        &hp_rt_handle,
                        &hp_tiles,
                        &hp_weak_overlay,
                        &hp_capture_overlay,
                        true,
                    );
                }
            }
        },
    );

    // ===== Phase E6 v42 — push-to-record (hold mic/sys → STT → AI tile) =====
    //
    // Hold a record button → a std::thread runs audio::record_source_until_
    // stop with a shared stop flag (one PTT at a time). Release flips the
    // flag; the thread finishes and ships the PCM through ptt_pcm_tx. A
    // UI-thread Timer drains it (TileWindow isn't Send — same constraint as
    // the spawn channel) and calls fire_ptt_ask, which transcribes via Groq
    // then streams the AI answer into a tile (same path as F9).
    struct PttRec {
        is_mic: bool,
        stop: Arc<AtomicBool>,
    }
    let ptt_state: Rc<RefCell<Option<PttRec>>> = Rc::new(RefCell::new(None));
    let (ptt_pcm_tx, mut ptt_pcm_rx) =
        tokio_mpsc::unbounded_channel::<(audio::AudioSource, Arc<AtomicBool>, Vec<i16>)>();
    // V5 — voice follow-up channel: a tile 🎤 ships (convo_id, route, text)
    // here once recorded + transcribed; the drain below routes it to the tile.
    let (vfu_tx, mut vfu_rx) = tokio_mpsc::unbounded_channel::<(i32, AskRoute, String)>();
    let _ = VFU_TX.set(vfu_tx);

    {
        let ptt_state = ptt_state.clone();
        let weak = overlay.as_weak();
        let cfg_p = cfg.clone();
        let tx = ptt_pcm_tx.clone();
        overlay.on_ptt_mic_pressed(move || {
            if ptt_state.borrow().is_some() {
                return; // one PTT at a time
            }
            // M2 — single-mic guard (shared with voice follow-up + dictation).
            if !try_acquire_mic() {
                return; // mic held by a tile voice follow-up / dictation
            }
            let stop = Arc::new(AtomicBool::new(false));
            *ptt_state.borrow_mut() = Some(PttRec {
                is_mic: true,
                stop: stop.clone(),
            });
            if let Some(o) = weak.upgrade() {
                o.set_mic_recording(true);
            }
            let (mic_dev, sys_dev) = {
                let c = cfg_p.read();
                (c.mic_device.clone(), c.system_audio_device.clone())
            };
            let tx = tx.clone();
            let id = stop.clone();
            spawn_ptt_watchdog(stop.clone());
            std::thread::spawn(move || {
                let pcm = audio::record_source_until_stop(
                    audio::AudioSource::Mic,
                    mic_dev,
                    sys_dev,
                    stop,
                )
                .unwrap_or_else(|e| {
                    eprintln!("[overlay-host] PTT mic record failed: {e:#}");
                    Vec::new()
                });
                release_mic(); // M2 — free the mic before transcription
                let _ = tx.send((audio::AudioSource::Mic, id, pcm));
            });
            eprintln!("[overlay-host] PTT mic — recording (hold)…");
        });
    }
    {
        let ptt_state = ptt_state.clone();
        let weak = overlay.as_weak();
        overlay.on_ptt_mic_released(move || {
            let mut slot = ptt_state.borrow_mut();
            if let Some(rec) = slot.as_ref() {
                if rec.is_mic {
                    rec.stop.store(true, Ordering::Release);
                    *slot = None;
                }
            }
            drop(slot);
            if let Some(o) = weak.upgrade() {
                o.set_mic_recording(false);
            }
        });
    }
    {
        let ptt_state = ptt_state.clone();
        let weak = overlay.as_weak();
        let cfg_p = cfg.clone();
        let tx = ptt_pcm_tx.clone();
        overlay.on_ptt_sys_pressed(move || {
            if ptt_state.borrow().is_some() {
                return;
            }
            let stop = Arc::new(AtomicBool::new(false));
            *ptt_state.borrow_mut() = Some(PttRec {
                is_mic: false,
                stop: stop.clone(),
            });
            if let Some(o) = weak.upgrade() {
                o.set_sys_recording(true);
            }
            let (mic_dev, sys_dev) = {
                let c = cfg_p.read();
                (c.mic_device.clone(), c.system_audio_device.clone())
            };
            let tx = tx.clone();
            let id = stop.clone();
            spawn_ptt_watchdog(stop.clone());
            std::thread::spawn(move || {
                let pcm = audio::record_source_until_stop(
                    audio::AudioSource::System,
                    mic_dev,
                    sys_dev,
                    stop,
                )
                .unwrap_or_else(|e| {
                    eprintln!("[overlay-host] PTT sys record failed: {e:#}");
                    Vec::new()
                });
                let _ = tx.send((audio::AudioSource::System, id, pcm));
            });
            eprintln!("[overlay-host] PTT sys — recording (hold)…");
        });
    }
    {
        let ptt_state = ptt_state.clone();
        let weak = overlay.as_weak();
        overlay.on_ptt_sys_released(move || {
            let mut slot = ptt_state.borrow_mut();
            if let Some(rec) = slot.as_ref() {
                if !rec.is_mic {
                    rec.stop.store(true, Ordering::Release);
                    *slot = None;
                }
            }
            drop(slot);
            if let Some(o) = weak.upgrade() {
                o.set_sys_recording(false);
            }
        });
    }
    // UI-thread drain: transcribe + ask for each finished recording.
    let ptt_timer = Timer::default();
    {
        let bridge_p = bridge.clone();
        let events_p = events.clone();
        let cfg_p = cfg.clone();
        let rt_p = slint_rt.clone();
        let rth_p = rt_handle.clone();
        let tiles_p = tiles.clone();
        let ptt_state_t = ptt_state.clone();
        let weak = overlay.as_weak();
        ptt_timer.start(TimerMode::Repeated, Duration::from_millis(120), move || {
            while let Ok((source, rec_id, pcm)) = ptt_pcm_rx.try_recv() {
                if let Some(o) = weak.upgrade() {
                    o.set_mic_recording(false);
                    o.set_sys_recording(false);
                }
                // Self-heal: if this finished recording is still the active
                // slot (e.g. a pointer-up was lost mid-hold and the 30 s
                // watchdog stopped it), clear the guard so PTT isn't
                // permanently blocked. ptr_eq matches THIS recording only —
                // a newer hold's slot is left intact.
                {
                    let mut slot = ptt_state_t.borrow_mut();
                    if slot.as_ref().is_some_and(|r| Arc::ptr_eq(&r.stop, &rec_id)) {
                        *slot = None;
                    }
                }
                if pcm.is_empty() {
                    continue; // record error or empty hold — nothing to ask
                }
                fire_ptt_ask(
                    (source, pcm),
                    &bridge_p,
                    &events_p,
                    &cfg_p,
                    &rt_p,
                    &rth_p,
                    &tiles_p,
                    &weak,
                );
            }
        });
    }

    // V5 — voice follow-up drain (sibling to the PTT drain): a tile's 🎤
    // recorded + transcribed a question off-thread; route it into THAT tile's
    // conversation by convo_id (text endpoint for F9/PTT tiles, vision for F8).
    let vfu_timer = Timer::default();
    {
        let bridge_v = bridge.clone();
        let events_v = events.clone();
        let cfg_v = cfg.clone();
        let rt_v = slint_rt.clone();
        let rth_v = rt_handle.clone();
        let tiles_v = tiles.clone();
        vfu_timer.start(TimerMode::Repeated, Duration::from_millis(120), move || {
            while let Ok((convo_id, route, text)) = vfu_rx.try_recv() {
                let weak = tiles_v
                    .borrow()
                    .iter()
                    .find(|t| t.get_convo_id() == convo_id)
                    .map(|t| t.as_weak());
                let Some(weak) = weak else {
                    continue; // tile already closed — drop the result
                };
                if text.trim().is_empty() {
                    if let Some(t) = weak.upgrade() {
                        t.set_voice_recording(false);
                        t.set_followup_busy(false);
                        t.set_source_label(SharedString::from("stt · ничего не распознано"));
                    }
                    continue;
                }
                if let Some(t) = weak.upgrade() {
                    t.set_voice_recording(false);
                }
                fire_followup_ask(
                    (convo_id, text),
                    weak,
                    &bridge_v,
                    &events_v,
                    &cfg_v,
                    &rt_v,
                    &rth_v,
                    route,
                );
            }
        });
    }

    // ===== Stealth toggle on overlay bar =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        // Phase 1 (§5.1) — ONE registry clone replaces the seven hand-written
        // per-window clones + loops below. `registry.apply_stealth(on)` now
        // covers tiles / palette / text_ask / wizard / Settings AND (the FIX #6
        // windows) 🆘 help + the crash-recovery-offer, so none can be forgotten.
        let registry_stealth = registry.clone();
        let cfg_stealth = cfg.clone();
        overlay.on_stealth_toggle_clicked(move || {
            let new_stealth = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.stealth = !st.stealth;
                st.stealth
            };
            eprintln!("[overlay-host] stealth -> {new_stealth}");
            // #111 — source-of-truth so windows created later (palette /
            // Settings / freshly-spawned tiles) inherit stealth on realize.
            set_global_stealth(new_stealth);
            // #E10.2 — persist so stealth survives a restart.
            {
                let mut c = cfg_stealth.write();
                c.stealth_enabled = new_stealth;
                let _ = config::save(&c);
            }
            // Apply to overlay + light the bar 🎯 chip. The bar stays inline (NOT
            // in the registry): it also drops its taskbar button under stealth so
            // a screen-share viewer doesn't spot the app in the taskbar.
            if let Some(o) = weak.upgrade() {
                o.set_stealth_active(new_stealth);
                if let Ok(hwnd) = grab_hwnd(o.window()) {
                    let _ = set_stealth(hwnd, new_stealth);
                    let _ = set_skip_taskbar(hwnd, new_stealth);
                }
            }
            // Every other open window through the single registry path.
            registry_stealth.apply_stealth(new_stealth);
        });
    }

    // ===== Close all tiles (#110) =====
    // User: "не хватает кнопки закрыть все тайлы когда их много". Bulk-close
    // every open tile window in one click. Resets the spawn counter to 0,
    // which also hides the bar's "close all" chip again (it's gated on
    // tiles-spawned > 0).
    {
        let tiles_ref = tiles.clone();
        let s = state.clone();
        let weak = overlay.as_weak();
        // FIX #8 — prune each closed tile's conversation too (no-op for the
        // non-conversational ones), so bulk-close doesn't orphan ConvoState.
        let bridge_for_close_all = bridge.clone();
        // Phase 1 (§5.1) — refresh the bar's open-tile chip through the registry.
        let registry_close_all = registry.clone();
        overlay.on_close_all_tiles_clicked(move || {
            let n = {
                let mut v = tiles_ref.borrow_mut();
                let count = v.len();
                for t in v.iter() {
                    bridge_for_close_all.drop_conversation(t.get_convo_id());
                    let _ = t.hide();
                }
                v.clear();
                count
            };
            eprintln!("[overlay-host] close-all-tiles: closed {n} tile(s)");
            if let Ok(mut st) = s.lock() {
                st.tiles_spawned = 0;
            }
            if let Some(o) = weak.upgrade() {
                o.set_tiles_spawned(0);
                // #B1 — vec was just cleared; sync the live open-tile count to 0.
                registry_close_all.refresh_tiles_chip(&o);
            }
        });
    }

    // ===== 📷 capture chip — same flow as the F8 hotkey (screenshot → vision) =====
    {
        let bridge_c = bridge.clone();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        let slint_rt_c = slint_rt.clone();
        let rt_c = rt_handle.clone();
        let tiles_c = tiles.clone();
        let weak_c = overlay.as_weak();
        let cap_c = capture_overlay.clone();
        overlay.on_capture_clicked(move || {
            diag!("[overlay-host] 📷 capture chip — screenshot → vision");
            fire_f8_vision_capture(
                &bridge_c,
                &events_c,
                &cfg_c,
                &slint_rt_c,
                &rt_c,
                &tiles_c,
                &weak_c,
                &cap_c,
                false,
            );
        });
    }

    // ===== "Написать" — typed-question input window (V0.8.3) =====
    {
        let slot = text_ask.clone();
        let bridge_c = bridge.clone();
        let events_c = events.clone();
        let cfg_c = cfg.clone();
        let slint_rt_c = slint_rt.clone();
        let rt = rt_handle.clone();
        let tiles_c = tiles.clone();
        let weak_ov = overlay.as_weak();
        overlay.on_text_ask_clicked(move || {
            open_text_ask(
                &slot,
                &bridge_c,
                &events_c,
                &cfg_c,
                &slint_rt_c,
                &rt,
                &tiles_c,
                &weak_ov,
            );
        });
    }

    // ===== Spawn tile (Phase C: real AI ask via overlay_backend::ai) =====
    {
        let s = state.clone();
        let t = tiles.clone();
        let weak = overlay.as_weak();
        let cfg_ref = cfg.clone();
        let rt = rt_handle.clone();
        let slint_rt_c = slint_rt.clone();
        overlay.on_spawn_tile_clicked(move || {
            let Some(overlay) = weak.upgrade() else {
                return;
            };
            let seq = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.tiles_spawned += 1;
                st.tiles_spawned
            };
            overlay.set_tiles_spawned(seq as i32);

            let tile = match TileWindow::new() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("[overlay-host] TileWindow::new failed: {e}");
                    return;
                }
            };

            // "+ тайл" — real AI ask about the recent transcript. The tile is
            // shown IMMEDIATELY (below) with a ⏳ placeholder, then filled when
            // the resolved AI endpoint answers — so the button always gives
            // instant feedback even if the model is slow/down. User: "+ тайл
            // не прожимается".
            let recent_tx = {
                let st = slint_replay::runtime_state::lock(&slint_rt_c);
                select_recent_labeled(&st.transcript, 8).join("\n")
            };
            let has_tx = !recent_tx.trim().is_empty();
            let question = if has_tx {
                format!("Ты — ассистент на встрече/интервью. Последние реплики:\n{recent_tx}\n\nДай ПОЛЕЗНЫЙ ответ по последней реплике: если это вопрос — ответь по делу; если это утверждение, тема или новость — кратко объясни суть и дай релевантный комментарий или факты. НЕ проси уточнить и НЕ переспрашивай — всегда отвечай содержательно на основе контекста.")
            } else {
                String::new()
            };
            let heading = if has_tx {
                format!("✋ Вопрос по встрече #{seq}")
            } else {
                format!("✋ Тайл #{seq}")
            };
            tile.set_sequence(seq as i32);
            tile.set_tile_title(SharedString::from(heading.clone()));
            tile.set_source_label(SharedString::from("ai · asking…"));
            wire_tile_drag(&tile);

            // Initial body — shown instantly: the AI-in-flight hint, or the
            // no-transcript hint when there's nothing to ask yet.
            let placeholder = vec![MarkdownBlock {
                kind: markdown::kind::PARAGRAPH,
                text: SharedString::from(if has_tx {
                    "⏳ Спрашиваю AI…"
                } else {
                    "Нет транскрипта. Начните сессию (захват аудио) — когда появятся реплики, «+ тайл» спросит AI по последним из них."
                }),
                lang: SharedString::from(""),
            }];
            tile.set_blocks(ModelRc::new(VecModel::from(placeholder)));

            let weak_tile = tile.as_weak();
            let vec_for_close = t.clone();
            let weak_overlay_close = weak.clone();
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (spawn-poll) close_clicked fired");
                if let Some(tw) = weak_tile.upgrade() {
                    let close_hwnd = grab_hwnd(tw.window()).ok();
                    let _ = tw.hide();
                    if let Some(target) = close_hwnd {
                        vec_for_close.borrow_mut().retain(|item| {
                            grab_hwnd(item.window()).ok() != Some(target)
                        });
                        refresh_open_tiles(&weak_overlay_close, &vec_for_close);
                    }
                }
            });
            let weak_pin = tile.as_weak();
            tile.on_pin_clicked(move || {
                if let Some(tw) = weak_pin.upgrade() {
                    let new = !tw.get_pinned();
                    tw.set_pinned(new);
                    eprintln!("[overlay-host] tile (spawn-poll) pin -> {new}");
                }
            });
            let weak_max = tile.as_weak();
            tile.on_maximize_clicked(move || {
                if let Some(tw) = weak_max.upgrade() {
                    let Ok(hwnd) = grab_hwnd(tw.window()) else { return };
                    toggle_tile_maximize(hwnd, &tw);
                }
            });

            present_tile_window(&tile);
            apply_tile_hwnd_with_monitor(&tile);

            // Capture a Weak handle the tokio task can post back to
            // the UI thread via slint::invoke_from_event_loop.
            let weak_for_ai = tile.as_weak();
            t.borrow_mut().push(tile);
            refresh_open_tiles(&weak, &t);

            // No transcript → the placeholder already shows the hint; done.
            if !has_tx {
                if let Some(t) = weak_for_ai.upgrade() {
                    t.set_source_label(SharedString::from(""));
                }
                return;
            }
            // Resolve the ACTIVE endpoint (local vs cloud) — the old code used
            // the cloud fields unconditionally, which silently failed for a
            // local-provider user (the cloud bridge wasn't even running).
            let ep = cfg_ref.read().ai_endpoint(false);
            let is_local = ep.is_local;
            let (base_url, bearer, model) = (ep.base_url, ep.bearer, ep.model);
            // Cloud needs a bearer; a LOCAL server (llama.cpp / Ollama) usually
            // doesn't — so an empty LOCAL bearer must NOT block the ask. This is
            // why "+ tile" wrongly said "AI не настроен" for a working local model.
            if base_url.is_empty() || (!is_local && bearer.is_empty()) {
                if let Some(t) = weak_for_ai.upgrade() {
                    let blocks: Vec<MarkdownBlock> = markdown::parse(
                        "**AI не настроен.** Откройте Настройки → AI и выберите провайдера (локальный сервер или облачный мост).",
                    )
                    .into_iter()
                    .map(|b| MarkdownBlock {
                        kind: b.kind,
                        text: SharedString::from(b.text),
                        lang: SharedString::from(b.lang),
                    })
                    .collect();
                    t.set_blocks(ModelRc::new(VecModel::from(blocks)));
                    t.set_source_label(SharedString::from("ai · не настроен"));
                }
                return;
            }

            let question_for_task = question.clone();
            let heading_for_task = heading.clone();
            let slint_rt_cost = slint_rt_c.clone();
            let weak_overlay_cost = weak.clone();
            rt.spawn(async move {
                let messages = vec![ai::ChatMessage {
                    role: "user".to_string(),
                    content: ai::MessageContent::Text(question_for_task.clone()),
                }];
                let result = ai::complete_with_usage(
                    &base_url,
                    &bearer,
                    &model,
                    messages,
                    AI_MAX_TOKENS,
                )
                .await;

                // Post result back to UI thread.
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(tile) = weak_for_ai.upgrade() else {
                        return;
                    };
                    match result {
                        Ok((response, usage)) => {
                            // Local inference is free — don't bill it (mirrors
                            // every other ask path; otherwise a local "+ tile"
                            // would inflate the meter at cloud Sonnet pricing).
                            let cost_micro = if is_local {
                                0
                            } else {
                                ai::cost_microcents(&model, usage.input, usage.output)
                            };
                            let cost_usd = cost_micro as f64 / 100_000_000.0;
                            let md = format!("# {heading_for_task}\n\n{response}\n");
                            let blocks: Vec<MarkdownBlock> = markdown::parse(&md)
                                .into_iter()
                                .map(|b| MarkdownBlock {
                                    kind: b.kind,
                                    text: SharedString::from(b.text),
                                    lang: SharedString::from(b.lang),
                                })
                                .collect();
                            tile.set_blocks(ModelRc::new(VecModel::from(blocks)));
                            tile.set_source_label(SharedString::from(format!(
                                "ai · {} · ${:.4}",
                                model, cost_usd
                            )));
                            // Bill the session like F6/F9 so the cost cap can see
                            // "+ tile" spend. This was a silent hole: cloud
                            // "+ tile" clicks never accumulated into the session
                            // meter, so max_session_cost_usd never tripped and the
                            // bar $ label stayed frozen. Refresh it to the new
                            // session total (matches the cost:update consumer).
                            let session_total = {
                                let mut st =
                                    slint_replay::runtime_state::lock(&slint_rt_cost);
                                st.session_cost_microcents =
                                    st.session_cost_microcents.saturating_add(cost_micro);
                                (st.session_cost_microcents as f64) / 100_000_000.0
                            };
                            if let Some(ov) = weak_overlay_cost.upgrade() {
                                ov.set_cost_label(SharedString::from(format!(
                                    "${session_total:.3}"
                                )));
                            }
                        }
                        Err(e) => {
                            // Privacy: classify the error rather than dump
                            // the full chain — reqwest errors typically
                            // include the full base_url (LAN IP) which
                            // would leak into screenshots saved under
                            // target/visual/. Caught by review-agent
                            // 2026-05-27.
                            let category = classify_ai_error(&format!("{e:#}"));
                            let md = format!(
                                "# {heading_for_task}\n\n**Не удалось получить ответ AI:** {category}\n\nПроверьте локальный AI-сервер или AI-мост (Настройки → AI).",
                            );
                            let blocks: Vec<MarkdownBlock> = markdown::parse(&md)
                                .into_iter()
                                .map(|b| MarkdownBlock {
                                    kind: b.kind,
                                    text: SharedString::from(b.text),
                                    lang: SharedString::from(b.lang),
                                })
                                .collect();
                            tile.set_blocks(ModelRc::new(VecModel::from(blocks)));
                            tile.set_source_label(SharedString::from("ai · error"));
                        }
                    }
                });
            });
        });
    }

    // ===== 🆘 Help (F1 / 🆘 chip) =====
    {
        let help_ref = help.clone();
        let ow = overlay.as_weak();
        overlay.on_help_clicked(move || {
            open_help(&help_ref, &ow);
        });
    }

    // ===== Settings =====
    {
        let s = state.clone();
        let settings_ref = settings.clone();
        let tiles_ref = tiles.clone();
        let cfg_for_settings = cfg.clone();
        let overlay_weak = overlay.as_weak();
        // Phase 1 (§5.1) — the Settings-tab stealth toggle + scheme switch (and
        // its nested "Run setup wizard") reach every open window via this registry.
        let registry_settings = registry.clone();
        overlay.on_open_settings_clicked(move || {
            open_settings(
                &s,
                &settings_ref,
                &tiles_ref,
                &cfg_for_settings,
                &overlay_weak,
                &registry_settings,
            );
        });
    }

    // ===== Aggressive auto-tile toggle =====
    // Phase E6 v10 — surface backend's cfg.auto_tile_every_line as a
    // bar-level switch. Reads current value into chip state at startup,
    // then toggles on click + persists to config.json. The backend
    // detector pipeline in slint_session already honours this flag
    // (every_line=true → MAX_TILES_PER_MIN_AGGRESSIVE=20).
    {
        let cfg_for_agg = cfg.clone();
        let weak_for_agg = overlay.as_weak();
        // Sync initial state from cfg.
        if let Some(o) = weak_for_agg.upgrade() {
            o.set_aggressive_active(cfg_for_agg.read().auto_tile_every_line);
        }
        overlay.on_aggressive_toggle_clicked(move || {
            let new_state = {
                let mut c = cfg_for_agg.write();
                c.auto_tile_every_line = !c.auto_tile_every_line;
                let _ = overlay_backend::config::save(&c);
                c.auto_tile_every_line
            };
            eprintln!("[overlay-host] aggressive auto-tile -> {new_state}");
            if let Some(o) = weak_for_agg.upgrade() {
                o.set_aggressive_active(new_state);
            }
        });
    }

    // ===== Bar drag-to-move (Phase E6 v22 — manual cursor-delta) =====
    // drag-start-requested (pointer-down on status pill) records the
    // anchor; drag-moved (move while pressed) moves the window by the
    // cursor delta. No WM_NCLBUTTONDOWN modal loop → Slint sees the
    // mouse-up normally → TouchArea never sticks → chips stay
    // clickable after a drag. User: "вся зона стала drag".
    {
        let weak_for_drag = overlay.as_weak();
        overlay.on_drag_start_requested(move || {
            if let Some(o) = weak_for_drag.upgrade() {
                if let Ok(hwnd) = grab_hwnd(o.window()) {
                    drag_begin(hwnd);
                }
            }
        });
        let weak_for_move = overlay.as_weak();
        overlay.on_drag_moved(move || {
            if let Some(o) = weak_for_move.upgrade() {
                if let Ok(hwnd) = grab_hwnd(o.window()) {
                    drag_update(hwnd);
                }
            }
        });
    }

    // ===== Quit (two-step inline confirm) =====
    // The X press ARMS an inline "Quit? Yes/No" on the bar instead of
    // killing the app outright (user: "крестик моментально всё закрывает
    // без предупреждения"). A 4s timer auto-disarms so the bar doesn't
    // get stuck in the armed state if the user walks away.
    {
        let weak = overlay.as_weak();
        overlay.on_quit_clicked(move || {
            let Some(o) = weak.upgrade() else { return };
            o.set_quit_armed(true);
            // v0.8.2 (m1) — quit + restart confirms are mutually exclusive so
            // two inline "…? Yes No" prompts never share the fixed-width bar.
            o.set_restart_armed(false);
            diag!("quit armed (awaiting confirm)");
            let disarm = o.as_weak();
            Timer::single_shot(Duration::from_secs(4), move || {
                if let Some(o) = disarm.upgrade() {
                    if o.get_quit_armed() {
                        o.set_quit_armed(false);
                        diag!("quit auto-disarmed (timeout)");
                    }
                }
            });
        });
    }
    overlay.on_quit_confirm(|| {
        diag!("quit confirmed");
        let _ = slint::quit_event_loop();
    });
    {
        let weak = overlay.as_weak();
        overlay.on_quit_cancel(move || {
            if let Some(o) = weak.upgrade() {
                o.set_quit_armed(false);
            }
        });
    }

    // V0.8.0 (Поток B) — emergency restart (⟳). Two-step confirm like Quit
    // (restarting clears the current session transcript, so a stray click
    // shouldn't trigger it). On confirm: spawn the relaunch child, then quit so
    // teardown kills the (possibly hung) local-AI servers; the child waits on
    // the singleton mutex for us to exit, then comes up fresh — restoring the
    // SAME persisted settings incl. stealth (flash-free thanks to Поток C).
    {
        let weak = overlay.as_weak();
        overlay.on_restart_clicked(move || {
            let Some(o) = weak.upgrade() else { return };
            o.set_restart_armed(true);
            // v0.8.2 (m1) — mutually exclusive with the quit confirm (above).
            o.set_quit_armed(false);
            diag!("restart armed (awaiting confirm)");
            let disarm = o.as_weak();
            Timer::single_shot(Duration::from_secs(4), move || {
                if let Some(o) = disarm.upgrade() {
                    if o.get_restart_armed() {
                        o.set_restart_armed(false);
                        diag!("restart auto-disarmed (timeout)");
                    }
                }
            });
        });
    }
    {
        let weak = overlay.as_weak();
        overlay.on_restart_confirm(move || {
            if let Some(o) = weak.upgrade() {
                o.set_restart_armed(false);
            }
            diag!("restart confirmed — spawning relaunch child");
            if spawn_relaunch() {
                let _ = slint::quit_event_loop();
            } else {
                eprintln!("[overlay-host] restart aborted (could not spawn child); staying up");
            }
        });
    }
    {
        let weak = overlay.as_weak();
        overlay.on_restart_cancel(move || {
            if let Some(o) = weak.upgrade() {
                o.set_restart_armed(false);
            }
        });
    }

    // Smoke convenience: SLINT_OVERLAY_AUTO_TILE=1 spawns one tile
    // after 500 ms so screenshot scripts can verify markdown rendering
    // without driving the UI. Removable Phase 6 cleanup.
    if std::env::var("SLINT_OVERLAY_AUTO_TILE").is_ok() {
        let weak = overlay.as_weak();
        Timer::single_shot(Duration::from_millis(AUTO_TILE_DELAY_MS), move || {
            if let Some(o) = weak.upgrade() {
                o.invoke_spawn_tile_clicked();
            }
        });
    }

    // Phase E6 v13 — auto-enable sys (loopback) capture on startup.
    // User feedback: "почему каждый раз когда ты стартуешь ты не
    // прокликиваешь sys звук и не включаешь?" — every launch the
    // user had to click the sys chip manually before audio could
    // be captured, even though their use-case (interviews, Zoom,
    // YouTube prep) ALWAYS wants sys capture on. Opt-out via env
    // var SLINT_OVERLAY_NO_AUTO_SYS=1 if a future caller needs the
    // old behaviour (e.g. CI smoke runs).
    //
    // Phase E6 v14 — also auto-start session (timer) ~1.5s after
    // sys probe completes. User: "то что еще старт нужно прокликивать
    // это ко?". Sequence: sys-toggle (400 ms delay) → 3 s probe →
    // settle → timer-toggle (1900 ms total delay so the probe
    // finishes first). Opt-out: SLINT_OVERLAY_NO_AUTO_START=1.
    if std::env::var("SLINT_OVERLAY_NO_AUTO_SYS").is_err() {
        let weak = overlay.as_weak();
        Timer::single_shot(Duration::from_millis(400), move || {
            if let Some(o) = weak.upgrade() {
                eprintln!("[overlay-host] auto-enabling sys capture on startup");
                o.invoke_sys_toggle_clicked();
            }
        });
    }
    if std::env::var("SLINT_OVERLAY_NO_AUTO_START").is_err() {
        let weak = overlay.as_weak();
        Timer::single_shot(Duration::from_millis(1900), move || {
            if let Some(o) = weak.upgrade() {
                // Guard against the user manually starting the session inside the
                // 1.9s window — without this the auto-start would toggle it OFF.
                if o.get_timer_active() {
                    return;
                }
                eprintln!("[overlay-host] auto-starting session on startup");
                o.invoke_timer_toggle_clicked();
            }
        });
    }

    // V0.8.4 — first launch (no config.json): auto-open the guided setup wizard
    // a beat after the bar is up, so the bar has pinned + realized first. The
    // wizard is created stealth-aware (centred on the picked monitor). Step 1's
    // mode pick writes config.json, so this branch will not fire again next run.
    if first_run {
        eprintln!("[overlay-host] first run detected — auto-opening setup wizard");
        let wz = wizard.clone();
        let cfg_w = cfg.clone();
        let st = settings.clone();
        let state_w = state.clone();
        let ow = overlay.as_weak();
        // Phase 1 (§5.1) — the wizard's stealth toggle re-stealths every open
        // window through this registry clone (no per-window forwarding).
        let registry_w = registry.clone();
        Timer::single_shot(Duration::from_millis(2200), move || {
            open_wizard(&wz, &cfg_w, &state_w, &ow, &st, &registry_w);
        });
    }

    // Memory Phase 1 — crash-recovery offer. A beat after the bar is up (same
    // delayed-open as the wizard, so the bar pins/realizes first), check the
    // newest journal: if the previous run ended WITHOUT a clean stop, offer to
    // carry its context forward. Skipped on first run (no prior sessions, and
    // we never want two startup windows fighting). The detection is a single
    // bounded file read on the UI thread inside the timer — cheap; nothing is
    // shown when it returns None.
    //
    // GATED OFF by default (opt-in: SLINT_OVERLAY_RECOVERY) pending the
    // auto-start-sequencing fix. Regression sweep 2026-06-03 found 3 HIGH defects:
    // the 2200ms scan races the 1900ms auto-start and latches onto the just-
    // started LIVE session (false "recover previous session" on every launch), it
    // shadows any genuinely-crashed prior journal (newest-by-mtime), and a clean
    // Quit/restart/updater exit never writes SessionStop so it also looks like a
    // crash. Re-enable once (a) the scan runs BEFORE auto-start / excludes the
    // current session, (b) clean exits write SessionStop, and (c) accepting
    // recovery does not double-start. The detection (journal.rs) is sound + tested.
    if !first_run && std::env::var("SLINT_OVERLAY_RECOVERY").is_ok() {
        let ro = recover_offer.clone();
        let cfg_r = cfg.clone();
        let events_r = events.clone();
        let rt_r = slint_rt.clone();
        let rth_r = rt_handle.clone();
        let state_r = state.clone();
        let ow_r = overlay.as_weak();
        Timer::single_shot(Duration::from_millis(2200), move || {
            match journal::find_unfinished_session_in_default_dir() {
                Some(unfinished) => {
                    // Log the LINK id + counts only — never transcript/answer text.
                    eprintln!(
                        "[overlay-host] unfinished session detected ({}): {} line(s), qa={} — offering recovery",
                        unfinished.session_id,
                        unfinished.last_lines.len(),
                        unfinished.last_qa.is_some(),
                    );
                    open_recover_offer(
                        &ro, unfinished, &cfg_r, &events_r, &rt_r, &rth_r, &state_r, &ow_r,
                    );
                }
                None => {
                    eprintln!("[overlay-host] no unfinished session to recover");
                }
            }
        });
    }

    let result = overlay.run();
    // E10.4 — kill any local-AI servers the in-app installer launched so they
    // do not outlive the app (best-effort; clean-exit path only).
    {
        let mut s = state.lock().unwrap_or_else(|p| p.into_inner());
        for mut child in s.local_ai_servers.drain(..) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    // Tokio MT-runtime drop cancels spawned tasks at their next .await
    // (NOT graceful — they don't get to finish their HTTP response).
    // shutdown_timeout gives in-flight tasks a budgeted window to wrap
    // up; UI still exits promptly if they take too long. Comment fix
    // per review-agent finding 2026-05-27 (previous comment claimed
    // unconditional graceful drop, which is wrong).
    tokio_rt.shutdown_timeout(Duration::from_secs(2));
    result
}

// `classify_ai_error` moved to slint_replay::app_state so the unit
// tests can pin the categories table without spinning up the UI.
use slint_replay::app_state::classify_ai_error;

/// Recompute status pill based on capture flags.
fn refresh_status(overlay: &OverlayBarWindow, mic: bool, sys: bool) {
    let (text, color) = match (mic, sys) {
        (true, true) => ("recording 🎤🗣", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (true, false) => ("mic only 🎤", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (false, true) => ("sys only 🗣", slint::Color::from_rgb_u8(0x6c, 0xcf, 0xff)),
        (false, false) => ("idle", slint::Color::from_rgb_u8(0x88, 0x88, 0x8c)),
    };
    overlay.set_status_text(SharedString::from(text));
    overlay.set_status_color(color);
}

fn get_mic_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.mic_active).unwrap_or(false)
}

fn get_sys_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.sys_active).unwrap_or(false)
}

/// Apply transparent-overlay HWND flags to the overlay bar.
/// V0.8.0 (Поток B) — spawn a fresh copy of ourselves (with `--relaunch`) and
/// quit the current event loop so the post-`run()` teardown runs (kills the
/// possibly-hung local-AI servers; the child's `ensure_servers` then starts
/// fresh ones — this is what recovers a hung local model). The child blocks on
/// the singleton mutex until WE fully exit, so the two bars never overlap.
///
/// All persisted settings (incl. `stealth_enabled`) live in config.json, which
/// the child reloads — so the new instance comes up with the SAME stealth state
/// (and, thanks to Поток C, comes up flash-free under stealth). Returns true if
/// the child spawned (so the caller proceeds to quit); false if we couldn't
/// find/launch our own exe (then we must NOT quit — that would just close the
/// app with nothing to replace it).
fn spawn_relaunch() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[overlay-host] relaunch: current_exe failed: {e}; staying up");
            return false;
        }
    };
    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP so the child is fully
    // independent of this (exiting) process and its console/group.
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    match std::process::Command::new(&exe)
        .arg("--relaunch")
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
    {
        Ok(child) => {
            eprintln!(
                "[overlay-host] relaunch: spawned child pid={} from {:?}",
                child.id(),
                exe
            );
            true
        }
        Err(e) => {
            eprintln!("[overlay-host] relaunch: spawn failed: {e}; staying up");
            false
        }
    }
}

fn apply_overlay_hwnd(overlay: &OverlayBarWindow) {
    // Поток C (stealth bar-flash fix) — when stealth is on, park the bar OFF the
    // virtual desktop synchronously NOW (this fn runs before overlay.run(), which
    // composites the window). Without this the bar was shown at winit's default
    // position and only stealthed ~200 ms later by the timer below, so a screen-
    // share saw a flash of the bar on every cold start — and would on every
    // emergency restart (Поток B). The timer applies WDA *before* the pin moves
    // the bar on-screen, so the first on-screen frame is already capture-excluded.
    // Mirrors present_tile_window for tiles.
    if global_stealth() {
        overlay
            .window()
            .set_position(slint::PhysicalPosition::new(-32000, -32000));
    }
    let weak = overlay.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
        let Some(o) = weak.upgrade() else { return };
        match grab_hwnd(o.window()) {
            Ok(hwnd) => {
                match make_transparent_overlay(hwnd) {
                    Ok(()) => eprintln!("[overlay-host] overlay transparency wired"),
                    Err(e) => eprintln!("[overlay-host] overlay transparency failed: {e}"),
                }
                // #E10.2 — apply persisted stealth to the bar on launch.
                if global_stealth() {
                    let _ = set_stealth(hwnd, true);
                    let _ = set_skip_taskbar(hwnd, true);
                }
                // #127 — pin the bar to the PRIMARY monitor. The bar has no
                // position logic of its own; Slint/winit's default placement
                // can drop it onto the user's PORTRAIT secondary (at negative
                // X) or straddle two displays. Centre it near the top of
                // primary. One-shot at launch — the user can still drag it
                // afterward (the logo is a drag handle).
                // Поток C — the pin MUST always land the bar on-screen: under
                // stealth we parked it at (-32000) above, so any path that skips
                // the move would strand the bar off the desktop (the bar is the
                // whole control surface — the user would be locked out). Compute
                // the target with safe fallbacks (primary monitor → its origin →
                // (60, 24)) and ALWAYS move.
                let primary = enum_monitors().into_iter().find(|m| m.is_primary);
                let bar_w = get_window_rect(hwnd).map(|(_, _, w, _)| w).unwrap_or(0);
                let (x, y) = match primary {
                    Some(p) => (p.left + ((p.width() - bar_w) / 2).max(0), p.top + 24),
                    None => (60, 24),
                };
                match move_window_pos_only(hwnd, x, y) {
                    Ok(()) => eprintln!("[overlay-host] bar pinned at ({x}, {y})"),
                    Err(e) => {
                        // Last resort: even the pin failed — try a hard (60,24) so
                        // a stealth-parked bar can't stay invisible at (-32000).
                        eprintln!("[overlay-host] bar pin failed: {e}; retry at (60,24)");
                        let _ = move_window_pos_only(hwnd, 60, 24);
                    }
                }
            }
            Err(e) => eprintln!("[overlay-host] overlay HWND grab failed: {e}"),
        }
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
fn select_recent_labeled(
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
fn fire_f3_reask(
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
fn fire_f6_manual_spawn(
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
fn fire_f9_ask(
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
fn spawn_ptt_watchdog(stop: Arc<AtomicBool>) {
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
fn fire_ptt_ask(
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

/// V0.8.3 — "Написать": open (or re-focus) the small text-input window. On
/// submit it routes the typed text through `fire_f9_ask(.., Some(text))`, so the
/// whole tile-create + stream + cost + journal + follow-up pipeline is reused →
/// the answer lands in a standard tile. Stealth (WDA) + on-screen placement come
/// from `present_window_stealth_aware`; the decorate closure also grabs keyboard
/// focus so the user can type immediately. Esc (or submit) hides + drops it.
#[allow(clippy::too_many_arguments)]
fn open_text_ask(
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
fn open_help(
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

/// Open (or reuse) the KB palette window. Auto-spawn a tile when
/// the user activates a result, mimicking the React palette flow.
fn open_palette(
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

/// Open the settings window. Reuses existing instance if open.
/// Short, human display name for a model id: drop a `.gguf`/`.bin` extension,
/// then take the first token (or the tier after "claude-"). Used by the bar's
/// active-stack readout. (#E10.2)
fn short_model_name(full: &str) -> String {
    let base = full.trim_end_matches(".gguf").trim_end_matches(".bin");
    let parts: Vec<&str> = base
        .split(['-', ':', '/', ' '])
        .filter(|s| !s.is_empty())
        .collect();
    match parts.first() {
        Some(&"claude") if parts.len() > 1 => parts[1].to_string(),
        Some(first) => (*first).to_string(),
        None => "—".to_string(),
    }
}

/// Build the bar's "active stack" label: which STT engine + which AI model are
/// live, prefixed with 🟢 (all-local), ☁ (all-cloud), or ◐ (mixed). (#E10.2)
pub(crate) fn active_stack_label(c: &overlay_backend::config::Config) -> String {
    let (stt, stt_local): (String, bool) = match c.stt_provider.as_str() {
        // Show the GigaAM accelerator so the bar reflects GPU (DirectML) vs CPU.
        "gigaam" => (
            format!("GigaAM {}", if c.stt_gigaam_gpu { "GPU" } else { "CPU" }),
            true,
        ),
        "whisper" => ("Whisper".to_string(), true),
        _ => ("Groq".to_string(), false),
    };
    let ai_local = c.ai_provider == "local";
    let model_full = if ai_local {
        c.ai_local_model.as_str()
    } else {
        c.ai_model.as_str()
    };
    let model = short_model_name(model_full);
    // ASCII tag + Latin-1 middle dot only — fancier glyphs (✕/✓/arrows) render
    // as missing-glyph boxes on the user's Slint+skia font fallback.
    let tag = if stt_local && ai_local {
        "local"
    } else if !stt_local && !ai_local {
        "cloud"
    } else {
        "mixed"
    };
    format!("{tag}: {stt} · {model}")
}

/// Which model dropdown a fetch populates — the cloud bridge or the local server.
#[derive(Clone, Copy)]
enum ModelTarget {
    Cloud,
    Local,
}

/// Fetch a server's model list (`GET {base_url}/models`) off-thread and populate
/// the matching Settings dropdown (cloud bridge or local), pre-selecting the
/// saved model (kept in the list even if the server is down so it's never lost).
/// Reuses the test-button pattern — a throwaway current-thread runtime +
/// invoke_from_event_loop — because open_settings has no rt_handle. Reads cfg
/// inside the worker thread so it never contends with a config lock held on the
/// UI thread. No-op when the base URL is blank. (#E10.1)
fn fetch_models(
    weak: slint::Weak<SettingsWindow>,
    cfg: overlay_backend::config::SharedConfig,
    target: ModelTarget,
) {
    std::thread::spawn(move || {
        let (base_url, bearer, saved) = {
            let c = cfg.read();
            match target {
                ModelTarget::Cloud => (
                    c.ai_base_url.clone(),
                    c.ai_bearer.clone(),
                    c.ai_model.clone(),
                ),
                ModelTarget::Local => (
                    c.ai_local_base_url.clone(),
                    c.ai_local_bearer.clone(),
                    c.ai_local_model.clone(),
                ),
            }
        };
        if base_url.trim().is_empty() {
            return;
        }
        let models: Vec<String> = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .and_then(|rt| {
                rt.block_on(overlay_backend::ai::list_models(&base_url, &bearer))
                    .ok()
            })
            .unwrap_or_default();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut list = models;
            if !saved.is_empty() && !list.iter().any(|m| m == &saved) {
                list.insert(0, saved.clone());
            }
            let idx = list.iter().position(|m| m == &saved).unwrap_or(0) as i32;
            let shared: Vec<SharedString> = list.into_iter().map(SharedString::from).collect();
            let model = ModelRc::new(VecModel::from(shared));
            match target {
                ModelTarget::Cloud => {
                    w.set_ai_models(model);
                    w.set_ai_model_index(idx);
                }
                ModelTarget::Local => {
                    w.set_ai_local_models(model);
                    w.set_ai_local_model_index(idx);
                }
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn open_settings(
    state: &slint_replay::app_state::SharedState,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    tiles_ref: &TileWindows,
    cfg: &overlay_backend::config::SharedConfig,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    // Phase 1 (§5.1) — the Settings-tab stealth toggle + colour-scheme switch
    // now reach every open window through this registry (text_ask / palette /
    // wizard / 🆘 help / recover-offer included), and the nested "Run setup
    // wizard" button forwards the same registry to `open_wizard`.
    registry: &WindowRegistry,
) {
    // Light up the bar's ⚙ chip while Settings is open (user: "значок
    // настроек не загорается когда настройки открыты"). Cleared in the
    // window's close handler below.
    if let Some(o) = overlay_weak.upgrade() {
        o.set_settings_open(true);
    }
    let mut settings_slot = settings_ref.borrow_mut();
    if let Some(existing) = settings_slot.as_ref() {
        // Refresh token status + profiles — config might have changed since last open.
        populate_token_status(existing, cfg);
        populate_diagnostics(existing, cfg);
        {
            let snap = cfg.read();
            refresh_profiles(existing, &snap);
        }
        let _ = existing.show();
        return;
    }
    let win = match SettingsWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] SettingsWindow::new failed: {e}");
            return;
        }
    };
    {
        let st = state.lock().ok();
        if let Some(st) = st {
            win.set_always_on_top_toggle(st.always_on_top);
            win.set_stealth_toggle(st.stealth);
        }
    }
    populate_token_status(&win, cfg);
    populate_diagnostics(&win, cfg);
    // Phase E8 — show the running version in the Updates tab.
    win.set_app_version(SharedString::from(env!("CARGO_PKG_VERSION")));
    // Phase E6 v29 / F — load the profile list + active context into the editor,
    // and seed the Coaching + Auto-tiles controls (previously dead/cosmetic).
    {
        let snap = cfg.read();
        refresh_profiles(&win, &snap);
        win.set_coaching_debrief(snap.post_meeting_debrief_enabled);
        win.set_auto_tiles_enabled(snap.auto_tiles_enabled);
        win.set_trigger_keywords_input(SharedString::from(snap.trigger_keywords.as_str()));
    }

    // Phase E6 v23 — populate the Audio tab's mic dropdown from real
    // WASAPI capture endpoints + select the saved device. User: "Audio
    // не подгружает реальные микрофоны".
    {
        // V0.8.4 — WASAPI device enumeration (cold COM + a per-endpoint
        // friendly-name RPC to the audio service) was ~30-300ms of SYNCHRONOUS
        // pre-show stall on the UI thread, which made the gear feel laggy. Show a
        // placeholder now and fill the dropdown when enumeration returns from a
        // worker thread (mirrors the mic-test / fetch_models off-thread pattern).
        win.set_mic_devices(ModelRc::new(VecModel::from(vec![SharedString::from(
            "(loading devices…)",
        )])));
        win.set_mic_device_index(0);
        let saved = cfg.read().mic_device.clone();
        let weak = win.as_weak();
        std::thread::spawn(move || {
            let devices = overlay_backend::audio::list_devices()
                .map(|d| d.inputs)
                .unwrap_or_default();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(w) = weak.upgrade() else { return };
                let model: Vec<SharedString> = if devices.is_empty() {
                    vec![SharedString::from("(no capture devices found)")]
                } else {
                    devices
                        .iter()
                        .map(|d| SharedString::from(d.as_str()))
                        .collect()
                };
                // Find the saved device's index (default 0 = system default).
                let sel = saved
                    .as_deref()
                    .and_then(|name| devices.iter().position(|d| d == name))
                    .unwrap_or(0);
                w.set_mic_devices(ModelRc::new(VecModel::from(model)));
                w.set_mic_device_index(sel as i32);
            });
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_mic_device_selected(move |name| {
            let mut c = cfg_c.write();
            c.mic_device = Some(name.to_string());
            let _ = overlay_backend::config::save(&c);
            eprintln!("[overlay-host] mic_device -> {name}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_mic_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_mic_test_result(SharedString::from("recording 3s…"));
            let device = cfg_c.read().mic_device.clone();
            let weak_for_result = w.as_weak();
            // Blocking WASAPI record off the UI thread; post result back.
            std::thread::spawn(move || {
                // M-1: take the single-mic guard so this test can't collide with
                // PTT / voice follow-up / dictation (a 2nd WASAPI open garbles
                // both). Report busy instead of recording.
                let msg = if !try_acquire_mic() {
                    "[!] mic busy — close PTT / dictation and retry".to_string()
                } else {
                    let result = overlay_backend::audio::record_mic_blocking(3000, device);
                    release_mic();
                    match result {
                        Ok(samples) if samples.is_empty() => "no audio captured".to_string(),
                        Ok(samples) => {
                            // RMS energy + a -45 dBFS speech threshold (silent room
                            // is < -55 dBFS). Shared helper with the diagnostics tab
                            // — User: "я могу ничего не говорить, но всё равно OK"
                            // was the old peak==0 check passing on any tiny noise.
                            let dbfs = overlay_backend::audio::rms_dbfs(&samples);
                            if dbfs < -45.0 {
                                format!(
                                    "[!] too quiet ({dbfs:.0} dBFS) — say something / check mic"
                                )
                            } else {
                                format!("[ok] heard you ({dbfs:.0} dBFS RMS)")
                            }
                        }
                        Err(e) => format!("error: {e}"),
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_for_result.upgrade() {
                        w.set_mic_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    let s2 = state.clone();
    let tiles_ref2 = tiles_ref.clone();
    win.on_always_on_top_changed(move |on| {
        if let Ok(mut st) = s2.lock() {
            st.always_on_top = on;
        }
        for t in tiles_ref2.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_always_on_top(hwnd, on);
            }
        }
    });

    let s3 = state.clone();
    let overlay_for_stealth = overlay_weak.clone();
    let cfg_st = cfg.clone();
    // Phase 1 (§5.1) — ONE registry clone replaces the per-window stealth loops
    // (tiles / this Settings window / text_ask / palette / 🆘 help / recover-offer
    // / wizard). Note: unlike the bar + wizard handlers, the Settings-tab toggle
    // does NOT drop the bar's taskbar button — that behaviour is preserved by
    // driving the bar inline here without `set_skip_taskbar`.
    let registry_stealth = registry.clone();
    win.on_stealth_changed(move |on| {
        if let Ok(mut st) = s3.lock() {
            st.stealth = on;
        }
        // #111 — global source-of-truth so later-created windows inherit it.
        set_global_stealth(on);
        // #E10.2 — persist so stealth survives a restart.
        {
            let mut c = cfg_st.write();
            c.stealth_enabled = on;
            let _ = config::save(&c);
        }
        // #111 — also flip the overlay bar itself (toggling stealth here
        // previously left it visible to capture). The bar stays inline (it is not
        // in the registry); the Settings window itself is covered by the registry.
        if let Some(o) = overlay_for_stealth.upgrade() {
            o.set_stealth_active(on);
            if let Ok(hwnd) = grab_hwnd(o.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // Every other open window (incl. this Settings window) via the one path.
        registry_stealth.apply_stealth(on);
    });

    // V0.8.4 — Settings → Interface "🪄 Run setup wizard" button. Re-opens the
    // guided first-run wizard on demand (it is also auto-shown on first launch).
    {
        // The wizard slot lives in the registry; forward the same registry so the
        // wizard's stealth toggle reaches every open window (Phase 1 §5.1).
        let wz = registry.wizard.clone();
        let cfg_w = cfg.clone();
        let st = settings_ref.clone();
        let state_w = state.clone();
        let ow = overlay_weak.clone();
        let registry_w = registry.clone();
        win.on_open_wizard_clicked(move || {
            open_wizard(&wz, &cfg_w, &state_w, &ow, &st, &registry_w);
        });
    }

    // Phase E6 — token + AI bridge config save wires.
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_ai_bearer_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("[overlay-host] ai_bearer save skipped: empty input");
                return;
            }
            {
                let mut c = cfg_c.write();
                c.ai_bearer = trimmed;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_bearer save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] ai_bearer saved to config.json");
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_groq_api_key_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("[overlay-host] groq_api_key save skipped: empty input");
                return;
            }
            {
                let mut c = cfg_c.write();
                c.groq_api_key = trimmed;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] groq_api_key save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] groq_api_key saved to config.json");
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_base_url_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            {
                let mut c = cfg_c.write();
                c.ai_base_url = trimmed.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_base_url save failed: {e:#}");
                    return;
                }
            }
            // Log presence only — ai_base_url often embeds the user's LAN
            // IP / proxy port (network-topology leak). See ai.rs no-log note.
            eprintln!("[overlay-host] ai_base_url saved ({} chars)", trimmed.len());
            // #E10.1 — re-query the cloud model list against the new URL.
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Cloud);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_model_selected(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            {
                let mut c = cfg_c.write();
                c.ai_model = trimmed.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_model save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] ai_model selected: {trimmed}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_models_refresh(move || {
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Cloud);
        });
    }
    {
        // E9 — experimental prompt-caching toggle (default off; persists +
        // applies live via the ai.rs static).
        let cfg_c = cfg.clone();
        win.on_ai_prompt_cache_changed(move |on| {
            {
                let mut c = cfg_c.write();
                c.ai_prompt_cache = on;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_prompt_cache save failed: {e:#}");
                    return;
                }
            }
            overlay_backend::ai::set_prompt_cache(on);
            diag!("ai_prompt_cache -> {on}");
        });
    }
    // E9 Phase 1 — local AI provider switch + local-field saves + test.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_provider_changed(move |idx| {
            let provider = if idx == 1 { "local" } else { "cloud" };
            let mut c = cfg_c.write();
            c.ai_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_provider save failed: {e:#}");
                return;
            }
            overlay_backend::ai::set_local_no_think(provider == "local" && !c.ai_local_thinking);
            drop(c);
            diag!("ai_provider -> {provider}");
            // #E10.1 — switching to Local auto-populates the model dropdown.
            if provider == "local" {
                fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.ai_local_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_base_url save failed: {e:#}");
                return;
            }
            drop(c);
            // #E10.1 — re-query models against the new URL.
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.ai_local_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_bearer save failed: {e:#}");
            }
        });
    }
    // ===== V4 — vision (screenshot) channel: provider switch + field saves + test =====
    {
        let cfg_c = cfg.clone();
        win.on_vision_provider_changed(move |idx| {
            let provider = match idx {
                0 => "off",
                1 => "same",
                3 => "local",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.vision_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_provider save failed: {e:#}");
                return;
            }
            diag!("vision_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_phonetics_changed(move |on| {
            let mut c = cfg_c.write();
            c.vision_phonetics = on;
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_phonetics save failed: {e:#}");
                return;
            }
            diag!("vision_phonetics -> {on}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_model save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_model save failed: {e:#}");
            }
        });
    }
    {
        // Vision connection test — resolve the vision endpoint, reuse the AI
        // bridge tester. Off-thread so the HTTP round-trip can't freeze the UI.
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_vision_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_vision_test_result(SharedString::from("testing…"));
            let Some(ep) = cfg_c.read().vision_endpoint() else {
                w.set_vision_test_result(SharedString::from("[--] vision is off"));
                return;
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(
                        ep.base_url,
                        ep.bearer,
                        ep.model,
                    )) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_vision_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_model_selected(move |model| {
            let m = model.trim().to_string();
            if m.is_empty() {
                return;
            }
            let mut c = cfg_c.write();
            c.ai_local_model = m.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_model save failed: {e:#}");
                return;
            }
            diag!("ai_local_model selected: {m}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_models_refresh(move || {
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_vision_changed(move |on| {
            let mut c = cfg_c.write();
            c.ai_local_vision = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_thinking_changed(move |on| {
            let mut c = cfg_c.write();
            c.ai_local_thinking = on;
            let _ = overlay_backend::config::save(&c);
            // Mirror the boot-time + provider-switch logic: no-think is the
            // INVERSE of "thinking" and only applies to the local provider.
            overlay_backend::ai::set_local_no_think(c.ai_provider == "local" && !on);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_gigaam_gpu_changed(move |on| {
            let mut c = cfg_c.write();
            c.stt_gigaam_gpu = on;
            let _ = overlay_backend::config::save(&c);
            // Apply immediately: update the global ORT accelerator + drop the
            // cached model so the next transcription reloads on the new backend.
            // (The live session pipeline reloads its own copy next session.)
            overlay_backend::stt::configure_gigaam_accelerator(on);
            overlay_backend::stt::reset_gigaam_cache();
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_test_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_ai_local_test_result(SharedString::from("testing…"));
            let (base_url, bearer, model) = {
                let c = cfg_c.read();
                (
                    c.ai_local_base_url.clone(),
                    c.ai_local_bearer.clone(),
                    c.ai_local_model.clone(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::ai::test_connection(
                            base_url, bearer, model,
                        )) {
                            Ok(s) => format!("[ok] {s}"),
                            Err(e) => format!("[--] {e}"),
                        }
                    }
                    Err(e) => format!("[--] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_ai_local_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // E10.4 — one-click in-app local-AI installer. Runs the whole
    // download + launch pipeline on a worker thread, streams progress to
    // the panel, and on success stores the server handles (for kill-on-
    // quit), writes the local config (secrets preserved), and refreshes
    // the panel dropdowns + the bar's active-stack readout.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_install_local_ai_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_local_ai_installing() {
                return; // re-entry guard
            }
            w.set_local_ai_installing(true);
            w.set_local_ai_progress(0.0);
            w.set_local_ai_gpu(SharedString::from(""));
            w.set_local_ai_status(SharedString::from("Подготовка…"));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            // Shared cancel flag (lives in AppState so the Cancel button can
            // flip it); reset before each run.
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            std::thread::spawn(move || {
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_local_ai_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    let frac = if total > 0 {
                                        (done as f64 / total as f64) as f32
                                    } else {
                                        0.0
                                    };
                                    w.set_local_ai_progress(frac);
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_local_ai_status(SharedString::from(format!(
                                        "{label}: {:.0} / {:.0} MB",
                                        mb(done),
                                        mb(total)
                                    )));
                                }
                                overlay_backend::local_ai::Progress::Gpu(s) => {
                                    w.set_local_ai_on_gpu(s.starts_with("GPU"));
                                    w.set_local_ai_gpu(SharedString::from(s));
                                }
                            }
                        });
                    }
                };
                // Re-install hardening: stop any servers we previously launched
                // so a fresh `--mmproj` llama-server can bind :8080. Without this
                // a stale vision-less server keeps the port and the new one
                // silently fails to start (wait_ready still sees the old one and
                // reports success). Fresh installs have nothing to drain.
                {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    for mut child in s.local_ai_servers.drain(..) {
                        let _ = child.kill();
                    }
                }
                let opts = overlay_backend::local_ai::InstallOptions::default();
                match overlay_backend::local_ai::install(&opts, &cancel, &on) {
                    Ok(res) => {
                        let model = res.ai_local_model.clone();
                        let gigaam_dir = res.stt_gigaam_dir.clone();
                        let on_gpu = res.on_gpu;
                        {
                            let mut c = cfg_t.write();
                            overlay_backend::local_ai::apply_result(&mut c, &res);
                            if let Err(e) = overlay_backend::config::save(&c) {
                                eprintln!("[overlay-host] local-ai config save failed: {e:#}");
                            }
                            overlay_backend::ai::set_local_no_think(!c.ai_local_thinking);
                        }
                        {
                            let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                            s.local_ai_servers.extend(res.servers);
                        }
                        let weak_done = weak_t.clone();
                        let overlay_done = overlay_t.clone();
                        let cfg_done = cfg_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            diag!("local-ai installed: model={} gpu={}", model, on_gpu);
                            if let Some(w) = weak_done.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_progress(1.0);
                                w.set_local_ai_status(SharedString::from(
                                    "Готово. Локальный AI настроен и запущен.",
                                ));
                                w.set_ai_provider_index(1);
                                w.set_ai_local_base_url_input(SharedString::from(
                                    overlay_backend::local_ai::LLAMA_BASE_URL,
                                ));
                                w.set_stt_provider_index(2);
                                w.set_stt_whisper_url_input(SharedString::from(
                                    overlay_backend::local_ai::WHISPER_BASE_URL,
                                ));
                                w.set_stt_gigaam_dir_input(SharedString::from(gigaam_dir));
                            }
                            if let Some(o) = overlay_done.upgrade() {
                                o.set_active_stack(SharedString::from(active_stack_label(
                                    &cfg_done.read(),
                                )));
                            }
                        });
                    }
                    Err(e) => {
                        let cancelled = e
                            .to_string()
                            .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                        let msg = if cancelled {
                            "Отменено.".to_string()
                        } else {
                            eprintln!("[overlay-host] local-ai install failed: {e:#}");
                            format!("Ошибка установки: {e}")
                        };
                        let weak_err = weak_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_err.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_status(SharedString::from(msg));
                            }
                        });
                    }
                }
            });
        });
    }

    // E10.4 — Cancel button: flip the shared cancel flag the install worker
    // thread + the curl poll loop watch.
    {
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_cancel_local_ai_clicked(move || {
            {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(w) = weak.upgrade() {
                w.set_local_ai_status(SharedString::from("Отмена…"));
            }
        });
    }

    // Phase E6 v20 — tile opacity slider. Persists to config AND
    // applies to all currently-visible tiles via tiles_ref.
    {
        let cfg_c = cfg.clone();
        let tiles_c = tiles_ref.clone();
        win.on_tile_body_opacity_changed(move |new_value| {
            let clamped = new_value.clamp(0.5, 1.0);
            {
                let mut c = cfg_c.write();
                c.tile_body_opacity = clamped;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tile_body_opacity save failed: {e:#}");
                    return;
                }
            }
            // Phase E6 v36 — update the process-global so EVERY future
            // tile (F9 / F3 / KB-palette / auto-spawn) spawns at this
            // opacity, not just the ones currently on screen.
            set_global_tile_opacity(clamped);
            // Apply live to all currently-visible tiles.
            for tile in tiles_c.borrow().iter() {
                tile.set_body_opacity(clamped);
            }
            eprintln!("[overlay-host] tile_body_opacity -> {clamped:.2}");
        });
    }

    // Phase E6 v38 — interface-language switch. Selecting Русский/English
    // in the Interface tab switches the bundled translation LIVE (Slint
    // re-evaluates every @tr() binding) and persists ui_language so the
    // choice survives restart. Previously the dropdown was inert — it
    // showed "Русский" but never applied anything, so a stale .po made
    // the UI look English even though "ru" was nominally selected.
    {
        let cfg_lang = cfg.clone();
        win.on_language_selected(move |idx| {
            let lang = if idx == 1 { "en" } else { "ru" };
            match slint::select_bundled_translation(lang) {
                Ok(()) => eprintln!("[overlay-host] UI language -> {lang}"),
                Err(e) => eprintln!("[overlay-host] language {lang} not available: {e}"),
            }
            let mut c = cfg_lang.write();
            c.ui_language = lang.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ui_language save failed: {e:#}");
            }
        });
    }

    // Colour-scheme switch. Selecting a scheme in the Interface tab recolours
    // EVERY window live (Theme is a per-window global, so we walk each one),
    // updates the process-global so future tiles/palette inherit it, and
    // persists color_scheme. Mirrors the tile-opacity handler's shape.
    {
        let cfg_scheme = cfg.clone();
        let overlay_scheme = overlay_weak.clone();
        // Phase 1 (§5.1) — re-skin every open window through the registry (the
        // bar stays inline). This now also reaches the palette / text_ask /
        // wizard / 🆘 help / recover-offer windows if open — same "no window
        // forgotten" guarantee as stealth; previously only tiles + Settings were
        // re-skinned live (the others kept their construction-time scheme).
        let registry_scheme = registry.clone();
        win.on_color_scheme_selected(move |idx| {
            let scheme = clamp_scheme(idx);
            // Persist first so a crash mid-repaint still survives the choice.
            {
                let mut c = cfg_scheme.write();
                c.color_scheme = scheme;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] color_scheme save failed: {e:#}");
                    return;
                }
            }
            // Future windows (tiles, palette) read this at construction.
            set_global_scheme(scheme);
            // Re-skin all currently-live windows: bar inline, the rest via registry.
            if let Some(o) = overlay_scheme.upgrade() {
                apply_scheme_bar(&o, scheme);
            }
            registry_scheme.apply_scheme(scheme);
            eprintln!("[overlay-host] color_scheme -> {scheme}");
        });
    }

    // Phase E6 v27 — AI bridge connection test. Off-thread (local
    // current-thread tokio runtime) so the blocking HTTP round-trip
    // doesn't freeze the UI; result posted back via invoke_from_
    // event_loop. ASCII status prefixes (no ✓/✗ missing-glyphs).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_bridge_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_ai_bridge_test_result(SharedString::from("testing…"));
            let (base_url, bearer, model) = {
                let c = cfg_c.read();
                (
                    c.ai_base_url.clone(),
                    c.ai_bearer.clone(),
                    c.ai_model.clone(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(
                        base_url, bearer, model,
                    )) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_ai_bridge_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Phase E6 v27 — STT (Groq) connection test. Same off-thread
    // pattern; hits the Groq /models endpoint with the saved key.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_stt_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_stt_test_result(SharedString::from("testing…"));
            let backend = cfg_c.read().stt_backend();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::stt::test_connection_backend(&backend)) {
                            Ok(s) => format!("[ok] {s}"),
                            Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                        }
                    }
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_stt_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // P1.1 — "Copy report": redacted diagnostics → clipboard with a brief
    // "copied" confirmation. build_diag_report masks the LAN bridge IP and
    // carries no bearer / API key / transcript / profile text.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_diagnostics_copy_report_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let report = build_diag_report(&cfg_c);
            match clipboard_win::set_clipboard_string(&report) {
                Ok(()) => {
                    w.set_diag_copied(true);
                    let wk = w.as_weak();
                    Timer::single_shot(Duration::from_millis(1800), move || {
                        if let Some(w) = wk.upgrade() {
                            w.set_diag_copied(false);
                        }
                    });
                }
                Err(e) => eprintln!("[overlay-host] diag report copy failed: {e}"),
            }
        });
    }

    // #131 — diagnostics "Проверить всё": live-ping the ACTIVE AI endpoint
    // (resolved via ai_endpoint — NOT the raw cloud fields) + the active STT
    // backend, in ONE off-thread runtime, and write both rows back. Mic / sys
    // / stealth rows stay config-readiness (their live checks live on Audio).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_diagnostics_check_all_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_diag_ai_level(-1);
            w.set_diag_ai_detail(SharedString::from(""));
            w.set_diag_stt_level(-1);
            w.set_diag_stt_detail(SharedString::from(""));
            w.set_diag_mic_level(-1);
            w.set_diag_sys_level(-1);
            let (ai_base, ai_bearer, ai_model, stt_backend, mic_device, sys_device) = {
                let c = cfg_c.read();
                let ep = c.ai_endpoint(false);
                (
                    ep.base_url,
                    ep.bearer,
                    ep.model,
                    c.stt_backend(),
                    c.mic_device.clone(),
                    c.system_audio_device.clone(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                // 1. AI + STT live pings (async, on a throwaway runtime).
                let (ai_level, ai_msg, stt_level, stt_msg): (i32, String, i32, String) =
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => {
                            let (al, am): (i32, String) = match rt.block_on(
                                overlay_backend::ai::test_connection(ai_base, ai_bearer, ai_model),
                            ) {
                                Ok(s) => (0, format!("[ok] {s}")),
                                Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                            };
                            let (sl, sm): (i32, String) = match rt.block_on(
                                overlay_backend::stt::test_connection_backend(&stt_backend),
                            ) {
                                Ok(s) => (0, format!("[ok] {s}")),
                                Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                            };
                            (al, am, sl, sm)
                        }
                        Err(e) => {
                            let m = format!("[err] runtime: {e}");
                            (4, m.clone(), 4, m)
                        }
                    };
                let weak_a = weak_res.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_a.upgrade() {
                        w.set_diag_ai_level(ai_level);
                        w.set_diag_ai_detail(SharedString::from(ai_msg));
                        w.set_diag_stt_level(stt_level);
                        w.set_diag_stt_detail(SharedString::from(stt_msg));
                    }
                });
                // 2. Microphone — record 3s. "Готов" if the capture path works
                // (device opens + samples flow); a quiet result is fine (you
                // just didn't speak) — only a device error fails.
                // M-1: guard the diagnostics mic probe with the single-mic lock
                // too, so "Проверить всё" during an active session reports busy
                // instead of fighting PTT/voice/dictation for the device.
                let (mic_level, mic_msg): (i32, String) = if !try_acquire_mic() {
                    (
                        4,
                        "[!] mic busy — close PTT / dictation and retry".to_string(),
                    )
                } else {
                    let r = overlay_backend::audio::record_mic_blocking(3000, mic_device);
                    release_mic();
                    match r {
                        Ok(s) if s.is_empty() => (4, "[!] no audio captured".to_string()),
                        Ok(s) => {
                            let dbfs = overlay_backend::audio::rms_dbfs(&s);
                            if dbfs >= -45.0 {
                                (0, format!("[ok] heard you ({dbfs:.0} dBFS)"))
                            } else {
                                (0, format!("[ok] capture works · quiet ({dbfs:.0} dBFS)"))
                            }
                        }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    }
                };
                let weak_m = weak_res.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_m.upgrade() {
                        w.set_diag_mic_level(mic_level);
                        w.set_diag_mic_detail(SharedString::from(mic_msg));
                    }
                });
                // 3. System audio — SELF-TEST: play a short test tone through the
                // default output while capturing the loopback. If the loopback
                // hears our own tone, the output→loopback path works — the user
                // doesn't have to play anything.
                let (sys_level, sys_msg): (i32, String) =
                    match overlay_backend::audio::play_tone_and_capture(sys_device) {
                        Ok(s) => {
                            let dbfs = overlay_backend::audio::rms_dbfs(&s);
                            if dbfs > -60.0 {
                                (
                                    0,
                                    format!("[ok] loopback heard the test tone ({dbfs:.0} dBFS)"),
                                )
                            } else {
                                (
                                    4,
                                    "[!] test tone not captured — output device ≠ loopback source?"
                                        .to_string(),
                                )
                            }
                        }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_diag_sys_level(sys_level);
                        w.set_diag_sys_detail(SharedString::from(sys_msg));
                    }
                });
            });
        });
    }

    // Phase E10 — STT provider selector + local-engine fields.
    {
        let cfg_c = cfg.clone();
        win.on_stt_provider_changed(move |idx| {
            let provider = match idx {
                1 => "gigaam",
                2 => "whisper",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.stt_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_provider save failed: {e:#}");
                return;
            }
            diag!("stt_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_gigaam_dir_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_gigaam_dir = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_gigaam_dir save failed: {e:#}");
                return;
            }
            diag!("stt_gigaam_dir saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_url_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_url = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_url save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_url saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_bearer_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_bearer = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_bearer save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_bearer saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_model_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_model = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_model save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_model saved ({} chars)", trimmed.len());
        });
    }

    // P1.7 — config parsed from a picked server-settings file, awaiting the
    // user's explicit Apply (set by the import-preview handler, taken by Apply,
    // cleared by Cancel). Kept out of the live config until confirmed.
    let pending_server_import: Rc<RefCell<Option<overlay_backend::config::Config>>> =
        Rc::new(RefCell::new(None));

    // Phase E6 v28 — full-profile export (incl. keys). Native save
    // dialog via rfd; writes the whole config.json to the chosen path.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_export_profile_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Export overlay-mvp settings (contains API keys)")
                .set_file_name("suflyor-settings.json")
                .add_filter("JSON", &["json"])
                .save_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "export cancelled".to_string(),
                Some(path) => match overlay_backend::config::export_to(&path, &snapshot) {
                    Ok(()) => format!("[ok] exported to {}", path.display()),
                    Err(e) => format!("[err] {e:#}"),
                },
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // Phase E6 v28 — full-profile import. Native open dialog; loads +
    // persists the config, then re-syncs the token-status display.
    // Live re-apply of every field would need a broader refresh, so
    // we tell the user to restart for full effect.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_import_profile_clicked(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Import overlay-mvp settings")
                .add_filter("JSON", &["json"])
                .pick_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "import cancelled".to_string(),
                Some(path) => match overlay_backend::config::import_from(&path) {
                    Ok(imported) => {
                        // Push the freshly-loaded values into the shared
                        // config so the running session sees them, then
                        // refresh the token-status display.
                        *cfg_c.write() = imported;
                        msg_refresh_after_import(&w, &cfg_c)
                    }
                    Err(e) => format!("[err] {e:#}"),
                },
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — server-ONLY EXPORT. Native save dialog; writes ONLY the AI/STT
    // server fields (incl. creds — intentional for a PC->PC transfer) and none
    // of the machine-local fields (profiles/devices/snippets/context).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_export_server_settings_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Export server settings (AI/STT only — contains API keys)")
                .set_file_name("suflyor-server-settings.json")
                .add_filter("JSON", &["json"])
                .save_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "export cancelled".to_string(),
                Some(path) => {
                    match overlay_backend::config::export_server_settings_to(&path, &snapshot) {
                        Ok(()) => format!("[ok] server settings exported to {}", path.display()),
                        Err(e) => format!("[err] {e:#}"),
                    }
                }
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — server-ONLY settings import, NOW two-step: pick a file -> show a
    // REDACTED preview (provider/url/model old->new + key presence as set/—;
    // never a secret value) and stash the parsed config; the user then clicks
    // Apply (below) to actually merge. The machine-local GigaAM model path is
    // kept from THIS PC on apply (apply_server_settings).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_import_server_settings_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Import server settings (AI/STT only) from a backup")
                .add_filter("JSON", &["json"])
                .pick_file();
            let Some(w) = weak.upgrade() else { return };
            let Some(path) = picked else {
                w.set_profile_io_result(SharedString::from("import cancelled"));
                return;
            };
            // Read + parse + build the redacted preview. The parse error stays
            // value-free (parse_config_bytes inside). No save happens yet.
            match overlay_backend::config::preview_server_settings_from(&path, &snapshot) {
                Ok((preview, imported)) => {
                    apply_server_preview(&w, &preview);
                    *pending.borrow_mut() = Some(imported);
                    w.set_server_preview_ready(true);
                    w.set_profile_io_result(SharedString::from(
                        "review the changes below, then Apply",
                    ));
                }
                Err(e) => {
                    *pending.borrow_mut() = None;
                    w.set_server_preview_ready(false);
                    w.set_profile_io_result(SharedString::from(format!("[err] {e:#}")));
                }
            }
        });
    }

    // P1.7 — APPLY the previewed server settings. Merges the stashed config's
    // server fields onto the current one (EXCLUDING the machine-local GigaAM
    // dir), persists, applies live, and refreshes the token-status display.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_apply_server_settings_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let Some(imported) = pending.borrow_mut().take() else {
                w.set_server_preview_ready(false);
                w.set_profile_io_result(SharedString::from("nothing to apply"));
                return;
            };
            let merged = {
                let current = cfg_c.read().clone();
                overlay_backend::config::apply_server_settings(&current, imported)
            };
            w.set_server_preview_ready(false);
            let msg = match overlay_backend::config::save(&merged) {
                Ok(()) => {
                    // Apply to the running session + refresh token-status.
                    *cfg_c.write() = merged;
                    let _ = msg_refresh_after_import(&w, &cfg_c);
                    "[ok] server settings applied (AI/STT providers, URLs, models, keys). Local profiles, devices, UI and snippets kept; the local GigaAM model path was kept from this PC. Restart for full effect.".to_string()
                }
                Err(e) => format!("[err] {e:#}"),
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — CANCEL the preview: drop the stashed config + hide the diff.
    {
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_cancel_server_settings_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            *pending.borrow_mut() = None;
            w.set_server_preview_ready(false);
            w.set_profile_io_result(SharedString::from("import cancelled"));
        });
    }

    // Phase E6 v29 — meeting-context (Profile) save. Writes to
    // cfg.meeting_context + persists; new AI calls read it from cfg
    // so it applies immediately (no restart needed for this field).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_meeting_context_save(move |text| {
            {
                let mut c = cfg_c.write();
                // Phase F — also mirror into the active profile so the picker
                // and the live context never drift.
                c.save_active_context(&text);
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] meeting_context save failed: {e:#}");
                    if let Some(w) = weak.upgrade() {
                        w.set_meeting_context_result(SharedString::from("[err] save failed"));
                    }
                    return;
                }
            }
            let chars = text.chars().count();
            eprintln!("[overlay-host] meeting_context saved ({chars} chars)");
            if let Some(w) = weak.upgrade() {
                w.set_meeting_context_result(SharedString::from(format!(
                    "[ok] saved ({chars} chars)"
                )));
            }
        });
    }
    // Phase F — multi-profile picker handlers. Each mutates cfg, persists, and
    // refreshes the picker + editor from cfg so the UI mirrors config exactly.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_selected(move |idx| {
            if idx < 0 {
                return;
            }
            let mut c = cfg_c.write();
            c.select_profile(idx as usize);
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_add(move |name| {
            let mut c = cfg_c.write();
            let ok = c.add_profile(name.as_str()).is_some();
            if ok {
                let _ = overlay_backend::config::save(&c);
            }
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if ok {
                    "[ok] профиль добавлен"
                } else {
                    "[--] пустое или занятое имя"
                }));
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_rename(move |name| {
            let mut c = cfg_c.write();
            let ok = c.rename_active_profile(name.as_str());
            if ok {
                let _ = overlay_backend::config::save(&c);
            }
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if ok {
                    "[ok] переименовано"
                } else {
                    "[--] пустое или занятое имя"
                }));
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_delete(move || {
            let mut c = cfg_c.write();
            c.delete_active_profile();
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from("[ok] профиль удалён"));
            }
        });
    }
    // Phase F — Coaching + Auto-tiles toggles (were dead). Each persists; the
    // detector + session-stop logic read these from cfg at runtime, so changes
    // apply without a restart.
    {
        let cfg_c = cfg.clone();
        win.on_coaching_debrief_changed(move |on| {
            let mut c = cfg_c.write();
            c.post_meeting_debrief_enabled = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_auto_tiles_enabled_changed(move |on| {
            let mut c = cfg_c.write();
            c.auto_tiles_enabled = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_trigger_keywords_save(move |text| {
            // Clamp: these keywords prepend to EVERY STT prompt, so a huge paste
            // would balloon every transcription. Trim + cap (cf. kb::search's
            // 200-char DoS guard).
            let clamped: String = text.trim().chars().take(400).collect();
            let mut c = cfg_c.write();
            c.trigger_keywords = clamped;
            let _ = overlay_backend::config::save(&c);
        });
    }

    // Phase E6 v43 — "Structure via AI": one-shot ai::complete that turns
    // the free-form / dictated context into a clean interview profile, then
    // replaces the editor field (user reviews + Saves). Off-thread (tokio)
    // so the UI doesn't block; result posted back via invoke_from_event_loop.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_context_process_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let current = w.get_meeting_context_input().to_string();
            if current.trim().is_empty() {
                w.set_meeting_context_result(SharedString::from(
                    "[--] пусто — нечего обрабатывать",
                ));
                return;
            }
            let (base_url, bearer, model, is_local) = {
                let c = cfg_c.read();
                // Structuring uses the smarter "prep" model.
                let ep = c.ai_endpoint(true);
                (ep.base_url, ep.bearer, ep.model, ep.is_local)
            };
            if base_url.is_empty() || model.is_empty() || (!is_local && bearer.is_empty()) {
                w.set_meeting_context_result(SharedString::from(
                    "[--] AI мост не настроен (вкладка AI мост)",
                ));
                return;
            }
            w.set_context_processing(true);
            w.set_meeting_context_result(SharedString::from("обработка через AI…"));
            let weak2 = w.as_weak();
            // Off-thread with a local current-thread runtime (reqwest is
            // async-only); same pattern as the AI-bridge / STT test buttons.
            std::thread::spawn(move || {
                let messages = vec![
                    ai::ChatMessage {
                        role: "system".into(),
                        content: ai::MessageContent::Text(
                            "Преобразуй текст пользователя в чёткий профиль для интервью: \
                             роль, ключевые навыки, технологии, области фокуса. Кратко, по \
                             пунктам, на русском. Исправь ошибки распознавания речи. Без \
                             преамбулы — сразу профиль."
                                .into(),
                        ),
                    },
                    ai::ChatMessage {
                        role: "user".into(),
                        content: ai::MessageContent::Text(current),
                    },
                ];
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(ai::complete(&base_url, &bearer, &model, messages, 1024)),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak2.upgrade() else {
                        return;
                    };
                    w.set_context_processing(false);
                    match res {
                        Ok(text) if !text.trim().is_empty() => {
                            w.set_meeting_context_input(SharedString::from(
                                text.trim().to_string(),
                            ));
                            w.set_meeting_context_result(SharedString::from(
                                "[ok] обработано — проверь и нажми «Сохранить контекст»",
                            ));
                        }
                        Ok(_) => w.set_meeting_context_result(SharedString::from(
                            "[--] AI вернул пустой ответ",
                        )),
                        Err(e) => w.set_meeting_context_result(SharedString::from(format!(
                            "[--] ошибка AI: {e}"
                        ))),
                    }
                });
            });
        });
    }

    // Phase E6 v43 — voice dictation into the context field. Toggle:
    // click to start recording the mic, click again to stop. The record
    // thread (audio::record_source_until_stop) transcribes on a local
    // runtime then APPENDS the text to the editor (user reviews + Saves).
    // Reuses the PTT 30s watchdog so a forgotten "stop" can't leak a
    // thread. dictate_stop is owned by the handler closure.
    {
        let dictate_stop: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::new(RefCell::new(None));
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_context_dictate_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            // Toggle OFF: stop the in-flight recording.
            if let Some(stop) = dictate_stop.borrow_mut().take() {
                stop.store(true, Ordering::Release);
                w.set_context_dictating(false);
                w.set_meeting_context_result(SharedString::from("расшифровка…"));
                return;
            }
            // Toggle ON: start a new recording.
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
                w.set_meeting_context_result(SharedString::from(
                    "[--] ключ Groq не задан (вкладка STT)",
                ));
                return;
            }
            // M2 — single-mic guard (shared with PTT-mic + voice follow-up).
            if !try_acquire_mic() {
                w.set_meeting_context_result(SharedString::from("[--] микрофон занят"));
                return;
            }
            let stop = Arc::new(AtomicBool::new(false));
            *dictate_stop.borrow_mut() = Some(stop.clone());
            spawn_ptt_watchdog(stop.clone());
            w.set_context_dictating(true);
            w.set_meeting_context_result(SharedString::from("запись… (нажми «Остановить»)"));
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let pcm =
                    audio::record_source_until_stop(audio::AudioSource::Mic, mic_dev, None, stop)
                        .unwrap_or_else(|e| {
                            eprintln!("[overlay-host] dictation record failed: {e:#}");
                            Vec::new()
                        });
                release_mic(); // M2 — free the mic before transcription
                let text = if pcm.len() < 4800 {
                    String::new()
                } else {
                    let whisper_prompt =
                        stt::build_whisper_prompt(&trigger_keywords, &meeting_context);
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
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_res.upgrade() else {
                        return;
                    };
                    w.set_context_dictating(false);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        w.set_meeting_context_result(SharedString::from(
                            "[--] ничего не распознано",
                        ));
                        return;
                    }
                    let cur = w.get_meeting_context_input().to_string();
                    let joined = if cur.trim().is_empty() {
                        trimmed.to_string()
                    } else {
                        format!("{cur} {trimmed}")
                    };
                    w.set_meeting_context_input(SharedString::from(joined));
                    w.set_meeting_context_result(SharedString::from(
                        "[ok] добавлено — проверь и нажми «Сохранить контекст»",
                    ));
                });
            });
        });
    }

    // Phase E6 v25 — frameless Settings drag (cursor-delta, same as
    // bar + tiles). The "Settings" sidebar header is the handle.
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

    // Phase E8 — in-app auto-update (Updates tab). Network calls run on a
    // detached thread with a local current-thread tokio runtime (same
    // pattern as the AI/STT test buttons — open_settings has no rt_handle).
    {
        let weak = win.as_weak();
        win.on_check_updates_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_update_checking(true);
            w.set_update_available(false);
            w.set_update_status(SharedString::from("Проверка GitHub…"));
            diag!("update: checking GitHub for newer release");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::check_latest(env!(
                        "CARGO_PKG_VERSION"
                    ))),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak2.upgrade() else {
                        return;
                    };
                    w.set_update_checking(false);
                    match res {
                        Ok(info) if info.newer && !info.download_url.is_empty() => {
                            w.set_update_download_url(SharedString::from(info.download_url));
                            w.set_update_available(true);
                            w.set_update_status(SharedString::from(format!(
                                "Доступна версия {} — нажмите «Обновить сейчас»",
                                info.latest_version
                            )));
                        }
                        Ok(info) if info.newer => w.set_update_status(SharedString::from(format!(
                            "Есть версия {}, но в релизе нет установщика",
                            info.latest_version
                        ))),
                        Ok(info) => w.set_update_status(SharedString::from(format!(
                            "У вас последняя версия ({})",
                            info.latest_version
                        ))),
                        Err(e) => {
                            w.set_update_status(SharedString::from(format!("Ошибка проверки: {e}")))
                        }
                    }
                });
            });
        });
    }
    {
        let weak = win.as_weak();
        win.on_install_update_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let url = w.get_update_download_url().to_string();
            if url.is_empty() {
                return;
            }
            w.set_update_checking(true);
            w.set_update_status(SharedString::from("Скачивание установщика…"));
            diag!("update: downloading installer");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::download_installer(&url)),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                match res {
                    Ok(path) => match overlay_backend::update::run_installer(&path) {
                        Ok(()) => {
                            // Installer launched — quit so it can overwrite the
                            // running binary (its first page is interactive, so
                            // the app is gone before it reaches the File step).
                            diag!("update: installer launched, quitting app");
                            let _ = slint::invoke_from_event_loop(|| {
                                let _ = slint::quit_event_loop();
                            });
                        }
                        Err(e) => {
                            // P0.3: the installer failed to spawn (blocked exe /
                            // deleted file). Do NOT quit — stay open + show why.
                            diag!("update: installer spawn FAILED: {e:#}");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak2.upgrade() {
                                    w.set_update_checking(false);
                                    w.set_update_status(SharedString::from(
                                        "Не удалось запустить установщик — приложение оставлено открытым (см. лог)",
                                    ));
                                }
                            });
                        }
                    },
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak2.upgrade() {
                                w.set_update_checking(false);
                                w.set_update_status(SharedString::from(format!(
                                    "Ошибка обновления: {e}"
                                )));
                            }
                        });
                    }
                }
            });
        });
    }

    let weak_close = win.as_weak();
    let settings_close = settings_ref.clone();
    let overlay_for_close = overlay_weak.clone();
    let cfg_for_close = cfg.clone();
    win.on_close_clicked(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *settings_close.borrow_mut() = None;
        // Un-light the bar's ⚙ chip + refresh the active-stack readout (the
        // user may have switched STT/AI provider while Settings was open).
        if let Some(o) = overlay_for_close.upgrade() {
            o.set_settings_open(false);
            o.set_active_stack(SharedString::from(active_stack_label(
                &cfg_for_close.read(),
            )));
        }
    });

    // Phase E6 v26 — apply DWM per-pixel alpha so the frameless window's rounded
    // corners composite over the desktop (otherwise the corners show black).
    // make_transparent_tile = WS_EX_TOOLWINDOW + DWM blur-behind, NO click-
    // through (settings needs clicks). Review M1 — route through the stealth-
    // aware presenter so Settings, like tiles, never flashes on a screen-share
    // before WDA is applied; the DWM call is the `decorate` step (always runs).
    present_window_stealth_aware(&win, |hwnd| {
        let _ = make_transparent_tile(hwnd);
    });
    *settings_slot = Some(win);
}

/// Phase E6 v28 — after a profile import, refresh the token-status +
/// mic-opacity display so the user sees the new values, and return a
/// confirmation string for the result line.
fn msg_refresh_after_import(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) -> String {
    populate_token_status(win, cfg);
    "[ok] imported — restart binary for full effect".to_string()
}

/// P1.7 — compose the REDACTED server-import preview into the Settings props.
/// Each line is data-only (provider/url/model old->new + key PRESENCE as
/// "set"/"—"), built the same way the diagnostics `detail` strings are (shown
/// raw, not @tr'd). It NEVER carries a secret value — `preview_server_settings`
/// only ever fills booleans for keys, asserted by the redaction guard test.
fn apply_server_preview(win: &SettingsWindow, p: &overlay_backend::config::ServerSettingsPreview) {
    // "value" or "—" for an empty string; "set"/"—" for a presence bool.
    let v = |s: &str| {
        let t = s.trim();
        if t.is_empty() {
            "—".to_string()
        } else {
            t.to_string()
        }
    };
    let key = |present: bool| if present { "set" } else { "—" };
    let line = |g: &overlay_backend::config::PreviewGroup| -> String {
        // Mask the host in the URL portion (copyable text) — keeps scheme/port/
        // path, blanks the private LAN IP. Provider + model are non-secret.
        let url_old = overlay_backend::config::mask_host(&g.base_url_old);
        let url_new = overlay_backend::config::mask_host(&g.base_url_new);
        format!(
            "{}: provider {} -> {} | url {} -> {} | model {} -> {} | key {} -> {}",
            g.label,
            v(&g.provider_old),
            v(&g.provider_new),
            v(&url_old),
            v(&url_new),
            v(&g.model_old),
            v(&g.model_new),
            key(g.key_present_old),
            key(g.key_present_new),
        )
    };
    win.set_server_preview_cloud(SharedString::from(line(&p.cloud_ai)));
    win.set_server_preview_local(SharedString::from(line(&p.local_ai)));
    win.set_server_preview_vision(SharedString::from(line(&p.vision)));
    win.set_server_preview_stt(SharedString::from(line(&p.stt)));
    // GigaAM local model path: kept from THIS PC on apply. Show the incoming
    // path (masked is unnecessary — a filesystem path is not a secret, but it
    // IS machine-local) only when one side carries it, to keep the line useful.
    let gig = if p.gigaam_dir_incoming.trim().is_empty() && p.gigaam_dir_current.trim().is_empty() {
        String::new()
    } else {
        format!(
            "local GigaAM model path kept from this PC ({}); the imported file's path ({}) is NOT applied",
            v(&p.gigaam_dir_current),
            v(&p.gigaam_dir_incoming),
        )
    };
    win.set_server_preview_gigaam(SharedString::from(gig));
}

/// Push the multi-profile state into the Settings UI: the profile-name list, the
/// active index, and the active profile's context into the editor. Called on open
/// and after every add/select/rename/delete so the picker never drifts from cfg.
fn refresh_profiles(win: &SettingsWindow, c: &overlay_backend::config::Config) {
    let names: Vec<SharedString> = c
        .context_profiles
        .iter()
        .map(|p| SharedString::from(p.name.as_str()))
        .collect();
    win.set_profile_names(ModelRc::new(VecModel::from(names)));
    // Default to the first profile (0) when profiles exist but none is marked
    // active (e.g. after deleting the active one): otherwise the ComboBox bound
    // to -1 shows blank AND Rename/Delete stay disabled though selectable
    // profiles exist (audit #28). -1 only when there are no profiles at all.
    win.set_active_profile_index(match c.active_profile_index() {
        Some(i) => i as i32,
        None if !c.context_profiles.is_empty() => 0,
        None => -1,
    });
    win.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
}

/// Populate the Settings window's token-status display properties
/// from the current `cfg`. Phase E6 — gives the user a way to SEE
/// whether ai_bearer / groq_api_key are configured without leaking
/// the values themselves (shows length + first 3 chars as fingerprint).
fn populate_token_status(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) {
    // Phase E6 v18 — ASCII status prefixes ("[ok]" / "[--]") instead of
    // Unicode ✓ / ❌ which Slint+skia rendered as missing-glyph boxes
    // on the user's font fallback. Same root cause as the Close button
    // fix in settings_panel.slint and the quit chip fix in cycle 15.
    let c = cfg.read();
    let ai_status = if c.ai_bearer.is_empty() {
        "[--] not set".to_string()
    } else {
        let len = c.ai_bearer.chars().count();
        // #134: do NOT echo the key's leading chars into the UI — Settings is
        // captured on screen-share unless stealth is on. Show presence only.
        format!("[ok] set ({len} chars)")
    };
    let groq_status = if c.groq_api_key.is_empty() {
        "[--] not set".to_string()
    } else {
        let len = c.groq_api_key.chars().count();
        format!("[ok] set ({len} chars)")
    };
    win.set_ai_bearer_status(SharedString::from(ai_status));
    win.set_groq_api_key_status(SharedString::from(groq_status));
    // Phase E6 v20 — load tile opacity from config so the slider
    // reflects the saved value on Settings re-open.
    win.set_tile_body_opacity(c.tile_body_opacity);
    win.set_ai_base_url_input(SharedString::from(c.ai_base_url.clone()));
    // V4 — vision section: provider index + non-secret fields (bearers stay blank
    // on screen; saving a blank field is a no-op the user controls).
    win.set_vision_provider_index(match c.vision_provider.as_str() {
        "off" => 0,
        "same" => 1,
        "local" => 3,
        _ => 2,
    });
    win.set_vision_base_url_input(SharedString::from(c.vision_base_url.clone()));
    win.set_vision_model_input(SharedString::from(c.vision_model.clone()));
    win.set_vision_local_base_url_input(SharedString::from(c.vision_local_base_url.clone()));
    win.set_vision_local_model_input(SharedString::from(c.vision_local_model.clone()));
    win.set_vision_test_result(SharedString::from(""));
    win.set_ai_prompt_cache(c.ai_prompt_cache);
    win.set_ai_provider_index(i32::from(c.ai_provider == "local"));
    win.set_ai_local_base_url_input(SharedString::from(c.ai_local_base_url.clone()));
    // #E10.1 — seed both model dropdowns (cloud bridge + local) with the saved
    // model so each shows immediately; the full lists are fetched from
    // {base_url}/models AFTER the read guard is released (see end of fn).
    let seed_one = |saved: &str| -> ModelRc<SharedString> {
        let v: Vec<SharedString> = if saved.is_empty() {
            vec![]
        } else {
            vec![SharedString::from(saved)]
        };
        ModelRc::new(VecModel::from(v))
    };
    win.set_ai_models(seed_one(&c.ai_model));
    win.set_ai_model_index(0);
    win.set_ai_local_models(seed_one(&c.ai_local_model));
    win.set_ai_local_model_index(0);
    win.set_ai_local_vision(c.ai_local_vision);
    win.set_vision_phonetics(c.vision_phonetics);
    win.set_ai_local_thinking(c.ai_local_thinking);
    // Phase E10 — STT provider selector + local-engine fields.
    win.set_stt_provider_index(match c.stt_provider.as_str() {
        "gigaam" => 1,
        "whisper" => 2,
        _ => 0,
    });
    win.set_stt_gigaam_dir_input(SharedString::from(c.stt_gigaam_dir.clone()));
    win.set_stt_gigaam_gpu(c.stt_gigaam_gpu);
    win.set_stt_whisper_url_input(SharedString::from(c.stt_whisper_url.clone()));
    win.set_stt_whisper_bearer_input(SharedString::from(c.stt_whisper_bearer.clone()));
    win.set_stt_whisper_model_input(SharedString::from(c.stt_whisper_model.clone()));
    // Phase E6 v38 — reflect the saved interface language in the
    // Interface-tab dropdown (0=Русский, 1=English).
    win.set_ui_language_index(if c.ui_language == "en" { 1 } else { 0 });
    // Reflect the saved colour scheme in the Interface-tab dropdown, and seed
    // this Settings window's own Theme global so it opens already skinned.
    win.set_color_scheme_index(clamp_scheme(c.color_scheme));
    apply_scheme_settings(win, c.color_scheme);

    // #E10.1 — release the config read guard, THEN fetch the model lists
    // off-thread (the worker also reads cfg, so we must not hold the guard
    // across the spawn). Cloud list always (the bridge field is always
    // shown); local only when it's the active provider.
    let is_local = c.ai_provider == "local";
    drop(c);
    fetch_models(win.as_weak(), cfg.clone(), ModelTarget::Cloud);
    if is_local {
        fetch_models(win.as_weak(), cfg.clone(), ModelTarget::Local);
    }
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
