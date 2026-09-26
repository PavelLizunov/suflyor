---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_819e9fd22d83"
source_path: "overlay-backend/src/health.rs"
batch_id: "B06"
total_lines: 246
symbols_count: 10
review_state: validated
---

# File Map: `overlay-backend/src/health.rs`

- **Batch:** B06
- **Physical Lines:** 246
- **Coverage:** 246/246 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `HealthSignals` | L14 | pub |
| struct | `HealthPayload` | L45 | pub |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `classify` | L62 | `fn classify(age_ms: Option<u64>, degraded: u64, down: u64) -> &'static str` |
| function | `rank` | L72 | `fn rank(state: &str) -> u8` |
| function | `worst` | L83 | `fn worst(a: &'static str, b: &'static str) -> &'static str` |
| function | `snapshot` | L92 | `fn snapshot(&self, now_ms: u64) -> HealthPayload` |
| function | `classify_thresholds` | L144 | `fn classify_thresholds() -> ()` |
| function | `ai_error_marks_down_immediately_then_clears_on_success` | L157 | `fn ai_error_marks_down_immediately_then_clears_on_success() -> ()` |
| function | `ai_idle_without_error_is_not_down` | L176 | `fn ai_idle_without_error_is_not_down() -> ()` |
| function | `mic_failure_cannot_hide_behind_healthy_system_audio` | L187 | `fn mic_failure_cannot_hide_behind_healthy_system_audio() -> ()` |
| function | `system_silence_with_live_mic_stays_ok` | L226 | `fn system_silence_with_live_mic_stays_ok() -> ()` |
| function | `mic_idle_between_sessions` | L238 | `fn mic_idle_between_sessions() -> ()` |
