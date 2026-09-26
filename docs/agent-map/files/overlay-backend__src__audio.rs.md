---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_08a762fee291"
source_path: "overlay-backend/src/audio.rs"
batch_id: "B07"
total_lines: 962
symbols_count: 24
review_state: validated
---

# File Map: `overlay-backend/src/audio.rs`

- **Batch:** B07
- **Physical Lines:** 962
- **Coverage:** 962/962 lines (100%)

## Types & Structures (6)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AudioSource` | L52 | pub |
| enum | `CaptureExit` | L265 | private |
| struct | `TranscriptLine` | L68 | pub |
| struct | `AudioChunk` | L75 | pub |
| struct | `DeviceList` | L83 | pub |
| struct | `CaptureHandle` | L142 | pub |

## Symbols & Routines (24)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `get_default_device` | L36 | `fn get_default_device(dir: &Direction) -> Result<wasapi::Device>` |
| function | `device_collection` | L41 | `fn device_collection(dir: &Direction) -> Result<wasapi::DeviceCollection>` |
| function | `list_devices` | L89 | `fn list_devices() -> Result<DeviceList>` |
| function | `enumerate` | L98 | `fn enumerate(dir: &Direction) -> Result<Vec<String>>` |
| function | `find_device_by_name` | L126 | `fn find_device_by_name(dir: &Direction, name: &str) -> Option<wasapi::Device>` |
| function | `drop` | L147 | `fn drop(&mut self) -> ()` |
| function | `start_capture` | L154 | `fn start_capture(mic_device: Option<String>, sys_device: Option<String>,) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)>` |
| function | `capture_with_recovery` | L203 | `fn capture_with_recovery(source: AudioSource, endpoint_dir: Direction, device_name: Option<String>, tx: mpsc::Sender<AudioChunk>, stop: Arc<AtomicBool>,) -> Result<()>` |
| function | `record_source_until_stop` | L502 | `fn record_source_until_stop(source: AudioSource, mic_device: Option<String>, sys_device: Option<String>, stop: Arc<AtomicBool>,) -> Result<Vec<i16>>` |
| function | `record_sys_blocking` | L608 | `fn record_sys_blocking(duration_ms: u64, sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `rms_dbfs` | L622 | `fn rms_dbfs(samples: &[i16]) -> f64` |
| function | `clip_frames_to_bytes` | L651 | `fn clip_frames_to_bytes(clip: &[i16], cursor: &mut usize, frames: usize) -> Vec<u8>` |
| function | `play_test_clip` | L669 | `fn play_test_clip(stop: Arc<AtomicBool>) -> Result<()>` |
| function | `play_tone_and_capture` | L729 | `fn play_tone_and_capture(sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `record_mic_blocking` | L745 | `fn record_mic_blocking(duration_ms: u64, mic_device: Option<String>) -> Result<Vec<i16>>` |
| function | `resample_and_quantise` | L803 | `fn resample_and_quantise(input: &[f32], ratio: f64) -> Vec<i16>` |
| function | `rms_dbfs_silence_and_full_scale` | L841 | `fn rms_dbfs_silence_and_full_scale() -> ()` |
| function | `clip_frames_to_bytes_length_and_wrap` | L857 | `fn clip_frames_to_bytes_length_and_wrap() -> ()` |
| function | `decimator_48k_to_16k_is_3_to_1` | L872 | `fn decimator_48k_to_16k_is_3_to_1() -> ()` |
| function | `decimator_handles_empty` | L880 | `fn decimator_handles_empty() -> ()` |
| function | `decimator_preserves_1khz_sine_frequency` | L889 | `fn decimator_preserves_1khz_sine_frequency() -> ()` |
| function | `decimator_ratio_one_is_identity_quantisation` | L923 | `fn decimator_ratio_one_is_identity_quantisation() -> ()` |
| function | `decimator_oversaturation_clamped` | L938 | `fn decimator_oversaturation_clamped() -> ()` |
| function | `decimator_preserves_average_amplitude` | L948 | `fn decimator_preserves_average_amplitude() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L158
- Spawns asynchronous thread/task at L499
- Spawns asynchronous thread/task at L611
- Spawns asynchronous thread/task at L732
