---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3d67e45bcbf6"
source_path: "slint-experiment/src/logging.rs"
batch_id: "B01"
total_lines: 169
symbols_count: 9
review_state: validated
---

# File Map: `slint-experiment/src/logging.rs`

- **Batch:** B01
- **Physical Lines:** 169
- **Coverage:** 169/169 lines (100%)

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
