# Goal: Comprehensive Codebase & Implementation Audit via Grok Swarm

- **Date:** 2026-09-16
- **Status:** Active / Planned
- **Coordinator:** Gemini Flash (`gemini-3.8-flash-high`)
- **Adversarial Swarm:** xAI Grok 4.6 (`ninitux/grok-4.6`, `grok-swarm`)
- **Scope:** Complete repository scan (~115,800 LOC across 5 standalone crates, scripts, and workflows)

---

## 1. Context & Objectives

Suflyor has completed major milestones:
1. Migration from React/Tauri to pure Rust + Slint.
2. Cross-platform support for Windows (production) and active macOS port.
3. Multi-crate decoupled architecture (5 standalone crates: `overlay-backend`, `slint-experiment`, `suflyor-tts`, `suflyor-teratts`, `suflyor-wsola`).
4. Hardened credential permissions and input sanitization (Sentinel PRs #180-#191).

The objective of this task is to perform an exhaustive, adversarial, end-to-end audit of all architectural boundaries, concurrency models, memory bounds, error recovery loops, and security vectors using **Grok 4.6** in batches of 1–5 workers via `grok-swarm`.

---

## 2. Invariants & Rules of Engagement

1. **Strict Concurrency Bounds:** Workers run in batches of 1–4 per wave to respect xAI single-account API rate limits and eliminate HTTP 429 back-off storms.
2. **Standard Contrarian Contract:** Every Grok worker must format its output strictly as:
   ```markdown
   ### [Index]. [Topic / Focus Area]
   - **Finding / Hypothesis:** <Exact claim, code construct, file & line>
   - **Rationale:** <Failure scenario, race condition, leak vector, exploit>
   - **Verification Method:** <Concrete reproduction step, unit test, or inspection command>
   ```
3. **Evidence-Based Coordinator Acceptance:** Grok findings are challenge hypotheses. The coordinator (Gemini) verifies each claim against actual repository sources and tests before accepting or dismissing it.
4. **Zero State Mutation during Audit:** Audit workers are strictly read-only; no code or git state is modified during analysis.
5. **Privacy Compliance:** In accordance with `docs/AGENTS.md`, all logs and reports must redact usernames (`%USERPROFILE%`), tokens, and private network addresses.

---

## 3. Wave Breakdown & Assignments

### 🌊 Wave 1: Backend Core — Data, State, Memory & Persistence (3 Workers)
- **Target Crate:** `overlay-backend` (~51,100 LOC)
- **Worker 1.1 (Persistence & Database Integrity):**
  - **Scope:** `persistence/sqlite_store.rs`, `persistence/migrations.rs`, `persistence/indexer.rs`, `journal/mod.rs`, `journal/writer.rs`.
  - **Focus:** `SQLITE_BUSY` contention, WAL handling, idempotent catalog rebuilds from raw JSONL, torn writes, event sequence ordering.
- **Worker 1.2 (Curated Memory & RAG Context Engine):**
  - **Scope:** `memory/context_builder.rs`, `memory/candidates.rs`, `memory/normalize.rs`, `memory/summary_ref.rs`.
  - **Focus:** Token/character budget overflows, prompt injection vectors via memory items, hallucinated fact extraction, deduplication latency.
- **Worker 1.3 (Config, Paths & Credential Security):**
  - **Scope:** `config.rs`, `config/repair.rs`, `credentials.rs`, `paths.rs`, `http_log.rs`.
  - **Focus:** Windows Credential Manager vs POSIX 0600/0700 file storage, path migration integrity, unredacted secrets in `Config` serialization.

---

### 🌊 Wave 2: Audio Ingestion, STT Streaming & Sidecars (4 Workers)
- **Target Crates:** `overlay-backend`, `suflyor-tts`, `suflyor-teratts`, `suflyor-wsola`
- **Worker 2.1 (Audio Ingestion & WASAPI/CoreAudio Drivers):**
  - **Scope:** `audio.rs`, `audio_route.rs`, `audio_macos.rs`, `recorder.rs`, `suflyor-wsola/`.
  - **Focus:** Ring buffer over/underflows, phase carry in `resample_and_quantise`, device disconnect recovery loops, PTT audio accumulation.
- **Worker 2.2 (Speech-to-Text Pipeline & VAD):**
  - **Scope:** `stt.rs`, `diarize.rs`, `diar_install.rs`.
  - **Focus:** Unbounded task spawn queues before semaphore acquire (memory leak risk in multi-hour meetings), HTTP 30s timeout cascades, GigaAM ONNX runtime lockups.
- **Worker 2.3 (TTS Sidecar Subprocesses & IPC Protocol):**
  - **Scope:** `tts.rs`, `suflyor-tts/src/main.rs`, `suflyor-tts/src/engine.rs`, `suflyor-teratts/src/`.
  - **Focus:** Uncapped `SPEAK <base64>` single-line stdio buffer, pipe deadlock handling, `JobObject` / `kill_on_drop` process leak prevention, dual-ONNX process isolation invariant.
- **Worker 2.4 (Local AI Server Orchestration):**
  - **Scope:** `local_ai.rs`, `local_ai/hardware_profile.rs`, `local_ai/model_choice.rs`, `local_ai/model_state.rs`.
  - **Focus:** Port 8080/8081 squatting and recovery, child process lifecycle, GGUF model verification, VRAM release and context-size adaptation.

---

### 🌊 Wave 3: UI Host Orchestration, Windowing & Concurrency (4 Workers)
- **Target Crate:** `slint-experiment` (~53,900 LOC)
- **Worker 3.1 (Tokio ↔ Slint Event Bridge & Concurrency):**
  - **Scope:** `slint_session.rs`, `slint_events.rs`, `runtime_state.rs`, `runtime.rs`.
  - **Focus:** Deadlock hazards between UI event loop and async tasks, contention on `std::sync::Mutex<SlintRuntime>`, generation fence checks (`session_gen`), orphaned auto-tile tasks.
- **Worker 3.2 (Window Lifecycle, Win32 DWM & Anti-Capture Stealth):**
  - **Scope:** `window_lifecycle.rs`, `win32.rs`, `bar_tray.rs`, `native/mod.rs`.
  - **Focus:** Screen-share exclusion (`WDA_EXCLUDEFROMCAPTURE`), asynchronous HWND realization races, multi-monitor geometry (portrait + landscape bounds).
- **Worker 3.3 (AI Streaming Tile Engine & Markdown Renderer):**
  - **Scope:** `tile_controller.rs`, `tile_window.rs`, `tile_ask.rs`, `tile_ptt.rs`, `tile_followup.rs`, `markdown.rs`.
  - **Focus:** Tile generation mismatch guards (`GenGatedEvents`), memory growth in CommonMark parser, rapid re-ask stream cancellation.
- **Worker 3.4 (Settings Controllers & Reused State Management):**
  - **Scope:** `settings_controller.rs`, `settings_ai.rs`, `settings_stt.rs`, `settings_voice.rs`, `settings_import_export.rs`.
  - **Focus:** Reused singleton window state cleanliness (`populate_token_status`), optimistic state flips before async confirmation, server import validation.

---

### 🌊 Wave 4: Security Boundaries, Release Tooling & CI/CD (3 Workers)
- **Target Directories:** `slint-experiment/src/bin/overlay_host/`, `scripts/`, `.github/`
- **Worker 4.1 (Privacy Scrubber & Diagnostic Export):**
  - **Scope:** `diagnostics.rs`, `vision_capture.rs`, `read_aloud.rs`.
  - **Focus:** Verification of "Собрать логи" export safety (no raw user paths, IP addresses, or tokens), screen capture region privacy boundaries.
- **Worker 4.2 (Network Downloaders & Model Installers):**
  - **Scope:** `download.rs`, `update.rs`, `ocr_install.rs`, `teratts_install.rs`, `hermes_install.rs`.
  - **Focus:** Pre-extraction SHA-256 validation, path traversal / archive slip defense in tar/zip extractors, HTTPS enforcement without insecure redirect downgrade.
- **Worker 4.3 (Build Automation, Native Gate & CI Workflows):**
  - **Scope:** `scripts/git-gate-native.ps1`, `scripts/build-slint-release.ps1`, `scripts/slint-installer.nsi`, `.github/workflows/ci.yml`.
  - **Focus:** Native diff classification accuracy (Docs vs Targeted vs Full), NSIS installer DLL hijacking prevention, CI test coverage gaps.

---

## 4. Execution Workflow

```
┌────────────────────────────────────────────────────────┐
│               Gemini Flash (Coordinator)               │
│ - Partitions files & prepares self-contained prompts   │
└──────────────────────────┬─────────────────────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                 ▼
   Wave 1 (3 workers)Wave 2 (4 workers)Wave 3 (4 workers) (Sequential waves)
   [xAI Grok 4.6]    [xAI Grok 4.6]    [xAI Grok 4.6]
         │                 │                 │
         └─────────────────┼─────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────┐
│             Integration & Triaged Matrix               │
│ - Verifies findings against current Rust codebase      │
│ - Categorizes: CRITICAL / HIGH / MEDIUM / LOW          │
│ - Authors actionable fix spec / backlogged tasks       │
└────────────────────────────────────────────────────────┘
```

---

## 5. Definition of Done
- [ ] All 4 waves executed and raw reports stored in `docs/audit-grok/`.
- [ ] Every finding reviewed, verified, or refuted with exact file and line evidence.
- [ ] Consolidated Risk & Remediation Matrix published in `docs/audit-grok-summary.md`.
- [ ] Actionable bug fixes extracted into task backlog / SDD specifications.
