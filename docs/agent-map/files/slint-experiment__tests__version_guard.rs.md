---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_89bab30037f5"
source_path: "slint-experiment/tests/version_guard.rs"
batch_id: "B05"
total_lines: 78
symbols_count: 3
review_state: validated
---

# File Map: `slint-experiment/tests/version_guard.rs`

- **Batch:** B05
- **Physical Lines:** 78
- **Coverage:** 78/78 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `nsi_product_version` | L20 | `fn nsi_product_version(nsi: &str) -> Option<String>` |
| function | `cargo_toml_version_matches_nsi_product_version` | L35 | `fn cargo_toml_version_matches_nsi_product_version() -> ()` |
| function | `cargo_toml_version_matches_macos_info_plist_version` | L54 | `fn cargo_toml_version_matches_macos_info_plist_version() -> ()` |
