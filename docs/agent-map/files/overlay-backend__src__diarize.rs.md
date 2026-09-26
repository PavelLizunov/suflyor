---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1a72846df745"
source_path: "overlay-backend/src/diarize.rs"
batch_id: "B07"
total_lines: 744
symbols_count: 31
review_state: validated
---

# File Map: `overlay-backend/src/diarize.rs`

- **Batch:** B07
- **Physical Lines:** 744
- **Coverage:** 744/744 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Progress` | L50 | pub |
| enum | `DiarEngine` | L70 | pub |
| struct | `SidecarOut` | L58 | private |
| struct | `AutoCandidate` | L285 | private |

## Symbols & Routines (31)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `models_ready` | L64 | `fn models_ready() -> bool` |
| function | `engine_ready` | L78 | `fn engine_ready(engine: DiarEngine) -> bool` |
| function | `timeline_reliable` | L104 | `fn timeline_reliable(recording_ms: Option<i64>, utts: &[Utterance]) -> bool` |
| function | `friendly_error` | L122 | `fn friendly_error(raw: &str) -> String` |
| function | `consider_auto_candidate` | L323 | `fn consider_auto_candidate(best: &mut Option<AutoCandidate>, candidate: AutoCandidate) -> bool` |
| function | `choose_auto_candidate` | L341 | `fn choose_auto_candidate(candidates: Vec<AutoCandidate>) -> Option<AutoCandidate>` |
| function | `guard_wav_len` | L352 | `fn guard_wav_len(wav: &Path) -> Result<()>` |
| function | `diarization_exe_path` | L373 | `fn diarization_exe_path() -> PathBuf` |
| function | `diarization_path_uses_the_platform_executable_suffix` | L383 | `fn diarization_path_uses_the_platform_executable_suffix() -> ()` |
| function | `run_sidecar` | L393 | `fn run_sidecar(exe: &Path, wav: &Path, seg: &Path, emb: &Path, num_speakers: i32,) -> Result<String>` |
| function | `system_windows` | L426 | `fn system_windows(utts: &[Utterance]) -> Vec<(usize, i64, i64)>` |
| function | `max_overlap_speaker` | L448 | `fn max_overlap_speaker(start_ms: i64, end_ms: i64, segments: &[DiarSegment]) -> Option<i32>` |
| function | `overlap_speakers` | L464 | `fn overlap_speakers(start_ms: i64, end_ms: i64, segments: &[DiarSegment]) -> Vec<i32>` |
| function | `align_all` | L531 | `fn align_all(utts: &[Utterance], segments: &[DiarSegment]) -> Vec<Option<i32>>` |
| function | `align_all_speakers` | L545 | `fn align_all_speakers(utts: &[Utterance], segments: &[DiarSegment]) -> Vec<Vec<i32>>` |
| function | `seg` | L560 | `fn seg(s: i64, e: i64, sp: i32) -> DiarSegment` |
| function | `utt` | L567 | `fn utt(source: &str, audio_ms: Option<i64>) -> Utterance` |
| function | `timeline_reliable_flags_short_legacy_recordings` | L578 | `fn timeline_reliable_flags_short_legacy_recordings() -> ()` |
| function | `friendly_error_maps_causes_and_never_leaks_paths` | L598 | `fn friendly_error_maps_causes_and_never_leaks_paths() -> ()` |
| function | `max_overlap_picks_the_dominant_segment` | L608 | `fn max_overlap_picks_the_dominant_segment() -> ()` |
| function | `overlap_speakers_keeps_real_changes_and_ignores_boundary_slivers` | L619 | `fn overlap_speakers_keeps_real_changes_and_ignores_boundary_slivers() -> ()` |
| function | `system_windows_bounds_by_next_and_cap` | L627 | `fn system_windows_bounds_by_next_and_cap() -> ()` |
| function | `filter_drops_phantom_and_renumbers` | L642 | `fn filter_drops_phantom_and_renumbers() -> ()` |
| function | `filter_keeps_secondary_speaker_inside_one_long_utterance` | L662 | `fn filter_keeps_secondary_speaker_inside_one_long_utterance() -> ()` |
| function | `filter_returns_zero_when_everything_is_dropped` | L670 | `fn filter_returns_zero_when_everything_is_dropped() -> ()` |
| function | `align_all_labels_system_lines_and_leaves_mic_none` | L685 | `fn align_all_labels_system_lines_and_leaves_mic_none() -> ()` |
| function | `align_all_speakers_reports_multiple_voices_in_one_block` | L697 | `fn align_all_speakers_reports_multiple_voices_in_one_block() -> ()` |
| function | `candidate` | L703 | `fn candidate(forced: i32, speakers: i64, switches: usize) -> AutoCandidate` |
| function | `auto_selection_stops_before_degenerate_cluster` | L713 | `fn auto_selection_stops_before_degenerate_cluster() -> ()` |
| function | `auto_selection_stops_before_fragmentation_jump` | L724 | `fn auto_selection_stops_before_fragmentation_jump() -> ()` |
| function | `auto_selection_keeps_highest_stable_candidate` | L735 | `fn auto_selection_keeps_highest_stable_candidate() -> ()` |
