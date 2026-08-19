//! macOS transcript session — direct reuse of the shared Slint session
//! orchestrator (`slint_replay::slint_session`).
//!
//! The bootstrap bar's mic chip starts and stops the SAME audio → STT →
//! transcript pipeline the Windows host runs. This module owns only:
//! - the one Tokio runtime that pipeline needs,
//! - the tiny `SlintUiBridge` that delivers the latest UI events through
//!   `Weak + invoke_from_event_loop` (latest transcript line + the honest
//!   session start/stop state of the mic chip),
//! - the bounded [`MacTileSpawner`] queue: tile spawns from ANY pipeline
//!   thread land here nonblocking and the macOS main drains them on the
//!   Slint thread into the single reusable macOS tile window. Monitor/
//!   stealth hints are Windows-only concepts and are ignored on Mac.
//!
//! No duplicated STT/audio/journal pipeline lives here: capture, STT,
//! health, journal and transcript bookkeeping all stay inside
//! `slint_session::start_session` / `stop_session`.
//!
//! Lifecycle honesty:
//! - `start` refreshes the shared config from disk first (Mac AI/setup
//!   saves land without a restart), then runs the orchestrator. On ANY
//!   error it cleans the partial state through `stop_session` and the
//!   chip flips to the generic failure state.
//! - `stop` is the ONE synchronous stop path: chip and quit both call it,
//!   it always emits `session:stopped`, and `stop_session` is idempotent.
//!
//! Capture liveness watchdog: the mic + system workers emit chunks
//! continuously (silence included), so a previously flowing stream whose
//! `emitted_chunks` counter freezes is a stall. The pure
//! [`CaptureWatchdog`] folds one snapshot per timer tick
//! ([`MacTranscriptSession::watchdog_tick`]) and answers with ONE bounded
//! stop/start recovery after sustained stagnation — capped per unhealthy
//! episode, re-armed by health or a manual chip start/stop. A stream that
//! never emitted is never "recovered".
//!
//! Secrets: a failed start surfaces one generic visible state and one
//! category-only log line in the host — STT credential checks happen
//! inside `slint_session` and never reach this module's log. Watchdog
//! logs are category-only too.

use overlay_backend::config::SharedConfig;
use overlay_backend::events::{MonitorHint, RuntimeEvents, TileKind, TileSpec};
use slint::SharedString;
use slint_replay::runtime_state::{lock, shared_runtime, SharedSlintRuntime};
use slint_replay::slint_events::{SlintEvents, SlintUiBridge};
use slint_replay::slint_session;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::time::Duration;

use crate::ui;

/// Same visible transcript cap as the Windows bridge.
const TRANSCRIPT_MAX_CHARS: usize = 120;
/// Quit-path budget for in-flight tokio tasks after the session stopped.
const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// One session-computed tile request on its way to the single reusable
/// macOS tile window. The answer inside `spec` is ALREADY final — the
/// drain side performs no AI call.
pub(super) struct MacTileSpawn {
    pub(super) spec: TileSpec,
    pub(super) kind: TileKind,
}

/// Thread-safe producer half of the tile-spawn queue. Pipeline threads
/// (tokio workers) call [`Self::spawn`] from ANY thread; the macOS main
/// owns the receiver and drains it on the Slint main thread. Nonblocking
/// by construction — a full or disconnected queue returns a generic
/// error instead of stalling the pipeline.
pub(super) struct MacTileSpawner {
    tx: SyncSender<MacTileSpawn>,
    /// Unique monotonic ids for the synchronous `schedule_spawn_tile`
    /// label contract — never reused, even across failed sends.
    next_id: AtomicU64,
}

impl MacTileSpawner {
    #[must_use]
    pub(super) fn new(tx: SyncSender<MacTileSpawn>) -> Self {
        Self {
            tx,
            next_id: AtomicU64::new(0),
        }
    }

    /// Enqueue one finalized tile without blocking.
    ///
    /// # Errors
    /// One generic message for both failure modes (queue full / receiver
    /// gone) — no spec content may leak into the error path.
    pub(super) fn spawn(&self, spec: TileSpec, kind: TileKind) -> Result<String, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        match self.tx.try_send(MacTileSpawn { spec, kind }) {
            Ok(()) => Ok(format!("mac-tile-{id}")),
            Err(_) => Err("tile spawn dropped: queue full or receiver gone".to_string()),
        }
    }
}

/// Watchdog timer cadence (the dedicated repeated Slint timer in
/// `overlay_host.rs`).
pub(super) const WATCHDOG_TICK_INTERVAL: Duration = Duration::from_secs(2);
/// Consecutive no-progress ticks marking an expected stream stagnant.
const WATCHDOG_STAGNANT_TICKS: u32 = 5;
/// Recovery restarts allowed per unhealthy episode.
const WATCHDOG_MAX_ATTEMPTS: u32 = 3;
/// Consecutive all-expected-progress ticks that re-arm the budget.
const WATCHDOG_REARM_TICKS: u32 = 5;

/// One watchdog tick outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WatchdogDecision {
    Idle,
    Restart,
}

/// Per-stream stagnation tracker. Pure value state — no locks, no I/O.
#[derive(Debug, Default, Clone, Copy)]
struct StreamWatch {
    /// The stream emitted at least one chunk since the last manual reset.
    /// Survives automatic restarts on purpose, so a dead stream cannot
    /// escape recovery by resetting its counters to zero.
    expected: bool,
    /// `emitted_chunks` at the last tick that counted as progress.
    baseline: u64,
    /// Consecutive ticks without progress.
    stagnant_ticks: u32,
}

impl StreamWatch {
    /// Fold one `emitted_chunks` sample; reports whether this tick made
    /// progress. A counter that went DOWN is a new capture generation —
    /// progress, never subtraction.
    fn observe(&mut self, emitted: u64) -> bool {
        if !self.expected {
            if emitted == 0 {
                return false;
            }
            self.expected = true;
            self.baseline = emitted;
            self.stagnant_ticks = 0;
            return true;
        }
        if emitted == self.baseline {
            self.stagnant_ticks = self.stagnant_ticks.saturating_add(1);
            return false;
        }
        self.baseline = emitted;
        self.stagnant_ticks = 0;
        true
    }

    fn stagnant(&self) -> bool {
        self.expected && self.stagnant_ticks >= WATCHDOG_STAGNANT_TICKS
    }
}

/// Pure capture-liveness state machine — no audio, UI, or tokio. Folded
/// once per watchdog timer tick on the Slint main thread.
#[derive(Debug, Default)]
pub(super) struct CaptureWatchdog {
    mic: StreamWatch,
    system: StreamWatch,
    /// Recovery restarts consumed in the current unhealthy episode.
    attempts_used: u32,
    /// Consecutive ticks where every expected stream progressed.
    healthy_ticks: u32,
}

impl CaptureWatchdog {
    /// Fold one tick. `None` means no capture handle (session stopped, or
    /// a failed automatic restart left no capture mid-recovery).
    pub(super) fn tick(&mut self, snapshot: Option<(u64, u64)>) -> WatchdogDecision {
        let Some((mic_emitted, system_emitted)) = snapshot else {
            // A missing capture never resets the budget: a failed
            // automatic restart keeps its remaining bounded attempts.
            self.healthy_ticks = 0;
            if self.mic.expected || self.system.expected {
                return self.restart_if_armed();
            }
            return WatchdogDecision::Idle;
        };
        let mic_progress = self.mic.observe(mic_emitted);
        let system_progress = self.system.observe(system_emitted);
        if self.mic.stagnant() || self.system.stagnant() {
            return self.restart_if_armed();
        }
        // Sustained progress of EVERY expected stream re-arms the budget.
        if (self.mic.expected || self.system.expected)
            && (!self.mic.expected || mic_progress)
            && (!self.system.expected || system_progress)
        {
            self.healthy_ticks = self.healthy_ticks.saturating_add(1);
            if self.healthy_ticks >= WATCHDOG_REARM_TICKS {
                self.attempts_used = 0;
                self.healthy_ticks = 0;
            }
        } else {
            self.healthy_ticks = 0;
        }
        WatchdogDecision::Idle
    }

    fn restart_if_armed(&mut self) -> WatchdogDecision {
        if self.attempts_used >= WATCHDOG_MAX_ATTEMPTS {
            return WatchdogDecision::Idle;
        }
        self.attempts_used += 1;
        self.healthy_ticks = 0;
        // Fresh stagnation window: attempts stay ~10 s apart instead of
        // refiring every tick while counters stay frozen.
        self.mic.stagnant_ticks = 0;
        self.system.stagnant_ticks = 0;
        WatchdogDecision::Restart
    }

    /// Manual chip start/stop: forget every episode state.
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// One owned Tokio runtime + the shared session primitives for the macOS
/// transcript session. Created once by the macOS main; every callback
/// reaches it through the `Rc`.
pub(super) struct MacTranscriptSession {
    /// Owned runtime; taken out exactly once by [`Self::shutdown`].
    runtime: RefCell<Option<tokio::runtime::Runtime>>,
    handle: tokio::runtime::Handle,
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    rt: SharedSlintRuntime,
    /// Slint main thread only — chip callbacks and the watchdog timer.
    watchdog: RefCell<CaptureWatchdog>,
}

impl MacTranscriptSession {
    /// Build the runtime + bridge from the startup `SharedConfig` (the
    /// macOS main's single startup config read wrapped via
    /// `config::shared_from`). `tile_spawner` is the bounded queue the
    /// main owns the receiver half of.
    ///
    /// # Errors
    /// Fails only when the Tokio runtime itself cannot start.
    pub(super) fn new(
        overlay_weak: slint::Weak<ui::OverlayBarWindow>,
        cfg: SharedConfig,
        tile_spawner: MacTileSpawner,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let handle = runtime.handle().clone();
        let bridge = Arc::new(MacSessionBridge {
            overlay_weak,
            tile_spawner,
        });
        Ok(Self {
            runtime: RefCell::new(Some(runtime)),
            handle,
            events: Arc::new(SlintEvents::new(bridge)),
            cfg,
            rt: shared_runtime(),
            watchdog: RefCell::new(CaptureWatchdog::default()),
        })
    }

    /// Re-read the on-disk config into the shared handle. Called before
    /// every start so saves written by the Mac AI/setup window are live
    /// without an app restart.
    fn refresh_config_from_disk(&self) {
        let fresh = overlay_backend::config::load();
        *self.cfg.write() = fresh;
    }

    /// Start the shared audio → STT → transcript pipeline. Synchronous so
    /// the chip's failure path never races: `Err` means the pipeline never
    /// reached the live state and the chip must flip to the generic
    /// failure state. Any partial state (journal opened, capture spawned)
    /// is cleaned through `stop_session` before the error surfaces.
    ///
    /// Manual chip start: resets the watchdog for the fresh session.
    ///
    /// # Errors
    /// Mirrors `slint_session::start_session` — missing STT credentials or
    /// a capture device failure.
    pub(super) fn start(&self) -> anyhow::Result<()> {
        self.watchdog.borrow_mut().reset();
        self.start_core()
    }

    /// Shared start core for the manual path and the watchdog restart —
    /// no watchdog bookkeeping, so auto-recovery never resets the budget.
    fn start_core(&self) -> anyhow::Result<()> {
        self.refresh_config_from_disk();
        let _guard = self.handle.enter();
        if let Err(error) =
            slint_session::start_session(self.events.clone(), self.cfg.clone(), self.rt.clone())
        {
            // A failed start may have opened a journal or spawned capture
            // before the error — leave nothing half-alive behind.
            let _snapshot = slint_session::stop_session(self.rt.clone(), &self.cfg);
            return Err(error);
        }
        Ok(())
    }

    /// True while a session owns a live capture handle.
    pub(super) fn is_active(&self) -> bool {
        lock(&self.rt).capture.is_some()
    }

    /// The ONE stop path — chip and quit both call it. Synchronous and
    /// idempotent: capture is torn down before the call returns, and
    /// `session:stopped` is emitted after every real stop (the bridge
    /// flips the chip back to the stopped state).
    ///
    /// Manual chip stop: resets the watchdog.
    pub(super) fn stop(&self) {
        self.watchdog.borrow_mut().reset();
        let _snapshot = slint_session::stop_session(self.rt.clone(), &self.cfg);
        self.events.emit("session:stopped", serde_json::Value::Null);
    }

    /// Automatic recovery restart. NOT `stop` + `start`: those public
    /// paths reset the watchdog, which would let a dying stream wipe its
    /// attempt budget.
    ///
    /// Note on session semantics: this performs emergency whole-session recovery
    /// via `stop_session` + `start_core`. The current journal and audio recordings
    /// are cleanly finalized without corruption, and a fresh session ID is opened.
    /// A hardware stall episode thus safely splits the meeting into two sessions
    /// rather than leaving behind unfinalized state or corrupted streams.
    #[cfg(target_os = "macos")]
    fn restart_internal(&self) -> anyhow::Result<()> {
        let _snapshot = slint_session::stop_session(self.rt.clone(), &self.cfg);
        self.start_core()
    }

    /// One watchdog tick (Slint main thread): fold the per-stream
    /// `emitted_chunks` counters and run the bounded recovery restart.
    /// No runtime lock or watchdog borrow is held across the restart.
    #[cfg(target_os = "macos")]
    pub(super) fn watchdog_tick(&self) {
        let snapshot = lock(&self.rt).capture.as_ref().map(|capture| {
            let metrics = capture.metrics_snapshot();
            (metrics.mic.emitted_chunks, metrics.system.emitted_chunks)
        });
        // Fold, drop the borrow, then act.
        let decision = self.watchdog.borrow_mut().tick(snapshot);
        if decision == WatchdogDecision::Restart {
            slint_replay::logging::line(
                "[macos] capture watchdog: restarting a stagnant capture session",
            );
            if self.restart_internal().is_err() {
                slint_replay::logging::line("[macos] capture watchdog: restart attempt failed");
            }
        }
    }

    /// Drain the owned runtime after the final [`Self::stop`] (quit path).
    pub(super) fn shutdown(&self) {
        if let Some(runtime) = self.runtime.borrow_mut().take() {
            runtime.shutdown_timeout(RUNTIME_SHUTDOWN_TIMEOUT);
        }
    }
}

/// The tiny macOS `SlintUiBridge`: routes the session's UI events into the
/// bootstrap bar's existing properties. Channels the bootstrap row does not
/// render are ignored; tile spawns enqueue into the bounded spawner queue
/// the macOS main drains on the Slint thread.
struct MacSessionBridge {
    overlay_weak: slint::Weak<ui::OverlayBarWindow>,
    tile_spawner: MacTileSpawner,
}

impl MacSessionBridge {
    fn set_capture_state(&self, state: i32) {
        let weak = self.overlay_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                window.set_mic_capture_state(state);
            }
        });
    }
}

impl SlintUiBridge for MacSessionBridge {
    fn forward_event(&self, channel: String, payload: serde_json::Value) {
        match channel.as_str() {
            // Every STT-completed line lands in the bootstrap row — no
            // throttling: each event is one finalized utterance, and the
            // user must see every one of them.
            "transcript:line" => {
                let text: String = payload
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(TRANSCRIPT_MAX_CHARS)
                    .collect();
                // Verified against both `overlay_backend::audio` seams:
                // `AudioSource` carries `#[serde(rename_all = "lowercase")]`
                // on Windows AND macOS, so the wire tags are "mic"/"system".
                // Map them to the bar's "mic"/"sys" source contract.
                let source = match payload.get("source").and_then(serde_json::Value::as_str) {
                    Some("mic") => "mic",
                    Some("system") => "sys",
                    _ => "",
                };
                let weak = self.overlay_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_last_transcript_line(SharedString::from(text));
                        window.set_last_transcript_source(SharedString::from(source));
                    }
                });
            }
            "session:started" => self.set_capture_state(1),
            "session:stopped" => self.set_capture_state(0),
            // health/cost/meeting channels have no bootstrap surface;
            // tiles arrive through `schedule_spawn_tile`, not `emit`.
            _ => {}
        }
    }

    fn schedule_spawn_tile(
        &self,
        spec: TileSpec,
        _monitor: MonitorHint,
        _stealth: bool,
        kind: TileKind,
    ) -> Result<String, String> {
        // Monitor placement and stealth capture are Windows-only concepts;
        // the one reusable Mac tile ignores both.
        self.tile_spawner.spawn(spec, kind)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use std::sync::mpsc::sync_channel;

    fn spec(question: &str, answer: &str) -> TileSpec {
        TileSpec {
            question: question.to_string(),
            answer: answer.to_string(),
            source: "ai".to_string(),
            is_translation: false,
            highlights: vec![],
            summary_session: None,
        }
    }

    fn label_id(label: &str) -> u64 {
        label
            .rsplit('-')
            .next()
            .expect("label ends in the numeric id")
            .parse()
            .expect("label id parses as u64")
    }

    #[test]
    fn spawner_ids_are_unique_monotonic_and_spawns_deliver_intact_in_order() {
        let (tx, rx) = sync_channel::<MacTileSpawn>(8);
        let spawner = MacTileSpawner::new(tx);
        let kinds = [
            TileKind::Auto,
            TileKind::Mic,
            TileKind::System,
            TileKind::Ai,
        ];
        let labels: Vec<String> = (0..4u64)
            .map(|i| {
                spawner
                    .spawn(spec(&format!("q{i}"), &format!("a{i}")), kinds[i as usize])
                    .expect("an 8-slot queue has room for four spawns")
            })
            .collect();

        let ids: Vec<u64> = labels.iter().map(|label| label_id(label)).collect();
        assert!(
            ids.windows(2).all(|pair| pair[0] < pair[1]),
            "ids must be strictly monotonic: {ids:?}"
        );

        // Every spawn lands intact, in send order.
        for (i, kind) in kinds.iter().enumerate() {
            let spawn = rx
                .try_recv()
                .expect("every accepted spawn must be delivered");
            assert_eq!(spawn.spec.question, format!("q{i}"));
            assert_eq!(spawn.spec.answer, format!("a{i}"));
            assert_eq!(spawn.kind, *kind);
        }
        assert!(rx.try_recv().is_err(), "no extra events may appear");
    }

    #[test]
    fn full_queue_fails_without_blocking() {
        let (tx, _rx) = sync_channel::<MacTileSpawn>(1);
        let spawner = MacTileSpawner::new(tx);
        assert!(spawner.spawn(spec("q1", "a1"), TileKind::Ai).is_ok());
        let error = spawner
            .spawn(spec("q2", "a2"), TileKind::Ai)
            .expect_err("the single slot is already taken");
        assert!(!error.is_empty());
    }

    #[test]
    fn disconnected_receiver_fails_cleanly() {
        let (tx, rx) = sync_channel::<MacTileSpawn>(8);
        drop(rx);
        let spawner = MacTileSpawner::new(tx);
        assert!(spawner.spawn(spec("q", "a"), TileKind::Mic).is_err());
    }

    /// Tick until one restart fires; fails loudly if it never does.
    fn stall_until_restart(wd: &mut CaptureWatchdog, snapshot: (u64, u64)) {
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            if wd.tick(Some(snapshot)) == WatchdogDecision::Restart {
                return;
            }
        }
        panic!("watchdog never restarted on a stagnant expected stream");
    }

    #[test]
    fn watchdog_progress_then_stall_restarts_at_threshold() {
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        assert_eq!(wd.tick(Some((2, 0))), WatchdogDecision::Idle);
        for _ in 0..WATCHDOG_STAGNANT_TICKS - 1 {
            assert_eq!(wd.tick(Some((2, 0))), WatchdogDecision::Idle);
        }
        assert_eq!(wd.tick(Some((2, 0))), WatchdogDecision::Restart);
    }

    #[test]
    fn watchdog_ignores_streams_that_never_emitted() {
        // First-run TCC / unavailable sources: zero counters and absent
        // capture must never trigger recovery.
        let mut wd = CaptureWatchdog::default();
        for _ in 0..WATCHDOG_STAGNANT_TICKS * 3 {
            assert_eq!(wd.tick(Some((0, 0))), WatchdogDecision::Idle);
            assert_eq!(wd.tick(None), WatchdogDecision::Idle);
        }
    }

    #[test]
    fn watchdog_either_expected_stream_triggers_independently() {
        // System stalls while the mic keeps flowing.
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((1, 10))), WatchdogDecision::Idle);
        for i in 0..WATCHDOG_STAGNANT_TICKS {
            let mic = 2 + u64::from(i);
            let decision = wd.tick(Some((mic, 10)));
            if i + 1 == WATCHDOG_STAGNANT_TICKS {
                assert_eq!(decision, WatchdogDecision::Restart);
            } else {
                assert_eq!(decision, WatchdogDecision::Idle);
            }
        }
        // Symmetric: mic stalls while the system keeps flowing.
        wd.reset();
        assert_eq!(wd.tick(Some((100, 1))), WatchdogDecision::Idle);
        for i in 0..WATCHDOG_STAGNANT_TICKS {
            let sys = 2 + u64::from(i);
            let decision = wd.tick(Some((100, sys)));
            if i + 1 == WATCHDOG_STAGNANT_TICKS {
                assert_eq!(decision, WatchdogDecision::Restart);
            } else {
                assert_eq!(decision, WatchdogDecision::Idle);
            }
        }
    }

    #[test]
    fn watchdog_counter_decrease_is_a_new_generation_not_underflow() {
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((50, 0))), WatchdogDecision::Idle);
        // Counter drops (new capture generation): progress, not underflow.
        assert_eq!(wd.tick(Some((0, 0))), WatchdogDecision::Idle);
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        // A stall in the new generation still recovers.
        for _ in 0..WATCHDOG_STAGNANT_TICKS - 1 {
            assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        }
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Restart);
    }

    #[test]
    fn watchdog_caps_attempts_at_three_including_no_capture_retries() {
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        // Attempt 1.
        stall_until_restart(&mut wd, (1, 0));
        // The restart failed, capture is None: the remaining bounded
        // attempts still fire, the fourth request is denied.
        assert_eq!(wd.tick(None), WatchdogDecision::Restart);
        assert_eq!(wd.tick(None), WatchdogDecision::Restart);
        assert_eq!(wd.tick(None), WatchdogDecision::Idle);
        // A lingering stagnant snapshot cannot spend a fourth attempt.
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        }
    }

    #[test]
    fn watchdog_rearm_requires_every_expected_stream_to_progress() {
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((1, 1))), WatchdogDecision::Idle);
        // Exhaust the episode budget: three restarts, then capped.
        for _ in 0..WATCHDOG_MAX_ATTEMPTS {
            stall_until_restart(&mut wd, (1, 1));
        }
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            assert_eq!(wd.tick(Some((1, 1))), WatchdogDecision::Idle);
        }
        // Half-healthy ticks (the system frozen) must not re-arm it.
        for i in 0..WATCHDOG_REARM_TICKS * 2 {
            let mic = 2 + u64::from(i);
            assert_eq!(wd.tick(Some((mic, 1))), WatchdogDecision::Idle);
        }
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            assert_eq!(wd.tick(Some((1, 1))), WatchdogDecision::Idle);
        }
        // Sustained progress of BOTH streams re-arms the budget...
        const BASE: u64 = 100;
        for i in 0..WATCHDOG_REARM_TICKS {
            let counter = BASE + u64::from(i);
            assert_eq!(wd.tick(Some((counter, counter))), WatchdogDecision::Idle);
        }
        // ...and the next unhealthy episode gets a fresh restart.
        let stalled = BASE + u64::from(WATCHDOG_REARM_TICKS);
        let mut fired = false;
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            if wd.tick(Some((stalled, stalled))) == WatchdogDecision::Restart {
                fired = true;
                break;
            }
        }
        assert!(fired, "the re-armed budget must allow a restart");
    }

    #[test]
    fn watchdog_manual_reset_clears_episode_state() {
        let mut wd = CaptureWatchdog::default();
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        // Stagnation just below the threshold...
        for _ in 0..WATCHDOG_STAGNANT_TICKS - 1 {
            assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        }
        // ...is forgotten by a reset: the counter reads as a first
        // emission again.
        wd.reset();
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);

        // A reset also re-arms an exhausted budget.
        for _ in 0..WATCHDOG_MAX_ATTEMPTS {
            stall_until_restart(&mut wd, (1, 0));
        }
        for _ in 0..WATCHDOG_STAGNANT_TICKS + 1 {
            assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        }
        wd.reset();
        assert_eq!(wd.tick(Some((1, 0))), WatchdogDecision::Idle);
        stall_until_restart(&mut wd, (1, 0));
    }
}
