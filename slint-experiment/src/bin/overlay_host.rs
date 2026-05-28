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
use overlay_backend::{ai, audio, config, journal, kb};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_replay::app_state::{format_timer, new_shared_state, next_model};
use slint_replay::markdown;
use slint_replay::runtime_state::{shared_runtime, SharedSlintRuntime};
use slint_replay::slint_events::{SlintEvents, SlintUiBridge};
use slint_replay::slint_session;
use slint_replay::win32::{
    drag_begin, drag_update, enum_monitors, get_window_rect, grab_hwnd, make_transparent_overlay,
    make_transparent_tile, move_window_pos_only, pick_monitor, set_always_on_top, set_stealth,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

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
    MarkdownBlock, OverlayBarWindow, PaletteResult, PaletteWindow, SettingsWindow, TileWindow,
};

type TileWindows = Rc<RefCell<Vec<TileWindow>>>;

// ===== Phase E3 — OverlayBarBridge =====
//
// Implements SlintUiBridge so the ported overlay-backend fns (called
// via SlintEvents) can update the overlay bar UI + spawn tile windows.
// Tile spawning routes through an mpsc channel because slint::invoke_
// from_event_loop requires Send + 'static closures and TileWindow is
// not Send (Rc inside) — a Timer on the UI thread polls the channel
// and creates real TileWindows.
struct OverlayBarBridge {
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
}

/// Per-streaming-tile state: weak handle + accumulated answer text.
/// Bridge re-renders the full markdown tree on every Delta — cheap
/// at <500 tokens, can be windowed later if needed.
struct StreamingTile {
    weak: slint::Weak<TileWindow>,
    accumulated: String,
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
                    (stream.weak.clone(), stream.accumulated.clone())
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(tile) = weak.upgrade() else {
                        return;
                    };
                    let blocks: Vec<MarkdownBlock> = markdown::parse(&body)
                        .into_iter()
                        .map(|b| MarkdownBlock {
                            kind: b.kind,
                            text: SharedString::from(b.text),
                            lang: SharedString::from(b.lang),
                        })
                        .collect();
                    tile.set_blocks(ModelRc::new(VecModel::from(blocks)));
                });
            }
            ai::AiEvent::Done { reason } => {
                self.dec_ai_in_flight();
                let weak = {
                    let mut slot = match self.current_streaming.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    slot.take().map(|s| s.weak)
                };
                if let Some(weak) = weak {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(tile) = weak.upgrade() {
                            tile.set_source_label(SharedString::from(format!(
                                "ai · done ({reason})"
                            )));
                        }
                    });
                }
            }
            ai::AiEvent::Error { message } => {
                self.dec_ai_in_flight();
                let weak = {
                    let mut slot = match self.current_streaming.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    slot.take().map(|s| s.weak)
                };
                if let Some(weak) = weak {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(tile) = weak.upgrade() {
                            let blocks = vec![MarkdownBlock {
                                kind: markdown::kind::PARAGRAPH,
                                text: SharedString::from(format!("⚠ AI error: {message}")),
                                lang: SharedString::from(""),
                            }];
                            tile.set_blocks(ModelRc::new(VecModel::from(blocks)));
                            tile.set_source_label(SharedString::from("⚠ error"));
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
                    let any_down = matches!(st("audio"), Some("down"))
                        || matches!(st("stt"), Some("down"))
                        || matches!(st("ai"), Some("down"));
                    let any_degraded = matches!(st("audio"), Some("degraded"))
                        || matches!(st("stt"), Some("degraded"))
                        || matches!(st("ai"), Some("degraded"));
                    if any_down {
                        o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0x4b, 0x4b));
                    } else if any_degraded {
                        o.set_status_color(slint::Color::from_rgb_u8(0xe5, 0xb4, 0x4b));
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
/// Bookmark status flash auto-revert (shorter than mic/sys since the
/// success/failure message is brief, not data).
const BOOKMARK_REVERT_SECS: u64 = 3;
/// global-hotkey channel poll interval. 50 ms is the standard
/// responsiveness/CPU trade-off for desktop hotkeys.
const HOTKEY_POLL_MS: u64 = 50;
/// Delay after window.show() before grabbing the HWND. winit realizes
/// the native window lazily; calling earlier returns NotSupported.
const HWND_GRAB_DELAY_MS: u64 = 200;
/// SLINT_OVERLAY_AUTO_TILE auto-spawn delay (smoke-test convenience).
const AUTO_TILE_DELAY_MS: u64 = 500;
/// Periodic session-timer chip update interval.
const TIMER_TICK_SECS: u64 = 1;
/// Default tile window dimensions (match ui/tile.slint preferred-*
/// values so the spawned window isn't forcibly shrunk on first paint).
const TILE_DEFAULT_W: i32 = 460;
const TILE_DEFAULT_H: i32 = 360;
/// AI ask cap. Sized to fit typical session-question answers without
/// runaway cost; Phase 2.11 (Settings → AI bridge) will surface as
/// user-configurable.
const AI_MAX_TOKENS: u32 = 600;

fn main() -> Result<(), slint::PlatformError> {
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

    // Phase C — load config once at startup. SharedConfig (Arc<RwLock>)
    // because Settings tab will eventually mutate it.
    let cfg = config::shared();
    eprintln!(
        "[overlay-host] config loaded: ai_model={} ai_base_url={}",
        cfg.read().ai_model,
        if cfg.read().ai_base_url.is_empty() {
            "(unset)"
        } else {
            "(set)"
        }
    );

    let state = new_shared_state();
    let tiles: TileWindows = Rc::new(RefCell::new(Vec::new()));
    let settings: Rc<RefCell<Option<SettingsWindow>>> = Rc::new(RefCell::new(None));

    let overlay = OverlayBarWindow::new()?;

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
    // Initialize ai-model chip from loaded config. (Was previously
    // overwritten by a stale `set_ai_model("sonnet")` boilerplate line
    // — caught by review-agent catch-up audit 2026-05-27.)
    overlay.set_ai_model(SharedString::from(cfg.read().ai_model.clone()));
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
                let result = audio::record_mic_blocking(PROBE_DURATION_MS, mic_device);
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
                let _ = dropped.hide();
                eprintln!(
                    "[overlay-host] live tile cap hit (>= {MAX_LIVE_TILES}) — dropping oldest"
                );
            }
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
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (poll/F3) close_clicked fired");
                if let Some(t) = weak_tile.upgrade() {
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
            // (monitor placement applied via apply_tile_hwnd_with_monitor.)
            let _ = tile.show();
            apply_tile_hwnd_with_monitor(&tile);
            tiles_for_poll.borrow_mut().push(tile);
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

    // ===== AI model cycle (Phase C: writes to config) =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        let cfg_cycle = cfg.clone();
        overlay.on_ai_model_cycle_clicked(move || {
            let new_model = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.ai_model = next_model(&st.ai_model).to_string();
                st.ai_model.clone()
            };
            // Persist to config.json — next AI call uses the new model.
            {
                let mut w = cfg_cycle.write();
                w.ai_model = new_model.clone();
            }
            let snapshot = cfg_cycle.read().clone();
            if let Err(e) = config::save(&snapshot) {
                eprintln!("[overlay-host] config save failed: {e}");
            } else {
                eprintln!("[overlay-host] ai_model -> {new_model} (saved)");
            }
            if let Some(o) = weak.upgrade() {
                o.set_ai_model(SharedString::from(new_model));
            }
        });
    }

    // ===== Bookmark chip (Phase C + E3 slice 4: read from SlintRuntime) =====
    //
    // Reads SlintRuntime.last_question / last_answer (canonical state
    // post-port — populated by reask_last, manual_spawn_tile,
    // manual_ask_source via their shim writebacks) and appends to
    // %APPDATA%\overlay-mvp\bookmarks.md. Falls back to AppState
    // .last_tile_qa for the legacy +tile chip path which still uses
    // local AI calls (slated for future commit to route through
    // events.spawn_tile_full like the other tile producers).
    {
        let s = state.clone();
        let slint_rt_bookmark = slint_rt.clone();
        let weak = overlay.as_weak();
        overlay.on_bookmark_clicked(move || {
            let qa_opt = {
                // Prefer SlintRuntime (post-port canonical) — the ported
                // F-key handlers write here via shim writeback.
                let rt_guard = slint_replay::runtime_state::lock(&slint_rt_bookmark);
                let from_rt = rt_guard
                    .last_question
                    .clone()
                    .and_then(|q| rt_guard.last_answer.clone().map(|a| (q, a)));
                drop(rt_guard);
                if from_rt.is_some() {
                    from_rt
                } else {
                    // Legacy +tile chip path — AppState.last_tile_qa.
                    let st = match s.lock() {
                        Ok(g) => g,
                        Err(p) => p.into_inner(),
                    };
                    st.last_tile_qa.clone()
                }
            };
            let Some(o) = weak.upgrade() else { return };
            match qa_opt {
                None => {
                    o.set_status_text(SharedString::from("bookmark: spawn a tile first"));
                    o.set_status_color(slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24));
                    eprintln!("[overlay-host] bookmark: no last_tile_qa yet");
                }
                Some((question, answer)) => match journal::append_bookmark(&question, &answer) {
                    Ok(path) => {
                        eprintln!("[overlay-host] bookmark appended to {}", path.display());
                        o.set_status_text(SharedString::from("bookmark saved"));
                        o.set_status_color(slint::Color::from_rgb_u8(0xfc, 0xd3, 0x4d));
                    }
                    Err(e) => {
                        eprintln!("[overlay-host] bookmark write failed: {e:#}");
                        o.set_status_text(SharedString::from("bookmark failed"));
                        o.set_status_color(slint::Color::from_rgb_u8(0xf8, 0x71, 0x71));
                    }
                },
            }
            // Auto-revert status after 3s.
            let weak_revert = weak.clone();
            let s_revert = s.clone();
            slint::Timer::single_shot(Duration::from_secs(BOOKMARK_REVERT_SECS), move || {
                if let Some(o) = weak_revert.upgrade() {
                    refresh_status(&o, get_mic_active(&s_revert), get_sys_active(&s_revert));
                }
            });
        });
    }

    // Tips chip opens the palette manually. The F4 global hotkey
    // (registered below) does the same. Both routes converge on
    // open_palette() for state consistency.
    let palette: Rc<RefCell<Option<PaletteWindow>>> = Rc::new(RefCell::new(None));
    {
        let palette_ref = palette.clone();
        let tiles_ref = tiles.clone();
        let s = state.clone();
        let weak_overlay = overlay.as_weak();
        overlay.on_tips_clicked(move || {
            open_palette(&palette_ref, &tiles_ref, &s, &weak_overlay);
        });
    }

    // ===== Global hotkeys F3 / F4 / F7 (Phase D2 + B3 extra) =====
    //
    // global-hotkey 0.6 owns a single process-wide event receiver +
    // platform-specific manager. We register one hotkey per F-key,
    // then poll the receiver every 50 ms from a Slint Timer — fires
    // on UI thread so we can touch Rc-borrowed state without Send.
    //
    // Mirrors the React/Tauri v0.1.1 binding table (Settings ▸ Hotkeys):
    //   F3 — Ask the AI now (same flow as + tile chip)
    //   F4 — Open KB palette
    //   F7 — Bulk collapse/expand all tiles (stub — toggles a flag)
    let hotkey_manager = match global_hotkey::GlobalHotKeyManager::new() {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("[overlay-host] GlobalHotKeyManager init failed: {e}. Hotkeys disabled.");
            None
        }
    };
    let f3_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F3);
    let f4_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F4);
    // Phase E3 slice 3 — F6 manual spawn from last transcript line
    // (bypasses auto-detector). Matches src-tauri hotkey table.
    let f6_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F6);
    let f7_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F7);
    // Phase E3 slice 2 — F9 ask (live AI streaming via overlay-backend's
    // ask_stream_loop). Matches src-tauri/React-side semantic where F9
    // is the "ask AI with full transcript context" hotkey.
    let f9_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F9);
    let f3_id = f3_hotkey.id();
    let f4_id = f4_hotkey.id();
    let f6_id = f6_hotkey.id();
    let f7_id = f7_hotkey.id();
    let f9_id = f9_hotkey.id();
    if let Some(m) = hotkey_manager.as_ref() {
        for (label, hk) in [
            ("F3", f3_hotkey),
            ("F4", f4_hotkey),
            ("F6", f6_hotkey),
            ("F7", f7_hotkey),
            ("F9", f9_hotkey),
        ] {
            match m.register(hk) {
                Ok(()) => eprintln!("[overlay-host] {label} hotkey registered"),
                Err(e) => eprintln!("[overlay-host] {label} register failed: {e}"),
            }
        }
    }

    let hotkey_poll = Timer::default();
    let hp_palette = palette.clone();
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
                    eprintln!("[overlay-host] F4 pressed — opening palette");
                    open_palette(&hp_palette, &hp_tiles, &hp_state, &hp_weak_overlay);
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
                } else if event.id == f7_id {
                    eprintln!("[overlay-host] F7 pressed — collapse-all (stub)");
                    // Phase 4+ would call `tile.set_collapsed(true)` on
                    // every open tile via the registry.
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
                    );
                }
            }
        },
    );

    // ===== Stealth toggle on overlay bar =====
    {
        let s = state.clone();
        let tiles_ref = tiles.clone();
        let weak = overlay.as_weak();
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
            // Apply to overlay
            if let Some(o) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(o.window()) {
                    let _ = set_stealth(hwnd, new_stealth);
                }
            }
            // Apply to all tiles
            for t in tiles_ref.borrow().iter() {
                if let Ok(hwnd) = grab_hwnd(t.window()) {
                    let _ = set_stealth(hwnd, new_stealth);
                }
            }
        });
    }

    // ===== Spawn tile (Phase C: real AI ask via overlay_backend::ai) =====
    {
        let s = state.clone();
        let t = tiles.clone();
        let weak = overlay.as_weak();
        let cfg_ref = cfg.clone();
        let rt = rt_handle.clone();
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

            // Demo prompt — only used when SLINT_OVERLAY_DEMO=1 is
            // set in the env, otherwise the tile shows an informative
            // placeholder explaining that real live-transcript wiring
            // arrives with Phase B2 (runtime.rs port). Gating behind
            // env var prevents production users from seeing canned
            // Kubernetes nonsense when they click +tile.
            // Code-quality audit 2026-05-27 (priority cleanup #1).
            let demo_mode = std::env::var("SLINT_OVERLAY_DEMO").is_ok();
            let question = if demo_mode {
                format!("Explain Kubernetes in 3 sentences. (Tile #{seq})")
            } else {
                format!("Tile #{seq} — no active session prompt")
            };
            tile.set_sequence(seq as i32);
            tile.set_tile_title(SharedString::from(question.clone()));
            tile.set_source_label(SharedString::from("ai · asking…"));
            wire_tile_drag(&tile);

            // Initial placeholder body while the AI call is in flight.
            let placeholder = vec![MarkdownBlock {
                kind: markdown::kind::PARAGRAPH,
                text: SharedString::from("⏳ Asking AI…"),
                lang: SharedString::from(""),
            }];
            tile.set_blocks(ModelRc::new(VecModel::from(placeholder)));

            let weak_tile = tile.as_weak();
            let vec_for_close = t.clone();
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (spawn-poll) close_clicked fired");
                if let Some(tw) = weak_tile.upgrade() {
                    let close_hwnd = grab_hwnd(tw.window()).ok();
                    let _ = tw.hide();
                    if let Some(target) = close_hwnd {
                        vec_for_close.borrow_mut().retain(|item| {
                            grab_hwnd(item.window()).ok() != Some(target)
                        });
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

            let _ = tile.show();
            apply_tile_hwnd_with_monitor(&tile);

            // Capture a Weak handle the tokio task can post back to
            // the UI thread via slint::invoke_from_event_loop.
            let weak_for_ai = tile.as_weak();
            // Clone the Arc<Mutex<AppState>> for the AI task to use
            // when storing last_tile_qa on success. Cheap arc clone.
            let s_for_bookmark = s.clone();
            t.borrow_mut().push(tile);

            // Spawn the AI call on the tokio runtime. Read config under
            // the lock briefly, drop, then run async.
            let snapshot = {
                let cfg_r = cfg_ref.read();
                (
                    cfg_r.ai_base_url.clone(),
                    cfg_r.ai_bearer.clone(),
                    cfg_r.ai_model.clone(),
                )
            };
            let (base_url, bearer, model) = snapshot;

            if base_url.is_empty() || bearer.is_empty() || !demo_mode {
                // EITHER no AI config OR demo-mode disabled — render
                // an informative tile instead of firing an AI call
                // with a canned demo prompt. Phase B2 work will read
                // the live mic transcript here and ask about it.
                let md = if base_url.is_empty() || bearer.is_empty() {
                    format!(
                        "# {question}\n\n*AI bridge not configured.* Open Settings → AI bridge to set `base_url` + `bearer token`, then re-spawn this tile.\n\n## Sample fallback content\n\n{}",
                        markdown::sample_tile_markdown(seq)
                    )
                } else {
                    format!(
                        "# Tile #{seq}\n\n*Demo mode disabled.* Set `SLINT_OVERLAY_DEMO=1` to re-enable the canned 'Explain Kubernetes' prompt. Phase B2 (runtime.rs port) will wire this chip to the live mic transcript.\n\n## Sample fallback content\n\n{}",
                        markdown::sample_tile_markdown(seq)
                    )
                };
                let blocks: Vec<MarkdownBlock> = markdown::parse(&md)
                    .into_iter()
                    .map(|b| MarkdownBlock {
                        kind: b.kind,
                        text: SharedString::from(b.text),
                        lang: SharedString::from(b.lang),
                    })
                    .collect();
                if let Some(t) = weak_for_ai.upgrade() {
                    t.set_blocks(ModelRc::new(VecModel::from(blocks)));
                    let label = if base_url.is_empty() || bearer.is_empty() {
                        "ai · not configured"
                    } else {
                        "ai · demo-mode off"
                    };
                    t.set_source_label(SharedString::from(label));
                }
                return;
            }

            let question_for_task = question.clone();
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
                            let cost_micro =
                                ai::cost_microcents(&model, usage.input, usage.output);
                            let cost_usd = cost_micro as f64 / 100_000_000.0;
                            let md = format!("# {question_for_task}\n\n{response}\n");
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
                            // Phase C — remember this Q+A so the bookmark
                            // chip can append it to bookmarks.md.
                            if let Ok(mut st) = s_for_bookmark.lock() {
                                st.last_tile_qa =
                                    Some((question_for_task.clone(), response.clone()));
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
                                "# {question_for_task}\n\n**AI call failed:** {category}\n\nCheck Settings → AI bridge or the bridge process / network.",
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

    // ===== Settings =====
    {
        let s = state.clone();
        let settings_ref = settings.clone();
        let tiles_ref = tiles.clone();
        let cfg_for_settings = cfg.clone();
        overlay.on_open_settings_clicked(move || {
            open_settings(&s, &settings_ref, &tiles_ref, &cfg_for_settings);
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

    // ===== Quit =====
    overlay.on_quit_clicked(|| {
        eprintln!("[overlay-host] quit requested");
        let _ = slint::quit_event_loop();
    });

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
                eprintln!("[overlay-host] auto-starting session on startup");
                o.invoke_timer_toggle_clicked();
            }
        });
    }

    let result = overlay.run();
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
fn apply_overlay_hwnd(overlay: &OverlayBarWindow) {
    let weak = overlay.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
        let Some(o) = weak.upgrade() else { return };
        match grab_hwnd(o.window()) {
            Ok(hwnd) => match make_transparent_overlay(hwnd) {
                Ok(()) => eprintln!("[overlay-host] overlay transparency wired"),
                Err(e) => eprintln!("[overlay-host] overlay transparency failed: {e}"),
            },
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

/// Phase E3 slice 4 — chip-driven manual ask for a specific source
/// (sys chip → System, future PTT → Mic). Snapshots rt into
/// ManualAskSourceInputs, spawns the ported `manual_ask_source`,
/// applies outcome writeback. Shape mirrors fire_f6.
///
/// Currently unwired — the existing mic/sys chip handlers are 3-second
/// audio-device probes (useful diagnostic, kept for now). Proper PTT
/// (press-and-hold) requires Slint UI changes — adding
/// `pointer-event` callbacks on the chip TouchAreas — to distinguish
/// press from release. That's deferred to a Phase E UX commit. When
/// it lands, the press handler calls
/// `slint_session::manual_ask_window_start` and the release handler
/// calls `manual_ask_window_end` (already ported as B2 port #6).
#[allow(dead_code, reason = "wired in Phase E PTT UX phase")]
fn fire_manual_ask_source(
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    source: overlay_backend::audio::AudioSource,
) {
    let inputs = {
        let s = slint_replay::runtime_state::lock(slint_rt);
        let recent = select_recent_labeled(&s.transcript, 8);
        // Trigger = last line from THIS source.
        let trigger_text = s
            .transcript
            .iter()
            .rev()
            .find(|l| l.source == source)
            .map(|l| l.text.clone())
            .unwrap_or_default();
        let cap_usd = cfg.read().max_session_cost_usd;
        let cost_cap = cost_cap_reason(cap_usd, s.session_cost_microcents);
        overlay_backend::runtime::ManualAskSourceInputs {
            recent_transcript_labeled: recent,
            trigger_text,
            cost_cap_reason: cost_cap,
            source,
            journal: s.journal.clone(),
            health: s.health.clone(),
        }
    };
    let events_c = events.clone();
    let cfg_c = cfg.clone();
    let rt_c = slint_rt.clone();
    rt_handle.spawn(async move {
        let outcome =
            overlay_backend::runtime::manual_ask_source(events_c.clone(), cfg_c, inputs).await;
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
fn fire_f9_ask(
    bridge: &Arc<OverlayBarBridge>,
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    rt_handle: &tokio::runtime::Handle,
    tiles: &TileWindows,
) {
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
    tile.set_tile_title(SharedString::from("F9 ask · live"));
    tile.set_source_label(SharedString::from("ai · asking…"));
    // Phase E6 v12 — purple trigger badge for manual F9 ask so user
    // sees which tile came from a hotkey vs auto-detector.
    tile.set_trigger_label(SharedString::from("✋ F9 manual ask"));
    tile.set_trigger_color(slint::Color::from_rgb_u8(0xa7, 0x8b, 0xfa));
    wire_tile_drag(&tile);
    let placeholder = vec![MarkdownBlock {
        kind: markdown::kind::PARAGRAPH,
        text: SharedString::from("⏳ Asking AI…"),
        lang: SharedString::from(""),
    }];
    tile.set_blocks(ModelRc::new(VecModel::from(placeholder)));
    let weak_close = tile.as_weak();
    let vec_for_close = tiles.clone();
    tile.on_close_clicked(move || {
        eprintln!("[overlay-host] tile (F9) close_clicked fired");
        if let Some(t) = weak_close.upgrade() {
            let close_hwnd = grab_hwnd(t.window()).ok();
            let _ = t.hide();
            if let Some(target) = close_hwnd {
                vec_for_close
                    .borrow_mut()
                    .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
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
    let _ = tile.show();
    apply_tile_hwnd_with_monitor(&tile);
    let weak_for_stream = tile.as_weak();
    tiles.borrow_mut().push(tile);

    // ===== 2. Register the tile in the bridge's streaming slot =====
    {
        let mut slot = match bridge.current_streaming.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *slot = Some(StreamingTile {
            weak: weak_for_stream,
            accumulated: String::new(),
        });
    }

    // ===== 3. Snapshot cfg + cost-cap + transcript + screenshot =====
    let (base_url, bearer, model, meeting_context, response_language, cap_usd) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(),
            c.meeting_context.clone(),
            c.response_language.clone(),
            c.max_session_cost_usd,
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

    let messages = ai::build_request(
        &meeting_context,
        &response_language,
        &transcript_lines,
        screenshot.as_deref(),
        None,
    );

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
            purpose: "live_ask",
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
        let mut s = slint_replay::runtime_state::lock(&rt_for_cost);
        s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);
        (s.session_cost_microcents as f64) / 100_000_000.0
    });

    let t0 = std::time::Instant::now();
    let events_for_task = events.clone();
    // CRITICAL: `ai::stream_chat` internally calls `tokio::spawn`,
    // which panics with "there is no reactor running" when called
    // from a non-tokio thread (Slint UI / hotkey poll Timer closure).
    // The same trap is documented in src-tauri/src/runtime.rs:1804
    // ("must be tauri::async_runtime::spawn, NOT tokio::spawn").
    // We move stream_chat INSIDE the rt_handle.spawn future so the
    // tokio runtime context exists when it runs.
    let task = rt_handle.spawn(async move {
        let ai_rx = ai::stream_chat(base_url, bearer, model.clone(), messages, 4096);
        overlay_backend::runtime::ask_stream_loop(
            events_for_task,
            ai_rx,
            model,
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

/// Atomic counter for tile-slot index — increments per spawn so
/// successive tiles distribute across a 2-column grid on the right
/// half of the chosen monitor.
static TILE_SLOT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Monotonic counter for the tile-title #N badge. Increments per
/// spawn (never wraps) so the user can tell tiles apart in a busy
/// session. Reset only at process restart.
static TILE_DISPLAY_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Phase E6 v17 — maximize toggle helper. User: "нет функционала
/// развернуть, нужно отдельной кнопкой или даб-кликом". Maximized
/// tile is 800×600 (~1.7× default); restored back to 460×360. Uses
/// Win32 SetWindowPos with current position so the tile expands in
/// place from its top-left corner. Flips tile.maximized so the
/// button glyph updates.
fn toggle_tile_maximize(_hwnd: windows::Win32::Foundation::HWND, tile: &TileWindow) {
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
    eprintln!("[overlay-host]   tile maximized -> {new} (size={w}x{h})");
}

/// Wire the chrome-row drag callbacks on a tile so the user can move
/// it by pressing+dragging the title area. Phase E6 v22 — manual
/// cursor-delta drag (drag_begin on down, drag_update on move-while-
/// pressed). REPLACES the old WM_NCLBUTTONDOWN modal system-drag
/// which consumed the mouse-up before Slint saw it, leaving the
/// TouchArea stuck "pressed" → tile became undraggable/unclickable.
/// User: "вызванный тайл завис, двигается но ничего не прожимается".
fn wire_tile_drag(tile: &TileWindow) {
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
/// Phase E6 fix v2 (2026-05-27): previous "right-edge stack" math
/// overflowed monitor.bottom after ~slot 2 (tile_h+12 × N > screen
/// height) → user complaint "тайлы уходят за экран". Now uses a
/// 2-column × dynamic-rows grid with hard clamps to monitor bounds.
/// Pre-port React/Tauri used src-tauri's tile.rs::grid_position
/// (~80 LOC of layered math); this is a simpler 2-col wrap that
/// fits on any landscape monitor without overflow.
fn apply_tile_hwnd_with_monitor(tile: &TileWindow) {
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
            // secondary at negative x).
            let x_clamped = x.clamp(mon.left + 8, mon.right - real_w - 8);
            let y_clamped = y.clamp(mon.top + 8, mon.bottom - real_h - 8);

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
            eprintln!("[overlay-host] tile placement: no monitor returned by pick_monitor — tile not moved");
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
            tile.on_close_clicked(move || {
                eprintln!("[overlay-host] tile (KB-palette) close_clicked fired");
                if let Some(t) = weak_tile.upgrade() {
                    let close_hwnd = grab_hwnd(t.window()).ok();
                    let _ = t.hide();
                    if let Some(target) = close_hwnd {
                        vec_for_close
                            .borrow_mut()
                            .retain(|item| grab_hwnd(item.window()).ok() != Some(target));
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

            let _ = tile.show();
            apply_tile_hwnd_with_monitor(&tile);
            tiles_ref2.borrow_mut().push(tile);
        }
        // Close palette after activation.
        if let Some(p) = weak_self.upgrade() {
            let _ = p.hide();
        }
        *palette_after.borrow_mut() = None;
    });

    let _ = win.show();
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
fn open_settings(
    state: &slint_replay::app_state::SharedState,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    tiles_ref: &TileWindows,
    cfg: &overlay_backend::config::SharedConfig,
) {
    let mut settings_slot = settings_ref.borrow_mut();
    if let Some(existing) = settings_slot.as_ref() {
        // Refresh token status — config might have changed since last open.
        populate_token_status(existing, cfg);
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

    // Phase E6 v23 — populate the Audio tab's mic dropdown from real
    // WASAPI capture endpoints + select the saved device. User: "Audio
    // не подгружает реальные микрофоны".
    {
        let devices = overlay_backend::audio::list_devices()
            .map(|d| d.inputs)
            .unwrap_or_default();
        let saved = cfg.read().mic_device.clone();
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
        win.set_mic_devices(ModelRc::new(VecModel::from(model)));
        win.set_mic_device_index(sel as i32);
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
                let result = overlay_backend::audio::record_mic_blocking(3000, device);
                let msg = match result {
                    Ok(samples) if samples.is_empty() => "no audio captured".to_string(),
                    Ok(samples) => {
                        // Phase E6 v28 — use RMS (average energy) not just
                        // peak, + a dBFS threshold. User: "я могу ничего
                        // не говорить, но всё равно будет OK" — the old
                        // peak==0 check passed on any tiny noise. Real
                        // speech RMS is > -40 dBFS; a silent room is
                        // < -55 dBFS. Threshold at -45 dBFS.
                        let sum_sq: f64 = samples
                            .iter()
                            .map(|s| {
                                let v = f64::from(*s) / 32768.0;
                                v * v
                            })
                            .sum();
                        let rms = (sum_sq / samples.len() as f64).sqrt();
                        let dbfs = if rms <= 0.0 {
                            f64::NEG_INFINITY
                        } else {
                            20.0 * rms.log10()
                        };
                        if dbfs < -45.0 {
                            format!("[!] too quiet ({dbfs:.0} dBFS) — say something / check mic")
                        } else {
                            format!("[ok] heard you ({dbfs:.0} dBFS RMS)")
                        }
                    }
                    Err(e) => format!("error: {e}"),
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
    let tiles_ref3 = tiles_ref.clone();
    win.on_stealth_changed(move |on| {
        if let Ok(mut st) = s3.lock() {
            st.stealth = on;
        }
        for t in tiles_ref3.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
    });

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
            eprintln!("[overlay-host] ai_base_url saved: {trimmed}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_model_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("[overlay-host] ai_model save skipped: empty input");
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
            eprintln!("[overlay-host] ai_model saved: {trimmed}");
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
            // Apply live to all currently-visible tiles.
            for tile in tiles_c.borrow().iter() {
                tile.set_body_opacity(clamped);
            }
            eprintln!("[overlay-host] tile_body_opacity -> {clamped:.2}");
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
            let api_key = cfg_c.read().groq_api_key.clone();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::stt::test_connection(api_key)) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
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

    let weak_close = win.as_weak();
    let settings_close = settings_ref.clone();
    win.on_close_clicked(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *settings_close.borrow_mut() = None;
    });

    let _ = win.show();
    // Phase E6 v26 — apply DWM per-pixel alpha so the frameless
    // window's rounded corners composite over the desktop (otherwise
    // the corners show black). make_transparent_tile = WS_EX_TOOLWINDOW
    // + DWM blur-behind, NO click-through (settings needs clicks).
    {
        let weak = win.as_weak();
        Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
            if let Some(w) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    let _ = make_transparent_tile(hwnd);
                }
            }
        });
    }
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
        let prefix: String = c.ai_bearer.chars().take(3).collect();
        format!("[ok] set ({len} chars, starts: {prefix}***)")
    };
    let groq_status = if c.groq_api_key.is_empty() {
        "[--] not set".to_string()
    } else {
        let len = c.groq_api_key.chars().count();
        let prefix: String = c.groq_api_key.chars().take(3).collect();
        format!("[ok] set ({len} chars, starts: {prefix}***)")
    };
    win.set_ai_bearer_status(SharedString::from(ai_status));
    win.set_groq_api_key_status(SharedString::from(groq_status));
    // Phase E6 v20 — load tile opacity from config so the slider
    // reflects the saved value on Settings re-open.
    win.set_tile_body_opacity(c.tile_body_opacity);
    win.set_ai_base_url_input(SharedString::from(c.ai_base_url.clone()));
    win.set_ai_model_input(SharedString::from(c.ai_model.clone()));
}
