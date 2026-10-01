---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_09da0f5f23f2"
source_path: "slint-experiment/src/bin/overlay_host/read_aloud.rs"
batch_id: "B03"
total_lines: 263
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "e966ac760521c054dad26bab867c5333562b0c8551a5b2e7f02517ae96b222b1"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/read_aloud.rs`

- **Batch:** B03
- **Physical Lines:** 263
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `restore_text_clipboard` | L27 | `fn restore_text_clipboard(saved: &Option<String>) -> ()` |
| function | `ocr_source_label` | L185 | `fn ocr_source_label() -> &'static str` |
| function | `fill_ocr_tile` | L197 | `fn fill_ocr_tile(weak: slint::Weak<TileWindow>, convo_id: i32, bridge: &Arc<OverlayBarBridge>, text: &str, ui_is_ru: bool,) -> ()` |
| function | `fill_ocr_error_tile` | L248 | `fn fill_ocr_error_tile(weak: slint::Weak<TileWindow>, ui_is_ru: bool) -> ()` |
