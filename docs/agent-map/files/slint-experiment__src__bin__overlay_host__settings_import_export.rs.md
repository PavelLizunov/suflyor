---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1f67bc62d5c0"
source_path: "slint-experiment/src/bin/overlay_host/settings_import_export.rs"
batch_id: "B02"
total_lines: 253
symbols_count: 3
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "7e2c6085fe927c60d7541b91a5763a0618215ddf9515a0dbe367ef3a7bb05057"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/settings_import_export.rs`

- **Batch:** B02
- **Physical Lines:** 253
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `wire_import_export` | L48 | `fn wire_import_export(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig, pending: &Rc<RefCell<Option<overlay_backend::config::Config>>>, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `apply_server_preview` | L186 | `fn apply_server_preview(win: &SettingsWindow, p: &overlay_backend::config::ServerSettingsPreview,) -> ()` |
| function | `gigaam_preview_path_redacts_user_home_directory` | L243 | `fn gigaam_preview_path_redacts_user_home_directory() -> ()` |
