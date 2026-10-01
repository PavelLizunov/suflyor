---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_58fd0e4d2e6f"
source_path: "suflyor-teratts/src/protocol.rs"
batch_id: "B12"
total_lines: 332
symbols_count: 14
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "0535d3876c6f96b0552fed195b5d57f451423a5e7c4deb58fdb31e8aa8e34d9e"
source_matches_reconciliation_baseline: true
---

# File Map: `suflyor-teratts/src/protocol.rs`

- **Batch:** B12
- **Physical Lines:** 332
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (3)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `Cmd` | L21 | pub |
| enum | `RejectReason` | L44 | pub |
| enum | `Event` | L146 | pub |

## Symbols & Routines (14)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `token` | L57 | `fn token(self) -> &'static str` |
| function | `parse_cmd` | L74 | `fn parse_cmd(line: &str) -> Result<Option<Cmd>, RejectReason>` |
| function | `to_line` | L175 | `fn to_line(&self) -> String` |
| function | `b64` | L201 | `fn b64(text: &str) -> String` |
| function | `parses_speak_with_valid_base64` | L207 | `fn parses_speak_with_valid_base64() -> ()` |
| function | `rejects_invalid_base64` | L213 | `fn rejects_invalid_base64() -> ()` |
| function | `rejects_base64_that_is_not_utf8` | L225 | `fn rejects_base64_that_is_not_utf8() -> ()` |
| function | `parses_control_commands` | L235 | `fn parses_control_commands() -> ()` |
| function | `parses_rate_and_rejects_out_of_range` | L243 | `fn parses_rate_and_rejects_out_of_range() -> ()` |
| function | `parses_seek_and_playback_speed_with_strict_bounds` | L254 | `fn parses_seek_and_playback_speed_with_strict_bounds() -> ()` |
| function | `parses_voice_and_lang` | L269 | `fn parses_voice_and_lang() -> ()` |
| function | `unknown_commands_are_rejected` | L283 | `fn unknown_commands_are_rejected() -> ()` |
| function | `blank_lines_are_ignored` | L295 | `fn blank_lines_are_ignored() -> ()` |
| function | `event_lines_are_ascii_and_stable` | L301 | `fn event_lines_are_ascii_and_stable() -> ()` |
