---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_3c939c2ef666"
source_path: "overlay-backend/src/ai/types.rs"
batch_id: "B06"
total_lines: 124
symbols_count: 8
review_state: validated
---

# File Map: `overlay-backend/src/ai/types.rs`

- **Batch:** B06
- **Physical Lines:** 124
- **Coverage:** 124/124 lines (100%)

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
