---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_c5aecc233372"
source_path: "overlay-backend/src/ai/pricing.rs"
batch_id: "B06"
total_lines: 60
symbols_count: 3
review_state: validated
---

# File Map: `overlay-backend/src/ai/pricing.rs`

- **Batch:** B06
- **Physical Lines:** 60
- **Coverage:** 60/60 lines (100%)

## Types & Structures (1)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| struct | `TokenUsage` | L47 | pub |

## Symbols & Routines (3)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `pricing_per_million` | L2 | `fn pricing_per_million(model: &str) -> (f64, f64)` |
| function | `microcents_to_usd` | L30 | `fn microcents_to_usd(microcents: u64) -> f64` |
| function | `cost_microcents` | L36 | `fn cost_microcents(model: &str, input_tokens: u64, output_tokens: u64) -> u64` |
