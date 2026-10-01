---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b0904b6667b4"
source_path: "overlay-backend/examples/stt_macos_lifecycle_probe.rs"
batch_id: "B09"
total_lines: 206
symbols_count: 8
review_state: unverified_heuristic_index
extraction_method: regex_rust_or_slint
source_sha256_at_historical_inventory: "fae9e1b2c6715850050ce930f511831c35893a319a743a02c64bcc16494f6aa5"
source_matches_reconciliation_baseline: true
---

# File Map: `overlay-backend/examples/stt_macos_lifecycle_probe.rs`

- **Batch:** B09
- **Physical Lines:** 206
- **Semantic coverage:** not measured; no full-line review evidence.

## Types & Structures (0)

*No type candidates extracted; absence of declarations is not established.*

## Symbols & Routines (8)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `usage` | L47 | `fn usage() -> &'static str` |
| function | `read_pcm` | L52 | `fn read_pcm(path: &str) -> Result<Vec<i16>>` |
| function | `phase` | L73 | `fn phase(name: &str) -> Result<()>` |
| function | `transcript_matches` | L84 | `fn transcript_matches(event: &TranscriptEvent, expected: &str) -> bool` |
| function | `start_live` | L89 | `fn start_live(backend: &SttBackendCfg, pcm: &[i16], expected: &str,) -> Result<(mpsc::Sender<AudioChunk>, mpsc::Receiver<TranscriptEvent>)>` |
| function | `stop_live` | L132 | `fn stop_live(audio_tx: mpsc::Sender<AudioChunk>, mut transcript_rx: mpsc::Receiver<TranscriptEvent>,) -> Result<()>` |
| function | `main` | L151 | `fn main() -> Result<()>` |
| function | `main` | L203 | `fn main() -> ()` |

## Heuristic behavior and concurrency matches

- Spawns asynchronous thread/task at L152
