---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e0029b029c1e"
source_path: "overlay-backend/src/ai/inspect.rs"
batch_id: "B06"
total_lines: 195
symbols_count: 5
review_state: validated
---

# File Map: `overlay-backend/src/ai/inspect.rs`

- **Batch:** B06
- **Physical Lines:** 195
- **Coverage:** 195/195 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `test_connection` | L8 | `fn test_connection(base_url: String, bearer: String, model: String) -> Result<String>` |
| function | `test_connection_endpoint` | L20 | `fn test_connection_endpoint(endpoint: AiEndpoint) -> Result<String>` |
| function | `test_connection_messages` | L31 | `fn test_connection_messages(endpoint: AiEndpoint, messages: Vec<ChatMessage>,) -> Result<String>` |
| function | `list_models` | L132 | `fn list_models(base_url: &str, bearer: &str) -> Result<Vec<String>>` |
| function | `list_models_endpoint` | L144 | `fn list_models_endpoint(endpoint: &AiEndpoint) -> Result<Vec<String>>` |
