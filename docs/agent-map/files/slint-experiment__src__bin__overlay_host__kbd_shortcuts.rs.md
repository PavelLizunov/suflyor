---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8c6791371210"
source_path: "slint-experiment/src/bin/overlay_host/kbd_shortcuts.rs"
batch_id: "B04"
total_lines: 111
symbols_count: 5
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/kbd_shortcuts.rs`

- **Batch:** B04
- **Physical Lines:** 111
- **Coverage:** 111/111 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `install` | L29 | `fn install(win: &slint::Window) -> ()` |
| function | `key_down` | L77 | `fn key_down(vk: VIRTUAL_KEY) -> bool` |
| function | `vk_to_letter` | L85 | `fn vk_to_letter(vk: u32) -> Option<char>` |
| function | `install` | L93 | `fn install(_win: &slint::Window) -> ()` |
| function | `vk_to_letter_maps_letters_only` | L101 | `fn vk_to_letter_maps_letters_only() -> ()` |
