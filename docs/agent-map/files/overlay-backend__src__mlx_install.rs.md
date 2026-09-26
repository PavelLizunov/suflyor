---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_935e9c8cd0dd"
source_path: "overlay-backend/src/mlx_install.rs"
batch_id: "B06"
total_lines: 997
symbols_count: 32
review_state: validated
---

# File Map: `overlay-backend/src/mlx_install.rs`

- **Batch:** B06
- **Physical Lines:** 997
- **Coverage:** 997/997 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `ModelRole` | L28 | pub |
| struct | `CatalogFile` | L34 | pub |
| struct | `CatalogModel` | L41 | pub |

## Symbols & Routines (32)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `catalog_for_role` | L222 | `fn catalog_for_role(role: ModelRole) -> Vec<&'static CatalogModel>` |
| function | `catalog_model` | L233 | `fn catalog_model(id: &str) -> Option<&'static CatalogModel>` |
| function | `safe_key` | L237 | `fn safe_key(model: &CatalogModel) -> String` |
| function | `safe_relative` | L255 | `fn safe_relative(path: &str) -> bool` |
| function | `snapshot_path_in` | L265 | `fn snapshot_path_in(data_root: &Path, model: &CatalogModel) -> PathBuf` |
| function | `installed_snapshot` | L274 | `fn installed_snapshot(id: &str) -> Option<PathBuf>` |
| function | `installed_snapshot_verified` | L281 | `fn installed_snapshot_verified(id: &str) -> Option<PathBuf>` |
| function | `file_matches` | L463 | `fn file_matches(path: &Path, file: &CatalogFile) -> bool` |
| function | `hash_file` | L468 | `fn hash_file(path: &Path) -> Result<String>` |
| function | `check_snapshot_fast` | L482 | `fn check_snapshot_fast(model: &CatalogModel, path: &Path) -> bool` |
| function | `check_snapshot_verified` | L493 | `fn check_snapshot_verified(model: &CatalogModel, path: &Path) -> bool` |
| function | `path_present` | L502 | `fn path_present(path: &Path) -> bool` |
| function | `remaining_bytes` | L507 | `fn remaining_bytes(model: &CatalogModel, staging: &Path) -> Result<u64>` |
| function | `partial_bytes` | L521 | `fn partial_bytes(path: &Path, file: &CatalogFile) -> u64` |
| function | `first_partial_bytes` | L536 | `fn first_partial_bytes(model: &CatalogModel, staging: &Path) -> u64` |
| function | `required_with_headroom` | L551 | `fn required_with_headroom(bytes: u64) -> Option<u64>` |
| function | `parse_df_available` | L556 | `fn parse_df_available(output: &str) -> Option<u64>` |
| function | `ensure_disk_space` | L565 | `fn ensure_disk_space(path: &Path, remaining: u64) -> Result<()>` |
| function | `validate_catalog_model` | L596 | `fn validate_catalog_model(model: &CatalogModel) -> Result<()>` |
| function | `reject_symlink` | L615 | `fn reject_symlink(path: &Path) -> Result<()>` |
| function | `reject_symlink_if_present` | L626 | `fn reject_symlink_if_present(path: &Path) -> Result<()>` |
| function | `prepare_child_dir` | L634 | `fn prepare_child_dir(parent: &Path, name: &str) -> Result<PathBuf>` |
| function | `recover_invalid_snapshot` | L660 | `fn recover_invalid_snapshot(model_root: &Path, model: &CatalogModel, final_dir: &Path, staging: &Path,) -> Result<()>` |
| function | `remove_exact_file_if_present` | L690 | `fn remove_exact_file_if_present(root: &Path, file: &Path) -> Result<()>` |
| function | `remove_exact_file` | L697 | `fn remove_exact_file(root: &Path, file: &Path) -> Result<()>` |
| function | `catalog_is_exact_role_filtered_and_valid` | L713 | `fn catalog_is_exact_role_filtered_and_valid() -> ()` |
| function | `paths_are_fixed_below_managed_root` | L778 | `fn paths_are_fixed_below_managed_root() -> ()` |
| function | `disk_math_parser_and_overflow_fail_closed` | L798 | `fn disk_math_parser_and_overflow_fail_closed() -> ()` |
| function | `tiny_install_resumes_verifies_and_publishes` | L822 | `fn tiny_install_resumes_verifies_and_publishes() -> ()` |
| function | `cancel_preserves_partial_and_bad_download_removes_only_bad_file` | L861 | `fn cancel_preserves_partial_and_bad_download_removes_only_bad_file() -> ()` |
| function | `complete_corrupt_partial_is_restarted_and_fast_marker_is_exact` | L912 | `fn complete_corrupt_partial_is_restarted_and_fast_marker_is_exact() -> ()` |
| function | `invalid_final_recovers_verified_files_without_broken_snapshots` | L956 | `fn invalid_final_recovers_verified_files_without_broken_snapshots() -> ()` |
