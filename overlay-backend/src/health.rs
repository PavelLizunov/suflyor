//! Subsystem health signals — shared between stt/audio/ai modules
//! and the runtime that emits `health:update` events to the UI.
//!
//! Extracted from `src-tauri/src/runtime.rs` lines 110-178 as part
//! of Phase B1 (overlay-backend extraction). The struct + its
//! `snapshot()` method are pure Rust with zero Tauri dependencies.

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
        let stt_age = read(&self.last_stt_ok_ms);
        let ai_age = read(&self.last_ai_ok_ms);
        HealthPayload {
            audio: Self::classify(audio_age, 15_000, 60_000),
            stt: Self::classify(stt_age, 60_000, 180_000),
            ai: Self::classify(ai_age, 180_000, 600_000),
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
}
