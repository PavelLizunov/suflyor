---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e7890948f74e"
source_path: "overlay-backend/src/ai/provider.rs"
batch_id: "B06"
total_lines: 518
symbols_count: 14
review_state: validated
---

# File Map: `overlay-backend/src/ai/provider.rs`

- **Batch:** B06
- **Physical Lines:** 518
- **Coverage:** 518/518 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `ParsedStreamEvent` | L5 | pub(super) |

## Symbols & Routines (14)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `endpoint_path` | L14 | `fn endpoint_path(protocol: AiProtocol) -> &'static str` |
| function | `authorize` | L23 | `fn authorize(request: reqwest::RequestBuilder, protocol: AiProtocol, credential: &str,) -> reqwest::RequestBuilder` |
| function | `request_body` | L39 | `fn request_body(protocol: AiProtocol, model: &str, messages: &[ChatMessage], max_tokens: u32, stream: bool, prompt_cache: bool,) -> Result<Value>` |
| function | `openai_content` | L130 | `fn openai_content(content: &MessageContent) -> Value` |
| function | `anthropic_content` | L148 | `fn anthropic_content(content: &MessageContent) -> Result<Value>` |
| function | `parse_data_url` | L172 | `fn parse_data_url(url: &str) -> Result<(&str, &str)>` |
| function | `parse_stream` | L191 | `fn parse_stream(protocol: AiProtocol, value: &Value) -> ParsedStreamEvent` |
| function | `parse_completion` | L303 | `fn parse_completion(protocol: AiProtocol, value: &Value, elapsed_secs: f64,) -> (String, TokenUsage)` |
| function | `messages` | L415 | `fn messages() -> Vec<ChatMessage>` |
| function | `responses_body_uses_native_fields_and_image_shape` | L438 | `fn responses_body_uses_native_fields_and_image_shape() -> ()` |
| function | `anthropic_body_uses_header_protocol_shape_and_cache_control` | L456 | `fn anthropic_body_uses_header_protocol_shape_and_cache_control() -> ()` |
| function | `parses_responses_and_anthropic_stream_events` | L475 | `fn parses_responses_and_anthropic_stream_events() -> ()` |
| function | `parses_openai_stream_decode_metrics` | L489 | `fn parses_openai_stream_decode_metrics() -> ()` |
| function | `parses_native_usage_and_text` | L504 | `fn parses_native_usage_and_text() -> ()` |
