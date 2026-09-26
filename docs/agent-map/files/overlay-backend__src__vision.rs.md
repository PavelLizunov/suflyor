---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_24fadcad6c74"
source_path: "overlay-backend/src/vision.rs"
batch_id: "B10"
total_lines: 459
symbols_count: 16
review_state: validated
---

# File Map: `overlay-backend/src/vision.rs`

- **Batch:** B10
- **Physical Lines:** 459
- **Coverage:** 459/459 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `VisionMode` | L135 | pub |

## Symbols & Routines (16)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `translate_prompt` | L46 | `fn translate_prompt(phonetics: bool) -> String` |
| function | `build_vision_request` | L74 | `fn build_vision_request(image_data_url: &str, prompt: &str) -> Vec<ChatMessage>` |
| function | `build_vision_request_with_context` | L106 | `fn build_vision_request_with_context(image_data_url: &str, prompt: &str, meeting_context: &str,) -> Vec<ChatMessage>` |
| function | `test_practice_prompt` | L153 | `fn test_practice_prompt(response_lang: &str) -> String` |
| function | `build_test_practice_request` | L184 | `fn build_test_practice_request(image_data_url: &str, response_lang: &str) -> Vec<ChatMessage>` |
| function | `test_connection` | L230 | `fn test_connection(base_url: String, bearer: String, model: String) -> Result<String>` |
| function | `test_connection_endpoint` | L242 | `fn test_connection_endpoint(endpoint: crate::ai::AiEndpoint) -> Result<String>` |
| function | `vision_request_has_text_then_image_part` | L256 | `fn vision_request_has_text_then_image_part() -> ()` |
| function | `vision_context_prepends_system_turn_only_when_set` | L272 | `fn vision_context_prepends_system_turn_only_when_set() -> ()` |
| function | `test_practice_request_is_answer_plus_explanation_and_refuses_to_fabricate` | L308 | `fn test_practice_request_is_answer_plus_explanation_and_refuses_to_fabricate() -> ()` |
| function | `test_practice_prompt_honors_response_language` | L348 | `fn test_practice_prompt_honors_response_language() -> ()` |
| function | `test_practice_prompt_covers_fill_blank_and_multi_answer` | L356 | `fn test_practice_prompt_covers_fill_blank_and_multi_answer() -> ()` |
| function | `translate_prompt_composes_phonetics_suffix` | L373 | `fn translate_prompt_composes_phonetics_suffix() -> ()` |
| function | `ocr_prompt_demands_verbatim_and_forbids_translation` | L395 | `fn ocr_prompt_demands_verbatim_and_forbids_translation() -> ()` |
| function | `empty_prompt_falls_back_to_default` | L421 | `fn empty_prompt_falls_back_to_default() -> ()` |
| function | `synthetic_test_image_is_a_nontrivial_png_data_url` | L431 | `fn synthetic_test_image_is_a_nontrivial_png_data_url() -> ()` |
