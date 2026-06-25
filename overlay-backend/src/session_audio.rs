//! Load + downmix a session's saved recordings for in-app playback (ТЗ2b).
//!
//! The recorder writes two per-channel WAVs (`mic.wav` + `system.wav`, 16 kHz
//! mono i16, both aligned to session t=0). The player needs ONE stream seeked to
//! a clicked line's timecode (decision: mix on the fly, no channel toggle), so we
//! sum the channels into a single buffer here — pure + testable; the player feeds
//! the result to the audio engine. Same sample rate + shared t=0 ⇒ sample index
//! IS time, so no resampling/alignment is needed.

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
    let mic_pcm = mic.map(|(s, _)| s).unwrap_or_default();
    let sys_pcm = sys.map(|(s, _)| s).unwrap_or_default();
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

/// Sample index for a session-relative offset (ms) at `sample_rate`, clamped to
/// `[0, total]` so a click past the end seeks to the end, never out of bounds
/// (ТЗ2b: "таймкод за пределами — clamp"). Sample index IS time here — both
/// channels share t=0 and one sample rate, so no alignment is needed.
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

/// Session-relative START offset (ms) of utterance `i`, per decision #6. Utterances
/// are timestamped at FINALIZE — `slint_session` writes the `transcript_line` with
/// `now_unix_ms()` when STT returns, i.e. ≈ the line's END — so a line's START is the
/// PREVIOUS line's timestamp, and the first line starts at the session origin.
/// Returns `None` when the session has no usable start (old/crashed) → the caller
/// shows no timecode and seeks to 0. `utts` must be the full chronological slice;
/// `i` indexes into it.
#[must_use]
pub fn line_start_offset_ms(
    utts: &[crate::persistence::Utterance],
    i: usize,
    session_start_ms: Option<i64>,
) -> Option<i64> {
    let origin = session_start_ms.filter(|&ms| ms > 0)?;
    if i == 0 {
        Some(0)
    } else {
        Some((utts[i - 1].unix_ms - origin).max(0))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn line_start_offset_uses_previous_timestamp() {
        use crate::persistence::Utterance;
        let u = |ms: i64| Utterance {
            session_id: "s".into(),
            unix_ms: ms,
            source: "system".into(),
            text: String::new(),
        };
        let utts = vec![u(30_000), u(135_000), u(140_000)];
        let start = Some(1_000);
        // First line → session origin (00:00), NOT its own finalize time (29s).
        assert_eq!(line_start_offset_ms(&utts, 0, start), Some(0));
        // Line i → previous line's finalize time − origin (its start ≈ prev end).
        assert_eq!(line_start_offset_ms(&utts, 1, start), Some(29_000));
        assert_eq!(line_start_offset_ms(&utts, 2, start), Some(134_000));
        // No usable origin → None (caller shows no timecode + seeks 0).
        assert_eq!(line_start_offset_ms(&utts, 1, None), None);
        assert_eq!(line_start_offset_ms(&utts, 1, Some(0)), None);
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
    fn load_mixed_one_channel_only() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write_wav(&dir.join("mic.wav"), &[5, 6, 7]);
        let (pcm, _) = load_mixed_from_dir(dir).unwrap();
        assert_eq!(pcm, vec![5, 6, 7]); // system silent → mic passes through
    }
}
