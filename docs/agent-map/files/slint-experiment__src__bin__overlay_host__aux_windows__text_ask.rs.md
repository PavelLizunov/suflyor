---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_239f8b0ec857"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows/text_ask.rs"
batch_id: "B03"
total_lines: 197
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "1a3273f718745baf459a5182d42ce1471585b91f07bfe482daf7a3edee1148b1"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows/text_ask.rs`

- **Batch:** B03
- **Physical Lines:** 197
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `text_ask_profile_label` | L9 | `fn text_ask_profile_label(cfg: &overlay_backend::config::SharedConfig) -> String` |
| function | `text_ask_profile_label_for` | L18 | `fn text_ask_profile_label_for(active_profile: Option<&str>, has_custom_context: bool, is_ru: bool,) -> String` |
| function | `persist_text_ask_pos` | L42 | `fn persist_text_ask_pos(win: &TextAskWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |
| function | `open_text_ask` | L64 | `fn open_text_ask(slot_ref: &Rc<RefCell<Option<TextAskWindow>>>, bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, tiles: &TileWindows, weak_overlay: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `text_ask_profile_label_follows_ui_language` | L183 | `fn text_ask_profile_label_follows_ui_language() -> ()` |
