---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_e0029b029c1e"
source_path: "overlay-backend/src/ai/inspect.rs"
batch_id: "B06"
total_lines: 195
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "1b8ab03ebef8a1299f87e54a46493e1673d80900ed66c388b73a4eee817550a1"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ai/inspect.rs`

- **Batch:** B06
- **Physical Lines:** 195
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `test_connection` | L8 | `fn test_connection(base_url: String, bearer: String, model: String) -> Result<String>` |
| function | `test_connection_endpoint` | L20 | `fn test_connection_endpoint(endpoint: AiEndpoint) -> Result<String>` |
| function | `test_connection_messages` | L31 | `fn test_connection_messages(endpoint: AiEndpoint, messages: Vec<ChatMessage>,) -> Result<String>` |
| function | `list_models` | L132 | `fn list_models(base_url: &str, bearer: &str) -> Result<Vec<String>>` |
| function | `list_models_endpoint` | L144 | `fn list_models_endpoint(endpoint: &AiEndpoint) -> Result<Vec<String>>` |
