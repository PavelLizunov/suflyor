---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_585b11f96b16"
source_path: "overlay-backend/src/ai/tps.rs"
batch_id: "B06"
total_lines: 282
symbols_count: 17
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "6a1f4061a4e7c7531cb91b3273007b1fef16a83db2a0caf3f8cc871cd817d260"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ai/tps.rs`

- **Batch:** B06
- **Physical Lines:** 282
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `RequestPerf` | L14 | pub |
| struct | `TimedRequestPerf` | L22 | private |
| struct | `RequestState` | L27 | private |

## Symbols & Routines (17)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `record_tps` | L33 | `fn record_tps(value: f64) -> ()` |
| function | `avg_tps` | L49 | `fn avg_tps() -> Option<f64>` |
| function | `begin_request` | L58 | `fn begin_request() -> u64` |
| function | `latest_request_perf` | L70 | `fn latest_request_perf() -> Option<RequestPerf>` |
| function | `clear_request_perf` | L83 | `fn clear_request_perf() -> ()` |
| function | `stream_tps` | L98 | `fn stream_tps(server_tps: Option<f64>, completion_tokens: Option<u64>, delta_count: u32, elapsed_secs: f64,) -> Option<f64>` |
| function | `millis` | L116 | `fn millis(duration: Duration) -> u64` |
| function | `request_perf` | L120 | `fn request_perf(request_started_at: Instant, first_delta_at: Option<Instant>, terminal_at: Instant, server_tps: Option<f64>, completion_tokens: Option<u64>,) -> RequestPerf` |
| function | `format_opt_u64` | L144 | `fn format_opt_u64(value: Option<u64>) -> String` |
| function | `format_opt_f64` | L148 | `fn format_opt_f64(value: Option<f64>) -> String` |
| function | `format_stream_tps_metrics` | L154 | `fn format_stream_tps_metrics(perf: &RequestPerf, completion_tokens: Option<u64>) -> String` |
| function | `record_stream_tps` | L166 | `fn record_stream_tps(request_id: u64, request_started_at: Instant, delta_count: u32, first_delta_at: Option<Instant>, server_tps: Option<f64>, completion_tokens: Option<u64>,) -> ()` |
| function | `streamed_tps_prefers_server_metric_over_chunks_and_usage` | L211 | `fn streamed_tps_prefers_server_metric_over_chunks_and_usage() -> ()` |
| function | `request_perf_separates_ttft_decode_and_end_to_end` | L220 | `fn request_perf_separates_ttft_decode_and_end_to_end() -> ()` |
| function | `newer_request_suppresses_older_or_failed_telemetry` | L232 | `fn newer_request_suppresses_older_or_failed_telemetry() -> ()` |
| function | `detailed_decode_never_calls_chunk_rate_tokens` | L243 | `fn detailed_decode_never_calls_chunk_rate_tokens() -> ()` |
| function | `format_stream_tps_metrics_formats_numeric_and_dash_placeholders` | L257 | `fn format_stream_tps_metrics_formats_numeric_and_dash_placeholders() -> ()` |
