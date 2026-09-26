---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8e46d80a3290"
source_path: "overlay-backend/src/audio_unavailable.rs"
batch_id: "B07"
total_lines: 160
symbols_count: 10
review_state: validated
---

# File Map: `overlay-backend/src/audio_unavailable.rs`

- **Batch:** B07
- **Physical Lines:** 160
- **Coverage:** 160/160 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AudioSource` | L25 | pub |
| struct | `TranscriptLine` | L35 | pub |
| struct | `AudioChunk` | L42 | pub |
| struct | `DeviceList` | L50 | pub |
| struct | `CaptureHandle` | L62 | pub |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `list_devices` | L56 | `fn list_devices() -> Result<DeviceList>` |
| function | `start_capture` | L67 | `fn start_capture(_mic_device: Option<String>, _sys_device: Option<String>,) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)>` |
| function | `record_source_until_stop` | L75 | `fn record_source_until_stop(_source: AudioSource, _mic_device: Option<String>, _sys_device: Option<String>, _stop: Arc<AtomicBool>,) -> Result<Vec<i16>>` |
| function | `record_sys_blocking` | L86 | `fn record_sys_blocking(_duration_ms: u64, _sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `rms_dbfs` | L93 | `fn rms_dbfs(samples: &[i16]) -> f64` |
| function | `play_tone_and_capture` | L113 | `fn play_tone_and_capture(_sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `record_mic_blocking` | L118 | `fn record_mic_blocking(_duration_ms: u64, _mic_device: Option<String>) -> Result<Vec<i16>>` |
| function | `entry_points_fail_with_unsupported_error` | L128 | `fn entry_points_fail_with_unsupported_error() -> ()` |
| function | `record_source_until_stop_is_unsupported` | L146 | `fn record_source_until_stop_is_unsupported() -> ()` |
| function | `rms_dbfs_matches_windows_helper` | L153 | `fn rms_dbfs_matches_windows_helper() -> ()` |
