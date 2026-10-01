---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_bad079dcd257"
source_path: "overlay-backend/src/ocr_install.rs"
batch_id: "B10"
total_lines: 127
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "da34445eda8f83e81111607ae94350f0ddc457445c9e10e6ceb22e56d0d96cb0"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ocr_install.rs`

- **Batch:** B10
- **Physical Lines:** 127
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `OcrProgress` | L28 | pub |

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `is_installed` | L39 | `fn is_installed() -> bool` |
| function | `dest_has_engine` | L91 | `fn dest_has_engine(dest: &Path) -> bool` |
| function | `bundle_pin_is_valid` | L100 | `fn bundle_pin_is_valid() -> ()` |
| function | `dest_has_engine_requires_binary_and_tessdata` | L108 | `fn dest_has_engine_requires_binary_and_tessdata() -> std::io::Result<()>` |
