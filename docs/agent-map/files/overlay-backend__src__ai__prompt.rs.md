---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_9a2579ad9e5a"
source_path: "overlay-backend/src/ai/prompt.rs"
batch_id: "B06"
total_lines: 126
symbols_count: 1
review_state: validated
---

# File Map: `overlay-backend/src/ai/prompt.rs`

- **Batch:** B06
- **Physical Lines:** 126
- **Coverage:** 126/126 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (1)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `build_request` | L5 | `fn build_request(meeting_context: &str, response_language: &str, transcript_lines: &[String], screenshot_data_url: Option<&str>, user_question: Option<&str>,) -> Vec<ChatMessage>` |
