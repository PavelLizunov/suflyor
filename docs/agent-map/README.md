# Suflyor Agent-Readable Project Map (v1.0)

Welcome to the canonical, machine-queryable agent map of the **Suflyor** repository.
This document and its sibling directories provide exhaustive, line-by-line architectural transparency across the entire codebase (~115,000 LoC, 303 code files, 5 standalone crates + scripts).

---

## 1. Quick Navigation & Directory Layout

- **[`inventory/`](inventory/)**:
  - `files.jsonl`: Catalog of all 828 tracked repository paths with SHA-256 hashes, physical line counts, and analysis policies.
  - `partitions.json`: Distribution of files across batches **B01** through **B15**.
- **[`records/`](records/)**:
  - `symbols.jsonl`: **4,500** extracted AST functions, methods, and routines with visibility and signatures.
  - `types.jsonl`: **406** structs, enums, traits, and Slint components.
  - `errors.jsonl` & `error_sites.jsonl`: Error variant catalogs and 461 audited bailout/recovery locations.
  - `configs.jsonl`: 213 mapped configuration access sites.
  - `hotkeys.jsonl`: Global Win32/macOS hotkey registrations and dispatch sites.
  - `behaviors.jsonl`: 457 concurrency, threading, and IPC channel points.
- **[`files/`](files/)**:
  - 303 individual Markdown documentation files providing a 1:1 map for every source file in the repository.
- **[`reviews/`](reviews/)**:
  - `O1_analysis.md` .. `O8_analysis.md`: Deep architectural reasoning vectors executed by Claude Opus covering process isolation, SQLite persistence, audio synchronization, Slint reactivity, Win32 stealth, AI dataflow, distribution gates, and whole-map adversarial audits.
- **[`architecture/`](architecture/)**:
  - `process_topology.md`: Mermaid sequence diagrams, process isolation blueprints, and thread ownership graphs.

---

## 2. Core Invariants & Architecture Summary

1. **Process Isolation for Neural Sidecars**:
   - Because statically linking two different ONNX Runtime builds into the same binary causes a native segmentation fault / access violation on the second model load, Suflyor runs in-process GigaAM STT in `overlay-host`, Piper TTS in `suflyor-tts`, and TeraTTSv2 in `suflyor-teratts`.
2. **Two-Tier Hybrid Persistence**:
   - Primary source of truth is the append-only JSONL journal (`%APPDATA%/suflyor/sessions/*.jsonl`).
   - SQLite catalog (`catalog.sqlite`) operates in WAL mode with FTS5 search and stores permanent user-owned personal memory and speaker diarization records.
3. **Win32 Stealth & Multi-Monitor Geometry**:
   - Windows display affinity (`WDA_EXCLUDEFROMCAPTURE`) is enforced and verified with readback.
   - Screen coordinates explicitly handle negative virtual desktop coordinates and portrait secondary monitors.
4. **UI Thread Safety & Window Reuse**:
   - Slint event loop mutations are guarded via `slint::Weak` and `invoke_from_event_loop`.
   - Settings window state is reset on every open via `populate_token_status` to prevent transient status ghosting.

---

## 3. Querying the Map

Agents can search and query this map programmatically:
```bash
# Find any function signature by name
grep -i "replace_session" docs/agent-map/records/symbols.jsonl

# Inspect error handling at a specific path
grep "sqlite_store.rs" docs/agent-map/records/error_sites.jsonl

# Read the dedicated per-file map
cat docs/agent-map/files/overlay-backend__src__persistence__sqlite_store.rs.md
```
