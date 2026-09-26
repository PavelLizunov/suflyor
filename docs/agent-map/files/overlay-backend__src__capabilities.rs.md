---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_35e49a125b04"
source_path: "overlay-backend/src/capabilities.rs"
batch_id: "B09"
total_lines: 154
symbols_count: 5
review_state: validated
---

# File Map: `overlay-backend/src/capabilities.rs`

- **Batch:** B09
- **Physical Lines:** 154
- **Coverage:** 154/154 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `CapabilityState` | L7 | pub |
| enum | `PermissionKind` | L18 | pub |
| enum | `CapabilityReason` | L25 | pub |
| enum | `PlatformFeature` | L35 | pub |

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `platform_capability` | L47 | `fn platform_capability(feature: PlatformFeature) -> CapabilityState` |
| function | `state_category` | L79 | `fn state_category(state: CapabilityState) -> &'static str` |
| function | `every_state_has_a_distinct_category` | L92 | `fn every_state_has_a_distinct_category() -> ()` |
| function | `payloads_remain_part_of_state_equality` | L127 | `fn payloads_remain_part_of_state_equality() -> ()` |
| function | `platform_capability_reports_correct_states` | L143 | `fn platform_capability_reports_correct_states() -> ()` |
