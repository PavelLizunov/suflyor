//! Speaker-diarization CLIENT + alignment (D2). The onnx work runs in the
//! `suflyor-tts diarize` sidecar (a separate process — sherpa's onnxruntime can't
//! share ours); this module spawns it, parses its JSON, filters phantom clusters,
//! and aligns the speaker segments to a session's transcript lines.
//!
//! Flow: `run_diarization` (worker thread) resolves the session's `system.wav` +
//! the installed models, runs the sidecar, parses `{num_speakers,segments}`, drops
//! clusters that never attribute a line (the app's own read-aloud voice is recorded
//! INTO system.wav; hold-music; pings), renumbers the survivors, and returns a
//! [`Diarization`] for the UI thread to persist. `align_all` then maps each
//! transcript line to a speaker at render time (both pure + unit-tested).
//!
//! The store is `!Sync` and lives on the UI thread, so the worker is handed the
//! utterances as an owned slice and never touches the catalog.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::persistence::{DiarSegment, Diarization, Utterance};

/// Provenance string stored with each result (which engine + models produced it).
const MODEL_ID: &str = "pyannote-3.0+wespeaker-resnet34";

/// Refuse to diarize a `system.wav` longer than this. The sidecar loads the WHOLE
/// waveform (global clustering needs it): 1 h ≈ 230 MB f32, so a multi-hour file is
/// a memory/CPU risk. Mirrors `re_transcribe`'s length guard.
const MAX_DIAR_SECS: u64 = 3 * 60 * 60;

/// Cap on how long after its start a transcript line may "own" the timeline when
/// aligning to segments — so a long trailing silence before the next line can't let
/// a stray segment win the whole gap. ~one long utterance.
const WINDOW_CAP_MS: i64 = 30_000;

/// A wall-clock-padded recording (post-D0.5) matches the transcript's timeline within
/// sub-second skew + chunk granularity; a legacy one runs short by its idle gaps. Flag
/// the timeline unreliable only when the recording is short by MORE than this — small
/// enough to catch real gaps, large enough not to cry wolf on skew/rounding.
const TIMELINE_TOL_MS: i64 = 5_000;

const AUTO_MIN_SPEAKERS: i32 = 2;
const AUTO_MAX_SPEAKERS: i32 = 6;
const MULTI_SPEAKER_MIN_OVERLAP_MS: i64 = 400;

/// Coarse progress for the archive UI (mirrors `re_transcribe::Progress`).
#[derive(Debug, Clone)]
pub enum Progress {
    Step(String),
}

/// The sidecar's stdout contract: `{"num_speakers":N,"segments":[{"s","e","sp"}]}`.
/// We only need `segments` (the real speaker count is the post-filter one); serde
/// ignores `num_speakers`. `DiarSegment` deserializes straight from `{s,e,sp}`.
#[derive(Debug, Deserialize)]
struct SidecarOut {
    segments: Vec<DiarSegment>,
}

/// True when both diarization models are installed (the button gate delegates here).
#[must_use]
pub fn models_ready() -> bool {
    crate::diar_install::models_installed()
}

/// Whether `system.wav`'s length covers the SYSTEM-line span, so speaker segments
/// align to the right lines. `recording_ms` is the WAV length (see
/// `session_audio::system_recording_ms`); the span is the max `audio_ms` over SYSTEM
/// utterances ONLY. E-fix (fable 2026-07-05): diarization aligns to system.wav, and
/// system.wav is padded only up to the last DELIVERED system chunk (loopback freezes
/// when the call ends), so comparing it against a later MIC line — a goodbye said into
/// the mic after hangup — made this fire on nearly EVERY session (meaningless banner).
/// Returns false ONLY when the recording is short by more than [`TIMELINE_TOL_MS`] —
/// the legacy/unpadded case where `audio_ms`→sample drifts. Missing data → true.
#[must_use]
pub fn timeline_reliable(recording_ms: Option<i64>, utts: &[Utterance]) -> bool {
    let wall = utts
        .iter()
        .filter(|u| u.source == "system")
        .filter_map(|u| u.audio_ms)
        .max()
        .unwrap_or(0);
    match recording_ms {
        Some(rec) if wall > 0 => rec + TIMELINE_TOL_MS >= wall,
        _ => true,
    }
}

/// Map a diarization failure (already a String — see the worker's `map_err`) to a
/// clean, user-facing RU message. NEVER echoes the raw chain (it can carry the
/// `system.wav` path): recognized causes get a specific reason, everything else the
/// generic line — so a screenshot can't leak a filesystem path.
#[must_use]
pub fn friendly_error(raw: &str) -> String {
    if raw.contains("слишком длинная") {
        "Запись длиннее 3 часов — определение говорящих недоступно.".to_string()
    } else if raw.contains("no speakers detected") {
        "Речь не распознана — говорящие не найдены.".to_string()
    } else {
        "Не удалось определить говорящих.".to_string()
    }
}

/// Run diarization for a finished session and return the result to persist.
/// `num_speakers > 0` forces the speaker count (the primary control — auto is
/// unreliable on VoIP audio); `≤ 0` lets the sidecar auto-detect. `utts` is the
/// session's transcript (owned — the worker can't touch the `!Sync` store).
///
/// # Errors
/// If the models aren't installed, the `system.wav` is missing or too long, the
/// sidecar fails, or its output can't be parsed.
pub fn run_diarization(
    session_id: &str,
    num_speakers: i32,
    utts: &[Utterance],
    on_progress: &impl Fn(Progress),
) -> Result<Diarization> {
    let started = Instant::now();
    let seg = crate::diar_install::seg_model_path()
        .filter(|p| p.is_file())
        .context("segmentation model not installed")?;
    let emb = crate::diar_install::emb_model_path()
        .filter(|p| p.is_file())
        .context("embedding model not installed")?;
    let wav = crate::recorder::recordings_dir()?
        .join(session_id)
        .join("system.wav");
    if !wav.is_file() {
        bail!("no system-audio recording for this session");
    }
    guard_wav_len(&wav)?;

    let exe = crate::tts::sidecar_exe_path();
    let windows = system_windows(utts);
    let (segments, real_speakers) = if num_speakers > 0 {
        on_progress(Progress::Step("Определение говорящих…".to_string()));
        let pass = Instant::now();
        let stdout = run_sidecar(&exe, &wav, &seg, &emb, num_speakers)?;
        log::info!(
            "diarization: fixed N={num_speakers} completed in {:.1}s",
            pass.elapsed().as_secs_f32()
        );
        let parsed: SidecarOut =
            serde_json::from_str(stdout.trim()).context("parse diarizer output")?;
        filter_and_renumber(&parsed.segments, &windows)
    } else {
        let mut best = None;
        for count in AUTO_MIN_SPEAKERS..=AUTO_MAX_SPEAKERS {
            on_progress(Progress::Step(format!(
                "Автоподбор говорящих: {count}/{AUTO_MAX_SPEAKERS}…"
            )));
            let pass = Instant::now();
            let stdout = run_sidecar(&exe, &wav, &seg, &emb, count)?;
            log::info!(
                "diarization: auto N={count} completed in {:.1}s",
                pass.elapsed().as_secs_f32()
            );
            let parsed: SidecarOut =
                serde_json::from_str(stdout.trim()).context("parse diarizer output")?;
            if !consider_auto_candidate(
                &mut best,
                auto_candidate(count, &parsed.segments, &windows),
            ) {
                log::info!("diarization: auto sweep stopped before N={count}");
                break;
            }
        }
        let best = best.context("no speakers detected during automatic selection")?;
        (best.segments, best.speakers)
    };
    // Guard the silent dead-end: if EVERY cluster was dropped (no speech, or the
    // transcript has no aligned timecodes), fail loudly so the UI can prompt a
    // re-run instead of persisting a "success" with nothing to show.
    if real_speakers == 0 {
        bail!("no speakers detected (no speech, or transcript has no aligned timecodes)");
    }
    log::info!(
        "diarization: completed in {:.1}s, speakers={real_speakers}",
        started.elapsed().as_secs_f32()
    );
    Ok(Diarization {
        session_id: session_id.to_string(),
        created_at_ms: crate::journal::now_unix_ms() as i64,
        num_speakers: real_speakers,
        model_id: MODEL_ID.to_string(),
        segments,
        speaker_names: std::collections::BTreeMap::new(),
    })
}

struct AutoCandidate {
    forced: i32,
    segments: Vec<DiarSegment>,
    speakers: i64,
    switches: usize,
}

fn auto_candidate(
    forced: i32,
    raw: &[DiarSegment],
    windows: &[(usize, i64, i64)],
) -> AutoCandidate {
    let (clean, _) = filter_and_renumber(raw, windows);
    let total_ms: i64 = clean.iter().map(|s| s.end_ms - s.start_ms).sum();
    let min_ms = (total_ms / 50).max(1_500);
    let mut duration = std::collections::HashMap::<i32, i64>::new();
    for seg in &clean {
        *duration.entry(seg.speaker).or_default() += seg.end_ms - seg.start_ms;
    }
    let retained: Vec<DiarSegment> = clean
        .into_iter()
        .filter(|seg| duration.get(&seg.speaker).copied().unwrap_or_default() >= min_ms)
        .collect();
    let (segments, speakers) = filter_and_renumber(&retained, windows);
    let switches = segments
        .windows(2)
        .filter(|pair| pair[0].speaker != pair[1].speaker)
        .count();
    AutoCandidate {
        forced,
        segments,
        speakers,
        switches,
    }
}

/// Feed one forced-N result into the auto selector. `false` means the candidate
/// is already worse than the current best, so later N values must not be run.
fn consider_auto_candidate(best: &mut Option<AutoCandidate>, candidate: AutoCandidate) -> bool {
    let Some(current) = best.as_ref() else {
        if candidate.speakers > 0 {
            *best = Some(candidate);
        }
        return true;
    };
    let degenerate =
        candidate.speakers < i64::from(candidate.forced) || candidate.speakers <= current.speakers;
    let fragmented = candidate.switches > current.switches.saturating_mul(2).saturating_add(5);
    if degenerate || fragmented {
        return false;
    }
    *best = Some(candidate);
    true
}

#[cfg(test)]
fn choose_auto_candidate(candidates: Vec<AutoCandidate>) -> Option<AutoCandidate> {
    let mut best = None;
    for candidate in candidates {
        if !consider_auto_candidate(&mut best, candidate) {
            break;
        }
    }
    best
}

/// Refuse a `system.wav` over [`MAX_DIAR_SECS`] (cheap header read — no decode).
fn guard_wav_len(wav: &Path) -> Result<()> {
    let reader = hound::WavReader::open(wav).with_context(|| format!("open {}", wav.display()))?;
    let spec = reader.spec();
    let secs = if spec.sample_rate > 0 {
        u64::from(reader.duration()) / u64::from(spec.sample_rate)
    } else {
        0
    };
    if secs > MAX_DIAR_SECS {
        bail!(
            "запись слишком длинная для анализа (> {} ч)",
            MAX_DIAR_SECS / 3600
        );
    }
    Ok(())
}

/// Spawn `suflyor-tts.exe diarize …`, wait, and return its stdout. Non-zero exit →
/// error carrying the sidecar's stderr reason. No console window.
fn run_sidecar(
    exe: &Path,
    wav: &Path,
    seg: &Path,
    emb: &Path,
    num_speakers: i32,
) -> Result<String> {
    let mut cmd = Command::new(exe);
    cmd.arg("diarize")
        .arg(wav)
        .arg("--seg")
        .arg(seg)
        .arg("--emb")
        .arg(emb);
    if num_speakers > 0 {
        cmd.arg("--num-speakers").arg(num_speakers.to_string());
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = crate::download::no_window(&mut cmd)
        .output()
        .context("spawn suflyor-tts diarize")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("diarization failed: {}", err.trim());
    }
    String::from_utf8(out.stdout).context("diarizer stdout is not UTF-8")
}

/// The `(utterance_index, start_ms, end_ms)` window each SYSTEM utterance owns for
/// alignment: from its `audio_ms` to the next system utterance's start, capped at
/// [`WINDOW_CAP_MS`]. Mic utterances and those without `audio_ms` are skipped.
fn system_windows(utts: &[Utterance]) -> Vec<(usize, i64, i64)> {
    let sys: Vec<(usize, i64)> = utts
        .iter()
        .enumerate()
        .filter(|(_, u)| u.source == "system")
        .filter_map(|(i, u)| u.audio_ms.map(|a| (i, a.max(0))))
        .collect();
    let mut out = Vec::with_capacity(sys.len());
    for k in 0..sys.len() {
        let (idx, start) = sys[k];
        let capped = start + WINDOW_CAP_MS;
        let end = sys
            .get(k + 1)
            .map_or(capped, |&(_, next)| next.min(capped))
            .max(start + 1);
        out.push((idx, start, end));
    }
    out
}

/// Max-overlap speaker for `[start_ms, end_ms)` among `segments`; `None` if nothing
/// overlaps (silence / non-speech → not attributed).
fn max_overlap_speaker(start_ms: i64, end_ms: i64, segments: &[DiarSegment]) -> Option<i32> {
    let mut best: Option<i32> = None;
    let mut best_ov: i64 = 0;
    for s in segments {
        let ov = end_ms.min(s.end_ms) - start_ms.max(s.start_ms);
        if ov > best_ov {
            best_ov = ov;
            best = Some(s.speaker);
        }
    }
    best
}

/// Speakers with a meaningful overlap in one utterance window, ordered by first
/// appearance. Tiny boundary slivers are ignored; a very short window falls back
/// to its dominant speaker so it never loses an otherwise valid label.
fn overlap_speakers(start_ms: i64, end_ms: i64, segments: &[DiarSegment]) -> Vec<i32> {
    let mut overlaps = Vec::<(i32, i64)>::new();
    for segment in segments {
        let overlap = end_ms.min(segment.end_ms) - start_ms.max(segment.start_ms);
        if overlap <= 0 {
            continue;
        }
        if let Some((_, total)) = overlaps.iter_mut().find(|(id, _)| *id == segment.speaker) {
            *total += overlap;
        } else {
            overlaps.push((segment.speaker, overlap));
        }
    }
    let mut speakers: Vec<i32> = overlaps
        .into_iter()
        .filter(|(_, overlap)| *overlap >= MULTI_SPEAKER_MIN_OVERLAP_MS)
        .map(|(speaker, _)| speaker)
        .collect();
    if speakers.is_empty() {
        if let Some(speaker) = max_overlap_speaker(start_ms, end_ms, segments) {
            speakers.push(speaker);
        }
    }
    speakers
}

/// Phantom filter + renumber. Keeps only speakers with meaningful overlap in ≥1
/// utterance window — dropping clusters that never attribute a line (the app's own
/// TTS voice in system.wav, hold-music, notification pings). Renumbers survivors to
/// `0..M` by first appearance (segments sorted by start). Returns `(clean segments,
/// M)`. Dropping never-winners can't change any winner, so a later `align_all` over
/// the clean segments reproduces the same attribution.
fn filter_and_renumber(
    raw: &[DiarSegment],
    windows: &[(usize, i64, i64)],
) -> (Vec<DiarSegment>, i64) {
    let mut winners: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    for &(_, s, e) in windows {
        winners.extend(overlap_speakers(s, e, raw));
    }
    let mut kept: Vec<DiarSegment> = raw
        .iter()
        .copied()
        .filter(|seg| winners.contains(&seg.speaker))
        .collect();
    kept.sort_by_key(|s| (s.start_ms, s.end_ms));

    let mut remap: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    for seg in &kept {
        let next = remap.len() as i32;
        remap.entry(seg.speaker).or_insert(next);
    }
    let out: Vec<DiarSegment> = kept
        .iter()
        .map(|seg| DiarSegment {
            speaker: *remap.get(&seg.speaker).unwrap_or(&0),
            ..*seg
        })
        .collect();
    (out, remap.len() as i64)
}

/// Per-utterance display speaker over the (clean, stored) `segments`, same length +
/// order as `utts`: `Some(id)` for a SYSTEM line attributed to a speaker, `None` for
/// a mic line (caller labels it «Вы») or a system line with no overlapping segment
/// (caller labels it «Система»). Pure — the «По голосам» view's render helper.
#[must_use]
pub fn align_all(utts: &[Utterance], segments: &[DiarSegment]) -> Vec<Option<i32>> {
    let mut result = vec![None; utts.len()];
    for (idx, s, e) in system_windows(utts) {
        if idx < result.len() {
            result[idx] = max_overlap_speaker(s, e, segments);
        }
    }
    result
}

/// Per-utterance significant speakers. Unlike [`align_all`], this preserves a
/// real speaker change inside one long STT block instead of hiding every voice
/// except the dominant one.
#[must_use]
pub fn align_all_speakers(utts: &[Utterance], segments: &[DiarSegment]) -> Vec<Vec<i32>> {
    let mut result = vec![Vec::new(); utts.len()];
    for (idx, start, end) in system_windows(utts) {
        if idx < result.len() {
            result[idx] = overlap_speakers(start, end, segments);
        }
    }
    result
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn seg(s: i64, e: i64, sp: i32) -> DiarSegment {
        DiarSegment {
            start_ms: s,
            end_ms: e,
            speaker: sp,
        }
    }
    fn utt(source: &str, audio_ms: Option<i64>) -> Utterance {
        Utterance {
            session_id: "s".into(),
            unix_ms: 0,
            source: source.into(),
            text: String::new(),
            audio_ms,
        }
    }

    #[test]
    fn timeline_reliable_flags_short_legacy_recordings() {
        // 10-min transcript span (last audio_ms = 600 s).
        let utts = vec![utt("system", Some(0)), utt("system", Some(600_000))];
        assert!(timeline_reliable(Some(600_000), &utts)); // padded → covers the span
        assert!(timeline_reliable(Some(598_000), &utts)); // 2 s short → within tol
        assert!(!timeline_reliable(Some(540_000), &utts)); // 1 min short → legacy/unpadded
        assert!(timeline_reliable(None, &utts)); // unknown length → don't cry wolf
        assert!(timeline_reliable(Some(1000), &[utt("system", None)])); // no audio_ms → ditto
                                                                        // E-fix: a late MIC line (goodbye said after the call hung up) must NOT count —
                                                                        // only SYSTEM lines define the span, so a WAV covering the system span stays reliable
                                                                        // even though the mic line is timestamped 5 min later (this used to fire the banner).
        let late_mic = vec![
            utt("system", Some(0)),
            utt("system", Some(600_000)),
            utt("mic", Some(900_000)),
        ];
        assert!(timeline_reliable(Some(600_000), &late_mic));
    }

    #[test]
    fn friendly_error_maps_causes_and_never_leaks_paths() {
        assert!(friendly_error("запись слишком длинная для анализа (> 3 ч)").contains("3 час"));
        assert!(friendly_error("no speakers detected (no speech, ...)").contains("не найдены"));
        // Unknown / path-carrying chain → generic line, no filesystem leak.
        let g = friendly_error("open C:\\Users\\bob\\recordings\\x\\system.wav: not found");
        assert_eq!(g, "Не удалось определить говорящих.");
        assert!(!g.contains("bob"));
    }

    #[test]
    fn max_overlap_picks_the_dominant_segment() {
        let segs = [seg(0, 1000, 0), seg(900, 3000, 1)];
        // window [0,1000): sp0 overlaps 1000, sp1 overlaps 100 → sp0
        assert_eq!(max_overlap_speaker(0, 1000, &segs), Some(0));
        // window [1000,3000): only sp1 → sp1
        assert_eq!(max_overlap_speaker(1000, 3000, &segs), Some(1));
        // window in a gap → None
        assert_eq!(max_overlap_speaker(5000, 6000, &segs), None);
    }

    #[test]
    fn overlap_speakers_keeps_real_changes_and_ignores_boundary_slivers() {
        let segs = [seg(0, 900, 0), seg(900, 1800, 1), seg(1900, 2500, 2)];
        assert_eq!(overlap_speakers(0, 1800, &segs), vec![0, 1]);
        assert_eq!(overlap_speakers(1000, 2000, &segs), vec![1]);
        assert_eq!(overlap_speakers(1900, 2100, &segs), vec![2]);
    }

    #[test]
    fn system_windows_bounds_by_next_and_cap() {
        let utts = [
            utt("mic", Some(100)),     // skipped (mic)
            utt("system", Some(1000)), // → next system at 2000 → [1000,2000)
            utt("system", None),       // skipped (no audio_ms)
            utt("system", Some(2000)), // → last → capped [2000, 2000+30000)
        ];
        let w = system_windows(&utts);
        assert_eq!(w, vec![(1, 1000, 2000), (3, 2000, 2000 + WINDOW_CAP_MS)]);
        // a >CAP gap is capped, not extended to the next start
        let far = [utt("system", Some(0)), utt("system", Some(500_000))];
        assert_eq!(system_windows(&far)[0], (0, 0, WINDOW_CAP_MS));
    }

    #[test]
    fn filter_drops_phantom_and_renumbers() {
        // sp0 + sp2 win real windows; sp1 is a phantom (its segment overlaps no
        // utterance window). Survivors renumber 0,2 → 0,1 by first appearance.
        let raw = [
            seg(0, 1000, 0),
            seg(1500, 1600, 1), // phantom: sits in the gap, never a window
            seg(3000, 4000, 2),
        ];
        let utts = [utt("system", Some(0)), utt("system", Some(3000))];
        let windows = system_windows(&utts); // [0,3000)+[3000,3000+cap)
        let (clean, m) = filter_and_renumber(&raw, &windows);
        assert_eq!(m, 2, "two real speakers");
        // phantom sp1 dropped; sp0→0, sp2→1
        assert_eq!(clean.len(), 2);
        assert_eq!(clean[0].speaker, 0);
        assert_eq!(clean[1].speaker, 1);
        assert!(clean.iter().all(|s| s.speaker != 1 || s.start_ms == 3000));
    }

    #[test]
    fn filter_keeps_secondary_speaker_inside_one_long_utterance() {
        let raw = [seg(0, 4_000, 0), seg(4_000, 6_000, 1)];
        let (clean, count) = filter_and_renumber(&raw, &[(0, 0, 6_000)]);
        assert_eq!(count, 2);
        assert_eq!(clean, raw);
    }

    #[test]
    fn filter_returns_zero_when_everything_is_dropped() {
        // No windows (no system utterances with audio_ms) → no winners → all dropped.
        // This is the condition run_diarization turns into a loud error, not an
        // empty "success".
        let raw = [seg(0, 1000, 0), seg(2000, 3000, 1)];
        let (clean, m) = filter_and_renumber(&raw, &[]);
        assert!(clean.is_empty());
        assert_eq!(m, 0);
        // Empty input → (empty, 0) too.
        let (empty, n) = filter_and_renumber(&[], &[]);
        assert!(empty.is_empty());
        assert_eq!(n, 0);
    }

    #[test]
    fn align_all_labels_system_lines_and_leaves_mic_none() {
        let segs = [seg(0, 2000, 0), seg(2000, 5000, 1)];
        let utts = [
            utt("mic", Some(100)),     // → None (caller = «Вы»)
            utt("system", Some(200)),  // window [200,2200)→ mostly sp0
            utt("system", Some(3000)), // window [3000,..)→ sp1
        ];
        let a = align_all(&utts, &segs);
        assert_eq!(a, vec![None, Some(0), Some(1)]);
    }

    #[test]
    fn align_all_speakers_reports_multiple_voices_in_one_block() {
        let segs = [seg(0, 1_000, 0), seg(1_000, 2_000, 1)];
        let utts = [utt("system", Some(0))];
        assert_eq!(align_all_speakers(&utts, &segs), vec![vec![0, 1]]);
    }

    fn candidate(forced: i32, speakers: i64, switches: usize) -> AutoCandidate {
        AutoCandidate {
            forced,
            segments: vec![seg(0, 1, 0)],
            speakers,
            switches,
        }
    }

    #[test]
    fn auto_selection_stops_before_degenerate_cluster() {
        let chosen = choose_auto_candidate(vec![
            candidate(2, 2, 8),
            candidate(3, 3, 12),
            candidate(4, 3, 13),
        ])
        .unwrap();
        assert_eq!(chosen.forced, 3);
    }

    #[test]
    fn auto_selection_stops_before_fragmentation_jump() {
        let chosen = choose_auto_candidate(vec![
            candidate(2, 2, 10),
            candidate(3, 3, 15),
            candidate(4, 4, 40),
        ])
        .unwrap();
        assert_eq!(chosen.forced, 3);
    }

    #[test]
    fn auto_selection_keeps_highest_stable_candidate() {
        let chosen = choose_auto_candidate(vec![
            candidate(2, 2, 10),
            candidate(3, 3, 14),
            candidate(4, 4, 18),
        ])
        .unwrap();
        assert_eq!(chosen.forced, 4);
    }
}
