# Integration & Static Guard Test Guide (`slint-experiment/tests/`)

Guide for inspecting, authoring, running, and maintaining integration and static guard tests in `slint-experiment/tests/`.

---

## 1. Guard Test Purpose & Categorization

Integration and static guard tests in `slint-experiment/tests/` protect Suflyor against structural regressions, translation drift, stale window state, version desynchronization, asset non-conformance, and layout compression *before* code is built for release or submitted to manual/visual QA.

Tests in `tests/*.rs` fall into two primary categories:

1. **Static Source & Asset Integrity Guards (Source Scanners):**
   - Directly parse `.slint`, `.rs`, `.po`, `.nsi`, or `.svg` workspace files to enforce invariants beyond the reach of the Rust compiler or Clippy.
   - *Key Examples:*
     - `i18n_guard.rs`: Enforces `@tr("...")` usage for user-facing UI text in `.slint` and verifies matching `msgid` entries in `translations/ru/LC_MESSAGES/slint-replay.po`.
     - `settings_reset_guard.rs`: Scans `ui/settings_panel.slint` for transient `*-status`/`*-result` string properties and ensures matching `win.set_<name>(...)` calls exist inside `populate_token_status()` (`settings_controller.rs`) to prevent stale UI state on reused windows.
     - `version_guard.rs`: Ensures version synchronization between `slint-experiment/Cargo.toml` (`CARGO_PKG_VERSION`) and `scripts/slint-installer.nsi` (`!define PRODUCT_VERSION`).
     - `icon_guard.rs`: Validates SVG icons under `assets/icons/` against standard `viewBox="0 0 16 16"` grid and `stroke-width="1.6"` conventions.
     - `codex_copy_guard.rs`: Verifies device code UI bindings, clipboard module isolation, and accessible labels.

2. **Headless Slint Window & Layout Geometry Tests (Component Integration):**
   - Use `slint::include_modules!()` and `i_slint_backend_testing::ElementHandle` to compile Slint components headlessly and assert size, visibility, and layout behavior without spawning real OS desktop windows.
   - *Key Examples:*
     - `overlay_bar_geometry.rs`: Sizes `OverlayBarWindow` headlessly, toggles window properties (`set_open_tiles`, `set_deep_lock`), and verifies element metric bounds (e.g. element widths at 1280px vs 1600px).
     - `lock_chip_geometry_guard.rs`, `lock_chip_layout_guard.rs`, `lock_mode_menu_guard.rs`: Verify chip layout calculations and menu bounds.
     - `tera_tts_layout_guard.rs`, `tile_player_layout_guard.rs`: Verify component layout structure.

---

## 2. Gate Script Selection & Execution

### Running Tests via Cargo

- **Run all integration and guard tests:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml
  ```
- **Run a single guard test target:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test i18n_guard
  ```

### Gate Script Selection (`scripts/git-gate-native.ps1`)

- **UI-Only Targeted Gate:** When a diff in `slint-experiment` touches ONLY `.slint` files, assets, or `.po` translations (with zero changes to `.rs`, `.toml`, `.lock`, or `build.rs`), `git-gate-native.ps1` runs a targeted UI check executing a specific array of static guards:
  `codex_copy_guard`, `i18n_guard`, `icon_guard`, `lock_chip_geometry_guard`, `lock_chip_layout_guard`, `lock_mode_menu_guard`, `rc3_regression_guard`, `settings_reset_guard`, `tera_tts_layout_guard`, `tray_guard`, `version_guard`.
- **Registering New Static Guards:** If you author a new static UI guard test that should run during UI-only targeted gates, add its test name to the `$guards` list in `scripts/git-gate-native.ps1` (and `scripts/git-gate-macos.sh` if applicable).
- **Full Rust Diff:** Any Rust file change in `slint-experiment` triggers a full `cargo test`, running all test targets in `tests/*.rs`.

---

## 3. Fixture & Path Sensitivity

- **Manifest-Relative Resolution:** `cargo test` executes test binaries with a working directory that may vary based on how the toolchain or gate scripts are invoked. ALL static file reads MUST construct paths relative to `env!("CARGO_MANIFEST_DIR")`:
  ```rust
  let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
  let slint_path = root.join("ui/settings_panel.slint");
  let nsi_path = root.join("../scripts/slint-installer.nsi");
  ```
- **Platform-Neutral Paths:** Avoid hardcoded OS-specific path separators (e.g. `ui\\settings_panel.slint`). Use `Path::join()` or forward slashes in string literals, which Rust normalizes across Windows, Linux, and macOS.
- **Descriptive Failure Context:** When reading workspace files in static guards, provide clear file path context in error panics:
  ```rust
  let src = std::fs::read_to_string(&file_path)
      .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));
  ```

---

## 4. Required Allow Attributes for Test Targets

- **Clippy Lint Inheritance:** `slint-experiment/Cargo.toml` specifies strict production lints (`unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`).
- **File-Level Inner Attribute Requirement:** Each integration test file in `tests/*.rs` is compiled as an independent integration test target crate. EVERY file in `tests/*.rs` MUST begin with the file-level inner allow attribute:
  ```rust
  #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
  ```
- **Why It Is Mandatory:** Without `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, running `cargo clippy --all-targets` or `cargo test` will fail with lint errors on standard test assertions (`assert!`, `assert_eq!`, `.expect()`, `.unwrap()`).

---

## 5. Scope Boundaries: When Runtime Visual Evidence Remains Necessary

Static guard tests and headless `ElementHandle` geometry assertions provide fast, automated checks, but they operate within strict boundaries.

### What Guard Tests & Headless Tests Catch
- Structural text wrapping (`@tr`) and translation `.po` parity.
- Clean property reset invariants on reused windows (`populate_token_status`).
- Version string alignment across configuration and installer scripts.
- Icon SVG grid (`16x16`) and stroke (`1.6`) specification conformance.
- Headless widget size and element presence metrics.

### What Guard Tests CANNOT Catch
- **Windows DWM Transparency & Rendering:** `WS_EX_LAYERED`, `WS_EX_TRANSPARENT`, acrylic/blur window composition, and alpha channel rendering on Windows.
- **Visual Color Scheme & Contrast Accuracy:** Actual theme color rendering (Glacier, Graphite, Obsidian, Light Frost) across UI elements.
- **Tofu & Glyph Fallbacks:** Missing Unicode symbol rendering in the Skia graphics engine (e.g. emoji rendered as missing glyph squares).
- **Multi-Monitor & Z-Order Behavior:** `HWND_TOPMOST` ordering, screen capture stealth affinity (`WDA_EXCLUDEFROMCAPTURE`), and multi-monitor positioning across mixed DPI or landscape/portrait displays.
- **Live Paint & Repaint Glitches:** Repaint artifacts on reused windows or live animation transitions.

### Mandatory Visual Gate Rule
Passing static and headless tests in `tests/*.rs` is necessary, but **NOT sufficient** for declaring UI work complete. Any `.slint` modification or UI-affecting Rust change MUST be visually verified using the embedded Slint MCP server (`cargo build --features ui-mcp`, `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=9123`, `take_screenshot`) as documented in `slint-experiment/ui/AGENTS.md`.
