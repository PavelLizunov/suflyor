---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8fa5ae321113"
source_path: "overlay-backend/src/re_transcribe.rs"
batch_id: "B07"
total_lines: 428
symbols_count: 15
review_state: validated
---

# File Map: `overlay-backend/src/re_transcribe.rs`

- **Batch:** B07
- **Physical Lines:** 428
- **Coverage:** 428/428 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Progress` | L40 | pub |

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `chunk_secs` | L62 | `fn chunk_secs(backend: &SttBackendCfg) -> u64` |
| function | `validate_wav_spec` | L72 | `fn validate_wav_spec(spec: &hound::WavSpec) -> Result<()>` |
| function | `read_next_chunk` | L89 | `fn read_next_chunk(samples: &mut hound::WavIntoSamples<R, i16>, max: usize,) -> Result<Vec<i16>>` |
| function | `append_chunk_text` | L104 | `fn append_chunk_text(acc: &mut String, piece: &str) -> ()` |
| function | `is_silent_window` | L126 | `fn is_silent_window(buf: &[i16]) -> bool` |
| function | `load_wav_pcm` | L137 | `fn load_wav_pcm(path: &Path) -> Result<Vec<i16>>` |
| function | `assemble_lines` | L151 | `fn assemble_lines(mic_text: &str, system_text: &str) -> Vec<TranscriptLine>` |
| function | `load_wav_round_trips_recorder_format` | L314 | `fn load_wav_round_trips_recorder_format() -> ()` |
| function | `load_wav_rejects_wrong_format` | L333 | `fn load_wav_rejects_wrong_format() -> ()` |
| function | `assemble_labels_and_orders_channels` | L350 | `fn assemble_labels_and_orders_channels() -> ()` |
| function | `assemble_drops_empty_channels` | L361 | `fn assemble_drops_empty_channels() -> ()` |
| function | `read_next_chunk_windows_a_wav_without_loss` | L371 | `fn read_next_chunk_windows_a_wav_without_loss() -> ()` |
| function | `chunk_secs_is_per_backend_and_bounded` | L395 | `fn chunk_secs_is_per_backend_and_bounded() -> ()` |
| function | `append_chunk_text_joins_with_single_space_and_skips_blank` | L412 | `fn append_chunk_text_joins_with_single_space_and_skips_blank() -> ()` |
| function | `is_silent_window_skips_padding_and_noise_but_keeps_speech` | L422 | `fn is_silent_window_skips_padding_and_noise_but_keeps_speech() -> ()` |
