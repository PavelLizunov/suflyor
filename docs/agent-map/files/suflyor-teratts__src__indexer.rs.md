---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_99ab94fcc6d4"
source_path: "suflyor-teratts/src/indexer.rs"
batch_id: "B12"
total_lines: 125
symbols_count: 10
review_state: validated
---

# File Map: `suflyor-teratts/src/indexer.rs`

- **Batch:** B12
- **Physical Lines:** 125
- **Coverage:** 125/125 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `UnicodeIndexer` | L12 | pub |

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `load` | L17 | `fn load(path: &Path) -> Result<Self>` |
| function | `from_json` | L23 | `fn from_json(raw: &str) -> Result<Self>` |
| function | `token` | L38 | `fn token(&self, c: char) -> i64` |
| function | `supports` | L48 | `fn supports(&self, c: char) -> bool` |
| function | `batch` | L61 | `fn batch(&self, text: &str) -> Result<(Vec<i64>, Vec<f32>)>` |
| function | `fake_table` | L84 | `fn fake_table(f: impl Fn(u32) -> i64) -> UnicodeIndexer` |
| function | `rejects_wrong_table_size` | L90 | `fn rejects_wrong_table_size() -> ()` |
| function | `batch_encodes_and_masks` | L96 | `fn batch_encodes_and_masks() -> ()` |
| function | `batch_rejects_unsupported_and_empty` | L104 | `fn batch_rejects_unsupported_and_empty() -> ()` |
| function | `supports_follows_nfkd_components` | L111 | `fn supports_follows_nfkd_components() -> ()` |
