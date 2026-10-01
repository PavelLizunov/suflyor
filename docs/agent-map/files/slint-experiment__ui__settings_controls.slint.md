---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_615bd21f0875"
source_path: "slint-experiment/ui/settings_controls.slint"
batch_id: "B05"
total_lines: 185
symbols_count: 26
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "d665ac5cd85474fd39a73ddb722129178b101b1bc90790ef687b89c3fc9a5717"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/ui/settings_controls.slint`

- **Batch:** B05
- **Physical Lines:** 185
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (9)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `IconSlot` | L7 | - |
| slint_component | `BrandHeaderMark` | L28 | - |
| slint_component | `DarkCheck` | L45 | - |
| slint_component | `SettingsCard` | L92 | - |
| slint_component | `SettingsComboBox` | L115 | - |
| slint_component | `SettingsLineEdit` | L119 | - |
| slint_component | `SettingsSlider` | L123 | - |
| slint_component | `NavGroupHeader` | L127 | - |
| slint_component | `NavItem` | L143 | - |

## Symbols & Routines (26)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `IconSlot` | L7 | `-` |
| ui_component | `BrandHeaderMark` | L28 | `-` |
| ui_component | `DarkCheck` | L45 | `-` |
| ui_component | `SettingsCard` | L92 | `-` |
| ui_component | `SettingsComboBox` | L115 | `-` |
| ui_component | `SettingsLineEdit` | L119 | `-` |
| ui_component | `SettingsSlider` | L123 | `-` |
| ui_component | `NavGroupHeader` | L127 | `-` |
| ui_component | `NavItem` | L143 | `-` |
| slint_property | `icon` | L9 | `image` |
| slint_property | `tint` | L10 | `color` |
| slint_property | `glyph-size` | L11 | `length` |
| slint_property | `slot-width` | L12 | `length` |
| slint_property | `slot-height` | L13 | `length` |
| slint_property | `mark-size` | L30 | `length` |
| slint_property | `text` | L47 | `string` |
| slint_property | `checked` | L48 | `bool` |
| slint_property | `enabled` | L49 | `bool` |
| slint_property | `title` | L94 | `string` |
| slint_property | `label` | L129 | `string` |
| slint_property | `label` | L145 | `string` |
| slint_property | `icon` | L146 | `image` |
| slint_property | `tab-index` | L147 | `int` |
| slint_property | `active` | L148 | `bool` |
| slint_callback | `toggled` | L50 | `-` |
| slint_callback | `chosen` | L149 | `-` |

## Heuristic behavior and concurrency matches

- UI Callback `toggled` declared at L50
- UI Callback `chosen` declared at L149
