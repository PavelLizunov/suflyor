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
    ArchiveRow, ArchiveWindow, CaptureOverlay, HelpWindow, MarkdownBlock, MemoryRow,
    OverlayBarWindow, PaletteResult, PaletteWindow, RecoverOfferWindow, SettingsWindow,
    TextAskWindow, TileWindow, WizardWindow,
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

// Phase 7a of the modularization (docs/overlay-host-modularization-plan.md §5.10):
// the AI-ask / tile-streaming / conversation machinery — the `OverlayBarBridge`
// (`SlintUiBridge`/`RuntimeEvents` sink + conversation map + the SOLE
// `handle_ai_event` writer), the streaming-tile install + generation gating
// (`install_streaming_tile`/`GenGatedEvents`/`gated_events`, the wrong-tile-race
// guard), the per-PTT `PttStreamSink`, the ask/stream entrypoints
// (`fire_f3_reask`/`fire_f6_manual_spawn`/`fire_f9_ask`/`fire_ptt_ask`/
// `fire_followup_ask`/`fire_regenerate`), the route model
// (`AskRoute`/`LiveRoute`/`live_route`), the per-tile wiring + placement helpers
// (`wire_copy`/`wire_voice_followup`/`wire_escalate`/`wire_tile_drag`/
// `present_tile_window`/`apply_tile_hwnd_with_monitor`/`toggle_tile_maximize`/
// `ptt_tile_error`/`spawn_ptt_watchdog`), and the 📋-copy/conversation-format
// helpers (+ their unit tests) live in their own file alongside the binary.
// `use tile_controller::*;` re-exports them so `main`'s hotkey DISPATCH +
// bar-chip wiring + the spawn-tile / voice-follow-up drain timers resolve
// unchanged. `to_md_blocks`, the mic guard (`try_acquire_mic`/`release_mic`/
// `MIC_BUSY`), and the shared tuning constants (`AI_STREAM_MAX_TOKENS`,
// `HWND_GRAB_DELAY_MS`, `TILE_DEFAULT_W`/`TILE_DEFAULT_H`) stay here and are
// reached from tile_controller via its glob.
#[path = "overlay_host/tile_controller.rs"]
mod tile_controller;
use tile_controller::*;

// Leaf modules carved out of `tile_controller.rs` (it had grown into a mini-
// monolith — docs/overlay-host-current-review.md §"tile_controller.rs стал
// новым мини-монолитом", plan §5.10). `tile_window` holds the tile
// presentation / HiDPI placement / maximize / drag win32 glue + the per-spawn
// slot counters; `tile_copy` holds the 📋-copy / conversation-format helpers +
// the follow-up directive + their unit tests. `tile_controller` reaches the
// moved items (and they reach the `OverlayBarBridge` / `conversations_evict_
// keys` that stay in `tile_controller`) through these crate-root globs.
#[path = "overlay_host/tile_window.rs"]
mod tile_window;
use tile_window::*;
#[path = "overlay_host/tile_copy.rs"]
mod tile_copy;
use tile_copy::*;
// Wave 2 of the `tile_controller.rs` split (plan §5.10): the ASK-INITIATION
// side — the route model (`AskRoute`/`LiveRoute`/`live_route`), the ask
// entrypoints (`fire_f3_reask`/`fire_f6_manual_spawn`/`fire_f9_ask`/
// `fire_ptt_ask`), the follow-up/escalate flow (`fire_followup_ask`/
// `fire_regenerate`/`wire_escalate`/`wire_voice_followup` + its `VFU_TX` drain),
// the PTT helpers (`spawn_ptt_watchdog`/`ptt_tile_error`), and the cost/
// transcript helpers (`warn_if_over_cost_cap`/`cost_cap_reason`/
// `select_recent_labeled`). `main`'s hotkey DISPATCH + bar-chip wiring resolve
// these via `use tile_ask::*;`. The STREAM-WRITE side (`OverlayBarBridge` +
// `handle_ai_event`, `install_streaming_tile`/`gated_events`, `PttStreamSink`)
// stays in `tile_controller.rs`; the moved code reaches it through the glob.
#[path = "overlay_host/tile_ask.rs"]
mod tile_ask;
use tile_ask::*;

// `tile_cost.rs` — pure cost-cap + transcript-selection helpers split out of
// `tile_ask.rs` (P1 `tile_ask` split, docs/overlay-host-modular-structure-current.md).
// `use tile_cost::*;` re-exports them so the ask entrypoints still in `tile_ask.rs`
// reach `cost_cap_reason` / `warn_if_over_cost_cap` / `select_recent_labeled`
// through the crate root.
#[path = "overlay_host/tile_cost.rs"]
mod tile_cost;
use tile_cost::*;

// `tile_routes.rs` — the AskRoute (Text/Vision/Cloud) model + LiveRoute split out
// of `tile_ask.rs` (P1 split). `use tile_routes::*;` re-exports it so the ask
// entrypoints + the other tile modules (vision_capture, tile_controller) reach
// `AskRoute` / `LiveRoute` / `live_route` through the crate root.
#[path = "overlay_host/tile_routes.rs"]
mod tile_routes;
use tile_routes::*;

// `tile_ptt.rs` — push-to-talk ask flow (the 30s watchdog + the PTT tile-error
// helper + `fire_ptt_ask`) split out of `tile_ask.rs` (P1 split). `use
// tile_ptt::*;` re-exports it so `main`'s PTT hotkey dispatch reaches
// `fire_ptt_ask` through the crate root.
#[path = "overlay_host/tile_ptt.rs"]
mod tile_ptt;
use tile_ptt::*;

// `tile_followup.rs` — the tile continuation surfaces (follow-up reframe + the
// `6ffbc40` fix, `fire_followup_ask` / `fire_regenerate`, `wire_escalate` /
// `wire_voice_followup` + `VFU_TX`) split out of `tile_ask.rs` (P1 split). `use
// tile_followup::*;` re-exports them so the F9 / PTT tiles + `main`'s drains
// reach them through the crate root.
#[path = "overlay_host/tile_followup.rs"]
mod tile_followup;
use tile_followup::*;

// Phase 7b of the modularization (docs/overlay-host-modularization-plan.md
// §5.8/§5.9) — the FINAL phase: the Settings window controller. `open_settings`
// (the Settings fn with its remaining inline handlers + the stealth/scheme/
// tile-opacity closures + the full-PROFILE import/export + the "Run setup
// wizard" button) lives here; the per-domain tab clusters are now sibling
// modules it only CALLs — AI/STT/Vision (Waves 1-3) plus the Wave-4 split of the
// server import/export (`wire_import_export`), the updater (`wire_updates`), and
// the local-AI installer (`wire_local_ai`). `ModelTarget` + `fetch_models` and
// the Settings helpers (`msg_refresh_after_import`/`refresh_profiles`/
// `populate_token_status`) live in their own files alongside the binary
// (`apply_server_preview` moved into settings_import_export.rs).
// `use settings_controller::*;` re-exports them so `main`'s ⚙ gear-chip handler
// resolves unchanged. The window openers `open_text_ask`/`open_help`/
// `open_palette` (+ palette helpers) moved to `aux_windows.rs` (P2); only
// `short_model_name`/`active_stack_label` stay in `main`. The moved Settings code
// reaches the diagnostics REDACTED `build_diag_report`, `open_wizard`, the
// `WindowRegistry`, and the mic guard through the existing crate-root globs.
#[path = "overlay_host/settings_controller.rs"]
mod settings_controller;
use settings_controller::*;

// The Vision (screenshot) Settings-tab callbacks (provider switch + cloud/local
// field saves + the live connection test) live in their own file alongside the
// binary (P1 domain split of `settings_controller.rs`). `use settings_vision::*;`
// re-exports `wire_vision_settings`, which `open_settings` calls in place of the
// old inline V4 block; the moved code reaches `SettingsWindow` / the `diag!`
// macro / the `overlay_backend` helpers through the crate-root globs.
#[path = "overlay_host/settings_vision.rs"]
mod settings_vision;
use settings_vision::*;

// The STT (speech-to-text) Settings-tab callbacks (provider switch + GigaAM GPU
// toggle + GigaAM/Whisper field saves + the live connection test) live in their
// own file alongside the binary (P1 domain split of `settings_controller.rs`).
// `use settings_stt::*;` re-exports `wire_stt_settings`, which `open_settings`
// calls in place of the old inline STT blocks; the moved code reaches
// `SettingsWindow` / the `diag!` macro / the `overlay_backend` helpers through
// the crate-root globs.
#[path = "overlay_host/settings_stt.rs"]
mod settings_stt;
use settings_stt::*;

// Phase 3b.3 — the 💭 Memory Settings tab (curated-memory review). `use
// settings_memory::*;` re-exports `wire_memory`, which `open_settings` calls to
// bind the candidate/item lists + approve/reject/delete/extract over the SQLite
// memory tables (3b.1) + the heuristic extractor (3b.2a).
#[path = "overlay_host/settings_memory.rs"]
mod settings_memory;
use settings_memory::*;

// The AI (cloud bridge + local server) Settings-tab callbacks (provider switch,
// token / base-url / model saves, `{base_url}/models` dropdown refresh, the
// prompt-cache toggle, and the bridge + local connection tests) live in their
// own file alongside the binary (P1 domain split of `settings_controller.rs`).
// `use settings_ai::*;` re-exports `wire_ai_settings` (which `open_settings`
// calls in place of the old inline AI blocks) plus `ModelTarget` + `fetch_models`
// (moved with them — `populate_token_status`, which STAYS in
// `settings_controller.rs`, reaches them back through these crate-root globs).
#[path = "overlay_host/settings_ai.rs"]
mod settings_ai;
use settings_ai::*;

// The server-settings import/export Settings-tab callbacks (server-only export,
// the two-step import preview, Apply, Cancel) live in their own file alongside
// the binary (P1 domain split of `settings_controller.rs`). `use
// settings_import_export::*;` re-exports `wire_import_export` (which
// `open_settings` calls in place of the old inline server import/export blocks)
// plus `apply_server_preview` (moved with it — its only caller is the moved
// import closure). The PROFILE export/import + `refresh_profiles` +
// `msg_refresh_after_import` STAY in `settings_controller.rs`; the moved code
// reaches `msg_refresh_after_import` back through these crate-root globs.
#[path = "overlay_host/settings_import_export.rs"]
mod settings_import_export;
use settings_import_export::*;

// The Updates Settings-tab callbacks (GitHub release check + the
// download-then-run installer action) live in their own file alongside the
// binary (P1 domain split of `settings_controller.rs`). `use
// settings_updates::*;` re-exports `wire_updates`, which `open_settings` calls
// in place of the old inline Updates blocks; the moved code reaches the `diag!`
// macro / `overlay_backend::update` through the crate-root scope. SECURITY: the
// download -> verify -> spawn sequence is unchanged (verification lives in
// `overlay_backend::update`).
#[path = "overlay_host/settings_updates.rs"]
mod settings_updates;
use settings_updates::*;

// The one-click local-AI installer Settings-tab callbacks (install pipeline +
// Cancel) live in their own file alongside the binary (P1 domain split of
// `settings_controller.rs`). `use settings_local_ai::*;` re-exports
// `wire_local_ai`, which `open_settings` calls in place of the old inline
// install/cancel blocks; the moved code reaches `active_stack_label` + the
// `diag!` macro / `overlay_backend::local_ai` through the crate-root scope.
// SECURITY: the download -> verify -> spawn sequence is unchanged (verification
// lives in `overlay_backend::local_ai`).
#[path = "overlay_host/settings_local_ai.rs"]
mod settings_local_ai;
use settings_local_ai::*;

// The auxiliary on-demand windows — the "✏ Написать" text-ask, the 🆘 Help
// window, and the F4 KB palette (+ its pure result helpers) — live in their own
// file alongside the binary (P2 split of the `overlay_host.rs` root). `use
// aux_windows::*;` re-exports `open_text_ask` / `open_help` / `open_palette` so
// the F1 / F4 / ✏ dispatch + the 🆘 chip in `main` resolve unchanged; the moved
// code reaches the tile / stealth / scheme helpers + `fire_f9_ask` through the
// crate-root scope.
#[path = "overlay_host/aux_windows.rs"]
mod aux_windows;
use aux_windows::*;

pub(crate) type TileWindows = Rc<RefCell<Vec<TileWindow>>>;

/// Parse markdown source into the Slint `MarkdownBlock` rows a tile body
/// renders. Shared by the streaming Delta/Error paths + follow-ups.
pub(crate) fn to_md_blocks(md: &str) -> Vec<MarkdownBlock> {
    markdown::parse(md)
        .into_iter()
        .map(|b| MarkdownBlock {
            kind: b.kind,
            text: SharedString::from(b.text),
            lang: SharedString::from(b.lang),
        })
        .collect()
}

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

/// RAII release for the single-mic lock. Dropping this — on ANY exit path,
/// including a panic unwinding the record thread (WASAPI/COM fault, USB mic
/// yanked mid-capture) — frees the mic. Replaces the old bare `release_mic()`
/// statement, which a panic would skip, leaving every mic consumer permanently
/// reporting "занят" until an app restart (audit Q1). The `()` field is private,
/// so only `try_acquire_mic` can mint one.
pub(crate) struct MicGuard(());

impl Drop for MicGuard {
    fn drop(&mut self) {
        MIC_BUSY.store(false, Ordering::Release);
    }
}

/// Take the single-mic lock. `Some(guard)` = acquired (the mic is ours until the
/// guard drops); `None` = another recorder holds it. Acquire on the UI thread,
/// then MOVE the guard into the record worker so it releases on every exit incl.
/// unwind; `drop(guard)` right after recording to free the mic before
/// transcription (which never touches the device).
pub(crate) fn try_acquire_mic() -> Option<MicGuard> {
    MIC_BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        .then(|| MicGuard(()))
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
pub(crate) const HWND_GRAB_DELAY_MS: u64 = 200;
/// V0.8.4 — fast first reveal attempt (~2 frames). winit usually realizes the
/// HWND within 1-2 frames, so grabbing at ~33ms lets on-demand windows pop
/// nearly instantly instead of waiting the full 200ms; if the HWND isn't ready
/// yet, present_window_stealth_aware falls back to HWND_GRAB_DELAY_MS.
const HWND_REVEAL_FAST_MS: u64 = 33;
/// SLINT_OVERLAY_AUTO_TILE auto-spawn delay (smoke-test convenience).
const AUTO_TILE_DELAY_MS: u64 = 500;
/// Periodic session-timer chip update interval.
const TIMER_TICK_SECS: u64 = 1;
/// Local-AI watchdog: how often to confirm llama-server is still answering on
/// :8080. llama binds the HTTP port early (returns 503 while the model loads),
/// so a still-unreachable port at the next tick means the process is genuinely
/// dead, not slow-loading.
const WATCHDOG_SECS: u64 = 15;
/// Minimum gap between two auto-(re)start attempts, so a server that can't
/// start (missing model/binary) isn't hammered. A healthy server resets this.
const WATCHDOG_COOLDOWN_SECS: u64 = 30;
/// Stop auto-retrying after this many consecutive failed (re)starts so a
/// genuinely broken install doesn't spawn forever; a reachable server (e.g.
/// after a manual Install) re-arms the counter to 0.
const WATCHDOG_MAX_FAILS: u32 = 6;

/// The local-AI watchdog's pure decision state (cooldown timestamp + the
/// consecutive-failure cap), extracted from the live loop so the safety-critical
/// retry policy is unit-testable — the loop itself interleaves this with network
/// probes, the lifecycle lock, and process spawns, none of which a test can
/// reach (audit B1). Time is passed in as an `Instant` so tests can offset it.
#[derive(Default)]
struct WatchdogState {
    last_attempt: Option<std::time::Instant>,
    consecutive_fails: u32,
}

impl WatchdogState {
    /// Whether to attempt a (re)start NOW. The caller has already confirmed the
    /// server is unreachable and that local AI is wanted. True iff the cooldown
    /// has elapsed since the last attempt (or there was none) AND we're still
    /// under the consecutive-failure cap.
    fn should_restart(
        &self,
        now: std::time::Instant,
        cooldown: std::time::Duration,
        max_fails: u32,
    ) -> bool {
        let cooled = self
            .last_attempt
            .is_none_or(|t| now.duration_since(t) >= cooldown);
        cooled && self.consecutive_fails < max_fails
    }

    /// The server answered — re-arm the cap so any future crash retries fresh.
    fn note_reachable(&mut self) {
        self.consecutive_fails = 0;
    }

    /// Record a (re)start attempt at `now`: a confirmed `Switched` resets the
    /// fail count, anything else (PortBusy / FailedToStart) increments it.
    fn note_attempt(&mut self, now: std::time::Instant, switched: bool) {
        self.last_attempt = Some(now);
        if switched {
            self.consecutive_fails = 0;
        } else {
            self.consecutive_fails += 1;
        }
    }
}
/// Default tile window dimensions (match ui/tile.slint preferred-*
/// values so the spawned window isn't forcibly shrunk on first paint).
pub(crate) const TILE_DEFAULT_W: i32 = 460;
pub(crate) const TILE_DEFAULT_H: i32 = 360;
/// AI ask cap for the non-streaming auto-tile/reask `complete` path.
/// Sized to fit typical session-question answers without runaway cost.
const AI_MAX_TOKENS: u32 = 600;
/// Upper bound for the STREAMING F9/PTT/follow-up asks. Higher than
/// `AI_MAX_TOKENS` because these are interactive and may want a longer
/// answer; in streaming mode the cap does NOT affect time-to-first-token
/// (it only bounds the worst-case length). One source of truth for the
/// three `stream_chat` sites (was a bare `4096` literal repeated 3×).
pub(crate) const AI_STREAM_MAX_TOKENS: u32 = 4096;

fn main() -> Result<(), slint::PlatformError> {
    // fs-audit — one-time, fail-safe rename of the data dir from the legacy
    // Tauri name `overlay-mvp` to the brand `suflyor`. MUST run BEFORE logging /
    // config touch the data dir (they resolve through `paths::data_root`). Atomic
    // rename; on any failure the legacy dir is kept and still used, so no data is
    // lost. Outcome is logged once the log is open (just below).
    let data_migration = overlay_backend::paths::migrate_data_root();

    // Open the diagnostics log + install the panic hook FIRST so any
    // early failure (config, tokio, window create) is captured even in a
    // release build that has no console.
    slint_replay::logging::init();
    match &data_migration {
        overlay_backend::paths::DataMigration::Migrated => {
            eprintln!("[overlay-host] data dir migrated: overlay-mvp -> suflyor");
        }
        overlay_backend::paths::DataMigration::Failed(e) => {
            eprintln!("[overlay-host] data dir migration failed (staying on overlay-mvp): {e}");
        }
        _ => {}
    }

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
    // P2 — build the local session archive (SQLite catalog) OFF the hot path:
    // idempotently index the finished JSONL journals so the interview history is
    // searchable. On by default; the JSONL journals stay the source of truth, and
    // the catalog can be deleted + rebuilt. Best-effort — never blocks startup.
    // `None` = skip no live session (a session starts later; it indexes next launch).
    if cfg.read().session_archive_enabled {
        std::thread::spawn(
            || match overlay_backend::persistence::reindex_default(None) {
                Ok(stats) => diag!(
                    "[overlay-host] catalog: indexed {} sessions ({} skipped, {} failed)",
                    stats.indexed,
                    stats.skipped,
                    stats.failed
                ),
                Err(e) => diag!("[overlay-host] catalog reindex failed: {e:#}"),
            },
        );
    }
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
                // UI-audit 2026-06-13 (user: "Gemma works only after I press
                // Install local AI"): the old code did ONE test_connection and
                // gave up on the first HTTP 503 "Loading model" — but a freshly
                // launched llama-server returns 503 for the whole time the model
                // is loading into VRAM (seconds for 4B, much longer for 12B). So
                // the warm-up always "skipped", and the FIRST real ask hit a
                // still-loading server → felt broken until the user clicked
                // Install (which polls readiness). POLL here too: retry past 503
                // until the model answers or a generous deadline (12B cold-load).
                let t = std::time::Instant::now();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
                loop {
                    match overlay_backend::ai::test_connection(
                        ai_ep.base_url.clone(),
                        ai_ep.bearer.clone(),
                        ai_ep.model.clone(),
                    )
                    .await
                    {
                        Ok(_) => {
                            diag!("local AI warmed in {:?}", t.elapsed());
                            break;
                        }
                        Err(_) if std::time::Instant::now() < deadline => {
                            // still loading (503) or briefly unreachable — wait.
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        }
                        Err(e) => {
                            diag!(
                                "local AI not ready after {:?}: {e} — \
                                 open Settings → AI → Install local AI",
                                t.elapsed()
                            );
                            break;
                        }
                    }
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

    // E10.5 — auto-start the local AI servers the config points at. MOVED below
    // overlay creation (see the "Local-AI boot + watchdog" thread) so a recovery
    // can refresh the bar's active-stack readout. It now confirms readiness and
    // self-heals a dead :8080 mid-session instead of fire-and-forget launching.
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

    // ===== Local-AI boot + watchdog (E10.5, hardened 2026-06-13) =====
    // The local servers ARE the user's AI/STT brain. Previously boot did a
    // fire-and-forget `ensure_servers` launch: if llama-server crashed on spawn
    // or died mid-session, nothing noticed and nothing retried — the bar sat on
    // "AI недоступен" until the user manually hit Settings → "Install local AI"
    // (the ONLY path that frees :8080 + waits for readiness). Reported by the
    // user ("сломалась в какой-то момент… пофиксилось после Установить").
    //
    // This single off-UI thread makes boot AND mid-session self-healing:
    //   • once: best-effort launch whisper (:8081) if STT is local;
    //   • every WATCHDOG_SECS: if AI is local and :8080 is TRULY dead (a real
    //     connection-refused — NOT a server that merely returned an error, which
    //     `llama_reachable()` still counts as alive so we never kill a working
    //     server), free + relaunch + confirm-ready via `ensure_llama_serving`.
    // A manual install/switch holds `local_ai_lock` so we never race it; a 30 s
    // cooldown + a fail cap keep a broken install from spawning forever; a
    // reachable server re-arms the cap. On a confirmed (re)start we sync the
    // persisted model name and refresh the bar so the user always sees the
    // model that is actually serving (Gemma 4B / 12B).
    {
        let cfg_w = cfg.clone();
        let state_w = state.clone();
        let overlay_w = overlay.as_weak();
        std::thread::spawn(move || {
            let root = overlay_backend::local_ai::default_root();
            // fs-audit #1 — GC orphaned engine-update leftovers (a crashed
            // mid-update's `.llama-staging-update` + stale rollback backups) so
            // they don't accumulate when the user stops updating. Held UNDER the
            // lifecycle lock so it can't race a manual "Update engine" worker
            // (which also takes the lock + owns `.llama-staging-update`); any
            // staging dir present while we hold the lock is by definition orphaned.
            {
                let lifecycle_lock = {
                    let s = state_w.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let _g = lifecycle_lock.lock().unwrap_or_else(|p| p.into_inner());
                overlay_backend::local_ai::sweep_orphaned_engine_artifacts(&root);
            }
            // Bring the local servers UP FIRST (on the CURRENT engine) so AI + STT
            // are available immediately; the throttled engine refresh further down
            // then runs BEHIND the running servers, so a slow ~160 MB download can
            // never delay first AI/STT availability (review I1).
            //
            // One-time best-effort STT launch. Whisper has not shown llama's
            // dies-on-boot fragility; ensure_servers skips it if it already answers.
            let want_whisper = {
                let c = cfg_w.read();
                c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081")
            };
            if want_whisper {
                let started = overlay_backend::local_ai::ensure_servers(&root, false, true, false);
                if !started.is_empty() {
                    state_w
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .local_ai_servers
                        .extend(started);
                }
            }
            // Best-effort llama launch on the current engine. The watchdog loop
            // below also (re)starts it, but doing it here means local AI is up
            // BEFORE the engine-update block claims the lifecycle lock for a
            // potentially long download.
            {
                let (want_llama, prefer_quality) = {
                    let c = cfg_w.read();
                    (
                        c.ai_provider == "local" && c.ai_local_base_url.contains(":8080"),
                        c.ai_local_quality,
                    )
                };
                if want_llama {
                    let started = overlay_backend::local_ai::ensure_servers(
                        &root,
                        true,
                        false,
                        prefer_quality,
                    );
                    if !started.is_empty() {
                        state_w
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .local_ai_servers
                            .extend(started);
                    }
                }
            }
            // v0.18.2 — keep llama.cpp fresh: a throttled (weekly) auto-update of
            // the engine so the local model stays fast and 12B vision (gemma4uv)
            // works without the user ever touching it. Runs ONCE at boot, BEHIND
            // the servers started above, under the lifecycle lock so it can't race
            // the watchdog/install. Verify-before-swap inside means a bad build can
            // never brick local AI — on any failure the engine is left as-is. On a
            // real swap :8080 is stopped; the watchdog loop below brings the new
            // binary up within one tick.
            if overlay_backend::local_ai::should_check_engine_update(&root) {
                let lifecycle_lock = {
                    let s = state_w.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let guard = lifecycle_lock.lock().unwrap_or_else(|p| p.into_inner());
                let no_cancel = std::sync::atomic::AtomicBool::new(false);
                match overlay_backend::local_ai::update_llama_engine(&root, &no_cancel, &|p| {
                    if let overlay_backend::local_ai::Progress::Step(s) = p {
                        diag!("engine auto-update: {s}");
                    }
                }) {
                    Ok(overlay_backend::local_ai::EngineUpdate::Updated { from, to }) => {
                        diag!("llama.cpp auto-updated {from:?} -> b{to}");
                    }
                    Ok(overlay_backend::local_ai::EngineUpdate::UpToDate { build }) => {
                        diag!("llama.cpp already current (b{build})");
                    }
                    Ok(overlay_backend::local_ai::EngineUpdate::Skipped { reason }) => {
                        diag!("llama.cpp engine update skipped: {reason}");
                    }
                    Err(e) => diag!("llama.cpp engine update check failed: {e:#}"),
                }
                drop(guard);
                overlay_backend::local_ai::mark_engine_update_checked(&root);
            }
            let mut watchdog = WatchdogState::default();
            loop {
                let (want_llama, prefer_quality) = {
                    let c = cfg_w.read();
                    (
                        c.ai_provider == "local" && c.ai_local_base_url.contains(":8080"),
                        c.ai_local_quality,
                    )
                };
                if want_llama {
                    if overlay_backend::local_ai::llama_reachable() {
                        // Alive (serving or cold-loading) — re-arm the fail cap
                        // so any future crash is retried from scratch.
                        watchdog.note_reachable();
                    } else {
                        let now = std::time::Instant::now();
                        if watchdog.should_restart(
                            now,
                            std::time::Duration::from_secs(WATCHDOG_COOLDOWN_SECS),
                            WATCHDOG_MAX_FAILS,
                        ) {
                            // Claim the local-AI lifecycle lock so we never race a
                            // manual install/switch on :8080. try_lock → skip this
                            // tick if a manual op holds it (it will bring :8080 up).
                            // RAII: the guard releases on every path incl. panic, so
                            // a crash here can't wedge the lock.
                            let lifecycle_lock = {
                                let s = state_w.lock().unwrap_or_else(|p| p.into_inner());
                                s.local_ai_lock.clone()
                            };
                            let guard = match lifecycle_lock.try_lock() {
                                Ok(g) => Some(g),
                                Err(std::sync::TryLockError::Poisoned(p)) => Some(p.into_inner()),
                                // A manual install/switch owns the lifecycle — let
                                // it bring :8080 up; we re-check next tick.
                                Err(std::sync::TryLockError::WouldBlock) => None,
                            };
                            if let Some(_ai_guard) = guard {
                                // Re-check UNDER the lock — a manual op may have
                                // brought :8080 up between our probe and the lock.
                                if !overlay_backend::local_ai::llama_reachable() {
                                    diag!(
                                        "local AI :8080 not answering — auto-(re)starting llama-server"
                                    );
                                    let (outcome, started) =
                                        overlay_backend::local_ai::ensure_llama_serving(
                                            &root,
                                            prefer_quality,
                                        );
                                    let attempt_now = std::time::Instant::now();
                                    match outcome {
                                        overlay_backend::local_ai::ModelSwitch::Switched => {
                                            watchdog.note_attempt(attempt_now, true);
                                            {
                                                let mut s = state_w
                                                    .lock()
                                                    .unwrap_or_else(|p| p.into_inner());
                                                // Reap only definitively-exited
                                                // handles (Ok(Some)); keep running
                                                // (Ok(None)) AND unknown (Err) so a
                                                // live child is never lost from
                                                // kill-on-quit tracking.
                                                s.local_ai_servers.retain_mut(|c| {
                                                    !matches!(c.try_wait(), Ok(Some(_)))
                                                });
                                                s.local_ai_servers.extend(started);
                                            }
                                            // Sync the persisted model name to what
                                            // is actually serving + refresh the bar
                                            // so the readout shows the real model.
                                            let label = {
                                                let mut c = cfg_w.write();
                                                c.ai_local_model =
                                                    overlay_backend::local_ai::active_local_model_name(
                                                        &root,
                                                        prefer_quality,
                                                    );
                                                active_stack_label(&c)
                                            };
                                            diag!("local AI server ready ({label})");
                                            let ow = overlay_w.clone();
                                            let _ = slint::invoke_from_event_loop(move || {
                                                if let Some(o) = ow.upgrade() {
                                                    o.set_active_stack(SharedString::from(label));
                                                }
                                            });
                                        }
                                        overlay_backend::local_ai::ModelSwitch::PortBusy => {
                                            watchdog.note_attempt(attempt_now, false);
                                            // started is empty on PortBusy; harmless.
                                            overlay_backend::local_ai::terminate_servers(started);
                                            diag!(
                                                "local AI :8080 held by a foreign process — not restarting"
                                            );
                                        }
                                        overlay_backend::local_ai::ModelSwitch::FailedToStart => {
                                            watchdog.note_attempt(attempt_now, false);
                                            let fails = watchdog.consecutive_fails;
                                            // Kill the wedged/dead child instead of
                                            // leaking it until quit. No port sweep →
                                            // whisper (:8081) is left alone.
                                            overlay_backend::local_ai::terminate_servers(started);
                                            diag!(
                                                "local AI failed to (re)start (attempt {fails}/{WATCHDOG_MAX_FAILS}) \
                                                 — check the local model/binary in Settings → AI"
                                            );
                                        }
                                    }
                                }
                                // _ai_guard drops here → lifecycle lock released.
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(WATCHDOG_SECS));
            }
        });
    }
    overlay.set_stealth_active(cfg.read().stealth_enabled);
    overlay.set_cost_label(SharedString::from("$0.000"));
    overlay.set_timer_label(SharedString::from("00:00"));
    // v0.13.1 — the mic is LIVE (not muted) by default; the chip shows it lit.
    overlay.set_mic_active(true);
    overlay.set_mic_muted(false);
    {
        let mut st = match state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        st.mic_active = true;
    }

    apply_overlay_hwnd(&overlay);

    // ===== Mic chip = MUTE toggle (v0.13.1) =====
    //
    // Click mutes / un-mutes the microphone. When muted: mic-source transcript
    // lines are dropped (the transcript forwarder already honours rt.mic_muted)
    // AND mic audio is NOT written to the session recording (the recorder tee
    // honours it too) — one control, both effects. System audio is unaffected.
    // The "test mic level" probe now lives only in Settings → Audio.
    //
    // `mic_active` here means "mic is LIVE" (= NOT muted): it starts true, and
    // the bar chip shows a slashed-mic icon + dims when muted.
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        let slint_rt_mic = slint_rt.clone();
        overlay.on_mic_toggle_clicked(move || {
            let muted = {
                let mut rt = slint_replay::runtime_state::lock(&slint_rt_mic);
                rt.mic_muted = !rt.mic_muted;
                rt.mic_muted
            };
            {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.mic_active = !muted; // "live" when not muted
            }
            if let Some(o) = weak.upgrade() {
                o.set_mic_active(!muted);
                o.set_mic_muted(muted);
                if muted {
                    o.set_status_text(SharedString::from("mic muted"));
                    o.set_status_color(slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24));
                } else {
                    refresh_status(&o, true, get_sys_active(&s));
                }
            }
            eprintln!(
                "[overlay-host] mic {}",
                if muted {
                    "MUTED (transcript + recording off)"
                } else {
                    "live"
                }
            );
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
                    let snapshot = slint_session::stop_session(rt_c, &cfg_c);
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
            tile.set_tile_title(SharedString::from(
                slint_replay::app_state::tile_title_line(&req.spec.question),
            ));
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
            // formats it as "keyword ..." or "question ...".
            // Color: orange for keyword/aggressive, blue for question.
            if let Some(first) = req.spec.highlights.first() {
                tile.set_trigger_label(SharedString::from(first.clone()));
                let is_keyword = first.starts_with("keyword");
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
                    | TileKind::Summary
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
                // Audit (prompt-context): seed the completed-tile history with the
                // SAME effective context (profile + approved memory) the live answer
                // paths use, so a follow-up/regenerate matches the original request.
                let meeting_context =
                    overlay_backend::memory::context_for_meeting(&meeting_context);
                let question = req.spec.question.clone();
                // v0.12.2 — Summary tiles seed their dialog with the REAL recap
                // payload ([summary-prompt, transcript]) instead of the bare title,
                // so the generic regenerate/escalate (which re-ask the stored
                // [system, user] of a 1-user-turn history WITHOUT reframing) rebuild
                // the summary correctly. The transcript is read live from the
                // full-session accumulator at seed time (≈ completion time); for an
                // ongoing session this may include a few seconds more speech than the
                // displayed summary, which is the right input for a "rebuild".
                // Stays true for every non-Summary kind; for Summary it records
                // whether the seed actually captured a transcript. A session restart
                // between the Summary request and this tile painting clears
                // full_transcript (slint_session.rs) → empty seed → 🔄/🧠 would
                // rebuild from nothing, so we leave them OFF for that (rare) tile
                // below. The displayed answer is unaffected (already computed).
                let mut summary_seed_has_transcript = true;
                let mut messages = if matches!(req.kind, TileKind::Summary) {
                    let (is_local, transcript) = {
                        let is_local = cfg_for_poll.read().ai_endpoint(true).is_local;
                        let tx = slint_replay::runtime_state::lock(&slint_rt_for_poll)
                            .full_transcript
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>();
                        (is_local, tx)
                    };
                    summary_seed_has_transcript = !transcript.is_empty();
                    // v0.16.0 — same keyword-gated memory LOGIC as the bar
                    // build. Computed at seed time, so a 🔄/🧠 rebuild uses the
                    // CURRENT transcript + memory — the v0.12.2 "rebuild" sema:
                    // the transcript here is also read live, and a fact added
                    // after the bar press SHOULD shape the rebuild. Small
                    // read-only catalog query on a user-initiated path (same
                    // budget class as context_for_meeting, v0.11.2).
                    // v0.17.0 — for an OVER-BUDGET transcript the bar runs
                    // map-reduce (runtime.rs); a seeded pair can't replay N map
                    // calls, so this 🔄 seed falls back to the middle-truncated
                    // single pass — a documented degraded rebuild. The bar /
                    // archive button remains the quality path.
                    let is_ru = response_language == "ru";
                    // v0.17.1 (мега-аудит) — format ONCE and reuse for both the
                    // memory_ref gating and the seed (was two full passes over
                    // a potentially 20k-line transcript on the UI thread).
                    let formatted =
                        overlay_backend::runtime::format_transcript_for_summary(&transcript, is_ru);
                    let memory_ref =
                        overlay_backend::memory::summary_reference_for_transcript(&formatted);
                    overlay_backend::runtime::build_summary_seed_from_formatted(
                        &formatted,
                        is_ru,
                        is_local,
                        memory_ref.as_deref(),
                    )
                } else {
                    ai::build_request(
                        &meeting_context,
                        &response_language,
                        &[],
                        None,
                        Some(&question),
                    )
                };
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
                // v0.12.2 — Summary now seeds a REAL [summary-prompt, transcript]
                // dialog (above), so its 🔄/🧠 rebuild the summary correctly — they
                // get re-enabled. Debrief stays gated: its mic-only snapshot is not
                // retained after the coaching call, so there's nothing to rebuild
                // from, and a bare-title re-ask would still fail. (For Summary the
                // bar button rebuilds over the LIVE transcript; the tile 🔄 rebuilds
                // over the transcript frozen into this tile's dialog — distinct.)
                // `summary_seed_has_transcript` is always true off the Summary path;
                // it only suppresses reask for a Summary tile whose seed came back
                // empty (rare session-restart race — review v0.12.2 minor).
                let allow_reask =
                    !matches!(req.kind, TileKind::Debrief) && summary_seed_has_transcript;
                if allow_reask {
                    tile.set_can_regenerate(true);
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
                if allow_reask {
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
    // 🗄 Session-archive browser (F7 + 🗄 bar chip), created on demand like the
    // palette/help. Phase 3a — browse + FTS-search the SQLite session catalog.
    let archive: Rc<RefCell<Option<ArchiveWindow>>> = Rc::new(RefCell::new(None));
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
        f7_id,
        f8_id,
        sf8_id,
        f9_id,
        sf9_id,
    } = register_hotkeys();

    let hotkey_poll = Timer::default();
    let hp_palette = palette.clone();
    let hp_help = help.clone();
    let hp_archive = archive.clone();
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
                } else if event.id == f7_id {
                    // Phase 3a — F7 toggles the 🗄 session archive (focus-
                    // independent, like F4/F1; a hotkey-spawned always-on-top
                    // window starts unfocused, so a toggle is the reliable
                    // closer rather than relying on Esc landing).
                    let archive_open = hp_archive.borrow().is_some();
                    if archive_open {
                        eprintln!("[overlay-host] F7 pressed — closing archive (toggle)");
                        if let Some(a) = hp_archive.borrow_mut().take() {
                            let _ = a.hide();
                        }
                        if let Some(o) = hp_weak_overlay.upgrade() {
                            o.set_archive_open(false);
                        }
                    } else {
                        eprintln!("[overlay-host] F7 pressed — opening archive");
                        open_archive(
                            &hp_archive,
                            &hp_tiles,
                            &hp_state,
                            &hp_weak_overlay,
                            &hp_cfg,
                            &hp_events,
                            &hp_rt_handle,
                            &hp_rt,
                        );
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
                        // Plain F8: describe, OR test-practice if the Settings
                        // toggle is on (Shift+F8 below always = translate).
                        if hp_cfg.read().vision_test_practice {
                            overlay_backend::vision::VisionMode::TestPractice
                        } else {
                            overlay_backend::vision::VisionMode::Describe
                        },
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
                        overlay_backend::vision::VisionMode::Translate,
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
            let Some(mic_guard) = try_acquire_mic() else {
                return; // mic held by a tile voice follow-up / dictation
            };
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
                drop(mic_guard); // M2 — free the mic before transcription (RAII: also released on a record-thread panic)
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
                // Security-relevant: a silent save-fail means next launch starts
                // with stealth OFF → the bar is visible to screen capture with no
                // log line to explain it. Log so the failure is diagnosable (Q4).
                if let Err(e) = config::save(&c) {
                    eprintln!("[overlay-host] stealth save failed (will not persist across restart): {e:#}");
                }
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
                // 📷 chip mirrors plain F8 (describe / test-practice per Settings).
                if cfg_c.read().vision_test_practice {
                    overlay_backend::vision::VisionMode::TestPractice
                } else {
                    overlay_backend::vision::VisionMode::Describe
                },
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
                format!("Вопрос по встрече #{seq}")
            } else {
                format!("Тайл #{seq}")
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

    // ===== 🗄 Session archive (F7 / 🗄 chip) =====
    {
        let archive_ref = archive.clone();
        let tiles_ref = tiles.clone();
        let state_ref = state.clone();
        let ow = overlay.as_weak();
        let cfg_ar = cfg.clone();
        let events_ar = events.clone();
        let rt_ar = rt_handle.clone();
        let slint_rt_ar = slint_rt.clone();
        overlay.on_archive_clicked(move || {
            open_archive(
                &archive_ref,
                &tiles_ref,
                &state_ref,
                &ow,
                &cfg_ar,
                &events_ar,
                &rt_ar,
                &slint_rt_ar,
            );
        });
    }

    // ===== 📝 Meeting summary (v0.12.0 — Summary chip) =====
    // Snapshot the FULL session transcript (runtime_state accumulator, not
    // the 80-line rolling window) and run it through run_meeting_summary.
    // Works mid-session AND between Стоп and the next Старт (the
    // accumulator survives stop_session). The summary-busy bar property
    // doubles as the in-flight guard: re-clicks while lit are ignored, so
    // a slow model can't stack parallel summary calls.
    {
        let rt_for_summary = slint_rt.clone();
        let events_for_summary = events.clone();
        let cfg_for_summary = cfg.clone();
        let rt_handle_for_summary = rt_handle.clone();
        let weak_for_summary = overlay.as_weak();
        overlay.on_summary_clicked(move || {
            let Some(o) = weak_for_summary.upgrade() else {
                return;
            };
            if o.get_summary_busy() {
                eprintln!("[overlay-host] summary already in flight — click ignored");
                return;
            }
            let (transcript, truncated) = {
                let s = slint_replay::runtime_state::lock(&rt_for_summary);
                (
                    s.full_transcript.iter().cloned().collect::<Vec<_>>(),
                    s.full_transcript_truncated,
                )
            };
            if let Err(reason) = overlay_backend::runtime::summary_gate(&transcript) {
                eprintln!("[overlay-host] summary skipped: {reason}");
                // The button must visibly react — friendly notice tile
                // instead of silence (kind=Error → not conversational).
                // NOTE: advice mentions ONLY Старт — PTT results don't flow
                // through push_transcript_line, so suggesting PTT here would
                // send the user in a circle (review-agent finding).
                let (is_ru, stealth, preferred_monitor) = {
                    let c = cfg_for_summary.read();
                    (
                        c.response_language == "ru",
                        c.stealth_enabled,
                        c.tile_monitor_name.clone(),
                    )
                };
                let (title, msg) = if is_ru {
                    (
                        "Summary созвона",
                        "Транскрипта пока нет. Нажмите Старт, поговорите — \
                         и Summary соберёт итог встречи.",
                    )
                } else {
                    (
                        "Meeting summary",
                        "No transcript yet. Press Start and talk a bit — \
                         then Summary will assemble the meeting recap.",
                    )
                };
                // Same monitor policy as the real summary tile (and every
                // other ask path) — Named pin from config, else Auto.
                let hint = match preferred_monitor.as_deref() {
                    Some(name) if !name.is_empty() => MonitorHint::Named(name.to_string()),
                    _ => MonitorHint::Auto,
                };
                if let Err(e) = events_for_summary.spawn_tile_full(
                    TileSpec {
                        question: title.to_string(),
                        answer: msg.to_string(),
                        source: "summary".into(),
                        is_translation: false,
                        highlights: vec![],
                    },
                    hint,
                    stealth,
                    TileKind::Error,
                ) {
                    eprintln!("[overlay-host] summary notice tile spawn failed: {e}");
                }
                return;
            }
            if truncated {
                eprintln!(
                    "[overlay-host] summary: accumulator overflowed earlier — covering the \
                     most recent {} lines",
                    transcript.len()
                );
            }
            o.set_summary_busy(true);
            eprintln!(
                "[overlay-host] summary requested over {} transcript lines",
                transcript.len()
            );
            let events_c = events_for_summary.clone();
            let cfg_c = cfg_for_summary.clone();
            let weak_done = weak_for_summary.clone();
            rt_handle_for_summary.spawn(async move {
                overlay_backend::runtime::run_meeting_summary(events_c, cfg_c, transcript).await;
                // Success OR error — run_meeting_summary spawned a tile
                // either way; just release the busy latch on the UI thread.
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(o) = weak_done.upgrade() {
                        o.set_summary_busy(false);
                    }
                });
            });
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
    let local_ai_servers = {
        let mut s = state.lock().unwrap_or_else(|p| p.into_inner());
        s.local_ai_servers.drain(..).collect::<Vec<_>>()
    };
    overlay_backend::local_ai::stop_managed_servers(
        &overlay_backend::local_ai::default_root(),
        local_ai_servers,
    );
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
///
/// v0.17.1 (мега-аудит) — `mic == false` here means MUTED: the mic chip is
/// the ONLY writer of `AppState.mic_active` (since v0.13.1 it mirrors
/// `rt.mic_muted`), so any refresh while the mic is off must keep showing the
/// amber «mic muted» pill. Before this, a sys-chip click or the sys-probe
/// revert timer silently overwrote «mic muted» with «sys only»/«idle»,
/// hiding the privacy-relevant mute state from the user.
fn refresh_status(overlay: &OverlayBarWindow, mic: bool, sys: bool) {
    let (text, color) = match (mic, sys) {
        (true, true) => ("recording", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (true, false) => ("mic only", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (false, _) => ("mic muted", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24)),
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
                // Surface WHY transparency may look broken: per-pixel alpha needs
                // DWM composition. If it's off (RDP / a VM without a GPU / very old
                // driver) the overlay renders OPAQUE no matter the wiring. This is
                // NOT the Windows "Transparency effects" toggle. Logged so a
                // tester's "transparency doesn't work" report is diagnosable.
                if slint_replay::win32::composition_enabled() {
                    eprintln!(
                        "[overlay-host] DWM composition: ON (overlay transparency available)"
                    );
                } else {
                    eprintln!("[overlay-host] DWM composition: OFF — overlay renders OPAQUE (no per-pixel alpha). Cause is the environment (RDP/remote, a VM without a GPU, or an outdated GPU driver), not the app. NB: NOT the Windows 'Transparency effects' toggle.");
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
    // For a LOCAL model show the friendly "Gemma 4B" / "Gemma 12B" so the user
    // can tell the fast vs smart model apart at a glance (the user asked to see
    // the selected model more explicitly); cloud models keep the short id.
    let model = if ai_local {
        overlay_backend::local_ai::local_model_label(model_full)
    } else {
        short_model_name(model_full)
    };
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

#[cfg(test)]
mod watchdog_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts
    use super::WatchdogState;
    use std::time::{Duration, Instant};

    const COOLDOWN: Duration = Duration::from_secs(30);
    const MAX: u32 = 6;

    #[test]
    fn first_attempt_allowed_immediately() {
        // No prior attempt → cooled, under cap → attempt.
        assert!(WatchdogState::default().should_restart(Instant::now(), COOLDOWN, MAX));
    }

    #[test]
    fn within_cooldown_skips_then_attempts_after() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        wd.note_attempt(t0, false);
        assert!(
            !wd.should_restart(t0 + Duration::from_secs(10), COOLDOWN, MAX),
            "10s < 30s cooldown → skip"
        );
        assert!(
            wd.should_restart(t0 + Duration::from_secs(31), COOLDOWN, MAX),
            "31s ≥ 30s cooldown → attempt"
        );
    }

    #[test]
    fn fail_cap_stops_then_reachable_rearms() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        // MAX cooled failures in a row.
        for i in 0..MAX {
            let now = t0 + Duration::from_secs(31 * u64::from(i + 1));
            assert!(
                wd.should_restart(now, COOLDOWN, MAX),
                "attempt {i} under cap"
            );
            wd.note_attempt(now, false);
        }
        assert!(
            !wd.should_restart(t0 + Duration::from_secs(10_000), COOLDOWN, MAX),
            "hit the fail cap → stop attempting"
        );
        wd.note_reachable(); // server came back on its own
        assert!(
            wd.should_restart(t0 + Duration::from_secs(10_000), COOLDOWN, MAX),
            "a reachable server re-arms the cap"
        );
    }

    #[test]
    fn switched_resets_fail_count() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        wd.note_attempt(t0, false);
        wd.note_attempt(t0, false);
        wd.note_attempt(t0, true); // a confirmed restart
        assert_eq!(wd.consecutive_fails, 0, "Switched resets the counter");
        assert!(wd.should_restart(t0 + Duration::from_secs(31), COOLDOWN, MAX));
    }
}

#[cfg(test)]
mod mic_guard_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts
    use super::try_acquire_mic;

    /// The single-mic latch is shared by all six mic consumers (PTT mic, per-tile
    /// 🎤 voice follow-up, dictation toggle, diagnostics check, wizard mic-test,
    /// Settings mic-test). No live test exercises those call sites, so this pins
    /// the primitive: one holder at a time, a rejected acquire does NOT free the
    /// holder (the `then` vs `then_some` bug — an eager guard ctor on the failed
    /// branch would `Drop` and clear `MIC_BUSY`), and the mic frees on drop.
    ///
    /// `MIC_BUSY` is a process-global static, so this is the ONLY test that may
    /// touch it; it leaves the latch free on exit for any later-running test.
    #[test]
    fn mic_guard_is_a_single_latch() {
        let g1 = try_acquire_mic();
        assert!(g1.is_some(), "first acquire on a free mic must succeed");

        let g2 = try_acquire_mic();
        assert!(g2.is_none(), "a second acquire while held must fail");

        // The failed g2 must not have released g1's lock: a third attempt while
        // g1 is still alive must STILL fail. With a `then_some` bug, g2's eager
        // temporary guard would have dropped and freed the mic, so this would
        // wrongly succeed.
        let g3 = try_acquire_mic();
        assert!(
            g3.is_none(),
            "a FAILED acquire must NOT free the held mic (then vs then_some)"
        );

        drop(g1);
        let g4 = try_acquire_mic();
        assert!(g4.is_some(), "after the holder drops, the mic is reusable");
        drop(g4);

        // Leave the global latch free (and re-confirm release worked).
        assert!(
            try_acquire_mic().is_some(),
            "the latch is free again at end of test"
        );
    }
}
