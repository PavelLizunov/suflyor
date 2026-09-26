---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_6ddbcc7df57f"
source_path: "overlay-backend/src/runtime/summary_plan.rs"
batch_id: "B06"
total_lines: 468
symbols_count: 14
review_state: validated
---

# File Map: `overlay-backend/src/runtime/summary_plan.rs`

- **Batch:** B06
- **Physical Lines:** 468
- **Coverage:** 468/468 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (14)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `managed_summary_context` | L21 | `fn managed_summary_context(is_local: bool, base_url: &str, prefer_quality: bool, context_config: &str,) -> Option<u32>` |
| function | `prompt_fits_context` | L34 | `fn prompt_fits_context(prompt_tokens: u64, max_tokens: u32, context_tokens: u32,) -> bool` |
| function | `message_text_chars` | L45 | `fn message_text_chars(messages: &[ai::ChatMessage]) -> usize` |
| function | `summary_request_fits` | L61 | `fn summary_request_fits(base_url: &str, bearer: &str, model: &str, messages: &[ai::ChatMessage], max_tokens: u32, context_tokens: Option<u32>,) -> bool` |
| function | `summary_gate` | L95 | `fn summary_gate(transcript: &[TranscriptLine]) -> Result<(), &'static str>` |
| function | `format_transcript_for_summary` | L107 | `fn format_transcript_for_summary(transcript: &[TranscriptLine], is_ru: bool) -> String` |
| function | `truncate_transcript_middle` | L134 | `fn truncate_transcript_middle(text: &str, budget_chars: usize, is_ru: bool) -> (String, bool)` |
| function | `summary_system_prompt` | L190 | `fn summary_system_prompt(is_ru: bool, truncated: bool) -> String` |
| function | `build_summary_seed` | L273 | `fn build_summary_seed(transcript: &[TranscriptLine], is_ru: bool, is_local: bool, memory_ref: Option<&str>,) -> Vec<ai::ChatMessage>` |
| function | `build_summary_seed_from_formatted` | L292 | `fn build_summary_seed_from_formatted(formatted: &str, is_ru: bool, _is_local: bool, memory_ref: Option<&str>,) -> Vec<ai::ChatMessage>` |
| function | `push_memory_ref` | L316 | `fn push_memory_ref(system: &mut String, is_ru: bool, memory_ref: Option<&str>) -> ()` |
| function | `split_transcript_for_map` | L338 | `fn split_transcript_for_map(formatted: &str, budget_chars: usize) -> Vec<String>` |
| function | `partial_summary_prompt` | L392 | `fn partial_summary_prompt(is_ru: bool, part: usize, total: usize) -> String` |
| function | `build_summary_reduce_seed` | L431 | `fn build_summary_reduce_seed(partials: &[String], is_ru: bool, _is_local: bool, memory_ref: Option<&str>,) -> Vec<ai::ChatMessage>` |
