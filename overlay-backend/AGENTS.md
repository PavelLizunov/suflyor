# `overlay-backend` Crate Agent Guide

This guide extends the repository root `AGENTS.md`; do not duplicate root workflow or DSH/homelab policy here.

## Purpose
`overlay-backend` is Suflyor's pure-Rust, UI-free domain engine. It encapsulates shared data models, local and cloud AI pipeline drivers, speech recognition (STT) and synthesis (TTS), audio capture and multi-channel session recording, SQLite database persistence and JSONL indexing, an embedded knowledge base, platform credential storage, local sidecar/server process lifecycle management, and SHA-verified installer/update workflows consumed by the host overlay binary (`slint-experiment`).

## Code Map

### High-Risk Modules
- `credentials.rs`: storage for direct-provider API keys (Windows Credential Manager; a mode-0600 JSON file on Unix targets). **High Risk**: direct-provider keys must not be serialized into `Config`, exports, backups, or diagnostics; do not describe the Unix file as encrypted.
- `audio.rs`, `audio_route.rs`, `recorder.rs`: WASAPI audio capture, audio device route monitoring, and session WAV recording. **High Risk**: Risk of audio thread blocking, device lockup, or platform driver crashes.
- `persistence/` (`sqlite_store.rs`, `migrations.rs`, `indexer.rs`, `models.rs`): SQLite catalog handle, schema migrations, WAL mode, and JSONL indexing worker. **High Risk**: Risk of database lockup (`SQLITE_BUSY`), schema regression, or catalog corruption.
- `stt.rs`, `diarize.rs`, `diar_install.rs`: In-process GigaAM-v3 STT via ONNX (`transcribe-rs`/`ort-directml`) and diarization management. **High Risk**: Risk of ONNX runtime collisions or GPU accelerator failure.
- `download.rs`, `update.rs`, `*install.rs` (`teratts_install.rs`, `hermes_install.rs`, `ocr_install.rs`, `mlx_install.rs`): Network asset download and installer tools. **High Risk**: SHA-256 and SHA-1 checksum validation must be strictly enforced before archive extraction or execution.
- `local_ai.rs`: Spawning and managing local LLM (`llama-server`) and Whisper sidecar servers. **High Risk**: Risk of orphaned sub-processes, port squatting, or memory exhaustion; must register Windows `JobObjects`.
- `config/` (`config.rs`, `repair.rs`, `snippets.rs`): Application configuration parsing, repair routines, and defaults. **High Risk**: Risk of configuration corruption or setting loss.
- `http_log.rs`: Error message sanitization and redaction. **High Risk**: Prevents sensitive local IP addresses, bridge URLs, or internal endpoints from leaking into screenshot-visible UI error tiles.

### Core Domain & Support Modules
- `ai.rs`, `ai/provider.rs`: LLM prompt assembly, cloud/local provider routing, streaming response handling.
- `tts.rs`, `tts_normalize.rs`, `tts_install.rs`: Windows SAPI 5 interface, text normalization, sidecar base64 stdin command protocol.
- `kb.rs`: Embedded knowledge base (~1600 pre-lowercased entries) with a 200-character query limit (DoS protection).
- `journal.rs`, `conspect.rs`, `session_admin.rs`, `session_audio.rs`, `session_names.rs`: Session event tracking, bookmark management, and summary records.
- `memory/` (`context_builder.rs`, `candidates.rs`, `summary_ref.rs`, `normalize.rs`): Context assembly, history compaction, memory database interaction.
- `ocr.rs`, `vision.rs`: Screen capture OCR processing and image data preparation.
- `runtime.rs`, `deep_lock.rs`, `capabilities.rs`, `events.rs`, `health.rs`, `paths.rs`, `text.rs`, `bridge.rs`, `summary_source.rs`: Path resolution, runtime state, event dispatch, system health diagnostics, and string processing.
- `audio_macos.rs`, `audio_unavailable.rs`: Platform seams for macOS mic capture and unsupported non-Windows OS targets.

### Tests & Verification Guards
- Embedded unit tests: `src/tests.rs`, `src/config/tests.rs`, `src/local_ai/tests.rs`, `src/runtime/tests.rs`, and inline `#[cfg(test)]` modules.
- Integration test guards: `tests/no_window_guard.rs` (enforces `CREATE_NO_WINDOW` on external commands), `tests/archive_cycle.rs`, `tests/ai_eval.rs`, `tests/hermes_plugin_smoke.rs`.

## Invariants
1. **Process Isolation & ONNX Boundaries**: Neural TTS engines (`suflyor-tts` / Piper and `suflyor-teratts` / TeraTTSv2) MUST remain isolated in external sidecar executables. Linking sherpa-onnx directly into `overlay-backend` causes an in-process ONNX runtime symbol collision with GigaAM STT (`transcribe-rs`/`ort`) and crashes natively on secondary model load.
2. **Credential Storage Boundaries**: Direct OpenAI/Anthropic provider keys use `credentials.rs` and are not serialized in `Config`. Legacy bridge/Groq and server credentials still exist in configuration and explicit portable exports; treat every config/export/backup path as secret-bearing and preserve its redaction and backup rules.
3. **No-Console Process Spawning**: Every production process spawned via `std::process::Command` MUST apply `CREATE_NO_WINDOW` flags (via `download::no_window`, `hidden_command`, or `creation_flags`) to prevent console popups on Windows. This invariant is enforced by `tests/no_window_guard.rs`.
4. **JobObject Process Cleanup**: Child processes (such as `llama-server` and `whisper`) spawned in `local_ai.rs` MUST be assigned to a Windows `JobObject` (`Win32_System_JobObjects`) so that child processes terminate automatically if the parent process exits or is killed.
5. **Database Threading & Rebuildable Index**: `Store` in `sqlite_store.rs` is `!Sync` and runs on dedicated background worker threads to avoid blocking live audio/AI pipelines. The SQLite catalog is an idempotent index over JSONL session logs: deleting `catalog.sqlite` must allow complete re-indexing from raw logs without data loss. WAL mode and `busy_timeout = 2000` must be preserved.
6. **Digest Verification Before Execution**: All binary updates and downloaded models MUST verify their SHA-256 (or SHA-1 for Git blobs) hashes against pinned manifests before execution or extraction.
7. **Clippy Panic & Unwrap Denial**: Production crate code explicitly denies `unwrap_used`, `expect_used`, and `panic`. Test modules (`#[cfg(test)]`) and `tests/*.rs` files opt back in via `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
8. **Platform Seams**: Non-Windows targets MUST hit explicit fallback seams (`audio_macos.rs` or `audio_unavailable.rs`) that fail gracefully rather than faking success.

## Change Guidance
- **Persistence & Schema Updates**: When modifying database schema or persistence models, add a migration file in `migrations/`, update `LATEST_VERSION` in `migrations.rs`, update models in `models.rs`, and verify that SQLite index re-building from JSONL raw events remains idempotent.
- **Process Spawning**: Always wrap `std::process::Command` creation using `download::no_window(&mut cmd)` or `hidden_command`. Run `cargo test --manifest-path overlay-backend/Cargo.toml --test no_window_guard` after modifying process spawn sites.
- **Audio & STT Modifications**: Maintain DirectML runtime fallback to CPU for GigaAM STT. Do not introduce heavy build-time C/C++ dependencies (`libclang` or static ONNX libraries) directly into `overlay-backend`.
- **UI Error Formatting**: Sanitize error strings using `http_log` before forwarding error messages to host overlay UI tiles.

## Targeted Checks
Run these on the appropriate homelab worker against the exact candidate SHA, not on the DSH control plane:
- **Single-crate tests**: `cargo test --manifest-path overlay-backend/Cargo.toml`
- **Single-crate clippy**: `cargo clippy --manifest-path overlay-backend/Cargo.toml --all-targets`
- **Single-crate formatting**: `cargo fmt --manifest-path overlay-backend/Cargo.toml`
- **Native repository gate**: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual`
