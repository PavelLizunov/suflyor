---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4d73bf3fddff"
source_path: "slint-experiment/src/bin/markdown_spike.rs"
batch_id: "B01"
total_lines: 247
symbols_count: 4
review_state: validated
---

# File Map: `slint-experiment/src/bin/markdown_spike.rs`

- **Batch:** B01
- **Physical Lines:** 247
- **Coverage:** 247/247 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `block` | L52 | `fn block(kind: i32, text: String, lang: String) -> MarkdownBlock` |
| function | `parse_markdown` | L69 | `fn parse_markdown(source: &str) -> Vec<MarkdownBlock>` |
| function | `flush` | L184 | `fn flush(out: &mut Vec<MarkdownBlock>, text: &mut String, kind: &mut Option<i32>, lang: &mut String,) -> ()` |
| function | `main` | L202 | `fn main() -> Result<(), Box<dyn std::error::Error>>` |
