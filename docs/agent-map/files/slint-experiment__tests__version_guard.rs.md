---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_89bab30037f5"
source_path: "slint-experiment/tests/version_guard.rs"
batch_id: "B05"
total_lines: 78
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "b3e583211e3b43059807b037eefcaccc130e46c7c4fefe36f2bcbc336bffd3fb"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/version_guard.rs`

- **Batch:** B05
- **Physical Lines:** 78
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `nsi_product_version` | L20 | `fn nsi_product_version(nsi: &str) -> Option<String>` |
| function | `cargo_toml_version_matches_nsi_product_version` | L35 | `fn cargo_toml_version_matches_nsi_product_version() -> ()` |
| function | `cargo_toml_version_matches_macos_info_plist_version` | L54 | `fn cargo_toml_version_matches_macos_info_plist_version() -> ()` |
