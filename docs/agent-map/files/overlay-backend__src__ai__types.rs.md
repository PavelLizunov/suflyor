---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3c939c2ef666"
source_path: "overlay-backend/src/ai/types.rs"
batch_id: "B06"
total_lines: 124
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "ada2697adf16919c5df45fd4601d7e88ec03d69f6874dd538da27b6d653376ba"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ai/types.rs`

- **Batch:** B06
- **Physical Lines:** 124
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (7)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `AiProtocol` | L7 | pub |
| enum | `AiEvent` | L94 | pub |
| enum | `MessageContent` | L109 | pub |
| enum | `ContentPart` | L116 | pub |
| struct | `AiEndpoint` | L49 | pub |
| struct | `ChatMessage` | L102 | pub |
| struct | `ImageUrl` | L122 | pub |

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `label` | L18 | `fn label(self) -> &'static str` |
| function | `supports_model_listing` | L28 | `fn supports_model_listing(self) -> bool` |
| function | `supports_prompt_cache_control` | L36 | `fn supports_prompt_cache_control(self) -> bool` |
| function | `supports_live_answers` | L41 | `fn supports_live_answers(self) -> bool` |
| function | `requires_bearer` | L62 | `fn requires_bearer(&self) -> bool` |
| function | `is_unmetered` | L67 | `fn is_unmetered(&self) -> bool` |
| function | `accepts_images` | L72 | `fn accepts_images(&self) -> bool` |
| function | `fmt` | L78 | `fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` |
