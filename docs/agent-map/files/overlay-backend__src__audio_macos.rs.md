---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_58e3146c37bb"
source_path: "overlay-backend/src/audio_macos.rs"
batch_id: "B07"
total_lines: 1302
symbols_count: 57
review_state: validated
---

# File Map: `overlay-backend/src/audio_macos.rs`

- **Batch:** B07
- **Physical Lines:** 1302
- **Coverage:** 1302/1302 lines (100%)

## Types & Structures (11)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AudioSource` | L77 | pub |
| enum | `MicrophonePermission` | L181 | pub |
| enum | `SystemCaptureState` | L322 | pub |
| enum | `RetryResult` | L643 | private |
| struct | `TranscriptLine` | L88 | pub |
| struct | `AudioChunk` | L95 | pub |
| struct | `DeviceList` | L103 | pub |
| struct | `MicController` | L112 | private |
| struct | `SystemCaptureController` | L158 | private |
| struct | `MicStartupError` | L235 | private |
| struct | `CaptureHandle` | L328 | pub |

## Symbols & Routines (57)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `mic_capture_permission_status` | L117 | `fn mic_capture_permission_status() -> u32` |
| function | `mic_capture_start` | L122 | `fn mic_capture_start(buffer_frames: u32, out_sample_rate: *mut f64, out_error: *mut i32,) -> *mut MicController` |
| function | `mic_capture_ring_capacity` | L127 | `fn mic_capture_ring_capacity(controller: *const MicController) -> u32` |
| function | `mic_capture_read` | L128 | `fn mic_capture_read(controller: *mut MicController, dst: *mut f32, max_frames: u32) -> u32` |
| function | `mic_capture_take_dropped` | L129 | `fn mic_capture_take_dropped(controller: *mut MicController) -> u64` |
| function | `mic_capture_take_route_change` | L130 | `fn mic_capture_take_route_change(controller: *mut MicController) -> u32` |
| function | `mic_capture_stop` | L131 | `fn mic_capture_stop(controller: *mut MicController) -> ()` |
| function | `mic_capture_copy_default_input_name` | L132 | `fn mic_capture_copy_default_input_name() -> *mut c_char` |
| function | `mic_capture_copy_default_output_name` | L133 | `fn mic_capture_copy_default_output_name() -> *mut c_char` |
| function | `mic_capture_free_string` | L134 | `fn mic_capture_free_string(ptr: *mut c_char) -> ()` |
| function | `free_and_convert_c_string` | L140 | `fn free_and_convert_c_string(ptr: *mut c_char) -> Option<String>` |
| function | `system_capture_start` | L163 | `fn system_capture_start(buffer_frames: u32, out_sample_rate: *mut f64, out_error: *mut i32,) -> *mut SystemCaptureController` |
| function | `system_capture_ring_capacity` | L168 | `fn system_capture_ring_capacity(controller: *const SystemCaptureController) -> u32` |
| function | `system_capture_read` | L169 | `fn system_capture_read(controller: *mut SystemCaptureController, dst: *mut f32, max_frames: u32,) -> u32` |
| function | `system_capture_take_dropped` | L174 | `fn system_capture_take_dropped(controller: *mut SystemCaptureController) -> u64` |
| function | `system_capture_take_route_change` | L175 | `fn system_capture_take_route_change(controller: *mut SystemCaptureController) -> u32` |
| function | `system_capture_stop` | L176 | `fn system_capture_stop(controller: *mut SystemCaptureController) -> ()` |
| function | `from_raw` | L189 | `fn from_raw(raw: u32) -> Self` |
| function | `permission_callback` | L199 | `fn permission_callback(raw: u32, context: *mut c_void) -> ()` |
| function | `microphone_permission` | L212 | `fn microphone_permission() -> MicrophonePermission` |
| function | `request_microphone_permission` | L226 | `fn request_microphone_permission(callback: F) -> ()` |
| function | `start_error_message` | L240 | `fn start_error_message(code: i32) -> anyhow::Error` |
| function | `system_start_error_message` | L254 | `fn system_start_error_message(code: i32) -> &'static str` |
| function | `system_worker_joinable` | L281 | `fn system_worker_joinable(state: u8) -> bool` |
| function | `list_devices` | L288 | `fn list_devices() -> Result<DeviceList>` |
| function | `metrics_snapshot` | L338 | `fn metrics_snapshot(&self) -> AudioMetricsSnapshot` |
| function | `system_capture_state` | L344 | `fn system_capture_state(&self) -> SystemCaptureState` |
| function | `drop` | L358 | `fn drop(&mut self) -> ()` |
| function | `start_capture` | L394 | `fn start_capture(mic_device: Option<String>, sys_device: Option<String>,) -> Result<(mpsc::Receiver<AudioChunk>, CaptureHandle)>` |
| function | `record_source_until_stop` | L516 | `fn record_source_until_stop(source: AudioSource, _mic_device: Option<String>, _sys_device: Option<String>, stop: Arc<AtomicBool>,) -> Result<Vec<i16>>` |
| function | `record_sys_blocking` | L592 | `fn record_sys_blocking(duration_ms: u64, _sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `rms_dbfs` | L605 | `fn rms_dbfs(samples: &[i16]) -> f64` |
| function | `play_tone_and_capture` | L625 | `fn play_tone_and_capture(_sys_device: Option<String>) -> Result<Vec<i16>>` |
| function | `record_mic_blocking` | L630 | `fn record_mic_blocking(duration_ms: u64, _mic_device: Option<String>) -> Result<Vec<i16>>` |
| function | `retry_route_start` | L649 | `fn retry_route_start(stop: &AtomicBool, delay: Duration, mut start: impl FnMut() -> RetryResult<T>, ) -> Option<T>` |
| function | `reopen_mic` | L670 | `fn reopen_mic(stop: &AtomicBool) -> Option<(*mut MicController, f64)>` |
| function | `reopen_system` | L694 | `fn reopen_system(stop: &AtomicBool) -> Option<(*mut SystemCaptureController, f64)>` |
| function | `capture_worker` | L711 | `fn capture_worker(tx: mpsc::Sender<AudioChunk>, stop: Arc<AtomicBool>, started: std::sync::mpsc::Sender<Result<u32, MicStartupError>>, session_start: Instant, metrics: Arc<AudioMetrics>,) -> ()` |
| function | `system_capture_worker` | L844 | `fn system_capture_worker(tx: mpsc::Sender<AudioChunk>, stop: Arc<AtomicBool>, state: Arc<AtomicU8>, session_start: Instant, metrics: Arc<AudioMetrics>,) -> ()` |
| function | `resample_and_quantise` | L984 | `fn resample_and_quantise(input: &[f32], ratio: f64) -> Vec<i16>` |
| function | `list_devices_returns_default_device_names_and_handles_missing_safely` | L1023 | `fn list_devices_returns_default_device_names_and_handles_missing_safely() -> ()` |
| function | `record_source_until_stop_supported_shape` | L1030 | `fn record_source_until_stop_supported_shape() -> ()` |
| function | `start_error_categories_are_distinct_and_descriptive` | L1036 | `fn start_error_categories_are_distinct_and_descriptive() -> ()` |
| function | `native_bridge_symbol_links_without_opening_microphone` | L1048 | `fn native_bridge_symbol_links_without_opening_microphone() -> ()` |
| function | `system_capture_bridge_symbols_link_without_starting_capture` | L1062 | `fn system_capture_bridge_symbols_link_without_starting_capture() -> ()` |
| function | `route_restart_retry_is_stop_aware_and_recovers_on_success` | L1079 | `fn route_restart_retry_is_stop_aware_and_recovers_on_success() -> ()` |
| function | `system_start_error_categories_map_to_safe_generic_messages` | L1114 | `fn system_start_error_categories_map_to_safe_generic_messages() -> ()` |
| function | `capture_handle_metrics_snapshot_is_zeroed_before_any_capture` | L1158 | `fn capture_handle_metrics_snapshot_is_zeroed_before_any_capture() -> ()` |
| function | `system_worker_join_decision_never_joins_pending_start` | L1171 | `fn system_worker_join_decision_never_joins_pending_start() -> ()` |
| function | `permission_request_from_an_unbundled_process_answers_restricted` | L1180 | `fn permission_request_from_an_unbundled_process_answers_restricted() -> ()` |
| function | `permission_status_mapping_is_closed_and_stable` | L1198 | `fn permission_status_mapping_is_closed_and_stable() -> ()` |
| function | `rms_dbfs_matches_windows_helper` | L1222 | `fn rms_dbfs_matches_windows_helper() -> ()` |
| function | `decimator_48k_to_16k_is_3_to_1` | L1231 | `fn decimator_48k_to_16k_is_3_to_1() -> ()` |
| function | `decimator_handles_empty_and_bad_ratio` | L1238 | `fn decimator_handles_empty_and_bad_ratio() -> ()` |
| function | `decimator_oversaturation_clamped` | L1245 | `fn decimator_oversaturation_clamped() -> ()` |
| function | `decimator_preserves_average_amplitude` | L1255 | `fn decimator_preserves_average_amplitude() -> ()` |
| function | `decimator_preserves_1khz_sine_frequency` | L1272 | `fn decimator_preserves_1khz_sine_frequency() -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L411
- Instantiates IPC channel at L414
- Spawns asynchronous thread/task at L595
- Spawns asynchronous thread/task at L633
