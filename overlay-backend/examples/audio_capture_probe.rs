//! Capture-only macOS diagnostic for the public
//! `overlay_backend::audio::start_capture` seam. Runs for a short bounded
//! duration, counts microphone/system chunks + samples, and prints ONLY safe
//! aggregate evidence (max RMS, max absolute sample, nonzero sample count) —
//! no raw audio, no device names. Never initialises STT, AI, config, journal,
//! recorder, UAP, HTTP, or any external service.
//!
//! Exit status: nonzero only when `start_capture` itself fails, when NEITHER
//! source produces any chunk during the window, or when a stream's probe
//! received chunks exceed its metrics `emitted_chunks` by MORE than one
//! (exactly one chunk may legitimately be in flight between the producer's
//! channel send and its metrics update).
//!
//! Run (on the Mac):
//!   cargo run --manifest-path overlay-backend/Cargo.toml --example audio_capture_probe
//!   cargo run --manifest-path overlay-backend/Cargo.toml --example audio_capture_probe -- 8
//!
//! NOTE: the binary needs the same TCC grants the app needs (Microphone for
//! the mic source, Screen Recording for the system process tap) and, for the
//! system tap's first-run consent, an app bundle with a stable identity —
//! see the report accompanying this probe. Before `start_capture` the probe
//! queries `audio::microphone_permission` and, only while the state is
//! NotDetermined, calls `audio::request_microphone_permission` once, waiting
//! up to 30s on a one-shot channel for the callback. Any non-Authorized
//! outcome (Denied, Restricted, timeout) prints a safe category and the
//! probe continues system-only — mic unavailability is never a failure.

#[cfg(target_os = "macos")]
use overlay_backend::audio::{self, AudioSource, MicrophonePermission};
#[cfg(target_os = "macos")]
use std::sync::mpsc;
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tokio::sync::mpsc::error::TryRecvError;

#[cfg(target_os = "macos")]
const DEFAULT_SECS: u64 = 5;

/// Optional first CLI argument: probe window in seconds (1..=60).
#[cfg(target_os = "macos")]
fn probe_duration() -> Duration {
    match std::env::args().nth(1) {
        None => Duration::from_secs(DEFAULT_SECS),
        Some(raw) => match raw.parse::<u64>() {
            Ok(secs @ 1..=60) => Duration::from_secs(secs),
            _ => {
                eprintln!("usage: audio_capture_probe [seconds 1-60]");
                std::process::exit(2);
            }
        },
    }
}

/// Bounded wait for the asynchronous TCC microphone callback.
#[cfg(target_os = "macos")]
const MIC_PERMISSION_WAIT: Duration = Duration::from_secs(30);

/// Best-effort microphone grant before `start_capture`. Prompts only while
/// TCC is still NotDetermined; Denied/Restricted/callback timeout print a
/// safe category and downgrade the probe to system-only instead of failing.
#[cfg(target_os = "macos")]
fn ensure_microphone_permission() {
    match audio::microphone_permission() {
        MicrophonePermission::Authorized => println!("mic permission: authorized"),
        MicrophonePermission::NotDetermined => {
            println!(
                "mic permission: not determined, requesting (waits up to {}s)",
                MIC_PERMISSION_WAIT.as_secs()
            );
            let (tx, rx) = mpsc::channel();
            audio::request_microphone_permission(move |permission| {
                let _ = tx.send(permission);
            });
            match rx.recv_timeout(MIC_PERMISSION_WAIT) {
                Ok(MicrophonePermission::Authorized) => {
                    println!("mic permission: authorized");
                }
                Ok(permission) => {
                    println!("mic permission: {permission:?}, continuing system-only");
                }
                Err(_) => {
                    println!("mic permission: request timed out, continuing system-only");
                }
            }
        }
        permission => {
            println!("mic permission: {permission:?}, continuing system-only");
        }
    }
}

/// Safe aggregate evidence for one source — no raw samples ever leave it.
#[cfg(target_os = "macos")]
struct SourceStats {
    chunks: u64,
    samples: u64,
    nonzero_samples: u64,
    max_abs: u16,
    /// Highest per-chunk RMS in dBFS; NEG_INFINITY while every chunk silent.
    max_rms_db: f64,
}

#[cfg(target_os = "macos")]
impl SourceStats {
    fn new() -> Self {
        Self {
            chunks: 0,
            samples: 0,
            nonzero_samples: 0,
            max_abs: 0,
            max_rms_db: f64::NEG_INFINITY,
        }
    }

    fn observe(&mut self, pcm: &[i16]) {
        self.chunks += 1;
        self.samples += pcm.len() as u64;
        self.nonzero_samples += pcm.iter().filter(|s| **s != 0).count() as u64;
        for sample in pcm {
            self.max_abs = self.max_abs.max(sample.unsigned_abs());
        }
        self.max_rms_db = self.max_rms_db.max(audio::rms_dbfs(pcm));
    }

    fn report(&self) -> String {
        format!(
            "chunks={} samples={} nonzero={} max_abs={} max_rms={:.1} dBFS",
            self.chunks, self.samples, self.nonzero_samples, self.max_abs, self.max_rms_db
        )
    }
}

#[cfg(target_os = "macos")]
fn main() {
    let duration = probe_duration();
    println!(
        "audio capture probe: {}s window via overlay_backend::audio::start_capture",
        duration.as_secs()
    );

    ensure_microphone_permission();

    let (mut rx, handle) = match audio::start_capture(None, None) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("capture failed to start: {e:#}");
            std::process::exit(1);
        }
    };

    let deadline = Instant::now() + duration;
    let mut mic = SourceStats::new();
    let mut sys = SourceStats::new();
    loop {
        match rx.try_recv() {
            Ok(chunk) => match chunk.source {
                AudioSource::Mic => mic.observe(&chunk.pcm_i16),
                AudioSource::System => sys.observe(&chunk.pcm_i16),
            },
            Err(TryRecvError::Empty) => {
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(TryRecvError::Disconnected) => break,
        }
    }
    // Snapshot while the handle is still alive, then explicit teardown:
    // CaptureHandle::drop stops + joins the workers.
    let metrics = handle.metrics_snapshot();
    drop(handle);

    println!("mic:    {}", mic.report());
    println!("system: {}", sys.report());
    println!(
        "mic metrics:    emitted_chunks={} queue_drops={} ring_overflow_frames={} \
         max_pending_samples={} last_emitted_session_ms={}",
        metrics.mic.emitted_chunks,
        metrics.mic.queue_drops,
        metrics.mic.ring_overflow_frames,
        metrics.mic.max_pending_samples,
        metrics.mic.last_emitted_session_ms
    );
    println!(
        "system metrics: emitted_chunks={} queue_drops={} ring_overflow_frames={} \
         max_pending_samples={} last_emitted_session_ms={}",
        metrics.system.emitted_chunks,
        metrics.system.queue_drops,
        metrics.system.ring_overflow_frames,
        metrics.system.max_pending_samples,
        metrics.system.last_emitted_session_ms
    );

    // Each stream has a single producer worker that try_sends the chunk before
    // recording it in the metrics, so a concurrent snapshot can legitimately
    // see one received chunk not yet counted in emitted_chunks. Metrics may
    // also exceed received counts (a final chunk can still be queued). Fail
    // only when received chunks exceed the emitted metric by MORE than one.
    let mut consistent = true;
    if mic.chunks > metrics.mic.emitted_chunks.saturating_add(1) {
        eprintln!(
            "FAIL: mic probe received {} chunks, more than one past mic metrics \
             emitted_chunks {} (one in-flight chunk is allowed)",
            mic.chunks, metrics.mic.emitted_chunks
        );
        consistent = false;
    }
    if sys.chunks > metrics.system.emitted_chunks.saturating_add(1) {
        eprintln!(
            "FAIL: system probe received {} chunks, more than one past system \
             metrics emitted_chunks {} (one in-flight chunk is allowed)",
            sys.chunks, metrics.system.emitted_chunks
        );
        consistent = false;
    }
    if !consistent {
        std::process::exit(1);
    }

    if mic.chunks + sys.chunks == 0 {
        eprintln!("FAIL: neither source produced any chunk in the window");
        std::process::exit(1);
    }
    println!("OK: at least one source produced chunks");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("audio_capture_probe: this diagnostic exercises the macOS capture seam only; nothing to do on this platform");
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::SourceStats;

    #[test]
    fn observe_accumulates_safe_aggregates_only() {
        let mut stats = SourceStats::new();
        stats.observe(&[0, 0, 0, 0]);
        stats.observe(&[100, -200, 0, i16::MIN]);
        assert_eq!(stats.chunks, 2);
        assert_eq!(stats.samples, 8);
        assert_eq!(stats.nonzero_samples, 3);
        // i16::MIN must not overflow the absolute-amplitude aggregate.
        assert_eq!(stats.max_abs, 32768);
        assert!(stats.max_rms_db < 0.0 && stats.max_rms_db > -40.0);
    }

    #[test]
    fn pure_silence_keeps_negative_infinity_rms() {
        let mut stats = SourceStats::new();
        stats.observe(&[0, 0]);
        assert_eq!(stats.max_rms_db, f64::NEG_INFINITY);
        assert_eq!(stats.nonzero_samples, 0);
    }
}
