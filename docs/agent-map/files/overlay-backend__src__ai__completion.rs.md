---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_32484e64ed9b"
source_path: "overlay-backend/src/ai/completion.rs"
batch_id: "B06"
total_lines: 307
symbols_count: 9
review_state: validated
---

# File Map: `overlay-backend/src/ai/completion.rs`

- **Batch:** B06
- **Physical Lines:** 307
- **Coverage:** 307/307 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (9)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `complete_with_usage` | L13 | `fn complete_with_usage(base_url: &str, bearer: &str, model: &str, messages: Vec<ChatMessage>, max_tokens: u32,) -> Result<(String, TokenUsage)>` |
| function | `complete_with_usage_endpoint` | L32 | `fn complete_with_usage_endpoint(endpoint: &AiEndpoint, messages: Vec<ChatMessage>, max_tokens: u32,) -> Result<(String, TokenUsage)>` |
| function | `complete` | L72 | `fn complete(base_url: &str, bearer: &str, model: &str, messages: Vec<ChatMessage>, max_tokens: u32,) -> Result<String>` |
| function | `complete_endpoint` | L92 | `fn complete_endpoint(endpoint: &AiEndpoint, messages: Vec<ChatMessage>, max_tokens: u32,) -> Result<String>` |
| function | `complete_exclusive` | L116 | `fn complete_exclusive(base_url: &str, bearer: &str, model: &str, messages: Vec<ChatMessage>, max_tokens: u32,) -> Result<String>` |
| function | `complete_with_usage_inner` | L127 | `fn complete_with_usage_inner(protocol: AiProtocol, base_url: &str, bearer: &str, model: &str, messages: Vec<ChatMessage>, max_tokens: u32, force_no_think: bool,) -> Result<(String, TokenUsage)>` |
| function | `is_permanent_ai_error` | L188 | `fn is_permanent_ai_error(msg: &str) -> bool` |
| function | `complete_once` | L201 | `fn complete_once(protocol: AiProtocol, base_url: &str, bearer: &str, model: &str, messages: Vec<ChatMessage>, max_tokens: u32, force_no_think: bool,) -> Result<(String, TokenUsage)>` |
| function | `count_chat_tokens` | L269 | `fn count_chat_tokens(base_url: &str, bearer: &str, model: &str, messages: &[ChatMessage],) -> Result<u64>` |
