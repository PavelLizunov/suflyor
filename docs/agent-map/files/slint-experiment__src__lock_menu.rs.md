---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_ec1fbe23bacf"
source_path: "slint-experiment/src/lock_menu.rs"
batch_id: "B01"
total_lines: 166
symbols_count: 15
review_state: validated
---

# File Map: `slint-experiment/src/lock_menu.rs`

- **Batch:** B01
- **Physical Lines:** 166
- **Coverage:** 166/166 lines (100%)

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Phase` | L4 | private |
| enum | `FocusTransition` | L13 | pub |
| struct | `FocusState` | L21 | pub |

## Symbols & Routines (15)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `begin_open` | L28 | `fn begin_open(&mut self) -> u64` |
| function | `close` | L34 | `fn close(&mut self) -> ()` |
| function | `is_current_open` | L38 | `fn is_current_open(&self, generation: u64) -> bool` |
| function | `mark_revealed` | L42 | `fn mark_revealed(&mut self, generation: u64) -> bool` |
| function | `arm` | L53 | `fn arm(&mut self, generation: u64) -> bool` |
| function | `focus_changed` | L62 | `fn focus_changed(&mut self, focused: bool) -> FocusTransition` |
| function | `suppress_next_open` | L77 | `fn suppress_next_open(&mut self) -> ()` |
| function | `consume_suppressed_open` | L81 | `fn consume_suppressed_open(&mut self) -> bool` |
| function | `clear_suppressed_open` | L85 | `fn clear_suppressed_open(&mut self) -> ()` |
| function | `diagnostic_snapshot` | L89 | `fn diagnostic_snapshot(&self) -> (&'static str, u64)` |
| function | `open_and_arm` | L106 | `fn open_and_arm(state: &mut FocusState) -> u64` |
| function | `initial_focus_noise_cannot_dismiss_but_outside_focus_loss_does` | L114 | `fn initial_focus_noise_cannot_dismiss_but_outside_focus_loss_does() -> ()` |
| function | `second_open_arms_even_without_a_second_focus_gain` | L134 | `fn second_open_arms_even_without_a_second_focus_gain() -> ()` |
| function | `owner_focus_loss_swallow_prevents_chip_reopen_race` | L147 | `fn owner_focus_loss_swallow_prevents_chip_reopen_race() -> ()` |
| function | `stale_reveal_cannot_arm_a_new_open` | L157 | `fn stale_reveal_cannot_arm_a_new_open() -> ()` |
