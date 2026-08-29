# `slint-experiment` Crate Guide

Guide for inspecting, building, maintaining, and verifying the `slint-experiment` crate (`slint-replay` package), which produces the primary `overlay-host` executable for Suflyor.

---

## 1. Crate Map & Structure

`slint-experiment` (package name `slint-replay`) is the pure Rust + Slint overlay host and UI orchestration layer. It depends on `overlay-backend` via a path dependency and compiles UI markup directly into Rust code via `build.rs`.

- **Entrypoints (`src/bin/` & `src/`):**
  - `src/bin/overlay_host.rs` — **Thin binary entrypoint**. Sets subsystem options and delegates platform runtime execution via `overlay_host_windows.rs`.
  - `src/bin/overlay_host/` — **Canonical runtime host directory**. Contains host orchestration logic including `settings_controller.rs`, `tile_controller.rs`, `hotkeys.rs`, `window_lifecycle.rs`, the `aux_windows.rs` facade with `aux_windows/{text_ask,help_palette,archive,transcript}.rs`, `read_aloud.rs`, `bar_tray.rs`, `local_watchdog.rs`, `status_copy.rs`, `vision_capture.rs`, `recovery.rs`, `wizard.rs`, `diagnostics.rs`, `kbd_shortcuts.rs`, `mlx_lifecycle.rs`, `transcript_player.rs`, and tab-specific settings controllers.
  - `src/win32.rs` — Win32 HWND manipulation, overlay transparency (`WS_EX_LAYERED`, `WS_EX_TRANSPARENT`), topmost positioning (`HWND_TOPMOST`), click-through styling, monitor placement (`pick_monitor`), and screen-capture stealth affinity (`WDA_EXCLUDEFROMCAPTURE`).
  - `src/{app_state, runtime_state, slint_session, slint_events}.rs` — Shared application state, Tokio runtime integration, session management, and Slint-to-Rust event bridge.
  - `src/{logging, markdown, lock_menu, tray, capture}.rs` — Logging infrastructure (`overlay-host.log`), CommonMark parsing, lock menu widgets, tray notification icon integration, and screen capture logic.
  - `src/native/` — Native OS window and environment abstractions for Windows and macOS.
  - `src/bin/overlay_spike.rs` & `src/bin/markdown_spike.rs` — Lightweight verification spikes.

- **UI Markup (`ui/`):**
  - Slint UI component declarations (`ui/*.slint`). All components are re-exported through the single root compilation file `ui/index.slint`.
  - **Narrower Guide:** Detailed Slint UI guidelines, component rules, design tokens, glyph policies, and UI state lifecycles are documented in `slint-experiment/ui/AGENTS.md`.

- **Translations (`translations/`):**
  - `translations/ru/LC_MESSAGES/slint-replay.po` — GNU gettext translation catalog mapping English source `@tr` strings (`msgid`) to Russian (`msgstr`).

- **Assets (`assets/`):**
  - `assets/icon.ico` — Application icon embedded into `.exe` via `winresource` in `build.rs`.
  - `assets/icons/*.svg` — Vector iconography (standard `16x16` viewBox, `stroke-width: 1.6`).
  - `assets/brand-*` — Brand mark images and logos.

- **Integration & Guard Tests (`tests/`):**
  - Static safety and layout guards including `i18n_guard.rs`, `settings_reset_guard.rs`, `overlay_bar_geometry.rs`, `version_guard.rs`, `icon_guard.rs`, `tray_guard.rs`, and platform feature guards.

---

## 2. Key Invariants

1. **Thin Entrypoint Architecture:** Production entrypoint `src/bin/overlay_host.rs` must remain a thin platform wrapper. Host orchestration logic resides in `src/bin/overlay_host/` modules and `src/` platform helpers.
2. **Clippy Production Safety:** `Cargo.toml` enforces strict lints (`unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`). Non-production test code must explicitly allow these at the file/module level (`#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`).
3. **Single Compilation Root:** `build.rs` compiles `ui/index.slint` via `slint-build`. New `.slint` files must be imported and re-exported in `ui/index.slint`.
4. **Translation Coupling:** Every user-facing UI string must be wrapped in English `@tr("...")` and have a matching `msgid`/`msgstr` entry in `translations/ru/LC_MESSAGES/slint-replay.po`. `build.rs` enforces `DefaultTranslationContext::None`.
5. **Reused Window State Cleanliness:** Reused window singletons (e.g. `SettingsWindow`) must clear or reseed all transient `*-status` and `*-result` strings in `populate_token_status()` inside `src/bin/overlay_host/settings_controller.rs` when opened.
6. **Process Isolation:** Sidecar processes (`suflyor-tts`, `suflyor-teratts`) run separately from `overlay-host` to prevent dual ONNX Runtime crashes in a single process.

---

## 3. Verification & Checks

Run matching checks on the target homelab worker against the exact candidate SHA, not on the DSH control plane:

- **Quick Compilation Check:**
  ```powershell
  cargo check --bin overlay-host --manifest-path slint-experiment/Cargo.toml
  ```

- **Clippy Lint Verification:**
  ```powershell
  cargo clippy --manifest-path slint-experiment/Cargo.toml --bin overlay-host
  ```

- **Run All Crate Unit & Guard Tests:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml
  ```

- **Key Individual Guard Tests:**
  - *i18n Parity Guard:* `cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard`
  - *Settings Reset Guard:* `cargo test --manifest-path slint-experiment/Cargo.toml --test settings_reset_guard`
  - *Version Sync Guard:* `cargo test --manifest-path slint-experiment/Cargo.toml --test version_guard`
  - *Icon Standard Guard:* `cargo test --manifest-path slint-experiment/Cargo.toml --test icon_guard`
  - *Overlay Bar Geometry Guard:* `cargo test --manifest-path slint-experiment/Cargo.toml --test overlay_bar_geometry`

---

## 4. UI Audit Trigger & Visual Verification

Changes affecting `.slint` layouts, themes, icons, or visible UI behavior require the project `slint-mcp-ui-audit` skill on the target worker. That skill is the procedural source of truth for the `ui-mcp` build, launch, screenshots, Settings coverage, and hotkey smoke; do not maintain a second copy of its steps here.
