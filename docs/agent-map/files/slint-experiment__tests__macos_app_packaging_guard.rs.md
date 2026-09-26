---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3ed0aae709a6"
source_path: "slint-experiment/tests/macos_app_packaging_guard.rs"
batch_id: "B05"
total_lines: 343
symbols_count: 10
review_state: validated
---

# File Map: `slint-experiment/tests/macos_app_packaging_guard.rs`

- **Batch:** B05
- **Physical Lines:** 343
- **Coverage:** 343/343 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `has_plist_value` | L13 | `fn has_plist_value(key: &str, value: &str) -> bool` |
| function | `manifest_keeps_the_production_macos_identity` | L21 | `fn manifest_keeps_the_production_macos_identity() -> ()` |
| function | `plist_audio_capture_purpose_string_is_non_empty` | L57 | `fn plist_audio_capture_purpose_string_is_non_empty() -> ()` |
| function | `plist_microphone_purpose_string_is_non_empty` | L75 | `fn plist_microphone_purpose_string_is_non_empty() -> ()` |
| function | `entitlements_grant_microphone_access` | L93 | `fn entitlements_grant_microphone_access() -> ()` |
| function | `script_builds_and_ad_hoc_signs_the_app` | L105 | `fn script_builds_and_ad_hoc_signs_the_app() -> ()` |
| function | `mlx_metallib_is_built_from_the_audited_pinned_sources` | L204 | `fn mlx_metallib_is_built_from_the_audited_pinned_sources() -> ()` |
| function | `script_stays_free_local_packaging` | L239 | `fn script_stays_free_local_packaging() -> ()` |
| function | `dmg_script_uses_the_native_drag_install_layout` | L247 | `fn dmg_script_uses_the_native_drag_install_layout() -> ()` |
| function | `windows_updater_is_absent_but_backup_remains_on_the_macos_settings_surface` | L332 | `fn windows_updater_is_absent_but_backup_remains_on_the_macos_settings_surface() -> ()` |
