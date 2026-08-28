# Slint UI Authoring & Verification Guide (`slint-experiment/ui/`)

Guide for inspecting, authoring, and verifying Slint UI components (`ui/*.slint`) in the Suflyor host crate (`slint-experiment`). Run every Cargo or live-app command below on the target homelab worker against the exact candidate SHA, never on the DSH control plane.

---

## 1. i18n & Translation Coupling (`@tr` English Source / Russian PO)

- **English Source Rule:** Every user-facing UI string in `.slint` files MUST be wrapped in English `@tr("English string...")`. The literal argument of `@tr` serves as the `msgid`.
- **No Hardcoded Cyrillic:** Hardcoded Cyrillic text in `.slint` files is forbidden. Non-English text prevents runtime language toggling and causes fallback bugs.
- **Russian Translation PO:** Every `@tr` string used in `.slint` (including `text`, `placeholder-text`, `title`, and `accessible-label` properties) must have a corresponding `msgid`/`msgstr` entry in `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po`.
- **Non-Translatable Text:** Raw technical tokens, numbers, machine names, URLs, or dynamic strings generated inside Rust logic must NOT be wrapped in `@tr`.
- **Static Guard Test:** Validate translation parity and catch unwrapped strings with:
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard
  ```

---

## 2. Rendering Practices (No-Tofu Glyphs & SVG Icons)

- **No-Tofu Rule:** The Skia renderer used by Slint on Windows renders uncommon Unicode symbols (e.g., warning `⚠️`, checkmarks `✓`, emoji `🎤`, circled numbers `①`) as missing-glyph "tofu" squares.
- **Text Fallbacks:** For inline text status indicators and buttons, use plain ASCII representations:
  - `[!]` instead of warning emoji/symbols
  - `[ok]` instead of checkmark glyphs
  - `1)` or `[1]` instead of circled number glyphs
- **Vector Icons:** Use vector icons located in `slint-experiment/assets/icons/*.svg`.
  - Icon design standard: `16x16` viewBox with `stroke-width: 1.6`.
  - Reference icons in `.slint` via `@image-url("../assets/icons/icon_name.svg")` or use shared `Icon` / `IconButton` components from `controls.slint`.
  - Use `colorize: Theme.accent` (or appropriate token) on `Image` elements for theme-aware icon tinting.

---

## 3. Reused Settings Window State Lifecycle

- **Reused Window Pattern:** Unlike tile or palette windows which are recreated per invocation, `SettingsWindow` is instantiated once and reused across `show()` calls.
- **Stale State Hazard:** Any transient property (such as `*-status` or `*-result` strings, e.g., `ai-bearer-status`, `tts-download-status`, `whisper-model-status`) will linger across opens unless explicitly reset.
- **Reset Invariant:** Every `in-out property <string>` ending in `-status` or `-result` in `ui/settings_panel.slint` MUST be explicitly cleared or reseeded inside `populate_token_status()` (`slint-experiment/src/bin/overlay_host/settings_controller.rs`) on every window open.
- **Async State Rule:** Do not optimistically flip UI properties prior to async Rust operation completion. Update UI state only on verified completion callbacks.
- **Static Reset Guard Test:** Enforce reset coverage with:
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test settings_reset_guard
  ```

---

## 4. Component Architecture & Design Tokens

- **Single Compilation Root:** All UI files must be imported and re-exported in `slint-experiment/ui/index.slint`. `build.rs` compiles `index.slint` as a single root for `slint::include_modules!()`.
- **Per-Window Globals:** `Theme` (`theme.slint`) and `Metrics` (`metrics.slint`) globals are per-window instances in Slint. Live color scheme changes (`Theme.scheme`) must be propagated to each active window instance individually from Rust host code (`win.global::<Theme>().set_scheme(n)`).
- **Design Tokens over Hex Literals:** Use design token roles (`Theme.bg-base`, `Theme.bg-surface`, `Theme.text-primary`, `Theme.accent`, `Theme.font-sans`, `Theme.font-mono`, etc.) instead of introducing one-off hex values or system font strings. Recheck contrast visually across all built-in schemes; token use alone does not prove WCAG compliance.
- **Window Management:** HWND creation, transparency, topmost positioning (`HWND_TOPMOST`), click-through, monitor placement, and screen-capture stealth affinity (`WDA_EXCLUDEFROMCAPTURE`) are owned by Rust host helpers (`win32.rs`, `apply_overlay_hwnd`).

---

## 5. Mandatory Visual Evidence

Passing Clippy and tests does not verify layout, color, transparency, window behavior, or live state. Load and follow the project `slint-mcp-ui-audit` skill for every visible change; it exclusively owns the QA build/launch procedure and exact evidence matrix. Do not replace its screenshots or functional interaction checks with desktop capture, compile success, or hotkey registration logs.
