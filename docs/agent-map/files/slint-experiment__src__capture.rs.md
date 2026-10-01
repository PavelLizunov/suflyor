---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_95501bdc5a17"
source_path: "slint-experiment/src/capture.rs"
batch_id: "B04"
total_lines: 225
symbols_count: 10
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "664d11334f5a92995ba7ed2405979e213e0f65fbd99833ad33c529254bba9e1a"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/capture.rs`

- **Batch:** B04
- **Physical Lines:** 225
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `CapturedBgra` | L17 | pub |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `capture_monitor_under_cursor` | L27 | `fn capture_monitor_under_cursor() -> Result<CapturedBgra, Box<dyn std::error::Error>>` |
| function | `capture_monitor_under_cursor` | L51 | `fn capture_monitor_under_cursor() -> Result<CapturedBgra, Box<dyn std::error::Error>>` |
| function | `bgra_to_jpeg_data_url` | L63 | `fn bgra_to_jpeg_data_url(bgra: &[u8], width: u32, height: u32,) -> Result<String, Box<dyn std::error::Error>>` |
| function | `capture_virtual_desktop` | L114 | `fn capture_virtual_desktop() -> Result<(CapturedBgra, i32, i32), Box<dyn std::error::Error>>` |
| function | `capture_virtual_desktop` | L134 | `fn capture_virtual_desktop() -> Result<(CapturedBgra, i32, i32), Box<dyn std::error::Error>>` |
| function | `crop_bgra` | L153 | `fn crop_bgra(src: &CapturedBgra, x: u32, y: u32, w: u32, h: u32) -> CapturedBgra` |
| function | `crop_extracts_subrect` | L180 | `fn crop_extracts_subrect() -> ()` |
| function | `crop_clamps_out_of_bounds` | L199 | `fn crop_clamps_out_of_bounds() -> ()` |
| function | `bgra_to_jpeg_produces_data_uri` | L211 | `fn bgra_to_jpeg_produces_data_uri() -> ()` |
| function | `bgra_wrong_size_errors` | L221 | `fn bgra_wrong_size_errors() -> ()` |
