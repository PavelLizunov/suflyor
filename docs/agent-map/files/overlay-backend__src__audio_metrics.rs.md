---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9dd1eb8c69cc"
source_path: "overlay-backend/src/audio_metrics.rs"
batch_id: "B07"
total_lines: 191
symbols_count: 11
review_state: validated
---

# File Map: `overlay-backend/src/audio_metrics.rs`

- **Batch:** B07
- **Physical Lines:** 191
- **Coverage:** 191/191 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Stream` | L16 | pub |
| struct | `StreamSnapshot` | L25 | pub |
| struct | `AudioMetricsSnapshot` | L47 | pub |
| struct | `StreamState` | L55 | private |
| struct | `AudioMetrics` | L60 | pub |

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `state` | L66 | `fn state(&self, stream: Stream) -> &Mutex<StreamState>` |
| function | `record_emitted_chunk` | L82 | `fn record_emitted_chunk(&self, stream: Stream, session_ms: u64) -> ()` |
| function | `record_queue_drop` | L90 | `fn record_queue_drop(&self, stream: Stream) -> ()` |
| function | `record_ring_overflow` | L97 | `fn record_ring_overflow(&self, stream: Stream, frames: u64) -> ()` |
| function | `observe_pending_samples` | L104 | `fn observe_pending_samples(&self, stream: Stream, samples: u64) -> ()` |
| function | `snapshot` | L113 | `fn snapshot(&self) -> AudioMetricsSnapshot` |
| function | `emitted_count_and_timestamp` | L125 | `fn emitted_count_and_timestamp() -> ()` |
| function | `every_counter_saturates_at_u64_max` | L135 | `fn every_counter_saturates_at_u64_max() -> ()` |
| function | `high_watermark_keeps_max_only` | L158 | `fn high_watermark_keeps_max_only() -> ()` |
| function | `mic_and_system_state_is_independent` | L167 | `fn mic_and_system_state_is_independent() -> ()` |
| function | `snapshot_is_a_stable_copy` | L179 | `fn snapshot_is_a_stable_copy() -> ()` |
