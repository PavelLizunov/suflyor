---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_239f8b0ec857"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows/text_ask.rs"
batch_id: "B03"
total_lines: 197
symbols_count: 5
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows/text_ask.rs`

- **Batch:** B03
- **Physical Lines:** 197
- **Coverage:** 197/197 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `text_ask_profile_label` | L9 | `fn text_ask_profile_label(cfg: &overlay_backend::config::SharedConfig) -> String` |
| function | `text_ask_profile_label_for` | L18 | `fn text_ask_profile_label_for(active_profile: Option<&str>, has_custom_context: bool, is_ru: bool,) -> String` |
| function | `persist_text_ask_pos` | L42 | `fn persist_text_ask_pos(win: &TextAskWindow, cfg: &overlay_backend::config::SharedConfig) -> ()` |
| function | `open_text_ask` | L64 | `fn open_text_ask(slot_ref: &Rc<RefCell<Option<TextAskWindow>>>, bridge: &Arc<OverlayBarBridge>, events: &Arc<dyn RuntimeEvents>, cfg: &overlay_backend::config::SharedConfig, slint_rt: &SharedSlintRuntime, rt_handle: &tokio::runtime::Handle, tiles: &TileWindows, weak_overlay: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `text_ask_profile_label_follows_ui_language` | L183 | `fn text_ask_profile_label_follows_ui_language() -> ()` |
