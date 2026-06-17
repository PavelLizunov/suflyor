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
//!
//! ## Chunked streaming (v0.17.0 — план B, 7-8 h workdays)
//!
//! The v0.14 implementation loaded a channel's WHOLE WAV into RAM and ran ONE
//! STT inference over it, behind a hard 2-hour guard. That blocked exactly the
//! sessions the tester records (7-8+ h) and would have cost GigaAM a ~1.8 GB
//! f32 peak at 8 h. Now each channel is STREAMED from disk in per-backend
//! windows ([`chunk_secs`]) and transcribed chunk by chunk — peak RAM is one
//! chunk, the 2 h guard is replaced by a 24 h sanity ceiling, and the archive
//! header shows per-chunk progress. Fixed windows can split a word at a chunk
//! boundary (a rare, single-word artefact); silence-aware splitting is a
//! possible refinement, embeddings-era.

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

/// Full-load guard for the [`load_wav_pcm`] helper ONLY (tests/tools): 2 h of
/// 16 kHz mono i16 ≈ 230 MB of PCM. The production re-transcribe path streams
/// in chunks and is bounded by [`MAX_REASONABLE_SECS`] instead.
const MAX_CHANNEL_SAMPLES: usize = 16_000 * 60 * 120;

/// Sanity ceiling for the streamed path: refuse a channel claiming more than
/// 24 h of audio — that is not a workday recording, it is a corrupt header or
/// a runaway file (v0.17.0: replaces the old 2 h full-load guard).
const MAX_REASONABLE_SECS: u64 = 24 * 60 * 60;

/// Offline STT window per backend, in seconds. Peak RAM per channel = ONE
/// window (i16 + the backend's own transient copies), not the whole file:
/// - Cloud (Groq): 600 s → ~19 MB WAV per request — under the API's 25 MB
///   file cap; an 8 h channel becomes ~48 requests instead of one giant body.
/// - Local whisper server: 300 s — a moderate request for a local HTTP hop.
/// - GigaAM (in-process ONNX): 60 s — bounds the f32 conversion (~3.8 MB per
///   window vs ~1.8 GB for a full 8 h channel) and stays near the attention
///   lengths it was trained on (the live pipeline feeds it ≤25 s utterances).
fn chunk_secs(backend: &SttBackendCfg) -> u64 {
    match backend {
        SttBackendCfg::Cloud { .. } => 600,
        SttBackendCfg::Whisper { .. } => 300,
        SttBackendCfg::Gigaam { .. } => 60,
    }
}

/// Bail unless `spec` is the canonical 1 ch / 16 kHz / 16-bit format the
/// recorder writes.
fn validate_wav_spec(spec: &hound::WavSpec) -> Result<()> {
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
    Ok(())
}

/// Pull up to `max` samples from a WAV sample iterator into a fresh buffer.
/// An empty result means end-of-file. Split out of the chunk loop so the
/// windowing is unit-testable without an STT backend.
fn read_next_chunk<R: std::io::Read>(
    samples: &mut hound::WavIntoSamples<R, i16>,
    max: usize,
) -> Result<Vec<i16>> {
    // Cap the PRE-allocation at 2M samples (4 MB): a Cloud window is 9.6M
    // samples and pre-allocating 19 MB for a file that turns out to be 10 s
    // long is waste; Vec growth amortises the big-window case just fine.
    let mut buf = Vec::with_capacity(max.min(1 << 21));
    for s in samples.by_ref().take(max) {
        buf.push(s.context("read WAV samples")?);
    }
    Ok(buf)
}

/// Append a transcribed chunk to the channel text with a single-space joint.
fn append_chunk_text(acc: &mut String, piece: &str) {
    let piece = piece.trim();
    if piece.is_empty() {
        return;
    }
    if !acc.is_empty() {
        acc.push(' ');
    }
    acc.push_str(piece);
}

/// Read a saved 16 kHz mono i16 WAV back into PCM samples — FULL-LOAD helper
/// for tests/tools. The production path streams via [`read_next_chunk`].
///
/// # Errors
/// Returns Err if the file can't be opened, isn't the canonical 1 ch / 16 kHz /
/// 16-bit format the recorder writes, exceeds [`MAX_CHANNEL_SAMPLES`], or a
/// sample can't be decoded.
pub fn load_wav_pcm(path: &Path) -> Result<Vec<i16>> {
    let reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    validate_wav_spec(&reader.spec())?;
    if (reader.len() as usize) > MAX_CHANNEL_SAMPLES {
        bail!("recording too long for a full in-memory load (over 2 hours)");
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
        // v0.17.0 — STREAM the channel in per-backend windows instead of one
        // full-file load + one giant inference (план B: 7-8 h recordings).
        let reader =
            hound::WavReader::open(&path).with_context(|| format!("open {}", path.display()))?;
        validate_wav_spec(&reader.spec()).with_context(|| format!("load {file}"))?;
        let total_samples = u64::from(reader.len());
        if total_samples == 0 {
            continue;
        }
        if total_samples > MAX_REASONABLE_SECS * u64::from(recorder::SAMPLE_RATE) {
            bail!("recording too long to re-transcribe (over 24 hours)");
        }
        let chunk = usize::try_from(chunk_secs(&backend) * u64::from(recorder::SAMPLE_RATE))
            .unwrap_or(usize::MAX);
        let n_chunks = usize::try_from(total_samples.div_ceil(chunk as u64)).unwrap_or(1);
        log::info!(
            "re-transcribe {file}: {total_samples} samples → {n_chunks} chunk(s) × {}s",
            chunk_secs(&backend)
        );
        let mut samples = reader.into_samples::<i16>();
        let mut text = String::new();
        for part in 1..=n_chunks {
            on_progress(Progress::Step(if n_chunks > 1 {
                format!("Re-transcribing {who}… {part}/{n_chunks}")
            } else {
                format!("Re-transcribing {who}…")
            }));
            let buf =
                read_next_chunk(&mut samples, chunk).with_context(|| format!("load {file}"))?;
            if buf.is_empty() {
                break; // end-of-file (header over-claimed)
            }
            let piece = stt::transcribe_once(
                &backend,
                &buf,
                language.as_deref(),
                whisper_prompt.as_deref(),
            )
            .await
            .with_context(|| format!("re-transcribe {file} (part {part}/{n_chunks})"))?;
            append_chunk_text(&mut text, &piece);
        }
        texts[idx] = text;
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
    // Pass the archive session id so the conspect persists under it — a failed
    // reduce can then be retried from the saved parts without re-running STT.
    crate::runtime::run_meeting_summary(events, cfg, transcript, session_id.to_string()).await;
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

    // ── v0.17.0 chunked streaming ──

    #[test]
    fn read_next_chunk_windows_a_wav_without_loss() {
        // 5 samples, windows of 2 → [2, 2, 1, 0] and the content round-trips.
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("mic.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: recorder::SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for s in [1i16, 2, 3, 4, 5] {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        let mut samples = reader.into_samples::<i16>();
        assert_eq!(read_next_chunk(&mut samples, 2).unwrap(), vec![1, 2]);
        assert_eq!(read_next_chunk(&mut samples, 2).unwrap(), vec![3, 4]);
        assert_eq!(read_next_chunk(&mut samples, 2).unwrap(), vec![5]);
        assert!(read_next_chunk(&mut samples, 2).unwrap().is_empty());
    }

    #[test]
    fn chunk_secs_is_per_backend_and_bounded() {
        let cloud = SttBackendCfg::Cloud {
            api_key: "k".into(),
            model: "m".into(),
        };
        let gigaam = SttBackendCfg::Gigaam {
            model_dir: "d".into(),
        };
        // Cloud window must stay under Groq's 25 MB file cap:
        // secs × 16000 Hz × 2 B + 44 B header.
        let cloud_bytes = chunk_secs(&cloud) * u64::from(recorder::SAMPLE_RATE) * 2 + 44;
        assert!(cloud_bytes < 25 * 1024 * 1024, "{cloud_bytes}");
        // GigaAM windows are much smaller (in-process RAM bound).
        assert!(chunk_secs(&gigaam) <= 60);
    }

    #[test]
    fn append_chunk_text_joins_with_single_space_and_skips_blank() {
        let mut acc = String::new();
        append_chunk_text(&mut acc, "  первая часть ");
        append_chunk_text(&mut acc, "");
        append_chunk_text(&mut acc, "   ");
        append_chunk_text(&mut acc, "вторая");
        assert_eq!(acc, "первая часть вторая");
    }
}
