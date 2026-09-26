---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_209c0774711d"
source_path: "overlay-backend/src/journal/writer.rs"
batch_id: "B08"
total_lines: 363
symbols_count: 17
review_state: validated
---

# File Map: `overlay-backend/src/journal/writer.rs`

- **Batch:** B08
- **Physical Lines:** 363
- **Coverage:** 363/363 lines (100%)

## Types & Structures (4)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `WriterCmd` | L24 | pub(crate) |
| enum | `ShutdownState` | L36 | pub(crate) |
| struct | `Journal` | L18 | pub |
| struct | `WriterState` | L29 | pub(crate) |

## Symbols & Routines (17)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `sessions_dir` | L12 | `fn sessions_dir() -> Result<PathBuf>` |
| function | `counting_for_test` | L43 | `fn counting_for_test() -> Self` |
| function | `capturing_for_test` | L52 | `fn capturing_for_test() -> (Self, mpsc::UnboundedReceiver<WriterCmd>)` |
| function | `open_new_session` | L66 | `fn open_new_session() -> Result<Self>` |
| function | `open_new_session_with_limits` | L70 | `fn open_new_session_with_limits(keep_sessions: usize, max_bytes: u64) -> Result<Self>` |
| function | `write` | L114 | `fn write(&self, event: &JournalEvent<'_>) -> ()` |
| function | `snapshot_counters` | L132 | `fn snapshot_counters(&self) -> Option<SessionCounters>` |
| function | `current_path` | L136 | `fn current_path(&self) -> Option<PathBuf>` |
| function | `path` | L140 | `fn path(&self) -> Option<&Path>` |
| function | `session_id` | L144 | `fn session_id(&self) -> Option<String>` |
| function | `close` | L152 | `fn close(&self) -> ()` |
| function | `shutdown` | L156 | `fn shutdown(&self, timeout: std::time::Duration) -> Result<()>` |
| function | `shutdown_blocking` | L218 | `fn shutdown_blocking(&self, timeout: std::time::Duration) -> Result<(), String>` |
| function | `emit_summary_and_stop` | L222 | `fn emit_summary_and_stop(&self) -> ()` |
| function | `note_write_error` | L254 | `fn note_write_error(first_error: &mut Option<String>, e: std::io::Error) -> ()` |
| function | `spawn_writer` | L277 | `fn spawn_writer(mut rx: mpsc::UnboundedReceiver<WriterCmd>, file: std::fs::File,) -> std::io::Result<std::thread::JoinHandle<()>>` |
| function | `bump_counters` | L325 | `fn bump_counters(c: &mut SessionCounters, event: &JournalEvent<'_>) -> ()` |

## Key Behaviors & Concurrency

- Instantiates IPC channel at L53
- Instantiates IPC channel at L95
- Instantiates IPC channel at L176
