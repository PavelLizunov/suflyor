---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_8c6791371210"
source_path: "slint-experiment/src/bin/overlay_host/kbd_shortcuts.rs"
batch_id: "B04"
total_lines: 111
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "cd73732276270572f2b4686aead8fa2afcd8a0e63d2247436663736372ea03ea"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/src/bin/overlay_host/kbd_shortcuts.rs`

- **Batch:** B04
- **Physical Lines:** 111
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `install` | L29 | `fn install(win: &slint::Window) -> ()` |
| function | `key_down` | L77 | `fn key_down(vk: VIRTUAL_KEY) -> bool` |
| function | `vk_to_letter` | L85 | `fn vk_to_letter(vk: u32) -> Option<char>` |
| function | `install` | L93 | `fn install(_win: &slint::Window) -> ()` |
| function | `vk_to_letter_maps_letters_only` | L101 | `fn vk_to_letter_maps_letters_only() -> ()` |
