---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_678630582df1"
source_path: "overlay-backend/src/ai/control.rs"
batch_id: "B06"
total_lines: 129
symbols_count: 10
review_state: validated
---

# File Map: `overlay-backend/src/ai/control.rs`

- **Batch:** B06
- **Physical Lines:** 129
- **Coverage:** 129/129 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

## Symbols & Routines (10)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `acquire_exclusive_ai` | L9 | `fn acquire_exclusive_ai() -> Result<tokio::sync::SemaphorePermit<'static>>` |
| function | `http_client` | L16 | `fn http_client() -> reqwest::Client` |
| function | `transport_failure_kind` | L27 | `fn transport_failure_kind(error: &reqwest::Error) -> &'static str` |
| function | `set_prompt_cache` | L41 | `fn set_prompt_cache(on: bool) -> ()` |
| function | `apply_prompt_cache` | L45 | `fn apply_prompt_cache(body: &mut Value) -> ()` |
| function | `set_local_no_think` | L71 | `fn set_local_no_think(on: bool) -> ()` |
| function | `apply_local_no_think` | L75 | `fn apply_local_no_think(body: &mut Value, force: bool) -> ()` |
| function | `apply_managed_gemma_sampler` | L87 | `fn apply_managed_gemma_sampler(body: &mut Value, base_url: &str, model: &str) -> ()` |
| function | `is_managed_mlx_endpoint` | L100 | `fn is_managed_mlx_endpoint(endpoint: &AiEndpoint) -> bool` |
| function | `resolve_managed_mlx_endpoint` | L108 | `fn resolve_managed_mlx_endpoint(mut endpoint: AiEndpoint,) -> Result<(AiEndpoint, Option<crate::mlx_runtime::MlxRequestLease>)>` |
