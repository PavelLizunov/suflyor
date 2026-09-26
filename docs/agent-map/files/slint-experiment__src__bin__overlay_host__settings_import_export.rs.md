---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1f67bc62d5c0"
source_path: "slint-experiment/src/bin/overlay_host/settings_import_export.rs"
batch_id: "B02"
total_lines: 253
symbols_count: 3
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_import_export.rs`

- **Batch:** B02
- **Physical Lines:** 253
- **Coverage:** 253/253 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `wire_import_export` | L48 | `fn wire_import_export(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig, pending: &Rc<RefCell<Option<overlay_backend::config::Config>>>, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `apply_server_preview` | L186 | `fn apply_server_preview(win: &SettingsWindow, p: &overlay_backend::config::ServerSettingsPreview,) -> ()` |
| function | `gigaam_preview_path_redacts_user_home_directory` | L243 | `fn gigaam_preview_path_redacts_user_home_directory() -> ()` |
