# suflyor — architecture overview

**Audience:** developer forking or auditing the codebase.

**Stack:** pure **Rust + Slint** (the original Tauri 2 + React 19 + WebView2
surface was removed in the Phase 7 cut, 2026-05-28 — see
`docs/PHASE-7-CUT-PLAN.md`). No browser engine, no Node, no TypeScript. Two
standalone crates, NO root workspace:

- **`slint-experiment/`** — the `overlay-host` binary. Declarative UI in
  `ui/*.slint` (compiled into the binary at build time via `build.rs` +
  `slint-build`), orchestration in `src/bin/overlay_host.rs`, Win32 HWND
  helpers in `src/win32.rs`.
- **`overlay-backend/`** — the no-UI shared crate (audio / stt / ai /
  local_ai / config / runtime / events / journal / kb / health / update),
  consumed by `slint-experiment` via a path dep.

## Data flow

```
WASAPI loopback (system) + mic  (overlay-backend/src/audio.rs)
        │  tokio::mpsc — audio chunks @ 16 kHz mono i16
        ▼
STT (overlay-backend/src/stt.rs) — backend chosen by config `stt_provider`:
  · Cloud   → Groq /openai/v1/audio/transcriptions (Whisper)
  · Whisper → local whisper.cpp server (OpenAI-compatible endpoint)
  · GigaAM  → local, in-process (transcribe-rs / ort), CPU or DirectML
  + noise gate, retry on 5xx/429, artefact/repetition post-filter
        │  TranscriptLine { source, text }
        ▼
session driver (overlay-backend/src/runtime.rs + slint-experiment/src/slint_session.rs)
  · push to rolling VecDeque (cap ~80)
  · emit transcript line → SlintEvents bridge → overlay bar live line
  · auto-detector → maybe spawn a tile (gated by skip-mic / aggressive mode)
        │
        ▼
detect_trigger (question / keyword) → KB-search injection → prompt build
        │
        ▼
AI (overlay-backend/src/ai.rs) — endpoint chosen by config.ai_endpoint():
  · "local" → local llama.cpp server (OpenAI-compatible)
  · else    → cloud bridge (Claude OpenAI-compat)
  · complete_with_usage / stream_chat; 3 retries on 5xx/429, fail-fast 4xx
  · cost_microcents accumulation
        │
        ▼
tile spawn (slint-experiment/src/bin/overlay_host.rs)
  · events.spawn_tile_full → spawn-poll Timer drains the queue on the UI thread
  · TileWindow (ui/tile.slint), markdown via pulldown-cmark (src/markdown.rs)
  · Win32 transparency + monitor placement + optional stealth
```

## Windows

There is no URL router — each surface is a distinct Slint window component in
the **same process**, created on the UI thread and given Win32 treatment
(`win32.rs`: `make_transparent_overlay` / `make_transparent_tile` /
`set_always_on_top` / `set_stealth`):

- **Overlay bar** (`ui/overlay_bar.slint`) — the always-on-top HUD strip. Brand
  logo (drag handle) + status pill + live transcript line + chips. Pinned to
  the **primary** monitor at startup.
- **Tile** (`ui/tile.slint`) — a Q/A card with markdown body, pin / maximize /
  close, follow-up input.
- **Settings** (`ui/settings_panel.slint`) — grouped sidebar + panels.
- **F4 palette** (`ui/palette.slint`) — inline KB search.
- **Replay** (`ui/replay.slint`) — JSONL session-journal timeline.

## Hotkeys

Global, via the `global-hotkey` crate (registered in `overlay_host.rs`, drained
by a Slint `Timer`):

| Key | Action | Handler |
|---|---|---|
| F3 | Re-ask last question | `runtime::reask_last` |
| F4 | KB palette toggle | `open_palette` (focus-independent toggle) |
| F6 | Manual tile from last transcript line | `runtime::manual_spawn_tile` (resolves local/cloud endpoint) |
| F7 | Collapse-all tiles | stub |
| F9 | Live AI ask (streaming) | `fire_f9_ask` → `ask_stream_loop` |

## Key invariants

1. **Slint windows are created on the UI thread.** Backend code requests a tile
   via `events.spawn_tile_full(...)`; a 50 ms spawn-poll `Timer` in
   `overlay_host.rs` drains the queue and constructs the `TileWindow`. The
   `+ тайл` chip additionally spawns an INSTANT placeholder directly on the UI
   thread for immediate feedback.
2. **Bar pinned to the primary monitor at startup** (`apply_overlay_hwnd`) —
   winit has no monitor preference, so on a multi-monitor setup (esp. a
   portrait secondary at negative X) it would otherwise land unpredictably.
3. **AI endpoint via `config.ai_endpoint(prep)`** — resolves local vs cloud by
   `ai_provider`. The raw `ai_base_url` field is ALWAYS the cloud bridge; using
   it directly silently fails for local-provider users.
4. **Stop-session zeros health atomics + emits a final snapshot** — without it
   the HUD dots freeze on their last colour instead of going idle-gray.
5. **Cost cap is a SOFT warning**, not a hard block: emits `cost:cap-hit` with
   `blocking: false`; the UI shows a yellow "over budget" chip and AI calls
   continue. The user decides when to stop.
6. **Transparency / stealth is applied per-window after show** (a short
   HWND-grab delay) so winit/skia composition has settled first.

## Critical files

| File | ~Lines | Role |
|---|---|---|
| `slint-experiment/src/bin/overlay_host.rs` | 5200 | App entry, all windows + handlers, hotkey poll, tile spawn, session wiring |
| `overlay-backend/src/config.rs` | 2235 | Config struct + serde defaults + `ai_endpoint` resolver + default snippets |
| `overlay-backend/src/runtime.rs` | 1330 | Session lifecycle, transcript forwarder, AI ask flows, debrief, manual spawn |
| `overlay-backend/src/stt.rs` | 1260 | STT dispatch (Groq / whisper.cpp / GigaAM), VAD, prompt budgeting, retry |
| `overlay-backend/src/journal.rs` | 956 | JSONL writer, prune (count + size cap), session summary |
| `overlay-backend/src/local_ai.rs` | 910 | Local llama.cpp + GigaAM model management |
| `overlay-backend/src/audio.rs` | 550 | WASAPI loopback + mic, resampling, push-to-talk |
| `overlay-backend/src/ai.rs` | 840 | OpenAI-compat client (stream + non-stream + retry + cost) |
| `overlay-backend/src/kb.rs` | 427 | Embedded KB search (pre-lowercased) |
| `overlay-backend/src/events.rs` | 368 | `RuntimeEvents` trait (emit / spawn_tile / spawn_tile_full) |
| `slint-experiment/src/slint_session.rs` | 925 | Slint-side session orchestrator + STT pipeline |
| `slint-experiment/src/win32.rs` | 434 | HWND transparency, stealth, monitor enum, placement |
| `slint-experiment/src/markdown.rs` | 360 | pulldown-cmark → Slint block model |
| `slint-experiment/ui/settings_panel.slint` | 1318 | Grouped settings UI |
| `slint-experiment/ui/overlay_bar.slint` | 514 | Overlay bar HUD |
| `slint-experiment/ui/tile.slint` | 404 | Q/A tile card |

## Build & release

```pwsh
# Dev / build (from slint-experiment/; cargo is at ~/.cargo/bin/cargo.exe)
cargo run   --bin overlay-host
cargo build --release --bin overlay-host

# Tests + lint (both crates; no root workspace)
cargo test  --manifest-path overlay-backend\Cargo.toml
cargo clippy --manifest-path overlay-backend\Cargo.toml --all-targets
cargo clippy --manifest-path slint-experiment\Cargo.toml --bin overlay-host

# Installer (NSIS)
scripts\build-slint-release.ps1 -Installer
#   → slint-experiment/target/release/bundle/suflyor-slint-setup.exe

# Release
gh release create vX.Y.Z slint-experiment/target/release/bundle/suflyor-slint-setup.exe `
  --title "vX.Y.Z — …" --notes-file notes.md
```

Version is tracked in **2** places (keep in sync): `slint-experiment/Cargo.toml`
and `scripts/slint-installer.nsi` (`!define PRODUCT_VERSION`).

## Out-of-scope (deferred or won't-do)

- **Code signing** (Authenticode): personal tool, single user; SmartScreen
  "Unknown publisher" accepted.
- **Telemetry**: explicit non-goal.
- **overlay_host.rs split**: at ~5200 lines it's a split candidate, but the
  windows share a lot of closure-captured state; deferred until it hurts.
