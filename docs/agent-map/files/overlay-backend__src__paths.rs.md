---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_5da1663dc0a3"
source_path: "overlay-backend/src/paths.rs"
batch_id: "B09"
total_lines: 151
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "cb11101c416581912582306f8abc0fd48da41d7cc9741709e3700ce3d612e58d"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/paths.rs`

- **Batch:** B09
- **Physical Lines:** 151
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `DataMigration` | L45 | pub |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `data_root` | L27 | `fn data_root() -> Option<PathBuf>` |
| function | `data_root_in` | L32 | `fn data_root_in(base: &Path) -> PathBuf` |
| function | `migrate_data_root` | L72 | `fn migrate_data_root() -> DataMigration` |
| function | `migrate_in` | L80 | `fn migrate_in(base: &Path) -> DataMigration` |
| function | `data_root_in_prefers_brand_then_legacy_then_brand` | L101 | `fn data_root_in_prefers_brand_then_legacy_then_brand() -> ()` |
| function | `migrate_in_renames_legacy_to_brand_once_and_moves_contents` | L115 | `fn migrate_in_renames_legacy_to_brand_once_and_moves_contents() -> ()` |
| function | `migrate_in_fresh_install_is_noop` | L134 | `fn migrate_in_fresh_install_is_noop() -> ()` |
| function | `migrate_in_never_clobbers_when_brand_already_present` | L141 | `fn migrate_in_never_clobbers_when_brand_already_present() -> ()` |
