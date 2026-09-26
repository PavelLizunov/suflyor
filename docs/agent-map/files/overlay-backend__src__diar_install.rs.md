---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_4233de2c705d"
source_path: "overlay-backend/src/diar_install.rs"
batch_id: "B07"
total_lines: 820
symbols_count: 39
review_state: validated
---

# File Map: `overlay-backend/src/diar_install.rs`

- **Batch:** B07
- **Physical Lines:** 820
- **Coverage:** 820/820 lines (100%)

## Types & Structures (5)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `DiarProgress` | L106 | pub |
| struct | `InstallGuard` | L33 | private |
| struct | `DiarModel` | L52 | private |
| struct | `Sentinel` | L120 | private |
| struct | `BusyReset` | L454 | private |

## Symbols & Routines (39)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `drop` | L36 | `fn drop(&mut self) -> ()` |
| function | `try_acquire_install` | L41 | `fn try_acquire_install() -> Result<InstallGuard>` |
| function | `expected_sentinel` | L127 | `fn expected_sentinel() -> Sentinel` |
| function | `diar_dir` | L140 | `fn diar_dir() -> Option<PathBuf>` |
| function | `seg_model_path` | L146 | `fn seg_model_path() -> Option<PathBuf>` |
| function | `emb_model_path` | L152 | `fn emb_model_path() -> Option<PathBuf>` |
| function | `models_installed` | L163 | `fn models_installed() -> bool` |
| function | `models_installed_in` | L171 | `fn models_installed_in(root: &Path) -> bool` |
| function | `sentinel_valid` | L177 | `fn sentinel_valid(root: &Path) -> bool` |
| function | `invalidate_sentinel` | L259 | `fn invalidate_sentinel(root: &Path) -> Result<()>` |
| function | `commit_staged` | L319 | `fn commit_staged(m: &DiarModel, root: &Path, stage: &Path) -> Result<()>` |
| function | `write_sentinel` | L342 | `fn write_sentinel(root: &Path) -> Result<()>` |
| function | `clean_stale` | L362 | `fn clean_stale(root: &Path) -> ()` |
| function | `marker_top` | L382 | `fn marker_top(m: &DiarModel) -> &str` |
| function | `nemotron_model_path` | L395 | `fn nemotron_model_path() -> Option<PathBuf>` |
| function | `nemotron_installed` | L401 | `fn nemotron_installed() -> bool` |
| function | `nemotron_model_digest_ok` | L416 | `fn nemotron_model_digest_ok(path: &Path) -> Result<bool>` |
| function | `nemotron_installed_in` | L435 | `fn nemotron_installed_in(root: &Path) -> bool` |
| function | `install_nemotron` | L447 | `fn install_nemotron() -> Result<()>` |
| function | `drop` | L456 | `fn drop(&mut self) -> ()` |
| function | `nemotron_rejects_tampered_equal_length_model` | L505 | `fn nemotron_rejects_tampered_equal_length_model() -> ()` |
| function | `force_seg_marker` | L521 | `fn force_seg_marker(root: &Path) -> ()` |
| function | `force_emb_marker` | L528 | `fn force_emb_marker(root: &Path) -> ()` |
| function | `force_both_markers` | L532 | `fn force_both_markers(root: &Path) -> ()` |
| function | `pins_and_layout_are_valid` | L538 | `fn pins_and_layout_are_valid() -> ()` |
| function | `marker_top_is_the_extract_dir_or_the_file` | L566 | `fn marker_top_is_the_extract_dir_or_the_file() -> ()` |
| function | `empty_root_is_not_installed` | L579 | `fn empty_root_is_not_installed() -> ()` |
| function | `markers_without_sentinel_are_not_installed` | L585 | `fn markers_without_sentinel_are_not_installed() -> ()` |
| function | `staged_partial_tree_never_reports_installed` | L595 | `fn staged_partial_tree_never_reports_installed() -> ()` |
| function | `valid_sentinel_with_full_set_is_installed` | L619 | `fn valid_sentinel_with_full_set_is_installed() -> ()` |
| function | `valid_sentinel_with_a_missing_marker_is_not_installed` | L627 | `fn valid_sentinel_with_a_missing_marker_is_not_installed() -> ()` |
| function | `reinstall_invalidates_old_sentinel_until_final_commit` | L635 | `fn reinstall_invalidates_old_sentinel_until_final_commit() -> ()` |
| function | `stale_or_garbage_sentinel_is_not_installed` | L651 | `fn stale_or_garbage_sentinel_is_not_installed() -> ()` |
| function | `already_installed_is_a_no_op` | L682 | `fn already_installed_is_a_no_op() -> ()` |
| function | `install_guard_allows_only_one_writer` | L709 | `fn install_guard_allows_only_one_writer() -> ()` |
| function | `commit_staged_swaps_a_complete_tree_over_an_old_one` | L717 | `fn commit_staged_swaps_a_complete_tree_over_an_old_one() -> ()` |
| function | `commit_staged_rejects_a_partial_extraction` | L752 | `fn commit_staged_rejects_a_partial_extraction() -> ()` |
| function | `clean_stale_sweeps_staging_and_temp_downloads_only` | L781 | `fn clean_stale_sweeps_staging_and_temp_downloads_only() -> ()` |
| function | `sentinel_roundtrip_is_atomic_and_verifies` | L802 | `fn sentinel_roundtrip_is_atomic_and_verifies() -> ()` |
