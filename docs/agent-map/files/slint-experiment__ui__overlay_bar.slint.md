---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_1063a095537e"
source_path: "slint-experiment/ui/overlay_bar.slint"
batch_id: "B05"
total_lines: 1504
symbols_count: 134
review_state: validated
---

# File Map: `slint-experiment/ui/overlay_bar.slint`

- **Batch:** B05
- **Physical Lines:** 1504
- **Coverage:** 1504/1504 lines (100%)

## Types & Structures (9)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| slint_component | `IconSlot` | L21 | - |
| slint_component | `BrandStateMark` | L42 | - |
| slint_component | `Chip` | L65 | - |
| slint_component | `LockModeMenuRow` | L132 | - |
| slint_component | `LockChip` | L183 | - |
| slint_component | `LiveChip` | L269 | - |
| slint_component | `RecordBtn` | L325 | - |
| slint_component | `VDivider` | L450 | - |
| slint_component | `OverlayBarWindow` | L457 | - |

## Symbols & Routines (134)

| Kind | Name | Line | Signature |
|---|---|---|---|
| ui_component | `IconSlot` | L21 | `-` |
| ui_component | `BrandStateMark` | L42 | `-` |
| ui_component | `Chip` | L65 | `-` |
| ui_component | `LockModeMenuRow` | L132 | `-` |
| ui_component | `LockChip` | L183 | `-` |
| ui_component | `LiveChip` | L269 | `-` |
| ui_component | `RecordBtn` | L325 | `-` |
| ui_component | `VDivider` | L450 | `-` |
| ui_component | `OverlayBarWindow` | L457 | `-` |
| slint_property | `force-arrow` | L17 | `bool` |
| slint_property | `icon` | L23 | `image` |
| slint_property | `tint` | L24 | `color` |
| slint_property | `glyph-size` | L25 | `length` |
| slint_property | `slot-width` | L26 | `length` |
| slint_property | `slot-height` | L27 | `length` |
| slint_property | `assistant-active` | L44 | `bool` |
| slint_property | `mark-size` | L45 | `length` |
| slint_property | `label` | L66 | `string` |
| slint_property | `icon` | L71 | `image` |
| slint_property | `has-icon` | L72 | `bool` |
| slint_property | `a11y` | L73 | `string` |
| slint_property | `active` | L74 | `bool` |
| slint_property | `accent` | L75 | `color` |
| slint_property | `mono` | L78 | `bool` |
| slint_property | `label` | L133 | `string` |
| slint_property | `current` | L134 | `bool` |
| slint_property | `mode-color` | L138 | `color` |
| slint_property | `unlocked` | L139 | `bool` |
| slint_property | `locked` | L185 | `bool` |
| slint_property | `unlocked` | L186 | `bool` |
| slint_property | `deep` | L190 | `bool` |
| slint_property | `a11y` | L191 | `string` |
| slint_property | `managed` | L192 | `bool` |
| slint_property | `tight` | L197 | `bool` |
| slint_property | `mode` | L199 | `int` |
| slint_property | `accent` | L200 | `color` |
| slint_property | `phase` | L203 | `bool` |
| slint_property | `label` | L270 | `string` |
| slint_property | `icon` | L271 | `image` |
| slint_property | `has-icon` | L272 | `bool` |
| slint_property | `a11y` | L273 | `string` |
| slint_property | `active` | L274 | `bool` |
| slint_property | `accent` | L275 | `color` |
| slint_property | `label` | L326 | `string` |
| slint_property | `icon` | L328 | `image` |
| slint_property | `has-icon` | L329 | `bool` |
| slint_property | `accent` | L330 | `color` |
| slint_property | `recording` | L331 | `bool` |
| slint_property | `locked` | L343 | `bool` |
| slint_property | `press-y` | L344 | `length` |
| slint_property | `swallow-up` | L345 | `bool` |
| slint_property | `lock-dy` | L349 | `length` |
| slint_property | `widgets-light` | L461 | `bool` |
| slint_property | `status-text` | L506 | `string` |
| slint_property | `status-color` | L507 | `color` |
| slint_property | `mic-active` | L510 | `bool` |
| slint_property | `mic-muted` | L513 | `bool` |
| slint_property | `sys-active` | L514 | `bool` |
| slint_property | `timer-label` | L517 | `string` |
| slint_property | `timer-active` | L518 | `bool` |
| slint_property | `session-paused` | L521 | `bool` |
| slint_property | `session-name` | L525 | `string` |
| slint_property | `active-stack` | L530 | `string` |
| slint_property | `tok-per-sec` | L534 | `string` |
| slint_property | `app-memory` | L537 | `string` |
| slint_property | `mlx-memory` | L538 | `string` |
| slint_property | `tiles-spawned` | L541 | `int` |
| slint_property | `open-tiles` | L548 | `int` |
| slint_property | `can-restore-tile` | L550 | `bool` |
| slint_property | `aggressive-active` | L557 | `bool` |
| slint_property | `stealth-active` | L559 | `bool` |
| slint_property | `stealth-fault` | L569 | `bool` |
| slint_property | `presented-status` | L570 | `string` |
| slint_property | `last-transcript-line` | L579 | `string` |
| slint_property | `ai-streaming` | L580 | `bool` |
| slint_property | `last-transcript-source` | L583 | `string` |
| slint_property | `mic-recording` | L587 | `bool` |
| slint_property | `sys-recording` | L588 | `bool` |
| slint_property | `settings-open` | L593 | `bool` |
| slint_property | `help-open` | L594 | `bool` |
| slint_property | `archive-open` | L597 | `bool` |
| slint_property | `quit-armed` | L602 | `bool` |
| slint_property | `restart-armed` | L604 | `bool` |
| slint_property | `confirming` | L613 | `bool` |
| slint_property | `tight-open-tiles` | L616 | `bool` |
| slint_property | `summary-busy` | L621 | `bool` |
| slint_property | `compact-bar` | L627 | `bool` |
| slint_property | `bootstrap-mode` | L630 | `bool` |
| slint_property | `mic-permission-state` | L633 | `int` |
| slint_property | `mic-capture-state` | L635 | `int` |
| slint_property | `tray-available` | L638 | `bool` |
| slint_property | `suppress-tiles` | L641 | `bool` |
| slint_property | `deep-lock` | L645 | `bool` |
| slint_property | `lock-icon-unlocked` | L647 | `bool` |
| slint_property | `lock-a11y` | L650 | `string` |
| slint_property | `lock-menu-managed` | L651 | `bool` |
| slint_callback | `clicked` | L79 | `-` |
| slint_callback | `clicked` | L140 | `-` |
| slint_callback | `mode-selected` | L201 | `-` |
| slint_callback | `menu-opened` | L202 | `-` |
| slint_callback | `clicked` | L276 | `-` |
| slint_callback | `pressed` | L332 | `-` |
| slint_callback | `released` | L333 | `-` |
| slint_callback | `mic-toggle-clicked` | L654 | `-` |
| slint_callback | `lock-menu-opened` | L655 | `-` |
| slint_callback | `sys-toggle-clicked` | L656 | `-` |
| slint_callback | `ptt-mic-pressed` | L658 | `-` |
| slint_callback | `ptt-mic-released` | L659 | `-` |
| slint_callback | `ptt-sys-pressed` | L660 | `-` |
| slint_callback | `ptt-sys-released` | L661 | `-` |
| slint_callback | `timer-toggle-clicked` | L662 | `-` |
| slint_callback | `pause-toggle-clicked` | L664 | `-` |
| slint_callback | `spawn-tile-clicked` | L665 | `-` |
| slint_callback | `text-ask-clicked` | L667 | `-` |
| slint_callback | `close-all-tiles-clicked` | L668 | `-` |
| slint_callback | `restore-tile-clicked` | L669 | `-` |
| slint_callback | `capture-clicked` | L670 | `-` |
| slint_callback | `stealth-toggle-clicked` | L671 | `-` |
| slint_callback | `aggressive-toggle-clicked` | L672 | `-` |
| slint_callback | `lock-mode-selected` | L673 | `-` |
| slint_callback | `open-settings-clicked` | L674 | `-` |
| slint_callback | `help-clicked` | L675 | `-` |
| slint_callback | `archive-clicked` | L676 | `-` |
| slint_callback | `summary-clicked` | L677 | `-` |
| slint_callback | `quit-clicked` | L678 | `-` |
| slint_callback | `restart-clicked` | L680 | `-` |
| slint_callback | `restart-confirm` | L681 | `-` |
| slint_callback | `restart-cancel` | L682 | `-` |
| slint_callback | `quit-confirm` | L683 | `-` |
| slint_callback | `quit-cancel` | L684 | `-` |
| slint_callback | `drag-start-requested` | L688 | `-` |
| slint_callback | `drag-moved` | L689 | `-` |
| slint_callback | `compact-toggle-clicked` | L691 | `-` |
| slint_callback | `hide-to-tray-clicked` | L694 | `-` |

## Key Behaviors & Concurrency

- UI Callback `clicked` declared at L79
- UI Callback `clicked` declared at L140
- UI Callback `mode-selected` declared at L201
- UI Callback `menu-opened` declared at L202
- UI Callback `clicked` declared at L276
- UI Callback `pressed` declared at L332
- UI Callback `released` declared at L333
- UI Callback `mic-toggle-clicked` declared at L654
- UI Callback `lock-menu-opened` declared at L655
- UI Callback `sys-toggle-clicked` declared at L656
- UI Callback `ptt-mic-pressed` declared at L658
- UI Callback `ptt-mic-released` declared at L659
- UI Callback `ptt-sys-pressed` declared at L660
- UI Callback `ptt-sys-released` declared at L661
- UI Callback `timer-toggle-clicked` declared at L662
- UI Callback `pause-toggle-clicked` declared at L664
- UI Callback `spawn-tile-clicked` declared at L665
- UI Callback `text-ask-clicked` declared at L667
- UI Callback `close-all-tiles-clicked` declared at L668
- UI Callback `restore-tile-clicked` declared at L669
- UI Callback `capture-clicked` declared at L670
- UI Callback `stealth-toggle-clicked` declared at L671
- UI Callback `aggressive-toggle-clicked` declared at L672
- UI Callback `lock-mode-selected` declared at L673
- UI Callback `open-settings-clicked` declared at L674
- UI Callback `help-clicked` declared at L675
- UI Callback `archive-clicked` declared at L676
- UI Callback `summary-clicked` declared at L677
- UI Callback `quit-clicked` declared at L678
- UI Callback `restart-clicked` declared at L680
- UI Callback `restart-confirm` declared at L681
- UI Callback `restart-cancel` declared at L682
- UI Callback `quit-confirm` declared at L683
- UI Callback `quit-cancel` declared at L684
- UI Callback `drag-start-requested` declared at L688
- UI Callback `drag-moved` declared at L689
- UI Callback `compact-toggle-clicked` declared at L691
- UI Callback `hide-to-tray-clicked` declared at L694
