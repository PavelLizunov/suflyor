---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3d67e45bcbf6"
source_path: "slint-experiment/src/logging.rs"
batch_id: "B01"
total_lines: 169
symbols_count: 9
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "59adb2fffa6bcd8edc891e2b0d717806228cc5f03f0f4bbb5656e2373cae30c3"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/logging.rs`

- **Batch:** B01
- **Physical Lines:** 169
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `FacadeLogger` | L36 | private |

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `enabled` | L39 | `fn enabled(&self, metadata: &log::Metadata) -> bool` |
| function | `log` | L42 | `fn log(&self, record: &log::Record) -> ()` |
| function | `flush` | L52 | `fn flush(&self) -> ()` |
| function | `log_path` | L62 | `fn log_path() -> Option<PathBuf>` |
| function | `init` | L69 | `fn init() -> ()` |
| function | `line` | L134 | `fn line(msg: &str) -> ()` |
| function | `stamped_line` | L141 | `fn stamped_line(msg: &str) -> String` |
| function | `write_file_line` | L157 | `fn write_file_line(msg: &str) -> ()` |
| function | `write_file_stamped` | L161 | `fn write_file_stamped(stamped: &str) -> ()` |
