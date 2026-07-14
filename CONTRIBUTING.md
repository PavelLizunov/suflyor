# Contributing to suflyor

Single-user pet project, but if you fork or want to send a PR — here's the lay of the land. The app is **pure Rust + [Slint](https://slint.dev)** (no Node, no browser engine).

## Setup

**Prerequisites:**
- Windows 10/11 (only OS supported — uses WASAPI + Win32 native APIs)
- Rust + cargo via rustup, **MSVC toolchain** (not GNU): `rustup default stable-msvc`
- Visual Studio Build Tools 2022 with the C++ workload (for the MSVC linker)

**One-time:**
```bash
git clone https://github.com/PavelLizunov/suflyor.git
cd suflyor
```

**Dev loop:**
```bash
cd slint-experiment
cargo run --bin overlay-host          # builds + launches the overlay
```

`.slint` UI files are compiled at build time (via `build.rs` + `slint-build`); editing them triggers a rebuild on the next `cargo run`.

**Build release + Windows installer:**
```powershell
scripts\build-slint-release.ps1 -Installer
# → slint-experiment\target\release\bundle\suflyor-slint-setup.exe (NSIS)
```

## Tests / gates

All three crates must pass before any commit. Run the same full gate as CI:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1
```

It runs fmt, clippy with warnings denied, full tests for `overlay-backend`,
`slint-experiment`, and `suflyor-tts`, plus the i18n guard. Do not replace it
with `cargo test --lib`; that skips integration guard tests.

## Project layout

```
slint-experiment/        # the product: `overlay-host` binary
  src/bin/overlay_host.rs # thin entrypoint
  src/bin/overlay_host/   # multi-window manager, callbacks, settings, tiles, diagnostics
  src/                    # app_state, slint_session, win32 (HWND helpers), markdown, …
  ui/*.slint              # overlay_bar / tile / palette / settings_panel + tokens
  translations/ru/…/*.po  # bundled RU translation (gettext-style, msgctxt-free)
overlay-backend/         # shared crate (no UI): audio, stt, ai, journal, kb, config, runtime
  knowledge/*.md          # embedded KB (~1696 entries) — glossary + commands + patterns
suflyor-tts/             # separate read-aloud/diarization sidecar (must stay a separate process)
scripts/                 # build-slint-release.ps1, ci.ps1, capture/click helpers
docs/                    # ADRs + migration history (the React→Slint move)
.claude/                 # Claude Code hooks (git-gate, etc.)
README.md                # user-facing
```

## Critical invariants (DO NOT BREAK)

- Transparent always-on-top windows are set via Win32 in `slint-experiment/src/win32.rs` (`make_transparent_overlay`/`_tile`, `set_always_on_top`, `set_stealth`). Apply them after the HWND exists (a short Timer after `show()`).
- Tile windows are not `Send`; spawn them on the UI thread. Cross-thread work (record/STT/AI) ships results back via an mpsc channel drained by a UI-thread Timer, or `slint::invoke_from_event_loop` with a `slint::Weak` (Weak IS Send).
- Slint+skia font fallback renders some glyphs (✕ ✓ ⏹ …) as empty boxes — prefer ASCII or a `Rectangle` over an exotic glyph.
- gettext matches by exact msgid; `build.rs` uses `DefaultTranslationContext::None`, so the `.po` is context-free and every `@tr("…")` string must have an exact matching `msgid`.

## Code style

- Rust: `cargo fmt` (4-space). Comments explain WHY, not WHAT; cite live-test / bug-hunt origin where relevant.
- Crate lints deny `unwrap_used` / `expect_used` / `panic` — use `?`, `match`, `unwrap_or_*`.
- Commit messages: imperative mood, first line < 70 chars, body explains motivation + tradeoffs.

## Version bump / release

Releases and tags are owner-triggered only. When explicitly requested, keep
`slint-experiment/Cargo.toml` and `scripts/slint-installer.nsi`
(`PRODUCT_VERSION`) in sync, run the full gate, then build with
`scripts\build-slint-release.ps1 -Installer`. Do not publish or push a tag
without the owner's explicit command.

## Security

- `%APPDATA%\suflyor\config.json` holds live API credentials. NEVER print its contents to chat, logs, or screenshots.
- The diagnostic dump (Settings → Updates) blanks secrets before writing.

## License

GPL-3.0
