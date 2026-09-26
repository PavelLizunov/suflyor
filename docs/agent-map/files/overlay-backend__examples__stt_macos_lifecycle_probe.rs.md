---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_b0904b6667b4"
source_path: "overlay-backend/examples/stt_macos_lifecycle_probe.rs"
batch_id: "B09"
total_lines: 206
symbols_count: 8
review_state: validated
---

# File Map: `overlay-backend/examples/stt_macos_lifecycle_probe.rs`

- **Batch:** B09
- **Physical Lines:** 206
- **Coverage:** 206/206 lines (100%)

## Types & Structures (0)

*No standalone type declarations.*

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

## Key Behaviors & Concurrency

- Spawns asynchronous thread/task at L152
