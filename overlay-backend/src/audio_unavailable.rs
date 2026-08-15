//! Non-Windows audio seam — honest "no audio backend" replacement for
//! `audio.rs` (WASAPI capture, Windows-only).
//!
//! Mirrors the public `overlay_backend::audio` surface so the rest of the
//! backend compiles unchanged. Shared types and pure helpers (`AudioSource`,
//! `TranscriptLine`, `AudioChunk`, `DeviceList`, `TARGET_SAMPLE_RATE`,
//! `rms_dbfs`) are real; every capture, device-enumeration and recording
//! entry point fails fast with an explicit unsupported error. No OS calls, no
//! fake success, no silent fallback.

use anyhow::Result;
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Generic error for every entry point in this module.
const UNSUPPORTED: &str = "audio capture is not supported on this platform";

/// Target format for downstream STT (identical to the Windows implementation).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    /// What the other party says — system loopback on Windows.
    System,
    /// What you say — microphone endpoint.
    Mic,
}

/// One line in the rolling session transcript (shared type — identical to the
/// Windows implementation).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptLine {
    pub source: AudioSource,
    pub text: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub source: AudioSource,
    /// 16 kHz mono i16 PCM samples.
    pub pcm_i16: Vec<i16>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct DeviceList {
    pub outputs: Vec<String>,
    pub inputs: Vec<String>,
}

/// Enumerate render + capture endpoints — unsupported off Windows.
pub fn list_devices() -> Result<DeviceList> {
    anyhow::bail!(UNSUPPORTED)
}

/// Handle matching the Windows API. Never handed out on this platform
/// (`start_capture` always fails); nothing to stop, so drop is a no-op.
pub struct CaptureHandle {
    _private: (),
}

/// Start capture of system audio + microphone — unsupported off Windows.
pub fn start_capture(
    _mic_device: Option<String>,
    _sys_device: Option<String>,
) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)> {
    anyhow::bail!(UNSUPPORTED)
}

/// Push-to-talk capture until `stop` flips — unsupported off Windows.
pub fn record_source_until_stop(
    _source: AudioSource,
    _mic_device: Option<String>,
    _sys_device: Option<String>,
    _stop: Arc<AtomicBool>,
) -> Result<Vec<i16>> {
    anyhow::bail!(UNSUPPORTED)
}

/// Record the system (loopback) audio for a fixed duration — unsupported
/// off Windows.
pub fn record_sys_blocking(_duration_ms: u64, _sys_device: Option<String>) -> Result<Vec<i16>> {
    anyhow::bail!(UNSUPPORTED)
}

/// RMS energy of 16-bit PCM samples in dBFS (0 = full-scale, −∞ = silence).
/// Pure math — identical to the Windows implementation.
#[must_use]
pub fn rms_dbfs(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return f64::NEG_INFINITY;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|s| {
            let v = f64::from(*s) / 32768.0;
            v * v
        })
        .sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    if rms <= 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * rms.log10()
    }
}

/// Diagnostics system-audio self-test — unsupported off Windows.
pub fn play_tone_and_capture(_sys_device: Option<String>) -> Result<Vec<i16>> {
    anyhow::bail!(UNSUPPORTED)
}

/// Record the microphone for a fixed duration — unsupported off Windows.
pub fn record_mic_blocking(_duration_ms: u64, _mic_device: Option<String>) -> Result<Vec<i16>> {
    anyhow::bail!(UNSUPPORTED)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn entry_points_fail_with_unsupported_error() {
        let errors = [
            list_devices().err(),
            start_capture(None, None).err(),
            record_sys_blocking(10, None).err(),
            record_mic_blocking(10, None).err(),
            play_tone_and_capture(None).err(),
        ];
        assert!(errors.iter().all(Option::is_some));
        for err in errors.into_iter().flatten() {
            assert!(
                err.to_string().contains("not supported"),
                "error must say unsupported, got: {err}"
            );
        }
    }

    #[test]
    fn record_source_until_stop_is_unsupported() {
        let stop = Arc::new(AtomicBool::new(false));
        let err = record_source_until_stop(AudioSource::Mic, None, None, stop).unwrap_err();
        assert!(err.to_string().contains("not supported"));
    }

    #[test]
    fn rms_dbfs_matches_windows_helper() {
        assert_eq!(rms_dbfs(&[]), f64::NEG_INFINITY);
        assert_eq!(rms_dbfs(&[0, 0, 0, 0]), f64::NEG_INFINITY);
        let full = [i16::MAX, i16::MIN, i16::MAX, i16::MIN];
        let d = rms_dbfs(&full);
        assert!(d > -0.5 && d <= 0.0, "full-scale ~0 dBFS, got {d}");
    }
}
