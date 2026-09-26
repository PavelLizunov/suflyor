---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6b53a0bd6e77"
source_path: "slint-experiment/src/replay_backend.rs"
batch_id: "B01"
total_lines: 372
symbols_count: 11
review_state: validated
---

# File Map: `slint-experiment/src/replay_backend.rs`

- **Batch:** B01
- **Physical Lines:** 372
- **Coverage:** 372/372 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `SessionInfo` | L24 | pub |

## Symbols & Routines (11)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `list_sessions` | L32 | `fn list_sessions() -> Result<Vec<SessionInfo>>` |
| function | `load_session` | L75 | `fn load_session(path: &Path) -> Result<Vec<serde_json::Value>>` |
| function | `preview` | L118 | `fn preview(s: &str, n: usize) -> String` |
| function | `fmt_clock` | L131 | `fn fmt_clock(unix_ms: Option<u64>) -> String` |
| function | `ev_str` | L146 | `fn ev_str(ev: &serde_json::Value, key: &str) -> String` |
| function | `ev_u64` | L153 | `fn ev_u64(ev: &serde_json::Value, key: &str) -> Option<u64>` |
| function | `ev_f64` | L157 | `fn ev_f64(ev: &serde_json::Value, key: &str) -> Option<f64>` |
| function | `ev_bool` | L161 | `fn ev_bool(ev: &serde_json::Value, key: &str) -> bool` |
| function | `event_cost_usd` | L170 | `fn event_cost_usd(ev: &serde_json::Value) -> f64` |
| function | `render_event` | L180 | `fn render_event(ev: &serde_json::Value) -> (String, String)` |
| function | `total_cost_usd` | L362 | `fn total_cost_usd(events: &[serde_json::Value]) -> (f64, u64)` |
