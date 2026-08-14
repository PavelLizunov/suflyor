//! Load + downmix a session's saved recordings for in-app playback (ТЗ2b).
//!
//! The recorder writes two per-channel WAVs (`mic.wav` + `system.wav`, 16 kHz
//! mono i16), each appended contiguously from its FIRST captured chunk — so they
//! are APPROXIMATELY (not sample-perfectly) co-aligned: a sub-second per-channel
//! skew is possible (independent WASAPI device bring-up; no shared epoch is
//! stamped). The player needs ONE stream seeked to a clicked line's timecode
//! (decision: mix on the fly, no channel toggle), so we sum the channels into a
//! single buffer here — pure + testable; the player feeds the result to the audio
//! engine. Same sample rate ⇒ sample index ≈ time (within that sub-second skew),
//! so no resampling is needed. True sample-accuracy (a shared capture epoch +
//! per-channel WAV padding) is deferred — see F1 notes / line_start_offset_ms.

use crate::audio::AudioSource;
use crate::recorder::TtsMaskSpan;
use anyhow::{bail, Result};
use std::path::Path;

/// Sum two i16 PCM channels into one (clamped to i16), padding the shorter with
/// silence so the mix is as long as the longer channel.
#[must_use]
pub fn mix_pcm(a: &[i16], b: &[i16]) -> Vec<i16> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s =
            i32::from(a.get(i).copied().unwrap_or(0)) + i32::from(b.get(i).copied().unwrap_or(0));
        out.push(s.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16);
    }
    out
}

/// Silence the union of one source's app-TTS intervals overlapping this PCM
/// window. Duration is kept, so transcript timecodes do not move.
pub fn silence_tts_spans(
    pcm: &mut [i16],
    spans: &[TtsMaskSpan],
    source: AudioSource,
    window_start: u64,
) -> usize {
    let window_end = window_start.saturating_add(pcm.len() as u64);
    let mut ranges: Vec<(usize, usize)> = spans
        .iter()
        .filter(|span| span.source == source)
        .filter_map(|span| {
            let start = span.start_sample.max(window_start);
            let end = span.end_sample.min(window_end);
            (start < end).then_some((
                (start - window_start) as usize,
                (end - window_start) as usize,
            ))
        })
        .collect();
    ranges.sort_unstable();
    let mut masked = 0;
    let mut cursor = 0;
    for (start, end) in ranges {
        let start = start.max(cursor);
        if start >= end {
            continue;
        }
        pcm[start..end].fill(0);
        masked += end - start;
        cursor = end;
    }
    masked
}

/// Read one 16-bit WAV into PCM + its sample rate; `None` if absent/unreadable.
/// Lenient on a torn trailing sample (playback can tolerate one dropped frame).
fn read_wav_i16(path: &Path) -> Option<(Vec<i16>, u32)> {
    let reader = hound::WavReader::open(path).ok()?;
    let sr = reader.spec().sample_rate;
    let samples: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .collect();
    Some((samples, sr))
}

/// Mixed playback PCM for a session's recordings dir (`<dir>/mic.wav` +
/// `<dir>/system.wav`). Either channel may be absent (→ silence); at least one
/// must exist. Returns (mixed i16 PCM, sample_rate). Test seam for
/// [`load_mixed_session_audio`].
///
/// # Errors
/// When neither channel WAV is present/readable.
pub fn load_mixed_from_dir(session_dir: &Path) -> Result<(Vec<i16>, u32)> {
    let mic = read_wav_i16(&session_dir.join("mic.wav"));
    let sys = read_wav_i16(&session_dir.join("system.wav"));
    let Some(sample_rate) = mic.as_ref().or(sys.as_ref()).map(|(_, r)| *r) else {
        bail!("no recordings to play in {}", session_dir.display());
    };
    let mut mic_pcm = mic.map(|(s, _)| s).unwrap_or_default();
    let mut sys_pcm = sys.map(|(s, _)| s).unwrap_or_default();
    match crate::recorder::load_tts_mask_in(session_dir) {
        Ok(spans) if sample_rate == crate::recorder::SAMPLE_RATE => {
            silence_tts_spans(&mut mic_pcm, &spans, AudioSource::Mic, 0);
            silence_tts_spans(&mut sys_pcm, &spans, AudioSource::System, 0);
        }
        Ok(spans) if !spans.is_empty() => {
            log::warn!("session audio: ignored TTS mask for unexpected sample rate {sample_rate}");
        }
        Ok(_) => {}
        Err(error) => {
            log::warn!("session audio: ignored malformed TTS mask: {error:#}");
        }
    }
    Ok((mix_pcm(&mic_pcm, &sys_pcm), sample_rate))
}

/// True if the session has at least one saved recording channel — a CHEAP fs
/// check (two `exists()` stats, no audio load), used to show/hide the player UI
/// vs. the "Аудио не сохранено" note (ТЗ2b). Empty id → false.
#[must_use]
pub fn session_has_recordings(session_id: &str) -> bool {
    if session_id.is_empty() {
        return false;
    }
    let Ok(dir) = crate::recorder::recordings_dir() else {
        return false;
    };
    let d = dir.join(session_id);
    d.join("mic.wav").exists() || d.join("system.wav").exists()
}

/// Mixed playback PCM for an archived session by id (its `recordings/<id>/`).
///
/// # Errors
/// When the recordings dir can't be resolved or neither channel exists.
pub fn load_mixed_session_audio(session_id: &str) -> Result<(Vec<i16>, u32)> {
    let dir = crate::recorder::recordings_dir()?.join(session_id);
    load_mixed_from_dir(&dir)
}

/// Duration (ms) of a recordings dir's `system.wav`, read from the WAV HEADER
/// (cheap — no sample decode). Diarization aligns speaker segments to system.wav's
/// timeline, so comparing this against the transcript's wall-clock span tells a
/// wall-clock-padded recording (post-D0.5) from a legacy one that ran short by its
/// idle gaps. `None` when the file is absent/unreadable. Test seam for
/// [`system_recording_ms`].
#[must_use]
pub fn system_recording_ms_in_dir(session_dir: &Path) -> Option<i64> {
    let reader = hound::WavReader::open(session_dir.join("system.wav")).ok()?;
    let sr = reader.spec().sample_rate;
    Some(ms_for_sample(reader.duration() as usize, sr))
}

/// [`system_recording_ms_in_dir`] for an archived session by id.
#[must_use]
pub fn system_recording_ms(session_id: &str) -> Option<i64> {
    let dir = crate::recorder::recordings_dir().ok()?.join(session_id);
    system_recording_ms_in_dir(&dir)
}

/// Sample index for a session-relative offset (ms) at `sample_rate`, clamped to
/// `[0, total]` so a click past the end seeks to the end, never out of bounds
/// (ТЗ2b: "таймкод за пределами — clamp"). Sample index ≈ time here — one sample
/// rate; channels co-align only to ~sub-second (see the module header).
#[must_use]
pub fn sample_for_ms(ms: i64, sample_rate: u32, total: usize) -> usize {
    if ms <= 0 {
        return 0;
    }
    let s = (i128::from(ms) * i128::from(sample_rate) / 1000) as usize;
    s.min(total)
}

/// Session-relative offset (ms) for a sample index at `sample_rate` (0 → 0). The
/// inverse of [`sample_for_ms`]; drives the player's current-time readout.
#[must_use]
pub fn ms_for_sample(sample: usize, sample_rate: u32) -> i64 {
    if sample_rate == 0 {
        return 0;
    }
    (sample as i128 * 1000 / i128::from(sample_rate)) as i64
}

/// Session-relative START offset (ms) of utterance `i`. PREFERS the persisted
/// per-line `audio_ms` (ms from audio-capture start, on sessions recorded after
/// the audio_ms migration) — this removes the STT-FINALIZE lag that made the old
/// wall-clock basis SECONDS late. NOT sample-perfect: a sub-second residual remains
/// (chunk granularity ~200ms + per-channel capture skew — the recorder appends from
/// its first chunk, not a shared epoch), so it's "much closer", within the
/// «доли секунды» tolerance; true sample-accuracy would need a recording-epoch +
/// WAV-padding change to the live capture pipeline (deferred — see F1 notes).
/// When `audio_ms` is absent (old sessions), falls back to a wall-clock
/// approximation: utterances are stamped at FINALIZE (≈ the line's END), so a
/// line's START ≈ the PREVIOUS line's timestamp and the first line = the session
/// origin. Returns `None` only when NEITHER is available (no audio_ms AND no usable
/// session start) → the caller shows no timecode and seeks to 0. `utts` is the full
/// chronological slice; `i` indexes it.
#[must_use]
pub fn line_start_offset_ms(
    utts: &[crate::persistence::Utterance],
    i: usize,
    session_start_ms: Option<i64>,
) -> Option<i64> {
    // Accurate: the stored audio offset for this line.
    if let Some(Some(a)) = utts.get(i).map(|u| u.audio_ms) {
        return Some(a.max(0));
    }
    // Fallback (old sessions, no audio_ms): prev-line finalize wall-clock; first
    // line = the session origin.
    let origin = session_start_ms.filter(|&ms| ms > 0)?;
    match i {
        0 => Some(0),
        _ => utts.get(i - 1).map(|p| (p.unix_ms - origin).max(0)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn line_start_offset_prefers_audio_ms_else_prev_timestamp() {
        use crate::persistence::Utterance;
        let mk = |ms: i64, audio: Option<i64>| Utterance {
            session_id: "s".into(),
            unix_ms: ms,
            source: "system".into(),
            text: String::new(),
            audio_ms: audio,
        };
        // ACCURATE path: audio_ms present → returned directly (clamped ≥0),
        // regardless of unix_ms / session start.
        let rec = vec![
            mk(30_000, Some(0)),
            mk(135_000, Some(4_200)),
            mk(140_000, Some(9_900)),
        ];
        assert_eq!(line_start_offset_ms(&rec, 0, Some(1_000)), Some(0));
        assert_eq!(line_start_offset_ms(&rec, 1, Some(1_000)), Some(4_200));
        assert_eq!(line_start_offset_ms(&rec, 2, Some(1_000)), Some(9_900));

        // FALLBACK (old sessions, audio_ms None): prev-line finalize − origin;
        // first line = origin (00:00), NOT its own 29s finalize.
        let old = vec![mk(30_000, None), mk(135_000, None), mk(140_000, None)];
        let start = Some(1_000);
        assert_eq!(line_start_offset_ms(&old, 0, start), Some(0));
        assert_eq!(line_start_offset_ms(&old, 1, start), Some(29_000));
        assert_eq!(line_start_offset_ms(&old, 2, start), Some(134_000));
        // Neither audio_ms nor a usable origin → None (no timecode, seek 0).
        assert_eq!(line_start_offset_ms(&old, 1, None), None);
        assert_eq!(line_start_offset_ms(&old, 1, Some(0)), None);
    }

    #[test]
    fn sample_ms_mapping_roundtrips_and_clamps() {
        let sr = 16_000;
        assert_eq!(sample_for_ms(0, sr, 100_000), 0);
        assert_eq!(sample_for_ms(-5, sr, 100_000), 0); // negative offset → start
        assert_eq!(sample_for_ms(1000, sr, 100_000), 16_000); // 1s = sr samples
        assert_eq!(sample_for_ms(1000, sr, 8_000), 8_000); // past end → clamp to total
        assert_eq!(ms_for_sample(16_000, sr), 1000);
        assert_eq!(ms_for_sample(0, sr), 0);
        assert_eq!(ms_for_sample(8_000, 0), 0); // sample_rate guard
        for &ms in &[0_i64, 250, 1500, 37_000] {
            // round-trips for offsets that divide evenly at 16 kHz
            let s = sample_for_ms(ms, sr, usize::MAX);
            assert_eq!(ms_for_sample(s, sr), ms);
        }
    }

    #[test]
    fn mix_clamps_and_pads() {
        assert_eq!(mix_pcm(&[100, 200], &[100, 200]), vec![200, 400]);
        assert_eq!(mix_pcm(&[30000], &[30000]), vec![i16::MAX]); // clamp +
        assert_eq!(mix_pcm(&[-30000], &[-30000]), vec![i16::MIN]); // clamp -
        assert_eq!(mix_pcm(&[100], &[100, 50, 25]), vec![200, 50, 25]); // pad
        assert_eq!(mix_pcm(&[], &[]), Vec::<i16>::new());
    }

    fn write_wav(path: &Path, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for &s in samples {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn load_mixed_sums_channels_and_errors_when_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        assert!(load_mixed_from_dir(dir).is_err()); // neither channel → Err
        write_wav(&dir.join("mic.wav"), &[100, 200]);
        write_wav(&dir.join("system.wav"), &[10, 20, 30]);
        let (pcm, sr) = load_mixed_from_dir(dir).unwrap();
        assert_eq!(sr, 16_000);
        assert_eq!(pcm, vec![110, 220, 30]); // summed + padded
    }

    #[test]
    fn system_recording_ms_reads_header_duration() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        assert_eq!(system_recording_ms_in_dir(dir), None); // absent → None
        write_wav(&dir.join("system.wav"), &vec![0i16; 32_000]); // 2s @ 16 kHz
        assert_eq!(system_recording_ms_in_dir(dir), Some(2000));
    }

    #[test]
    fn load_mixed_one_channel_only() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write_wav(&dir.join("mic.wav"), &[5, 6, 7]);
        let (pcm, _) = load_mixed_from_dir(dir).unwrap();
        assert_eq!(pcm, vec![5, 6, 7]); // system silent → mic passes through
    }

    #[test]
    fn tts_mask_clamps_merges_and_filters_sources() {
        let spans = vec![
            TtsMaskSpan {
                source: AudioSource::Mic,
                start_sample: 2,
                end_sample: 6,
            },
            TtsMaskSpan {
                source: AudioSource::Mic,
                start_sample: 4,
                end_sample: 8,
            },
            TtsMaskSpan {
                source: AudioSource::System,
                start_sample: 8,
                end_sample: 99,
            },
        ];
        let mut mic = vec![1; 10];
        assert_eq!(silence_tts_spans(&mut mic, &spans, AudioSource::Mic, 0), 6);
        assert_eq!(mic, vec![1, 1, 0, 0, 0, 0, 0, 0, 1, 1]);

        let mut window = vec![1; 10];
        let cross_window = [TtsMaskSpan {
            source: AudioSource::System,
            start_sample: 95,
            end_sample: 105,
        }];
        assert_eq!(
            silence_tts_spans(&mut window, &cross_window, AudioSource::System, 100),
            5
        );
        assert_eq!(window, vec![0, 0, 0, 0, 0, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn playback_masks_tts_without_changing_timeline_and_malformed_fails_open() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write_wav(&dir.join("mic.wav"), &[10, 10, 10, 10, 10]);
        write_wav(&dir.join("system.wav"), &[1, 2, 3, 4, 5]);
        std::fs::write(
            dir.join(crate::recorder::TTS_MASK_FILE),
            "{\"source\":\"system\",\"start_sample\":1,\"end_sample\":4}\n",
        )
        .unwrap();
        let (masked, _) = load_mixed_from_dir(dir).unwrap();
        assert_eq!(masked, vec![11, 10, 10, 10, 15]);
        assert_eq!(masked.len(), 5, "masking must preserve timecodes");

        std::fs::write(dir.join(crate::recorder::TTS_MASK_FILE), "truncated").unwrap();
        let (raw, _) = load_mixed_from_dir(dir).unwrap();
        assert_eq!(raw, vec![11, 12, 13, 14, 15]);
    }
}
