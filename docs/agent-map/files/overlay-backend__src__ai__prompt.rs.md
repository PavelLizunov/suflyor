---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9a2579ad9e5a"
source_path: "overlay-backend/src/ai/prompt.rs"
batch_id: "B06"
total_lines: 126
symbols_count: 1
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "f9b573d67ea7a0b3f56bbc470eca67fb63761ada719bde860ba7db8f87246425"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ai/prompt.rs`

- **Batch:** B06
- **Physical Lines:** 126
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `build_request` | L5 | `fn build_request(meeting_context: &str, response_language: &str, transcript_lines: &[String], screenshot_data_url: Option<&str>, user_question: Option<&str>,) -> Vec<ChatMessage>` |
