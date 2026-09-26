---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8f597238d520"
source_path: "slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs"
batch_id: "B03"
total_lines: 248
symbols_count: 6
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs`

- **Batch:** B03
- **Physical Lines:** 248
- **Coverage:** 248/248 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `that` | L209 | private |
| trait | `PaletteResultExt` | L236 | private |

## Symbols & Routines (6)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `open_help` | L6 | `fn open_help(slot_ref: &Rc<RefCell<Option<HelpWindow>>>, overlay_weak: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `open_palette` | L84 | `fn open_palette(palette_ref: &Rc<RefCell<Option<PaletteWindow>>>, tiles_ref: &TileWindows, state: &slint_replay::app_state::SharedState, weak_overlay: &slint::Weak<OverlayBarWindow>,) -> ()` |
| function | `results_index` | L200 | `fn results_index(model: &slint::ModelRc<PaletteResult>, idx: i32) -> Option<PaletteResult>` |
| function | `kb_to_palette_results` | L210 | `fn kb_to_palette_results(entries: &[kb::KBEntry]) -> Vec<PaletteResult>` |
| function | `heading_or_key` | L237 | `fn heading_or_key(&self) -> String` |
| function | `heading_or_key` | L241 | `fn heading_or_key(&self) -> String` |
