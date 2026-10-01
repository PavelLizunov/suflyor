---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4d73bf3fddff"
source_path: "slint-experiment/src/bin/markdown_spike.rs"
batch_id: "B01"
total_lines: 247
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "2ed1ad9d409da94d11a1b5f30795568bf429af1ac49d7409a03c4cf62a0d1f8a"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/markdown_spike.rs`

- **Batch:** B01
- **Physical Lines:** 247
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `block` | L52 | `fn block(kind: i32, text: String, lang: String) -> MarkdownBlock` |
| function | `parse_markdown` | L69 | `fn parse_markdown(source: &str) -> Vec<MarkdownBlock>` |
| function | `flush` | L184 | `fn flush(out: &mut Vec<MarkdownBlock>, text: &mut String, kind: &mut Option<i32>, lang: &mut String,) -> ()` |
| function | `main` | L202 | `fn main() -> Result<(), Box<dyn std::error::Error>>` |
