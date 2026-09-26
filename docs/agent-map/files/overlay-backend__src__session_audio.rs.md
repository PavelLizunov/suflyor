---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4caf66e1498e"
source_path: "overlay-backend/src/session_audio.rs"
batch_id: "B07"
total_lines: 375
symbols_count: 20
review_state: validated
---

# File Map: `overlay-backend/src/session_audio.rs`

- **Batch:** B07
- **Physical Lines:** 375
- **Coverage:** 375/375 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (20)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `mix_pcm` | L22 | `fn mix_pcm(a: &[i16], b: &[i16]) -> Vec<i16>` |
| function | `silence_tts_spans` | L35 | `fn silence_tts_spans(pcm: &mut [i16], spans: &[TtsMaskSpan], source: AudioSource, window_start: u64,) -> usize` |
| function | `read_wav_i16` | L71 | `fn read_wav_i16(path: &Path) -> Option<(Vec<i16>, u32)>` |
| function | `load_mixed_from_dir` | L88 | `fn load_mixed_from_dir(session_dir: &Path) -> Result<(Vec<i16>, u32)>` |
| function | `session_has_recordings` | L116 | `fn session_has_recordings(session_id: &str) -> bool` |
| function | `load_mixed_session_audio` | L131 | `fn load_mixed_session_audio(session_id: &str) -> Result<(Vec<i16>, u32)>` |
| function | `system_recording_ms_in_dir` | L143 | `fn system_recording_ms_in_dir(session_dir: &Path) -> Option<i64>` |
| function | `system_recording_ms` | L151 | `fn system_recording_ms(session_id: &str) -> Option<i64>` |
| function | `sample_for_ms` | L161 | `fn sample_for_ms(ms: i64, sample_rate: u32, total: usize) -> usize` |
| function | `ms_for_sample` | L172 | `fn ms_for_sample(sample: usize, sample_rate: u32) -> i64` |
| function | `line_start_offset_ms` | L194 | `fn line_start_offset_ms(utts: &[crate::persistence::Utterance], i: usize, session_start_ms: Option<i64>,) -> Option<i64>` |
| function | `line_start_offset_prefers_audio_ms_else_prev_timestamp` | L218 | `fn line_start_offset_prefers_audio_ms_else_prev_timestamp() -> ()` |
| function | `sample_ms_mapping_roundtrips_and_clamps` | L251 | `fn sample_ms_mapping_roundtrips_and_clamps() -> ()` |
| function | `mix_clamps_and_pads` | L268 | `fn mix_clamps_and_pads() -> ()` |
| function | `write_wav` | L276 | `fn write_wav(path: &Path, samples: &[i16]) -> ()` |
| function | `load_mixed_sums_channels_and_errors_when_empty` | L291 | `fn load_mixed_sums_channels_and_errors_when_empty() -> ()` |
| function | `system_recording_ms_reads_header_duration` | L303 | `fn system_recording_ms_reads_header_duration() -> ()` |
| function | `load_mixed_one_channel_only` | L312 | `fn load_mixed_one_channel_only() -> ()` |
| function | `tts_mask_clamps_merges_and_filters_sources` | L321 | `fn tts_mask_clamps_merges_and_filters_sources() -> ()` |
| function | `playback_masks_tts_without_changing_timeline_and_malformed_fails_open` | L357 | `fn playback_masks_tts_without_changing_timeline_and_malformed_fails_open() -> ()` |
