---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c3c7e79e26a1"
source_path: "overlay-backend/src/ai/stream.rs"
batch_id: "B06"
total_lines: 366
symbols_count: 5
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "c44e3fcc43e6cb9643f60cde2c9d7047f05a1c2e02ab89fee9a6efe0545d7bc4"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/src/ai/stream.rs`

- **Batch:** B06
- **Physical Lines:** 366
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (5)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `stream_chat` | L12 | `fn stream_chat(base_url: String, bearer: String, model: String, messages: Vec<ChatMessage>, max_tokens: u32,) -> mpsc::Receiver<AiEvent>` |
| function | `stream_chat_endpoint` | L33 | `fn stream_chat_endpoint(endpoint: AiEndpoint, messages: Vec<ChatMessage>, max_tokens: u32,) -> mpsc::Receiver<AiEvent>` |
| function | `codex_failure_message` | L138 | `fn codex_failure_message(failure: crate::codex_subscription::TurnFailure,) -> &'static str` |
| function | `stream_inner` | L156 | `fn stream_inner(endpoint: AiEndpoint, messages: Vec<ChatMessage>, max_tokens: u32, tx: mpsc::Sender<AiEvent>, request_id: u64, request_started_at: std::time::Instant,) -> Result<()>` |
| function | `drain_complete_frames` | L340 | `fn drain_complete_frames(byte_buf: &mut Vec<u8>) -> String` |

## Heuristic behavior and concurrency matches

- Instantiates IPC channel at L40
- Spawns asynchronous thread/task at L42
