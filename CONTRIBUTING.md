# Contributing to suflyor

This is a single-user pet project, but if you fork or want to send a PR — here's the lay of the land.

## Setup

**Prerequisites:**
- Windows 10/11 (only OS supported — uses WASAPI native APIs)
- Rust + cargo via rustup, **MSVC toolchain** (not GNU): `rustup default stable-msvc`
- Visual Studio Build Tools 2022 with C++ workload (for MSVC linker)
- Node 20+ + npm
- WebView2 runtime (preinstalled on Win 11; old Win 10 needs the Edge runtime installer)

**One-time:**
```bash
git clone https://github.com/PavelLizunov/suflyor.git
cd suflyor
npm install
```

**Dev loop:**
```bash
npm run tauri dev          # full hot-reload, opens overlay window
# OR for frontend-only iteration:
npm run dev                # vite only, no Tauri (no overlay window, just URL preview)
```

**Build release MSI:**
```bash
npm run tauri build        # → src-tauri/target/release/bundle/msi/suflyor_X.Y.Z_x64_en-US.msi
```

⚠️ **Do NOT use bare `cargo build`** — it skips the vite frontend bundle, you'll get a working binary that fails at runtime trying to load `localhost:1420`. Always go through `npm run tauri build`.

## Tests

```bash
cargo test --lib                                    # ~230 tests, <1 sec
cargo clippy --bin overlay-mvp -- -D warnings       # strict lint
npm run build                                       # TS + vite build (tsc --noEmit then vite build)
```

All three should be green before any commit.

Test coverage map: see `docs/architecture.md` section "Test coverage".

## Version-bump checklist

When releasing a new version, update **THREE** files in sync:

1. `package.json` — `"version": "X.Y.Z"`
2. `src-tauri/Cargo.toml` — `version = "X.Y.Z"`
3. `src-tauri/tauri.conf.json` — `"version": "X.Y.Z"`

Then:
```bash
npm run tauri build                # produces installer files
git commit -am "vX.Y.Z — <theme>"  # standard commit message
git push
gh release create vX.Y.Z \
  src-tauri/target/release/bundle/msi/suflyor_X.Y.Z_x64_en-US.msi \
  src-tauri/target/release/bundle/nsis/suflyor_X.Y.Z_x64-setup.exe \
  --title "vX.Y.Z — <theme>" --notes-file release-notes.md
```

The in-app update button (Settings → 🆙 Обновления) reads `tag_name` from GitHub Releases API, so the git tag must match the version string.

## Hooks / autonomous mode caveat

`.claude/settings.json` configures Claude Code hooks for an autonomous-development mode this repo uses. If you fork and don't want them:

```bash
rm -rf .claude/hooks .claude/autonomous_active .claude/AUTONOMOUS_RULES.md
# OR comment out hooks in .claude/settings.json
```

If `.claude/autonomous_active` exists with a future ISO deadline, the stop-guard hook prevents Claude from ending a turn — that's intentional for marathon work but will surprise anyone unfamiliar.

## Project layout

```
src-tauri/src/         # Rust backend (audio, STT, AI, journal, tile manager)
src-tauri/capabilities/ # Tauri 2 capability files (overlay vs tile-* scope split)
src-tauri/knowledge/   # Embedded KB (1643 entries) — glossary + commands + patterns
src/                   # React frontend (Overlay, Settings, TileWindow, Replay)
docs/                  # architecture.md + local-whisper-options.md
.claude/               # Claude Code hooks + autonomous mode marker
NIGHT_RUN_PLAN.md      # Marathon work log (only useful if you're running autonomous mode)
README.md              # User-facing
```

Critical invariants (DO NOT BREAK):
- `assert_overlay(window)` guard at the top of sensitive Tauri commands (config, session, screenshot, mic, spawn_tile, expand_snippet, kb_spawn). Tile-* windows must not be able to invoke these — they could be poisoned by AI-rendered markdown.
- `core:window:default` permission set does NOT include `allow-start-dragging` — capability files explicitly add it. Same for any other window:* perm beyond the default.
- React StrictMode mounts → unmounts → re-mounts components in dev. `mountedRef.current = true` MUST be set at the start of every useEffect that uses it as a guard. Otherwise second-mount's mountedRef stays false (inherited from first cleanup) and the component silently no-ops.

## Code style

- Rust: `cargo fmt` (default 4-space indent). Comments explain WHY, not WHAT. Cite live-test or bug-hunt origin where relevant.
- TS/React: 2-space indent. Functional components only. No class components. State via useState/useReducer.
- CSS: plain CSS in `src/styles.css`, custom properties for theming. No Tailwind.
- Commit messages: imperative mood. First line under 70 chars. Body explains motivation + design tradeoffs.

## Security

Personal-use app, BUT:
- `config.json` at `%APPDATA%\overlay-mvp\config.json` contains live `groq_api_key` + `ai_bearer`. NEVER print these to chat / logs / screenshots.
- Release build excludes the auto-open DevTools call (which dev build has). In dev mode, DevTools is open by default and the entire React state (including secrets) is visible.

## Architecture deep dive

See `docs/architecture.md` for:
- 3-tier data flow diagram (audio → STT → AI → tile)
- Tauri 2 capability model
- All 7 global hotkeys
- 7 critical invariants
- Per-file size + role table
- Out-of-scope features (local Whisper, code signing, auto-update download, telemetry)

## License

GPL-3.0
