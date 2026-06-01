//! Slint-side runtime state — analog of `src-tauri/src/runtime.rs`
//! `RuntimeState`. Carries everything the overlay-backend ported fns
//! need (transcript, journal, health, last_qa, session_cost) plus
//! Slint-binary-specific bookkeeping (capture handle, in-flight task
//! JoinHandles for cancellation).
//!
//! Design rationale (Phase E1, 2026-05-27):
//!
//! The B2 plan deferred porting `RuntimeState` itself to overlay-backend
//! (would have meant a 25-field shared crate addition for one private
//! mutex). Instead each binary maintains its OWN runtime state with
//! the same shape — the trait-boundary contract is the `*Inputs` /
//! `*Outcome` structs in `overlay_backend::runtime`.
//!
//! Threading model:
//! - UI thread (Slint event loop) reads/writes via `lock()`.
//! - Tokio worker threads read/writes via `lock()` from inside
//!   spawned tasks (transcript forwarder, AI calls).
//! - UI updates from tokio side go through
//!   `slint::invoke_from_event_loop(...)` because Slint property
//!   setters must run on the main thread.

use overlay_backend::audio::{CaptureHandle, TranscriptLine};
use overlay_backend::health::HealthSignals;
use overlay_backend::journal::Journal;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

/// Slint-binary equivalent of src-tauri's `RuntimeState`. Owns the
/// rolling transcript + the active session's journal + the audio
/// capture handle + tokio JoinHandles for cancellation.
#[derive(Default)]
pub struct SlintRuntime {
    /// Active audio capture handle. `None` when no session is running.
    /// Dropping it signals the WASAPI thread to stop.
    pub capture: Option<CaptureHandle>,

    /// Rolling transcript window — capped at `TRANSCRIPT_MAX_LINES`
    /// (80) entries. Append-only; oldest evicted on overflow.
    pub transcript: VecDeque<TranscriptLine>,

    /// Last successful AI question shown to the user. Reask reads
    /// this for the F3 "reask the reask" flow.
    pub last_question: Option<String>,

    /// Last successful AI answer (raw markdown). Reask wraps it.
    pub last_answer: Option<String>,

    /// Cumulative session cost in microcents (1e-8 USD). Updated
    /// after every AI call via the shim writeback pattern from
    /// the ported fns (`ReaskOutcome.cost_microcents_delta` etc.).
    pub session_cost_microcents: u64,

    /// Active session's journal handle. `None` between sessions
    /// (or when journal open failed). Cloned cheaply (Arc-backed)
    /// into transcript-forwarder + per-port `*Inputs` snapshots.
    pub journal: Option<Journal>,

    /// Health-signals Arc shared with audio + STT + AI pipelines.
    /// Each subsystem bumps its `last_*_ok_ms` atomic on success;
    /// the 2-second health-emitter task snapshots + emits to UI.
    pub health: Arc<HealthSignals>,

    /// JoinHandle of the transcript-forwarder task spawned by
    /// `start_session`. Aborted on session stop or restart.
    pub transcript_task: Option<tokio::task::JoinHandle<()>>,

    /// JoinHandle of the in-flight F9 ask task (if any). Aborted
    /// on rapid F9 re-press so responses don't stack.
    pub ai_task: Option<tokio::task::JoinHandle<()>>,

    /// JoinHandle of the 2-second health emitter ticker. Aborted
    /// on session stop so it doesn't fire against stale state.
    pub health_task: Option<tokio::task::JoinHandle<()>>,

    /// True when the user has muted the mic chip — transcript
    /// forwarder drops mic-source lines silently while this is set.
    /// System audio remains unaffected. Toggled by the mic chip.
    pub mic_muted: bool,

    /// Sliding window of recent tile-spawn timestamps for the
    /// auto-detector rate-limit (drops triggers exceeding
    /// `MAX_TILES_PER_MIN` in the last 60s).
    pub recent_tile_triggers: VecDeque<Instant>,

    /// Normalized recent question prefixes for the auto-detector
    /// dedup (drops triggers that repeat within 60s).
    pub recent_question_prefixes: Vec<(String, Instant)>,

    /// QA cache for the auto-detector — keys are model+lang+ctx-
    /// hashed question prefixes; values are (answer, insert
    /// timestamp). TTL 10 min, bounded at 256 entries.
    pub qa_cache: HashMap<String, (String, Instant)>,

    /// Pending screenshot data URL — stashed by the screenshot
    /// helper, consumed (taken) on the next F9 ask. None when no
    /// screenshot is pending.
    pub last_screenshot: Option<String>,

    /// Rolling speech-coach window — last 60 seconds of mic-only
    /// (timestamp_ms, words, fillers) tuples. Drives the WPM +
    /// filler-density labels in the overlay.
    pub speech_window: VecDeque<(u64, u32, u32)>,

    /// One-shot guard so the meeting-ending detector emits
    /// `meeting:ending` exactly once per session.
    pub meeting_ending_emitted: bool,

    /// E9 — session generation, bumped on every start/stop. Auto-tile
    /// detector tasks capture it at spawn time and bail after their (long)
    /// AI call if it changed, so a call that outlives its session can't
    /// spawn a stray tile or bill its cost onto the next session.
    pub session_gen: u64,

    /// V0.8.0 (Поток A) — unix-ms of the last AI-error notice tile spawned by
    /// the auto-tile path. Debounce: during a sustained AI outage the detector
    /// fires once per transcript line, so without this the user would get one
    /// error tile per line. We spawn at most one error tile per
    /// `AI_ERROR_TILE_DEBOUNCE_MS`. Zero = none spawned yet.
    pub last_ai_error_tile_ms: u64,
}

/// Convenience alias matching src-tauri's `SharedRuntime`.
pub type SharedSlintRuntime = Arc<Mutex<SlintRuntime>>;

/// Construct a fresh, empty `SharedSlintRuntime`.
#[must_use]
pub fn shared_runtime() -> SharedSlintRuntime {
    Arc::new(Mutex::new(SlintRuntime::default()))
}

/// Convenience: lock + return MutexGuard, unwrapping poison errors
/// the same way the existing AppState code does. The Slint binary's
/// pattern (see `overlay_host.rs::on_mic_toggle_clicked`) treats a
/// poisoned mutex as "thread died; salvage the data" rather than
/// propagating up to the UI thread.
pub fn lock(rt: &SharedSlintRuntime) -> std::sync::MutexGuard<'_, SlintRuntime> {
    match rt.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Rolling transcript cap — matches src-tauri's value to keep
/// reask/auto-detector behavior identical across binaries.
pub const TRANSCRIPT_MAX_LINES: usize = 80;

/// Push a transcript line, evicting oldest if the cap is reached.
/// Caller already holds the rt lock.
pub fn push_transcript_line(rt: &mut SlintRuntime, line: TranscriptLine) {
    rt.transcript.push_back(line);
    while rt.transcript.len() > TRANSCRIPT_MAX_LINES {
        rt.transcript.pop_front();
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests need .expect() for invariant-violation messages on Option/Result; runtime code stays strict"
)]
mod tests {
    use super::*;
    use overlay_backend::audio::AudioSource;

    #[test]
    fn shared_runtime_starts_empty() {
        let rt = shared_runtime();
        let s = lock(&rt);
        assert!(s.transcript.is_empty());
        assert!(s.last_question.is_none());
        assert!(s.last_answer.is_none());
        assert_eq!(s.session_cost_microcents, 0);
        assert!(s.journal.is_none());
        assert!(s.capture.is_none());
        assert!(!s.mic_muted);
    }

    #[test]
    fn push_transcript_line_caps_at_max() {
        let rt = shared_runtime();
        let mut s = lock(&rt);
        for i in 0..(TRANSCRIPT_MAX_LINES + 5) {
            push_transcript_line(
                &mut s,
                TranscriptLine {
                    source: AudioSource::Mic,
                    text: format!("line {i}"),
                    timestamp_ms: i as u64,
                },
            );
        }
        assert_eq!(s.transcript.len(), TRANSCRIPT_MAX_LINES);
        // Newest 80 lines survive — first surviving line should be
        // "line 5" (the first 5 were evicted).
        let first = &s.transcript.front().expect("non-empty").text;
        assert_eq!(first, "line 5");
        let last = &s.transcript.back().expect("non-empty").text;
        assert_eq!(last, &format!("line {}", TRANSCRIPT_MAX_LINES + 4));
    }
}
