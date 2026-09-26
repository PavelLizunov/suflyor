---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_2b377895783d"
source_path: "slint-experiment/src/bin/overlay_host/local_watchdog.rs"
batch_id: "B01"
total_lines: 125
symbols_count: 7
review_state: validated
---

# File Map: `slint-experiment/src/bin/overlay_host/local_watchdog.rs`

- **Batch:** B01
- **Physical Lines:** 125
- **Coverage:** 125/125 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `WatchdogState` | L22 | pub(super) |

## Symbols & Routines (7)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `should_restart` | L32 | `fn should_restart(&self, now: Instant, cooldown: Duration, max_fails: u32,) -> bool` |
| function | `note_reachable` | L45 | `fn note_reachable(&mut self) -> ()` |
| function | `note_attempt` | L51 | `fn note_attempt(&mut self, now: Instant, switched: bool) -> ()` |
| function | `first_attempt_allowed_immediately` | L71 | `fn first_attempt_allowed_immediately() -> ()` |
| function | `within_cooldown_skips_then_attempts_after` | L77 | `fn within_cooldown_skips_then_attempts_after() -> ()` |
| function | `fail_cap_stops_then_reachable_rearms` | L92 | `fn fail_cap_stops_then_reachable_rearms() -> ()` |
| function | `switched_resets_fail_count` | L116 | `fn switched_resets_fail_count() -> ()` |
