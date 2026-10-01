---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e4328a193fca"
source_path: "slint-experiment/tests/tts_speech_policy_guard.rs"
batch_id: "B05"
total_lines: 85
symbols_count: 4
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "597bf20b2e69a8526657597220b8d55929699297cdd5db34c6b773afa7946600"
source_matches_reconciliation_baseline: true
---

# File Map: `slint-experiment/tests/tts_speech_policy_guard.rs`

- **Batch:** B05
- **Physical Lines:** 85
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (4)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `between` | L7 | `fn between(source: &'a str, start: &str, end: &str) -> &'a str` |
| function | `explicit_speech_replaces_the_current_utterance` | L13 | `fn explicit_speech_replaces_the_current_utterance() -> ()` |
| function | `settings_voice_test_reports_and_resets_a_generic_failure` | L39 | `fn settings_voice_test_reports_and_resets_a_generic_failure() -> ()` |
| function | `tile_speaker_tracks_backend_availability_and_rejection` | L55 | `fn tile_speaker_tracks_backend_availability_and_rejection() -> ()` |
