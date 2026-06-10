//! Offline re-transcription of a saved session's recordings (v0.14.0).
//!
//! v0.13.0 saves raw per-channel WAVs under `recordings/<session_id>/`. This
//! module re-runs STT over them OFFLINE — unconstrained by the real-time budget,
//! so the transcript is typically better than the live one — and assembles a
//! `Vec<TranscriptLine>` that feeds the SAME [`crate::runtime::run_meeting_summary`]
//! the live Summary button uses.
//!
//! ## Channel assembly (Option A)
//!
//! `mic.wav` → the user's full speech, `system.wav` → the other side's. Without
//! per-segment timestamps we can't interleave turns, so each channel becomes one
//! labelled block (Вы: / Собеседник:). The summary model extracts participants /
//! topics / decisions / tasks from both sides regardless of fine turn order, and
//! preserving the better re-STT text matters more than reconstructing
//! interleaving (a journal-scaffold upgrade could add ordering later).

use crate::audio::{AudioSource, TranscriptLine};
use crate::config::{SharedConfig, SttBackendCfg};
use crate::events::RuntimeEvents;
use crate::{recorder, stt};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::sync::Arc;

/// Coarse progress for the archive UI (mirrors the local_ai installer's Step).
#[derive(Debug, Clone)]
pub enum Progress {
    Step(String),
}

/// Hard memory/length guard: refuse a single channel longer than this. 2 h of
/// 16 kHz mono i16 ≈ 230 MB of PCM — well past any real meeting; anything bigger
/// is almost certainly a corrupt/over-long file we shouldn't load into RAM.
const MAX_CHANNEL_SAMPLES: usize = 16_000 * 60 * 120;

/// Read a saved 16 kHz mono i16 WAV back into PCM samples.
///
/// # Errors
/// Returns Err if the file can't be opened, isn't the canonical 1 ch / 16 kHz /
/// 16-bit format the recorder writes, exceeds [`MAX_CHANNEL_SAMPLES`], or a
/// sample can't be decoded.
pub fn load_wav_pcm(path: &Path) -> Result<Vec<i16>> {
    let reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != recorder::SAMPLE_RATE || spec.bits_per_sample != 16
    {
        bail!(
            "unexpected WAV format ({} ch, {} Hz, {} bit) — expected 1 ch / {} Hz / 16 bit",
            spec.channels,
            spec.sample_rate,
            spec.bits_per_sample,
            recorder::SAMPLE_RATE
        );
    }
    if (reader.len() as usize) > MAX_CHANNEL_SAMPLES {
        bail!("recording too long to re-transcribe (over 2 hours)");
    }
    let samples: std::result::Result<Vec<i16>, _> = reader.into_samples::<i16>().collect();
    samples.context("read WAV samples")
}

/// Assemble the two re-transcribed channels into a transcript. Pure + testable.
/// Empty/whitespace channels are dropped; mic is ordered before system.
#[must_use]
pub fn assemble_lines(mic_text: &str, system_text: &str) -> Vec<TranscriptLine> {
    let mut lines = Vec::new();
    if !mic_text.trim().is_empty() {
        lines.push(TranscriptLine {
            source: AudioSource::Mic,
            text: mic_text.trim().to_string(),
            timestamp_ms: 0,
        });
    }
    if !system_text.trim().is_empty() {
        lines.push(TranscriptLine {
            source: AudioSource::System,
            text: system_text.trim().to_string(),
            timestamp_ms: 1,
        });
    }
    lines
}

/// Re-transcribe a saved session's recordings into a transcript (Option A: one
/// labelled block per channel). Reads cfg ONCE up front. A channel with no saved
/// file (silent that session) is skipped.
///
/// # Errors
/// Returns Err if the recordings dir is missing, both channels are absent/empty,
/// a WAV is malformed, or the STT backend fails.
pub async fn transcribe_session(
    cfg: &SharedConfig,
    session_id: &str,
    on_progress: &impl Fn(Progress),
) -> Result<Vec<TranscriptLine>> {
    let dir = recorder::recordings_dir()?.join(session_id);
    if !dir.exists() {
        bail!("no recordings found for this session");
    }
    // Snapshot the STT config once (provider, language, whisper prompt).
    let (backend, language, whisper_prompt) = {
        let c = cfg.read();
        (
            c.stt_backend(),
            c.stt_language.clone(),
            stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
        )
    };

    // Fail fast (mirror `start_session_inner`'s pre-flight) BEFORE loading any
    // WAV into RAM: a misconfigured backend should error in milliseconds, not
    // after reading up to ~230 MB/channel only to fail inside `transcribe_once`.
    match &backend {
        SttBackendCfg::Cloud { api_key, .. } if api_key.trim().is_empty() => {
            bail!("cloud STT is not configured (empty API key)");
        }
        SttBackendCfg::Gigaam { model_dir } => stt::validate_gigaam_dir(model_dir)?,
        _ => {}
    }

    let mut texts = [String::new(), String::new()]; // [mic, system]
    for (idx, file, who) in [
        (0usize, "mic.wav", "you"),
        (1, "system.wav", "the other side"),
    ] {
        let path = dir.join(file);
        if !path.exists() {
            continue;
        }
        on_progress(Progress::Step(format!("Re-transcribing {who}…")));
        let pcm = load_wav_pcm(&path).with_context(|| format!("load {file}"))?;
        if pcm.is_empty() {
            continue;
        }
        texts[idx] = stt::transcribe_once(
            &backend,
            &pcm,
            language.as_deref(),
            whisper_prompt.as_deref(),
        )
        .await
        .with_context(|| format!("re-transcribe {file}"))?;
    }

    let lines = assemble_lines(&texts[0], &texts[1]);
    if lines.is_empty() {
        bail!("the recordings produced no transcript (silent or unreadable)");
    }
    Ok(lines)
}

/// Full flow: re-transcribe a session's saved recordings, then run the meeting
/// summary over the fresh transcript (spawns a Summary tile via `events`, exactly
/// like the live bar button). Returns the transcript line count for logging.
///
/// # Errors
/// Propagates [`transcribe_session`] errors. `run_meeting_summary` is
/// fire-and-forget (it spawns its own success/error tile), so a summary AI
/// failure does not error here.
pub async fn retranscribe_and_summarize(
    events: Arc<dyn RuntimeEvents>,
    cfg: SharedConfig,
    session_id: &str,
    on_progress: &impl Fn(Progress),
) -> Result<usize> {
    let transcript = transcribe_session(&cfg, session_id, on_progress).await?;
    let n = transcript.len();
    on_progress(Progress::Step("Building summary…".to_string()));
    crate::runtime::run_meeting_summary(events, cfg, transcript).await;
    Ok(n)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn load_wav_round_trips_recorder_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mic.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: recorder::SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for s in [100i16, -100, 200, -200, 32767, -32768] {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
        let pcm = load_wav_pcm(&path).unwrap();
        assert_eq!(pcm, vec![100, -100, 200, -200, 32767, -32768]);
    }

    #[test]
    fn load_wav_rejects_wrong_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("stereo.wav");
        let spec = hound::WavSpec {
            channels: 2, // not mono
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        w.write_sample(0i16).unwrap();
        w.write_sample(0i16).unwrap();
        w.finalize().unwrap();
        assert!(load_wav_pcm(&path).is_err());
    }

    #[test]
    fn assemble_labels_and_orders_channels() {
        let lines = assemble_lines("  привет это я  ", "ответ собеседника");
        assert_eq!(lines.len(), 2);
        assert!(matches!(lines[0].source, AudioSource::Mic));
        assert_eq!(lines[0].text, "привет это я");
        assert!(matches!(lines[1].source, AudioSource::System));
        assert_eq!(lines[1].text, "ответ собеседника");
        assert!(lines[0].timestamp_ms < lines[1].timestamp_ms);
    }

    #[test]
    fn assemble_drops_empty_channels() {
        assert!(assemble_lines("", "   ").is_empty());
        let only_sys = assemble_lines("", "они говорили");
        assert_eq!(only_sys.len(), 1);
        assert!(matches!(only_sys[0].source, AudioSource::System));
    }
}
