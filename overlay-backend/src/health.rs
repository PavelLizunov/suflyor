//! Subsystem health signals — shared between stt/audio/ai modules
//! and the runtime that emits `health:update` events to the UI.
//!
//! Extracted from `src-tauri/src/runtime.rs` as part of Phase B1
//! (overlay-backend extraction). The struct + its `snapshot()`
//! method are pure Rust with zero Tauri dependencies.

use std::sync::atomic::{AtomicU64, Ordering};

/// Health-tracking atomic counters bumped by audio/stt/ai pipelines.
/// Each value is the unix-ms timestamp of the last successful event;
/// Zero = never yet ok in this session.
#[derive(Debug, Default)]
pub struct HealthSignals {
    /// Bumped each time an audio frame arrives from the WASAPI thread.
    /// Stale (>15s) → audio device / loopback issue.
    pub last_audio_frame_ms: AtomicU64,
    /// Bumped on each successful Groq Whisper transcription.
    /// Stale (>60s) → Groq rate-limit / network / VPN issue.
    pub last_stt_ok_ms: AtomicU64,
    /// Bumped on each successful AI streaming completion OR
    /// non-streaming response.
    /// Stale (>180s) → AI proxy / model issue (or simply no recent ask).
    pub last_ai_ok_ms: AtomicU64,
    /// V0.8.0 (Поток A) — bumped when an AI call FAILS (timeout / refused /
    /// error). Lets `snapshot` report `ai="down"` IMMEDIATELY on a fresh
    /// failure instead of waiting for the 600s staleness threshold — the user
    /// must see "AI down" right away (they reported auto-tiles silently
    /// stopping). Auto-clears: a later success bumps `last_ai_ok_ms` past this,
    /// so the next snapshot returns to "ok". Zero = no failure this session.
    pub last_ai_err_ms: AtomicU64,
    /// Suflyor E2 — bumped on each MIC chunk arriving from capture (BEFORE the
    /// mute/pause gates in the Slint forwarder — a muted or paused mic is
    /// still alive). Zero = idle (between sessions). Seeded with the session
    /// start time so a mic that fails to open/capture ages through the same
    /// 15s/60s thresholds instead of hiding behind the system loopback, which
    /// keeps the shared `last_audio_frame_ms` fresh on its own.
    pub last_mic_frame_ms: AtomicU64,
}

/// Snapshot emitted on the `health:update` event every 2s while a
/// session is active. Frontend converts ages to color states (green/
/// yellow/red) and renders 3 dots in the overlay bar.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthPayload {
    /// "ok" | "degraded" | "down" | "idle"
    pub audio: &'static str,
    pub stt: &'static str,
    pub ai: &'static str,
    /// Suflyor E2 — per-source microphone state, folded into `audio` (the
    /// shared signal can stay green off system-loopback frames alone while
    /// the mic is dead). Same 4 labels; "idle" only between sessions.
    pub mic: &'static str,
    /// Milliseconds since each subsystem's last success. None = never yet.
    pub audio_age_ms: Option<u64>,
    pub stt_age_ms: Option<u64>,
    pub ai_age_ms: Option<u64>,
}

impl HealthSignals {
    /// Classify a signal's age into a 4-state health label.
    fn classify(age_ms: Option<u64>, degraded: u64, down: u64) -> &'static str {
        match age_ms {
            None => "idle",
            Some(a) if a < degraded => "ok",
            Some(a) if a < down => "degraded",
            Some(_) => "down",
        }
    }

    /// Severity order of the health labels (idle < ok < degraded < down).
    fn rank(state: &str) -> u8 {
        match state {
            "ok" => 1,
            "degraded" => 2,
            "down" => 3,
            _ => 0, // "idle"
        }
    }

    /// The more severe of two labels — Suflyor E2's fold of the per-source
    /// mic state into the shared `audio` label.
    fn worst(a: &'static str, b: &'static str) -> &'static str {
        if Self::rank(a) >= Self::rank(b) {
            a
        } else {
            b
        }
    }

    #[must_use]
    pub fn snapshot(&self, now_ms: u64) -> HealthPayload {
        let read = |a: &AtomicU64| -> Option<u64> {
            let v = a.load(Ordering::Relaxed);
            if v == 0 {
                None
            } else {
                Some(now_ms.saturating_sub(v))
            }
        };
        let audio_age = read(&self.last_audio_frame_ms);
        let mic_age = read(&self.last_mic_frame_ms);
        let stt_age = read(&self.last_stt_ok_ms);
        let ai_age = read(&self.last_ai_ok_ms);
        // V0.8.0 (Поток A) — AI is "down" IMMEDIATELY when the most recent AI
        // event was a FAILURE (err timestamp newer than the last ok), regardless
        // of the 600s staleness threshold. Otherwise fall back to age-based
        // classification (also covers "no recent ask" = idle/degraded). This is
        // why the bar can flip to "AI down" within one 2s health tick instead of
        // 10 minutes — the user reported auto-tiles silently stopping.
        let ai_ok_raw = self.last_ai_ok_ms.load(Ordering::Relaxed);
        let ai_err_raw = self.last_ai_err_ms.load(Ordering::Relaxed);
        let ai = if ai_err_raw != 0 && ai_err_raw >= ai_ok_raw {
            "down"
        } else {
            Self::classify(ai_age, 180_000, 600_000)
        };
        // Suflyor E2 — mic/system share `last_audio_frame_ms`, so a dead mic
        // stayed green while the system loopback kept bumping the shared
        // signal. Classify the per-source mic clock on the SAME thresholds
        // and fold the worse of the two into `audio`, so mic failure/staleness
        // can never be reported healthy. System-loopback semantics are
        // untouched: system silence alone never degrades `audio` (a live mic
        // keeps the shared signal fresh, exactly as before).
        let mic = Self::classify(mic_age, 15_000, 60_000);
        let audio = Self::worst(Self::classify(audio_age, 15_000, 60_000), mic);
        HealthPayload {
            audio,
            stt: Self::classify(stt_age, 60_000, 180_000),
            ai,
            mic,
            audio_age_ms: audio_age,
            stt_age_ms: stt_age,
            ai_age_ms: ai_age,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_thresholds() {
        assert_eq!(HealthSignals::classify(None, 1000, 5000), "idle");
        assert_eq!(HealthSignals::classify(Some(0), 1000, 5000), "ok");
        assert_eq!(HealthSignals::classify(Some(999), 1000, 5000), "ok");
        assert_eq!(HealthSignals::classify(Some(1000), 1000, 5000), "degraded");
        assert_eq!(HealthSignals::classify(Some(4999), 1000, 5000), "degraded");
        assert_eq!(HealthSignals::classify(Some(5000), 1000, 5000), "down");
        assert_eq!(HealthSignals::classify(Some(999_999), 1000, 5000), "down");
    }

    // V0.8.0 (Поток A) — a fresh AI failure flips `ai` to "down" immediately
    // (not after the 600s stale threshold), and a later success auto-clears it.
    #[test]
    fn ai_error_marks_down_immediately_then_clears_on_success() {
        let h = HealthSignals::default();
        let now = 1_000_000u64;

        // A recent SUCCESS → "ok" (well under 180s).
        h.last_ai_ok_ms.store(now - 1_000, Ordering::Relaxed);
        assert_eq!(h.snapshot(now).ai, "ok");

        // A FAILURE newer than the last success → "down" right away, even though
        // the last *success* is only 1s old (would classify "ok" by age alone).
        h.last_ai_err_ms.store(now - 500, Ordering::Relaxed);
        assert_eq!(h.snapshot(now).ai, "down");

        // A newer SUCCESS supersedes the error → back to "ok" (auto-clear).
        h.last_ai_ok_ms.store(now, Ordering::Relaxed);
        assert_eq!(h.snapshot(now).ai, "ok");
    }

    #[test]
    fn ai_idle_without_error_is_not_down() {
        let h = HealthSignals::default();
        // No ok, no err → genuinely idle (never asked), NOT a false "down".
        assert_eq!(h.snapshot(1_000_000).ai, "idle");
    }

    // Suflyor E2 — a mic that never delivers a frame (open/capture failed)
    // must NOT hide behind the system loopback keeping the shared audio
    // signal fresh. The mic clock is seeded with the session start time, so
    // it ages through the standard 15s/60s thresholds on its own.
    #[test]
    fn mic_failure_cannot_hide_behind_healthy_system_audio() {
        let h = HealthSignals::default();
        let start = 1_000_000u64;
        h.last_mic_frame_ms.store(start, Ordering::Relaxed);

        // 10s in: system frames keep the shared signal fresh, mic seeded at
        // start is still inside the 15s threshold → all green.
        let now = start + 10_000;
        h.last_audio_frame_ms.store(now - 200, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.mic, "ok");
        assert_eq!(snap.audio, "ok");

        // 16s in: the never-refreshed mic clock crosses 15s → audio can no
        // longer be reported healthy, even though system audio flows fine.
        let now = start + 16_000;
        h.last_audio_frame_ms.store(now - 200, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.mic, "degraded");
        assert_eq!(snap.audio, "degraded");

        // 61s in: mic down; shared signal still fresh off the loopback.
        let now = start + 61_000;
        h.last_audio_frame_ms.store(now - 200, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.mic, "down");
        assert_eq!(snap.audio, "down");

        // Mic recovers → audio green again on the very next snapshot.
        h.last_mic_frame_ms.store(now - 200, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.mic, "ok");
        assert_eq!(snap.audio, "ok");
    }

    // Suflyor E2 — preserve system-loopback semantics: system silence is
    // NORMAL (nothing is playing); a live mic keeps the shared signal fresh,
    // so audio stays "ok" with no system frames of their own.
    #[test]
    fn system_silence_with_live_mic_stays_ok() {
        let h = HealthSignals::default();
        let now = 1_000_000u64;
        h.last_mic_frame_ms.store(now - 200, Ordering::Relaxed);
        // Shared signal bumped by the mic chunks themselves.
        h.last_audio_frame_ms.store(now - 200, Ordering::Relaxed);
        let snap = h.snapshot(now);
        assert_eq!(snap.mic, "ok");
        assert_eq!(snap.audio, "ok");
    }

    #[test]
    fn mic_idle_between_sessions() {
        let h = HealthSignals::default();
        // Mic clock zero (never seeded / reset on stop) → idle, and idle
        // never drags the shared label past its own state.
        let snap = h.snapshot(1_000_000);
        assert_eq!(snap.mic, "idle");
        assert_eq!(snap.audio, "idle");
    }
}
