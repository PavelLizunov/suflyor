---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_7f98646b6312"
source_path: "slint-experiment/src/bin/overlay_spike.rs"
batch_id: "B01"
total_lines: 213
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "35d8ffc1b9a7160ab8823980d75cfebc1d6ba95b129054b97b353ab3df06f802"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_spike.rs`

- **Batch:** B01
- **Physical Lines:** 213
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `main` | L49 | `fn main() -> ()` |
| function | `main` | L71 | `fn main() -> Result<(), Box<dyn std::error::Error>>` |
| function | `apply_and_log` | L102 | `fn apply_and_log(hwnd: HWND) -> ()` |
| function | `grab_hwnd` | L206 | `fn grab_hwnd(window: &OverlaySpike) -> Result<HWND, Box<dyn std::error::Error>>` |
