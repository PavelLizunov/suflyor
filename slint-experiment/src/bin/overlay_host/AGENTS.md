# Overlay Host Orchestration Guide (`slint-experiment/src/bin/overlay_host/`)

Guide for inspecting, extending, and verifying the host orchestration modules of Suflyor's `overlay-host` binary. Run every Cargo or live-app command below on the target homelab worker against the exact candidate SHA, never on the DSH control plane.

---

## 1. Module Map & Ownership

The host orchestration layer lives under `slint-experiment/src/bin/overlay_host/`. It separates platform runtime assembly, UI-thread window management, hotkeys, AI tile streaming, and tab-specific settings controllers.

### Core Architecture & Modules

| Subsystem | Module | Primary Responsibility |
|---|---|---|
| **Platform Assembly** | `overlay_host.rs` | Thin binary entrypoint (`windows_subsystem = "windows"`), compiles Slint UI via `mod ui`, includes platform root. |
| | `overlay_host_windows.rs` | Canonical platform runtime root (`main()`), Tokio runtime initialization, event loop timers, module globs. |
| **Window Lifecycle & Stealth** | `window_lifecycle.rs` | `WindowRegistry`, global stealth (`STEALTH_ON`, `STEALTH_EFFECTIVE`), theme schemes (`COLOR_SCHEME`), tile opacity (`TILE_BODY_OPACITY_BITS`), off-screen window parking (`present_window_stealth_aware`). |
| **Hotkeys & Input** | `hotkeys.rs` | Global hotkey registration (`register_hotkeys`, `RegisteredHotkeys`) via `GlobalHotKeyManager`, hotkey diagnostics (`HotkeyDiag`). |
| | `kbd_shortcuts.rs` | Per-window Winit keyboard shortcut filter (`kbd_shortcuts::install`) handling Ctrl+C/V/X/A/Z/Y for editable Slint fields. |
| **Tile Engine** | `tile_controller.rs` | `OverlayBarBridge` (`RuntimeEvents` sink, conversation map, `handle_ai_event`), `install_streaming_tile`, wrong-tile race guard (`GenGatedEvents`), `PttStreamSink`. |
| | `tile_window.rs` | Tile presentation, HWND positioning (`pick_monitor`), HiDPI, layered transparency (`WS_EX_LAYERED`), stealth affinity, window drag/maximize, `TILE_DISPLAY_SEQ`. |
| | `tile_ask.rs` | Main ask trigger entrypoints (`fire_f3_reask`, `fire_f6_manual_spawn`, `fire_f9_ask`, `fire_ptt_ask`). Prompt assembly and AI stream initiation. |
| | `tile_routes.rs` | Routing enums (`AskRoute`: Text, Vision, Cloud; `LiveRoute`, `live_route()` provider selector). |
| | `tile_ptt.rs` | Push-to-talk ask stream handling, 30-second watchdog, PTT tile error notifications. |
| | `tile_followup.rs` | Tile follow-up actions (`fire_followup_ask`, `fire_regenerate`, `wire_escalate`, `wire_voice_followup`, `VFU_TX` drain channel). |
| | `tile_cost.rs` | Budget/cost cap calculation (`warn_if_over_cost_cap`), labeled transcript selection (`select_recent_labeled`). |
| | `tile_copy.rs` | Markdown block clipboard copy formatting, transcript formatting, unit tests. |
| **Settings System** | `settings_controller.rs` | Primary settings window controller (`open_settings`), profile management, setting tab wiring, `populate_token_status()` (resets transient properties). |
| | `settings_ai.rs` | Cloud AI & local server settings (`wire_ai_settings`), model dropdown fetch (`fetch_models`), `ModelTarget`. |
| | `settings_stt.rs` | Speech-to-Text provider settings (`wire_stt_settings`), GigaAM GPU toggle, Whisper configuration, connection tests. |
| | `settings_vision.rs` | Vision screenshot settings (`wire_vision_settings`), provider switch, endpoints, connection tests. |
| | `settings_voice.rs` | Neural TTS read-aloud voice settings (`wire_voice_settings`), Piper Irina/Ruslan selection, playback speed, test audio. |
| | `settings_memory.rs` | SQLite memory tab callbacks (`wire_memory`), memory review, extraction, and deletion. |
| | `settings_hermes.rs` | Hermes agent integration tab & process-lived handle (`settings_hermes::wire`). |
| | `settings_import_export.rs` | Server-settings import/export callbacks (`wire_import_export`), server preview application (`apply_server_preview`). |
| | `settings_local_ai.rs` | One-click local AI server installer pipeline (`wire_local_ai`). |
| | `settings_updates.rs` | GitHub release update checker and verified installer callbacks (`wire_updates`). |
| | `settings_mlx.rs` | Apple Silicon MLX local backend settings callbacks (`include!("settings_mlx.rs")` inside `settings_controller.rs`). |
| **Auxiliary & Specialized** | `aux_windows.rs` | Thin facade for shared auxiliary-window state and re-exports; implementations live in its child modules. |
| | `aux_windows/{text_ask,help_palette,archive,transcript}.rs` | Text Ask, Help/F4 Palette, session Archive, and transcript-window implementations. |
| | `read_aloud.rs` | Selected-text clipboard handling, read-aloud/OCR result tiles, and closed-read-tile restoration. |
| | `bar_tray.rs` | Bar status/size/placement, hide-to-tray lifecycle, tray menu dispatch, and macOS status visibility synchronization. |
| | `local_watchdog.rs` | Pure local-AI watchdog cooldown/failure policy and its unit tests; live probes and process starts remain in `main()`. |
| | `status_copy.rs` | Active-stack labels, model/status copy, manual-tile naming, and related pure formatting helpers. |
| | `vision_capture.rs` | Screen capture execution (F8 monitor, Shift+F8 drag region), BGRA to Slint image conversion, AI vision endpoint stream dispatch (`fire_f8_vision_capture`, `launch_vision_for_bgra`). |
| | `recovery.rs` | Crash context recovery markers (`build_recovery_block`, `strip_recovery_block`, `compose_recovery_context`), recovery dialog (`open_recover_offer`). |
| | `wizard.rs` | First-run setup onboarding wizard (`open_wizard`), step-by-step setup, mic check, summary generation. |
| | `diagnostics.rs` | Self-diagnostics tab population (`populate_diagnostics`), redacted diagnostic report generator (`build_diag_report`, `redact_ipv4`, `redact_urls`). |
| | `transcript_player.rs` | Audio player engine (`rodio`) for transcript window mini-player. |
| | `mlx_lifecycle.rs` | Process lifecycle and status management for local MLX backend processes on macOS. |
| | `capture_watchdog.rs` | Capture health watchdog policy (macOS screen capture privilege checks). |

---

## 2. Entrypoint & Runtime Assembly Relationship

1. **Thin Entrypoint (`src/bin/overlay_host.rs`):**
   - Configures the process subsystem (`#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`).
   - Compiles Slint UI files via `mod ui { slint::include_modules!(); }`.
   - Delegates execution by platform `include!("overlay_host_windows.rs");`.

2. **Root Compilation Module (`src/bin/overlay_host_windows.rs`):**
   - Declares and re-exports top-level host modules via `#[path = "overlay_host/<module>.rs"] mod <module>; use <module>::*;`.
   - `aux_windows.rs` is a thin facade that declares its normal child modules with `#[path = "aux_windows/<module>.rs"]` and re-exports only the root-facing window helpers.
   - Special inclusion case: `settings_mlx.rs` is included inside `settings_controller.rs` via `include!("settings_mlx.rs");`.
   - Houses `fn main()`, which initializes Tokio runtime (`shared_runtime()`), launches the Slint event loop, constructs the `OverlayBarWindow`, wires global timers for hotkey polling and async event drains, and processes UI events.

---

## 3. UI-Thread & Window Lifecycle Orchestration

Slint requires all window instantiation, widget property updates, and event callbacks to execute on the main UI thread. Long-running tasks (AI streaming, audio recording, web downloads) run asynchronously on Tokio background threads and communicate with the UI thread via thread-safe channels (`tokio::sync::mpsc`).

### Window Registry & Global States (`window_lifecycle.rs`)

- **`WindowRegistry`:** Maintains references to active Slint window handles (`OverlayBarWindow`, `SettingsWindow`, `PaletteWindow`, `TileWindow`, etc.) so global setting changes apply across all open windows.
- **Global Stealth (`STEALTH_ON`, `STEALTH_EFFECTIVE`):** Controls screen-share exclusion using Win32 `SetWindowDisplayAffinity(HWND, WDA_EXCLUDEFROMCAPTURE)`. When stealth is enabled, all newly realized windows query `global_stealth()` and set display affinity before becoming visible.
- **Off-screen Window Parking (`present_window_stealth_aware`):**
  To prevent white/black flash artifacts or un-decorated border glitches during window creation:
  1. Window is placed off-screen at physical coordinates `(-32000, -32000)`.
  2. `win.show()` is called while off-screen.
  3. HWND is grabbed and decorated (DWM transparency, stealth WDA affinity applied).
  4. Window is moved to target monitor coordinates (`pick_monitor`).
- **Theme Schemes (`COLOR_SCHEME`):** Manages 4 color schemes (Glacier, Graphite, Obsidian, Light Frost). Propagates `Theme.scheme` updates to all registered windows.
- **Global Opacity (`TILE_BODY_OPACITY_BITS`):** Thread-safe atomic `f32` (clamped 0.5..=1.0) consulted by every tile creation path.

---

## 4. Hotkeys & Keyboard Filtering

### Global Hotkeys (`hotkeys.rs`)

- Registered once at startup via `register_hotkeys()` using `global_hotkey::GlobalHotKeyManager`.
- System-wide hotkey table:
  - **F1:** Help window (`open_help`)
  - **F3:** Re-ask last question (`fire_f3_reask`)
  - **F4:** KB Palette inline search (`open_palette`)
  - **F6:** Manual AI prompt tile (`fire_f6_manual_spawn`)
  - **F7:** Session archive
  - **F8 / Shift+F8:** Vision capture (full monitor / drag region)
  - **F9 / Shift+F9:** Text ask main / variant (`fire_f9_ask`)
  - **Shift+Alt+1 / +2 / +3:** Neural read-aloud (read selection / OCR region / pause)
- Polled in `overlay_host_windows.rs` `main()` event loop. Registration state is exposed to diagnostics via `HotkeyDiag`.

### Winit Keyboard Filter (`kbd_shortcuts.rs`)

- `kbd_shortcuts::install(win.window())` installs a per-window Winit keyboard event filter.
- Binds standard editing shortcuts (`Ctrl+C`, `Ctrl+V`, `Ctrl+X`, `Ctrl+A`, `Ctrl+Z`, `Ctrl+Y`) layout-independently to editable Slint input fields across all auxiliary and settings windows.

---

## 5. Tile System Architecture (`tile_*.rs`)

The tile system handles prompt dispatch, AI response streaming, conversation sequence tracking, tile positioning, and markdown rendering.

```
Hotkey / UI Trigger
       │
       ▼
  tile_ask / tile_routes ────► Resolves Provider (Cloud/Local) & Prompt Context
       │
       ▼
  tile_controller ───────────► Spawns Tile Window & Registers OverlayBarBridge
       │
       ▼
  Async Tokio Task ───────────► Streams AI Events to handle_ai_event()
       │
       ▼
  tile_window ────────────────► Renders Animated Markdown & Handles HWND Drag/Maximize
```

- **`tile_controller.rs`:** Owns `OverlayBarBridge` (implements `SlintUiBridge` and `RuntimeEvents`), `install_streaming_tile`, wrong-tile race prevention via `GenGatedEvents` generation IDs, and `PttStreamSink`.
- **`tile_window.rs`:** Tile presentation, HWND positioning (`pick_monitor`), HiDPI scaling, layered transparency (`WS_EX_LAYERED`), stealth affinity, drag/maximize handlers, and sequence tracking (`TILE_DISPLAY_SEQ`).
- **`tile_ask.rs`:** Entrypoints for initiating ask workflows (`fire_f3_reask`, `fire_f6_manual_spawn`, `fire_f9_ask`, `fire_ptt_ask`).
- **`tile_routes.rs`:** `AskRoute` enum (Text, Vision, Cloud) and `LiveRoute` resolution.
- **`tile_ptt.rs`:** Handles Push-to-Talk streams and 30-second watchdog timers.
- **`tile_followup.rs`:** Handles follow-up prompts, re-framing (`fire_followup_ask`, `fire_regenerate`), and voice follow-up channels (`VFU_TX`).
- **`tile_cost.rs`:** Enforces budget/token caps (`warn_if_over_cost_cap`) and selects recent labeled transcript blocks.
- **`tile_copy.rs`:** Formats markdown text for clipboard export.

---

## 6. Settings System Architecture (`settings_*.rs`)

### Reused Window State Invariant

Unlike transient tiles, `SettingsWindow` is instantiated once and shown on demand.

- **Stale State Hazard:** Any transient property (ending in `-status` or `-result`, e.g. `ai-bearer-status`, `tts-download-status`) persists across window opens unless explicitly reset.
- **Mandatory Reset Rule:** Every transient status property MUST be cleared or reseeded inside `populate_token_status()` in `settings_controller.rs` whenever Settings opens.
- Enforced by integration test: `cargo test --manifest-path slint-experiment/Cargo.toml --test settings_reset_guard`.

### Domain Tab Wiring

Settings domain controllers expose `wire_<domain>_settings()` functions called during `open_settings()`:
- `settings_ai.rs`: Cloud/Local AI endpoints, base URLs, bearer tokens, model dropdowns (`fetch_models`).
- `settings_stt.rs`: Speech-to-Text provider settings, GigaAM GPU toggle, Whisper models.
- `settings_vision.rs`: Cloud/Local vision provider endpoints and screenshot settings.
- `settings_voice.rs`: Neural TTS read-aloud (Piper Irina/Ruslan) voice selection, speech rate, sample audio test.
- `settings_memory.rs`: Curated SQLite memory table inspection, heuristic extraction, deletion.
- `settings_hermes.rs`: Hermes agent bridge connection options and profile initialization.
- `settings_import_export.rs`: Server-settings import/export, configuration preview (`apply_server_preview`).
- `settings_local_ai.rs`: Managed local llama.cpp, whisper.cpp, and model installer/lifecycle pipeline.
- `settings_updates.rs`: GitHub update checking and binary download/installer invocation.
- `settings_mlx.rs`: Apple Silicon MLX local backend settings callbacks.

---

## 7. Platform Seams & High-Risk Areas

1. **HWND Realization Race (`HWND_GRAB_DELAY_MS` / `HWND_REVEAL_FAST_MS`):**
   Winit window HWND creation is asynchronous. Grabbing an HWND before realization returns null/invalid handles. Unconditional off-screen parking (`-32000, -32000`) ensures decorations and stealth affinity apply before the window's first visible frame.
2. **Screen-Share Stealth Leakage (`WDA_EXCLUDEFROMCAPTURE`):**
   New windows created while stealth is active must explicitly query `global_stealth()` and set display affinity. Skipping this in any window opener causes overlay contents to leak into screen recordings.
3. **Generation Race Guard (`GenGatedEvents`):**
   When a user rapidly re-asks or cancels a prompt, stale stream chunks from an earlier AI task could arrive after a new tile is created. `GenGatedEvents` validates sequence numbers to drop mismatched AI response chunks.
4. **Sidecar Process Isolation Boundary:**
   `suflyor-tts` and `suflyor-teratts` sidecars MUST run in separate processes from `overlay-host` to prevent dual ONNX Runtime crashes within a single process.

---

## 8. Verification & Checks

### Static Compilation & Lint Checks

```powershell
# Fast compilation check
cargo check --bin overlay-host --manifest-path slint-experiment/Cargo.toml

# Clippy lint check
cargo clippy --manifest-path slint-experiment/Cargo.toml --bin overlay-host
```

### Static Guard Tests

```powershell
# Settings transient reset guard (verifies populate_token_status resets all -status/-result fields)
cargo test --manifest-path slint-experiment/Cargo.toml --test settings_reset_guard

# i18n translation parity guard (@tr msgid / ru.po msgstr match)
cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard

# Run all slint-experiment crate unit and guard tests
cargo test --manifest-path slint-experiment/Cargo.toml
```

### Live UI verification

Any visible behavior change requires the project `slint-mcp-ui-audit` skill on the target worker against the exact candidate SHA. The skill owns the QA build, MCP interaction, screenshots, Settings coverage, and hotkey smoke procedure.
