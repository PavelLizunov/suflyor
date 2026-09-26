---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9f1e1f92292e"
source_path: "overlay-backend/src/teratts_install.rs"
batch_id: "B10"
total_lines: 934
symbols_count: 40
review_state: validated
---

# File Map: `overlay-backend/src/teratts_install.rs`

- **Batch:** B10
- **Physical Lines:** 934
- **Coverage:** 934/934 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `TeraInstalled` | L110 | pub |
| enum | `TeraProgress` | L161 | pub |
| struct | `Manifest` | L44 | pub |
| struct | `ManifestFile` | L52 | pub |

## Symbols & Routines (40)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `url_for` | L62 | `fn url_for(&self, file: &ManifestFile) -> String` |
| function | `manifest` | L70 | `fn manifest() -> Result<Manifest>` |
| function | `manifest_from_json` | L74 | `fn manifest_from_json(raw: &str) -> Result<Manifest>` |
| function | `release_dir` | L100 | `fn release_dir() -> Option<PathBuf>` |
| function | `installed_state` | L121 | `fn installed_state() -> TeraInstalled` |
| function | `check_dir` | L140 | `fn check_dir(manifest: &Manifest, dir: &Path) -> Result<()>` |
| function | `wipe_staging` | L286 | `fn wipe_staging(staging: &Path) -> ()` |
| function | `is_within` | L298 | `fn is_within(parent: &Path, child: &Path) -> bool` |
| function | `component_eq` | L312 | `fn component_eq(a: &Component<'_>, b: &Component<'_>) -> bool` |
| function | `quarantine_broken_release` | L327 | `fn quarantine_broken_release(tts_root: &Path, release: &Path, revision: &str) -> Result<()>` |
| function | `sweep_quarantined` | L360 | `fn sweep_quarantined(tts_root: &Path) -> ()` |
| function | `verify_file` | L385 | `fn verify_file(entry: &ManifestFile, path: &Path) -> Result<()>` |
| function | `pinned` | L447 | `fn pinned() -> Manifest` |
| function | `pinned_manifest_is_immutable_and_complete` | L452 | `fn pinned_manifest_is_immutable_and_complete() -> ()` |
| function | `pinned_manifest_ships_all_ten_voices_and_core_graphs` | L476 | `fn pinned_manifest_ships_all_ten_voices_and_core_graphs() -> ()` |
| function | `manifest_rejects_unsafe_paths_and_missing_pins` | L509 | `fn manifest_rejects_unsafe_paths_and_missing_pins() -> ()` |
| function | `blob_sha1_of_abc` | L522 | `fn blob_sha1_of_abc() -> String` |
| function | `tiny_manifest` | L529 | `fn tiny_manifest() -> Manifest` |
| function | `write_test_files` | L550 | `fn write_test_files(staging_like: &Path) -> ()` |
| function | `verify_file_accepts_matching_content_and_rejects_tampering` | L558 | `fn verify_file_accepts_matching_content_and_rejects_tampering() -> ()` |
| function | `install_with_downloads_verifies_and_publishes_atomically` | L582 | `fn install_with_downloads_verifies_and_publishes_atomically() -> ()` |
| function | `install_with_wipes_staging_on_hash_mismatch` | L616 | `fn install_with_wipes_staging_on_hash_mismatch() -> ()` |
| function | `install_with_honours_cancel_between_files` | L641 | `fn install_with_honours_cancel_between_files() -> ()` |
| function | `install_with_resumes_verified_staged_files` | L658 | `fn install_with_resumes_verified_staged_files() -> ()` |
| function | `install_with_is_idempotent_once_installed` | L683 | `fn install_with_is_idempotent_once_installed() -> ()` |
| function | `installed_state_tracks_marker_and_files` | L707 | `fn installed_state_tracks_marker_and_files() -> ()` |
| function | `installed_state_for_missing_dir` | L714 | `fn installed_state_for_missing_dir() -> TeraInstalled` |
| function | `ok_downloader` | L726 | `fn ok_downloader(_url: &str, dest: &Path) -> Result<()>` |
| function | `release_path` | L731 | `fn release_path(root: &Path, manifest: &Manifest) -> PathBuf` |
| function | `quarantine_dirs` | L735 | `fn quarantine_dirs(root: &Path) -> Vec<String>` |
| function | `is_within_rejects_prefix_neighbours_traversal_and_drives` | L748 | `fn is_within_rejects_prefix_neighbours_traversal_and_drives() -> ()` |
| function | `is_within_understands_drive_prefixes_and_case` | L770 | `fn is_within_understands_drive_prefixes_and_case() -> ()` |
| function | `quarantine_moves_the_broken_dir_within_the_same_root` | L780 | `fn quarantine_moves_the_broken_dir_within_the_same_root() -> ()` |
| function | `quarantine_names_are_unique_and_refuse_foreign_paths` | L796 | `fn quarantine_names_are_unique_and_refuse_foreign_paths() -> ()` |
| function | `install_with_self_heals_release_missing_its_marker` | L819 | `fn install_with_self_heals_release_missing_its_marker() -> ()` |
| function | `install_with_self_heals_release_with_corrupt_file` | L837 | `fn install_with_self_heals_release_with_corrupt_file() -> ()` |
| function | `install_with_self_heals_nonempty_junk_release_dir` | L855 | `fn install_with_self_heals_nonempty_junk_release_dir() -> ()` |
| function | `install_with_preserves_a_valid_release_and_makes_no_quarantine` | L872 | `fn install_with_preserves_a_valid_release_and_makes_no_quarantine() -> ()` |
| function | `install_with_cancel_leaves_broken_release_for_the_next_run` | L899 | `fn install_with_cancel_leaves_broken_release_for_the_next_run() -> ()` |
| function | `sweep_quarantined_removes_only_quarantine_dirs` | L921 | `fn sweep_quarantined_removes_only_quarantine_dirs() -> ()` |
