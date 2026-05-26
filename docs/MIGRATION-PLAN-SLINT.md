# overlay-mvp → Slint migration plan

**Status:** proposed 2026-05-27, awaiting Phase 0 pilot to commit.
**Stack research:** memory `[[slint-ecosystem]]` (full Slint 1.16
reference + Claude Code integration story).
**Reverses:** `docs/ADR-001-stack.md` decision to stay on React if
Phase 0 pilot confirms Slint viability.

## Goal

Eliminate the WebView2-class bugs (paint flakiness, focus races,
transparent-window quirks) that produced the marathon disaster, while
keeping every existing feature (audio capture, STT, AI, push-to-talk,
F4 KB, snippets, Replay viewer, 13 Settings panels, multi-monitor
tile manager, i18n RU/EN, stealth).

## Out of scope

- Mobile (Android/iOS) — Slint can target both but we don't ship there.
- Different language for the backend — Rust stays unchanged.
- Re-architecting domain logic (`runtime`, `journal`, `kb`, etc.) —
  rewrite is UI-only.

## Phase 0 — Pilot: Replay viewer in Slint (3 days)

**Goal:** test Slint's fit on the simplest panel before committing 12
weeks. Choose Replay because it's:
- One self-contained component (`Replay.tsx`, ~600 LOC)
- Mostly static: dropdown + timeline of typed events + filter chips
- No tiles, no overlay, no markdown, no multi-window — pure single
  window with scrollable content

**Branch:** `experiment/slint-replay`

### Day 1 — Foundation
- New `slint-experiment/` workspace member under `src-tauri/` (so cargo
  can build alongside the existing crate).
- `Cargo.toml`: `slint = "1.16"`, `slint-build` in `[build-dependencies]`,
  `i-slint-backend-testing` in `[dev-dependencies]` with `mcp` feature.
- `build.rs` calling `slint_build::compile()`.
- `replay.slint` — basic window with header, sidebar dropdown, empty
  timeline area, filter chip row. Wire up `slint-lsp` live preview.
- Smoke run: `cargo run --bin slint-replay` opens an empty window.

### Day 2 — Backend wiring
- Move `list_sessions` + `load_session` Rust functions out of Tauri
  command handlers into a standalone `replay_backend.rs` module that
  both Tauri commands AND the Slint pilot can call.
- Hook Slint's `set_sessions()` / `set_events()` from the Rust main
  loop.
- Render timeline events as a `for event in events: Rectangle { ... }`
  with kind-coded colors.
- Implement filter chip toggling via Slint's `in-out property` +
  callback.

### Day 3 — Tests + decision
- Write **3 Slint-MCP tests** via `i-slint-backend-testing`:
  1. Load a session → assert N events appear in timeline.
  2. Click a kind filter chip → assert events of that kind disappear.
  3. Switch session → assert filter resets to "all".
- Capture screenshots for visual baseline.
- Write `docs/PILOT-REPORT-SLINT.md` with:
  - Time spent per task vs. estimate
  - LOC for `replay.slint` vs `Replay.tsx`
  - Subjective: how does the DSL feel? compile-error UX? hot reload?
  - Performance: render time, memory, CPU at idle
  - Concrete blockers (if any)

### Gate: Go/No-Go decision

| If pilot... | Then... |
|---|---|
| Felt smooth, < 1.5× estimated time, no blockers | Commit to Phase 1, kill the React branch |
| Took 2-3× longer, blocked on tooling, surprised by edge cases | Stop. Roll back to Variant D (React+harness). Update ADR-001 to lock decision. |
| Mixed | Try second pilot on a harder panel (Settings AI tab) before deciding |

## Phase 1 — Foundation (1 week)

**Assumes pilot greenlit.** Branch: `slint/main`.

- Restructure repo: create `src-slint/` for all `.slint` files,
  `src-rs/` for new Rust UI controllers. Keep `src-tauri/src/` only
  for backend modules that survive.
- Drop Tauri dependencies from `Cargo.toml`: `tauri`, `tauri-plugin-*`,
  `tauri-build`. Add: `slint`, `slint-build`, `raw-window-handle`,
  `windows` crate (for HWND ops).
- Reuse intact: `audio`, `stt`, `ai`, `tile` (gets rewritten — see
  Phase 4), `runtime`, `kb`, `hotkeys`, `journal`, `screenshot`,
  `config`.
- Set up the **single canonical multi-window manager** in Rust: spawn
  overlay + N tile windows + Settings + Replay as independent Slint
  windows, all sharing one `Arc<Mutex<AppState>>`.
- HWND helpers in `src-rs/win32.rs`:
  - `make_transparent_overlay(hwnd)` — applies `WS_EX_LAYERED +
    WS_EX_TRANSPARENT + WS_EX_TOOLWINDOW + WDA_EXCLUDEFROMCAPTURE`.
  - `position_on_monitor(hwnd, monitor_name)` — `EnumDisplayMonitors`
    + `SetWindowPos`. Migrates the `tile.rs::pick_monitor` logic.
  - `set_always_on_top(hwnd, bool)`, `set_stealth(hwnd, bool)`.
- Smoke run: empty overlay window appears on primary monitor with
  transparent background.

**Gate:** the four invariants pass on a no-content shell:
- Transparent overlay window renders at top + always-on-top
- Stealth toggle hides from screen-share (test in Teams/Meet)
- Multi-monitor positioning targets the correct display
- Backend modules unchanged, still compile and pass their tests

## Phase 2 — Settings panels (4 weeks, 13 panels)

Port the existing 13 Settings sections one at a time. Each port = 1
working day (port + tests + visual baseline). Order from simplest to
hardest:

1. **Stealth** (single toggle) — sanity check
2. **Coaching** (toggle + numeric input)
3. **Hotkeys** (table view, no edits)
4. **Interface** (language switcher — also wires gettext for the rest)
5. **Updates** (button → external URL, diagnostic dump)
6. **Stats** (read-only dashboard with bars)
7. **Audio** (device dropdown, level meter, test button)
8. **Auto-tiles** (toggle + chip cloud editor)
9. **Snippets** (CRUD modal — first complex panel)
10. **Knowledge base** (search + entry preview)
11. **Profile + context** (multi-text-field with import/export)
12. **AI bridge** (probe button + cost cap input)
13. **STT** (model dropdown + Whisper prompt editor)

For each panel:
- New `settings/<name>.slint` component
- Slint-MCP test: open settings → switch to panel → assert key elements
  present → trigger one mutation → assert backend received it
- Copy-contract update: pin canonical string values
- Visual baseline screenshot

i18n migrated to `@tr("key", lang)` in `.slint` via gettext (Phase 1
sets up the .po file from existing TS strings).

## Phase 3 — Overlay bar (1 week)

The crowded chip surface (`Overlay.tsx`). Rebuild as
`overlay-bar.slint` with:
- Status text + 3-dot HUD
- STT lang chip, mic mute, push-to-talk hold buttons
- Session timer, $ cost chip (when budget set)
- F1-F11 hint, ⭐ bookmark, 💡 followups, ⚙ Settings
- Drag region (HWND-level `WM_NCLBUTTONDOWN(HTCAPTION)` instead of
  Tauri's `startDragging()`)

Slint-MCP tests:
- Each chip click triggers expected backend call
- Bar grows when content adds (e.g. timer starts)
- Tooltip text matches i18n

## Phase 4 — Tile windows + markdown (2 weeks)

The hardest part. Tiles need:
- Per-monitor spawn (Phase 1 helpers)
- Markdown body with code highlighting
- Chrome row (source label + age + chrome buttons + close)

Week 1: scaffold tile window (`tile.slint`), wire spawn from Rust,
non-markdown body (plain text first). Verify multi-monitor placement
matches current behavior.

Week 2: build the **pulldown-cmark + syntect → StyledText adapter** in
`src-rs/markdown.rs`:
- Walk pulldown events
- For paragraphs → `Text` with default style
- For code blocks → run syntect over content, emit `StyledText` with
  per-token color runs
- For lists → indented `Text` with bullet prefix
- For tables → `GridLayout` (Slint native)
- For inline code → `StyledText` with monospace font

Slint-MCP tests:
- Tile spawns on primary monitor (matches our 2026-05-26 default)
- Tile spawns at expected pixel coordinates
- Tile renders Cyrillic + emoji correctly
- Code block renders with syntect colors
- Click × closes tile
- Click 📌 pins tile (TTL reaper skips it)
- Double-click on tile-bar does NOT trigger OS maximize
  (`maximizable: false` via HWND — covers the bug #2 regression)

## Phase 5 — F4 KB palette + snippets (1 week)

Rebuild as `palette.slint`:
- Global F4 hotkey (via `windows::Win32::UI::Input::KeyboardAndMouse::RegisterHotKey`)
- Input field, RECENT chips, results list
- Enter spawns a tile via Phase 4 plumbing
- `/snippet` and `+key body` syntax preserved

Slint-MCP tests:
- Type "kubernetes" → results appear within 200 ms
- Arrow keys navigate, Enter spawns a tile
- Esc closes
- Click outside closes (the v0.1.2 regression doesn't reappear)
- `/snippet` lists snippets
- `+key body` adds a snippet

## Phase 6 — i18n migration + Slint MCP testing setup + visual gates (1 week)

- Extract canonical strings from `src/i18n.ts` into `i18n.pot` via
  `slint-tr-extractor`.
- Translate to `ru.po` + `en.po`.
- Bundle via `slint_build::CompilerConfiguration::with_bundled_translations()`.
- Rewrite all `.slint` strings to `@tr("...")`.
- **Wire Slint MCP server** (the killer integration): cargo feature
  `mcp` on `i-slint-backend-testing`, env var `SLINT_MCP_PORT=8080`,
  Claude Code config to connect.
- Update `CLAUDE.md § Methodology` to replace `scripts/visual_check.ps1
  + computer-use` with the Slint MCP flow.
- Delete `src/i18n.ts`, `src/i18n.test.ts`, the React copy-contract
  tests.

## Phase 7 — Cut React, ship v0.2.0 (1 week)

- Delete `src/` (React), `index.html`, `vite.config.ts`,
  `tsconfig.json`, `eslint.config.js`, `package.json`,
  `package-lock.json`, `vitest.config.ts`.
- Delete `node_modules/`.
- Update `CLAUDE.md`: remove React-specific sections (Tier 2, Tier 4
  Vitest), add Slint-specific (`@tr()`, `.slint` file conventions,
  Slint MCP testing).
- Update `git-gate.ps1` to drop `tsc` + `eslint` + `vitest` steps; add
  `cargo test --bin overlay-slint --features mcp` (the Slint integration
  test bin).
- Update `scripts/ci.ps1` to Rust-only.
- Final full live-test pass: every panel, every hotkey, every tile
  spawn path. The same 6-gate methodology but layers 3-6 are now done
  via Slint MCP instead of computer-use.
- New NSIS installer.
- Bump to `v0.2.0` (major bump — different stack, fresh release notes).
- Commit + push + GitHub release.

## Total timeline: 12 weeks (3 months) solo

| Phase | Effort | Cumulative |
|---|---|---|
| 0 Pilot | 3 days | 3 days |
| 1 Foundation | 1 week | 1.5 weeks |
| 2 Settings (13 panels × 1 day) | 4 weeks | 5.5 weeks |
| 3 Overlay bar | 1 week | 6.5 weeks |
| 4 Tiles + markdown | 2 weeks | 8.5 weeks |
| 5 F4 palette | 1 week | 9.5 weeks |
| 6 i18n + MCP + gates | 1 week | 10.5 weeks |
| 7 Cut React + ship | 1 week | 11.5 weeks |
| Buffer for surprises | 0.5 week | 12 weeks |

## Risk register

| Risk | Mitigation |
|---|---|
| Markdown adapter takes way longer than 2 weeks | Phase 4 expands; the rest is fixed-cost. If markdown drags past 4 weeks, re-evaluate going back to React only for tile rendering (hybrid). |
| Slint multi-monitor API has gaps we don't anticipate | Phase 1 includes full Win32 helpers as fallback. If even those don't work for a specific monitor configuration, we're stuck — re-open ADR. |
| Slint hot-reload UX breaks productivity | If `slint-lsp` live preview is unreliable, fall back to `cargo run` cycle (~2-5 s) — slow but works. |
| WesAudio / LibrePCB are the only production proof — what if they hit a wall we will too | Talk to one of them publicly (Slint forum) at end of Phase 1 to validate our architecture choices. |
| 12 weeks of solo work without intermediate ships will starve the user of features | The Tauri/React `master` branch keeps shipping critical fixes via the harness; Slint work lives on `slint/main` until Phase 7. |
| License: royalty-free attribution feels bad for a pet project | Decide upfront in Phase 0 docs. For a solo pet project shipping to <100 users, attribution is reasonable; commercial license is $$. |

## Pre-Phase-0 prerequisites

1. **Install `slint-lsp`** for IDE support during pilot:
   `cargo install slint-lsp` (once).
2. **Bookmark** the migration plan + reference doc; pin the Slint
   forum + discord for support.
3. **Pick license tier**: royalty-free (free + attribution) vs.
   commercial (paid, no attribution). For pet-project scope, royalty-free.

## Trigger to abort the whole migration

If at any phase gate:
- Slint produces a bug we cannot fix with HWND-level Win32 in < 2 days
- We discover the user-facing regression class isn't actually fixed
  (e.g. transparent overlay still flickers on some Windows driver
  combo)
- Energy / time runs out — react+harness is a viable long-term state

Stop. Document why in `docs/PILOT-REPORT-SLINT.md`. Lock ADR-001 to
React/Tauri permanent. Apply the 12 weeks to fixing existing bugs
under the methodology instead.
