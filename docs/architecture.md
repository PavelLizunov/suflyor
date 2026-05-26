# suflyor — architecture overview

**Audience:** developer forking or auditing the codebase. Last updated for v0.0.9 + post-marathon test commits (227 tests, autonomous run 2026-05-26).

## 3-tier data flow

```
WASAPI loopback + mic (Windows)
        │
        ▼
tokio::mpsc (audio chunks @ 16 kHz mono i16)
        │
        ▼
Whisper VAD pipeline (stt.rs):
  · per-source rolling buffer
  · noise gate (pre-Whisper)
  · Groq /openai/v1/audio/transcriptions
  · 3-attempt retry (1s/2s/4s) on 5xx/429
  · post-filter (artefacts, repetitions)
        │
        ▼
transcript events (TranscriptEvent {source, text})
        │
        ▼
runtime.rs forwarder:
  · push to rolling VecDeque (cap 80)
  · emit transcript:line Tauri event
  · maybe_spawn_tile (gated by detector_skip_mic)
  · push_speech_window (mic only)
        │
        ▼
detect_trigger (question/keyword) → KB-search injection → prompt build
        │
        ▼
Claude Haiku SSE via OAuth bridge (ai.rs)
  · complete_with_usage: 3 retries on 5xx/429, fail-fast on 4xx
  · cost_microcents accumulation
        │
        ▼
tile.rs::spawn_tile_with_stealth
  · grid_position picks first-free slot (HashSet diff)
  · TTL 120s with atomic pin-race fix
  · MAX_TILES=6 (2 cols × 3 rows)
        │
        ▼
React TileWindow renders ReactMarkdown + remark-gfm
```

## Tauri 2 security model

**Capability split:**

- `capabilities/default.json` — overlay window only. Has global-shortcut + opener + window:* perms incl. `core:window:allow-start-dragging` (v0.0.1 patch).
- `capabilities/tile.json` — tile-* windows. Narrow: no opener, no global-shortcut, no set-position/size. Can hide/show/close/drag self only.

**Caller-window guard:**

`assert_overlay(window)` is called at the top of 25 sensitive Tauri commands. Rejects any call from non-overlay window (e.g. a tile rendering AI markdown can't `invoke("export_config")` to leak the bearer + Groq key). Catches markdown-driven prompt injection.

Unprotected commands (read-only or low-blast-radius): `list_audio_devices`, `kb_search`, `kb_get`, `kb_stats`, `list_snippets`, `close_tile`, `pin_tile`, `list_monitors`.

## Frontend router

`main.tsx` dispatches by URL query param:

- `?settings=1` → Settings (13 sections, drag-region header, 760×900 window)
- `?replay=1` → Replay viewer (JSONL session journal timeline)
- `?tile=1&id=…&kind=…&q=…&a=…` → TileWindow (single Q/A card)
- default → Overlay (36px glass bar with HUD)

## Hotkeys (all global via tauri-plugin-global-shortcut)

| Key | Action | Implementation |
|---|---|---|
| F3 | Re-ask last question | `runtime::reask_last()` |
| F4 | KB palette toggle | emit `hotkey:kb-palette` → React |
| F6 | Manual tile from last transcript | `runtime::manual_spawn_tile()` |
| F8 | Pause/resume session | emit → React toggles start/stop_session |
| F9 | Ask AI now (with optional screenshot) | emit → React → invoke ai |
| F10 | Take screenshot for next ask | emit → React → invoke take_screenshot |
| F11 | PANIC HIDE (overlay + all tiles) | inline Rust handler — direct window.hide()/show() loop |
| F7 | DEBUG: spawn hardcoded test tile | debug-only |

## Key invariants

1. **Tile slot uniqueness** (`tile.rs`): each ActiveTile has a `slot: usize` field. Spawn picks the FIRST free slot via HashSet diff with occupied set. When MAX_TILES full, evict oldest and reuse its slot. Prevents the "tile spawned on tile" bug from v0.0.4 era.

2. **TTL pin race fix** (`tile.rs::take_if_unpinned`): atomic check-and-remove under single lock. Prevents the race where pin() sneaks in between is_pinned check and close().

3. **Single-instance lock** (`tauri-plugin-single-instance`): second launch focuses existing overlay window. Prevents orphan-process scenarios where global hotkeys silently fail to register on the second instance.

4. **Settings stale-state heal** (Settings.tsx): on window-focus event, re-fetch get_config. Without it, after a binary restart while the WebView stays open, Save would persist the stale React state and wipe secrets.

5. **mountedRef must reset on every mount** (Settings.tsx): React StrictMode mounts → unmounts → re-mounts in dev. Without explicit `mountedRef.current = true` at start of useEffect, second mount inherits `false` from first cleanup → all showPrompt/showConfirm silently no-op.

6. **Stop-session zeros health atomics + emits final snapshot** (`runtime.rs`): without this, HUD dots froze on last green/yellow color forever after Stop. Now they transition to idle gray.

7. **Cost cap is SOFT warning, not hard block** (v0.0.5): emits `cost:cap-hit` event with `blocking: false`, UI shows yellow "💰 over budget" chip, AI calls continue. User decides when to stop. Hard-block proved bad UX during interviews.

## Critical files

| File | Lines | Role |
|---|---|---|
| `src-tauri/src/lib.rs` | 1741 | Tauri Builder, all Tauri commands, bridge/update probes, export_safe, diagnostic dump |
| `src-tauri/src/runtime.rs` | 3259 | Session lifecycle, transcript forwarder, AI ask flows, voice coach, debrief |
| `src-tauri/src/config.rs` | 1665 | Config struct + serde defaults + default snippets |
| `src-tauri/src/stt.rs` | 965 | Whisper VAD pipeline, prompt budgeting, retry classifier |
| `src-tauri/src/tile.rs` | 758 | Tile manager, grid_position layout math, TTL reaper |
| `src-tauri/src/journal.rs` | 765 | JSONL writer, prune (count + size cap), session summary |
| `src-tauri/src/ai.rs` | 674 | Claude OpenAI-compat client (stream + non-stream + retry) |
| `src-tauri/src/kb.rs` | 319 | Embedded KB search (1643 entries, pre-lowercased) |
| `src-tauri/src/audio.rs` | 580 | WASAPI loopback + mic, resampling, push-to-talk |
| `src-tauri/src/hotkeys.rs` | 180 | Global hotkey registration |
| `src-tauri/src/tray.rs` | 106 | Tray icon + menu (Show/Hide/Settings/Quit) |
| `src/Overlay.tsx` | 763 | Main overlay bar (36px) |
| `src/Settings.tsx` | 1483 | 13-section config panel |
| `src/TileWindow.tsx` | 133 | Q/A tile card with markdown |
| `src/Replay.tsx` | 439 | JSONL journal timeline viewer |

## Test coverage (244 lib tests + 25 journal-eval tests as of v0.0.15 release)

Strong coverage:
- Config save/load + serde defaults + pricing-table sync (15 tests)
- AI retry classifier `is_permanent_ai_error` (8 tests)
- AI cost math `cost_microcents` (5 tests)
- KB search + key-trigger matcher (12 tests)
- Tile slot picker — gap reuse + eviction + ordering (4 tests)
- Tile lifecycle — pin/take/reap (8 tests)
- STT prompt budget — vocab fit, hard cap, overflow regression (9 tests)
- Detector — v2/v3/v4 patterns, fuzzy match, skip-mic gate (35 tests)
- Voice coach — WPM, fillers, pace buckets (10 tests)
- Debrief gate — duration/mic-lines/text-length thresholds (7 tests)
- Cost budget — disabled/under/at/over boundary (4 tests)
- Bridge probe model-not-found matcher (9 tests)
- Update semver compare (8 tests — equal/lower/higher, v-prefix, prerelease, empty, unequal segments, non-numeric)
- Journal — JSONL serialize, counters, prune count-based + size-cap (12 tests)
- Audio — WAV roundtrip, resample, decimator (~10 tests)
- **blank_share_secrets** (10 tests) — security-critical: each share-export
  field gets blanked / each kept field survives / idempotent
- **sanitize_diagnostic_text** (5 tests) — v0.0.15: redacts gsk_/Bearer/sk-
  patterns from crash report + journal tail before dump_diagnostics writes
- **is_permanent_ai_error** (8 tests) — retry classifier gate (400/401/403/
  404/413 permanent; 5xx/429/network transient; empty defensive)
- **Tile slot picker** (4 tests) — gap-reuse after middle close, oldest
  eviction with slot reuse, empty initial, unordered occupied

Honest gaps:
- No integration test that runs spawn_tile against a real Tauri AppHandle (mocking AppHandle is hard; pure-fn extracts cover most logic)
- No coverage for the actual SSE streaming path in `ai.rs::stream_inner` (mocking reqwest::Response::bytes_stream is heavy)
- WASAPI capture path tested only via decimator math + WAV roundtrip — no real-device test (CI doesn't have audio hardware)

## Build & release

```bash
# Dev
npm run dev               # vite only, no Tauri
npm run tauri dev         # full app with HMR

# Production
npm run tauri build       # → target/release/bundle/{msi,nsis}/

# Tests
cargo test --lib                                    # 227 tests, <1s
cargo clippy --bin overlay-mvp -- -D warnings       # strict lint

# Release
gh release create vX.Y.Z \
  target/release/bundle/msi/suflyor_X.Y.Z_x64_en-US.msi \
  target/release/bundle/nsis/suflyor_X.Y.Z_x64-setup.exe \
  --title "vX.Y.Z — …" --notes-file notes.md
```

Version is tracked in 3 places (must be kept in sync):
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

## Out-of-scope (deferred or won't-do)

- **Local Whisper** (CUDA/whisper-rs): research-only in `docs/local-whisper-options.md`. Defer until offline operation requested by a real user. Current Groq pipeline meets latency + quality needs.
- **Code signing** (Authenticode for MSI): personal tool, single user, would cost $300+/yr cert. SmartScreen "Unknown publisher" warning acceptable.
- **Auto-update download manager**: Settings → 🆙 Обновления currently opens the GitHub release page in browser. Auto-installing without signing is risky and would need elevation prompts.
- **Snippet edit/add modal** (full 3-field editor): deferred to v0.1.0. Current state: delete works (v0.0.9); add/edit via JSON in `%APPDATA%\overlay-mvp\config.json`.
- **Telemetry**: explicit non-goal. Tool is personal-use, no analytics.
