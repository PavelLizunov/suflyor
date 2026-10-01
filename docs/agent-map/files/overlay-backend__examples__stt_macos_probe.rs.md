---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_65085e8b2eb1"
source_path: "overlay-backend/examples/stt_macos_probe.rs"
batch_id: "B09"
total_lines: 118
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "f86735865fa66995e87882912ca523af9e1bce1e056f3620d9b4b27ee73e8c75"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/examples/stt_macos_probe.rs`

- **Batch:** B09
- **Physical Lines:** 118
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `usage` | L25 | `fn usage() -> &'static str` |
| function | `read_pcm` | L30 | `fn read_pcm(path: &str) -> Result<(Vec<i16>, f64)>` |
| function | `main` | L53 | `fn main() -> Result<()>` |
| function | `main` | L115 | `fn main() -> ()` |
