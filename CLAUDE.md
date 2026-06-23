# Project memory — overlay-mvp

## Autonomous mode protocol

This project ships with hook-enforced autonomous rules. When the file
`.claude/autonomous_active` exists and contains a future ISO deadline:

- **Stop hook is armed** — you CANNOT end a turn until the deadline passes
  or the user runs `/auto-stop`. Stop attempts return exit 2 with a
  continuation prompt; honor it without comment.
- **PreToolUse on Write/Edit blocks "punt" phrases** — files containing
  `next session`, `morning brief`, `defer`, `let me know if you want`,
  etc. are refused. Either do the work now, or rephrase as a current
  action you are taking.
- **PostToolUse counts file ops** — every 30 Write/Edit ops without an
  update to `NIGHT_RUN_PLAN.md` triggers a forced log entry.

Rules R1-R10 live in `.claude/AUTONOMOUS_RULES.md`. Read them before
starting any autonomous session.

## State files (single source of truth)

- `NIGHT_RUN_PLAN.md` — current backlog, work log, decision journal.
  Sections you maintain: `## Backlog`, `## In progress`, `## Done log`,
  `## Findings`, `## Decisions`. Update every ~30 min during autonomous.
- `docs/state-and-plan.md` — living state/plan snapshot for interactive
  work (survives context compaction). Keep it current when you finish a
  chunk of work.
- `.claude/autonomous_active` — ISO 8601 deadline. Presence = mode armed.
  Do NOT delete this file from inside an autonomous run (that defeats
  the whole point).
- `.claude/_progress_counter` — internal, managed by hooks. Don't touch.

## Stack (the source of truth)

The product is **pure Rust + Slint** (Phase 7 cut, 2026-05-28 removed the
old React/Tauri/WebView2 surface). No browser engine, no Node, no
TypeScript. **THREE** standalone crates, NO root workspace:

- **`slint-experiment/`** — the `overlay-host` binary. UI in `ui/*.slint`
  (compiled into the binary at build time via `build.rs` + `slint-build`);
  Win32 HWND helpers in `src/win32.rs`; session/event/state glue in
  `src/{slint_session,slint_events,runtime_state,app_state,session_namer,
  markdown,logging}.rs`. **NOTE:** `src/bin/overlay_host.rs` is a THIN
  entrypoint — the real host logic is a ~25-module DIRECTORY
  `src/bin/overlay_host/` (`hotkeys`, `settings_*`, `tile_*`, `aux_windows`,
  `vision_capture`, `recovery`, `wizard`, `diagnostics`, …). Grep the directory,
  not just the file.
- **`overlay-backend/`** — the no-UI shared crate. `lib.rs` exports 24 modules:
  ai, audio, components, config, conspect, events, health, journal, kb,
  local_ai, memory, ocr, ocr_install, paths, persistence, re_transcribe,
  recorder, runtime, session_names, stt, tts, tts_install, update, vision.
  `slint-experiment` depends on it via a path dep.
- **`suflyor-tts/`** — the neural read-aloud SIDECAR (`suflyor-tts.exe`, shipped
  beside overlay-host in the installer). Links sherpa-onnx (TTS) ONLY and MUST
  stay a separate process: two onnxruntimes in one binary crash on the 2nd model
  load (the app keeps `ort`/GigaAM STT). DO NOT merge it back into
  overlay-backend. Takes stdin line-commands (SPEAK/PAUSE/RESUME/STOP/VOICE/RATE).

Run/build from `slint-experiment/`:
```pwsh
# cargo lives at ~/.cargo/bin/cargo.exe; Git Bash often misses it — call it
# by full path or prepend it to PATH.
cargo run   --bin overlay-host
cargo build --release --bin overlay-host
```
Installer (NSIS): `scripts/build-slint-release.ps1 -Installer` →
`slint-experiment/target/release/bundle/suflyor-slint-setup.exe`. Version
lives in BOTH `slint-experiment/Cargo.toml` and `scripts/slint-installer.nsi`
(`!define PRODUCT_VERSION`) — keep them in sync.

## Methodology — verification before commit (adopted from vpnctl, 2026-05-26)

**Why this exists:** the v0.0.67 → v0.1.2 attempt was a 33-release marathon
where static checks (clippy + cargo test) passed every release but the user
caught regressions live in layout, focus races, multi-monitor geometry, and
i18n drift. The user cut 64 of 68 GitHub releases by hand and asked for the
vpnctl methodology (where there were no large bugs). **No marathons** — fewer,
better-verified releases. See memory `[[no-marathon-releases]]`.

### The layers

Each layer catches a strict subset the others miss. **Do not skip.**

| # | Layer | Tool | Catches |
|---|---|---|---|
| 1 | clippy | `cargo clippy --manifest-path overlay-backend\Cargo.toml --all-targets` and `... slint-experiment\Cargo.toml --bin overlay-host` | API misuse, dead code, `unwrap`/`expect`/`panic` outside `#[cfg(test)]` (both crates `deny` these via `[lints.clippy]`) |
| 2 | cargo test | `cargo test --manifest-path overlay-backend\Cargo.toml` (bulk of unit tests live here) + `... slint-experiment\Cargo.toml` | Rust unit + integration |
| 3 | fmt | `cargo fmt --manifest-path <crate>\Cargo.toml` (run, NOT `--check`, then commit any change) | rustfmt drift — the most common gate killer |
| 4 | review-agent | `Agent(subagent_type: general-purpose, prompt = docs/REVIEW_AGENT_PROMPT.md)` BEFORE commit | Logic bugs, security, library misuse, codebase duplicates |
| 5 | Live install + smoke | run the freshly-built `overlay-host.exe`, read the startup log + visually confirm | Runtime crashes, transparency/paint glitches, the bar landing on the wrong monitor, anything static checks can't see |

Logic-only changes (detector regex, kb parser, cost math) need 1-4.
Anything that touches the Slint UI / window geometry / transparency needs all five.

### Blocking workflow before every commit

```
1. review-agent      (independent — paste full diff + invariants, do NOT
                      reference "the discussion above")
2. clippy + test + fmt  (both crates — commands above)
3. live smoke        (run overlay-host.exe; read its log; confirm the bar +
                      the changed surface render correctly)
4. git commit / push (auto-gated by .claude/hooks/git-gate.ps1 — BLOCKS if
                      fmt/clippy fail on commit, or tests fail on push, for
                      EITHER crate; --no-verify bypasses)
```

The `git-gate.ps1` PreToolUse hook is the ONLY piece that genuinely BLOCKS
bad commits. Setup:
- `.claude/settings.json` registers the hook against the `Bash` matcher.
- `.claude/hooks/git-gate.ps1` runs `cargo fmt --check + clippy` (both
  crates) on every `git commit`, plus `cargo test` (both crates) on every
  `git push`.
- `--no-verify` bypasses with a WARN line (rare; hotfix only per below).
- After editing the hook OR `settings.json`: RESTART Claude Code (the
  settings watcher does not pick up changes mid-session).

**Hotfix-only short-circuit** (review-agent skippable ONLY if ALL THREE):
- impl ≤ 5 lines
- touches exactly ONE surface
- changes no user-facing string with a `ru.po` translation

### Live-smoke / visual verification (layer 5) — CRITICAL gotcha

**computer-use screenshots MIS-RENDER the transparent overlay's COLOURS**
(they showed the bar dark when the active theme is light). Ground truth =
**`CopyFromScreen` at the window's HWND rect** (Win32 `EnumWindows` +
`GetWindowRect`, filter by pid + window title `overlay-mvp (Slint)`), saved
to PNG and `Read`. Layout/text read fine in computer-use; colour does not.
Alternative: the embedded Slint MCP server — run the binary with
`SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=N` to inspect the UI tree / click /
type. The debug binary's `eprintln!` startup log (hotkey registration, bar
pin coords, transparency) is the cheapest smoke signal — launch, capture
stderr ~5s, kill, read it. See memory `[[overlay-host-visual-verification]]`.

### Release protocol (adopted 2026-06-13 — after the v0.17.1→v0.18.0 run)

**Why:** that run shipped 3 releases back-to-back, each with a "green gate",
and the USER caught every UI defect (crooked icon, emoji-not-SVG, UTC-not-MSK,
a stuck "Готово" status). Root cause: a green gate (clippy/test/fmt) means
"compiles + doesn't crash", NOT "UI verified". Layer 5 was fake for UI — it
booted + CopyFromScreen'd a couple of windows but never opened Settings or
clicked real controls, so button states / texts / signs / status-logic reached
the user unverified. And releases were published immediately after the
self-gate, with no human visual acceptance. See memory `[[release-protocol]]`.

**The rules (mandatory — chosen by the user):**
1. **NEVER auto-publish a GitHub release.** Build + self-gate → SHOW the user
   → wait for an explicit "релизь" → only then `gh release`. Release ≠ push;
   even a master push defaults to "only with a built, verified build".
2. **Accumulate** changes into one verified release — release is an event, not
   a per-task default. (hardening of `[[no-marathon-releases]]`.)
3. **Every UI diff passes THREE checks before the user is shown:**
   - **(a) screenshots + UI checklist** — `CopyFromScreen` the key windows in
     the RELEVANT states (Settings/bar/tile) + a written checklist: every
     string in `@tr` AND in the `.po`; no emoji where an SVG belongs; button
     states (enabled/disabled/active-marker) are logical; **status text matches
     real state** (no "Готово" when not done); signs/punctuation/spacing.
   - **(b) UI-review agent** on the `.slint` + wiring diff (the category that
     slips through static gates).
   - **(c) Slint-MCP** — `SLINT_EMIT_DEBUG_INFO=1 SLINT_MCP_PORT=N` binds
     `http://127.0.0.1:N/mcp` (VERIFIED working in the release build
     2026-06-13). Drive/read the UI tree programmatically — reliable, unlike
     computer-use clicks on the floating gear.
4. Present to the user as EVIDENCE ("here are the screenshots + checklist
   results, look at X"), never "all green, releasing".

### UI-audit toolkit (obkatano 2026-06-13 — these caught real bugs)

The "illogical UI" class is invisible to clippy/test. Run these on any UI diff:

- **i18n drift guard** — `cargo test --manifest-path slint-experiment\Cargo.toml
  --test i18n_guard`. Fails if a `@tr("English…")` has no `msgid` in
  `ru.po` (RU user → English fallback). It caught 3 strings whose `.po` msgids
  still had old emoji prefixes (`🎤 Dictate`) after the .slint was
  de-emojified — a silent drift. **This test is now part of the gate.**
- **Slint-MCP live inspection** — build, then launch with
  `SLINT_MCP_PORT=9123` (+ optional `SLINT_EMIT_DEBUG_INFO=1`). Stateless
  JSON-RPC at `http://127.0.0.1:9123/mcp`. Recipe:
  `curl -s -X POST .../mcp -H 'Content-Type: application/json'
  -H 'Accept: application/json, text/event-stream' -d '{jsonrpc,id,method,params}'`.
  `initialize` → `tools/call list_windows` → `get_element_tree {elementHandle}`
  → `query_element_descendants {queryStack:[{matchDescendants:true},{matchElementTypeNameOrBase:"Button"}]}`
  → `click_element` (open Settings via the gear) → `get_element_properties`
  (read REAL button text/enabled in a given state) → `take_screenshot`.
  Reliable where computer-use clicks on the floating gear are not. win0 is a
  parked 1×1; the bar is the 1200×60 window; Settings/tiles are separate windows
  that appear after a click/hotkey. **Gotcha (verified 2026-06-13):**
  `take_screenshot {windowHandle}` WORKS and returns TRUE colours (better than
  computer-use which mis-renders the transparent overlay) — **the primary MCP
  tool; `Read` the saved PNG to eyeball a window.** But
  `query_element_descendants` / `get_element_tree` return **0 children for EVERY
  window** in this build (accessibility tree not populated) — do NOT rely on
  programmatic text/state reads; verify VISUALLY via screenshot. **Open the
  target window by its GLOBAL HOTKEY** (F1 help / F4 palette / F7 archive —
  `keybd_event` fires regardless of foreground), then `list_windows` (new window
  = next index), then `take_screenshot`. The gear (Settings) has no hotkey and
  is a floating synth-click target — unreliable; prefer hotkey-reachable windows
  or let the user open Settings. Param names: screenshot=`windowHandle`,
  tree=`elementHandle`, descendants=`findAll` (no `maxElements`). Drive curl from
  a Python helper written to an ABSOLUTE path (Git-bash resets cwd between calls,
  so inline heredocs lose `/tmp` files).
- **The recurring UI bug shapes** (check the .rs side, not just .slint):
  1. **Stale status on a REUSED window** — the Settings window is reused, so
     every transient `*_status`/`*_result` string survives the next open unless
     `populate_token_status` clears it. (Caused the user's lingering
     "Готово: умная модель (12B)".)
  2. **Optimistic state-flip before an async result** — writing config + UI to
     the new value *before* the operation confirms; on failure the UI lies.
     Commit only on the confirmed-success branch.
  3. **A `.slint` default property with NO Rust setter** — renders fake data
     forever (palette `recent-chips: ["kubernetes",…]` had no `set_recent_chips`
     → always shown). Grep for `set_<prop>`; if absent, the default IS the
     production value.
  4. **emoji where the chrome standard is SVG** / **@tr↔.po drift after a string edit**.

### Lessons learned (the "we got burned" list)

1. **Don't skip a layer.** Every skip during the marathon reached the user.
2. **Don't run "fix waves"** when something's broken. Roll back to the last
   known-good state FIRST, then fix with the full layer cake.
3. **Static checks are necessary, not sufficient.** clippy + cargo test can
   all pass while the overlay renders wrong. Treat them as a sanity gate.
4. **The user has 1 portrait secondary** (1200×1920 at x=-1200) + 1 landscape
   primary (1920×1080). Any default that depends on monitor orientation needs
   both orientations live-tested. The bar pins to the PRIMARY at startup
   (`apply_overlay_hwnd`) for exactly this reason; tiles use
   `win32::pick_monitor` (primary unless a non-primary is landscape AND ≥
   primary width).
5. **Transparency is paint-sensitive** on Windows DWM — tile/bar backgrounds
   stay opaque-ish, never fully transparent, to avoid "created but invisible".
6. **No marathons.** Fewer, better-verified releases. See `[[no-marathon-releases]]`.

## i18n (RU + EN)

Strings live in the `.slint` source as **English `@tr("…")`** — the source
string IS the English msgid. The Russian translation is in
`slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po` (plain
`msgid`/`msgstr`, no `msgctxt`). `slint::select_bundled_translation("en"|"ru")`
switches live; `ui_language` in `%APPDATA%\suflyor\config.json` persists
it (en falls back to the msgid = English).

Adding a user-facing string: wrap it in `@tr("English…")`, append the
`msgid`/`msgstr` pair to `slint-replay.po`, rebuild. A **hardcoded Cyrillic
literal (no `@tr()`) won't translate** — that's a bug. Tiles/palette/settings
are separate Slint windows in the same process; they get their text from
`overlay_host.rs` at construction, so there's no per-window config fetch.

## Knowledge base

Embedded reference in `overlay-backend/src/kb.rs` (~1600 glossary / commands /
patterns entries, pre-lowercased). Accessed directly via `kb::search` /
`kb::get` (no IPC layer). The overlay's **F4** palette is the inline search
surface. Hyphenated keys (`kubectl-debug`) match via token-set check.
`kb::search` clamps the query to 200 chars (DoS guard).

## Voice coach (live + retrospective)

- **Live pill** in the overlay bar: WPM + filler density over a rolling 60s
  mic-only window.
- **Post-meeting debrief**: opt-in. On `stop_session`, the mic transcript + a
  3-point ask → a tile labeled "🎯 Debrief". Skip conditions: <30s session,
  <5 mic lines, empty AI bearer.

## Hotkeys (global — `src/bin/overlay_host/hotkeys.rs` is the source of truth)

| Key | Action |
|---|---|
| **F1** | Help window |
| **F3** | Re-ask the last question |
| **F4** | KB palette (inline `kb::search`) |
| **F6** | Manual "+ tile" (free-form AI tile) |
| **F7** | Session archive |
| **F9** / **Shift+F9** | Ask (main) / ask variant |
| **F8** / **Shift+F8** / **Ctrl+F8** | Vision capture: full-monitor / drag-region / variant |
| **Shift+Alt+1 / +2 / +3** | Read-aloud: read selection / OCR-region / pause (see below) |

Each registration logs `"<label> hotkey registered"` at boot (the cheapest smoke
signal). Dropping the `GlobalHotKeyManager` unregisters everything — `main` keeps
it alive for the process lifetime.

## Read-aloud (TTS + OCR) — since v0.20.0

On-screen / selected text → speech. Neural TTS (Piper Irina/Ruslan via
sherpa-onnx) runs in the **`suflyor-tts.exe` SIDECAR**, NOT in-process (see Stack
— two onnxruntimes crash in one binary). Tesseract OCR (a separate engine) reads
a screen region. Both engines install via buttons in **Settings → AI**
(SHA-pinned downloads). Hotkeys: **Shift+Alt+1** read selection, **+2**
OCR-region, **+3** pause; built-in anti-feedback so it never reads its own output.
Backend: `tts.rs` (SAPI live fallback) + `tts_install.rs` + `ocr.rs` +
`ocr_install.rs`. Full state in `docs/read-aloud-status.md`.

## Security boundaries

- **Single process, no IPC command surface.** Unlike the old Tauri build,
  there are no "commands" a tile window can `invoke`. Tile / palette /
  settings are Slint windows constructed by `overlay_host.rs`; they render
  only what they're handed and never read `config.json` themselves. So the
  old `assert_overlay` caller-guard is moot — secrets simply never reach a
  tile's scope.
- **AI endpoint:** resolve via `cfg.ai_endpoint(false)` (picks local vs cloud
  by `ai_provider`); the raw `ai_base_url` field is ALWAYS the cloud bridge.
- **AI error tiles** must use a GENERIC message (no error chain) so the
  `base_url` / LAN IP can't leak into a screenshot.
- **Stealth** (hide from screen capture) = Win32 `SetWindowDisplayAffinity`
  (`WDA_EXCLUDEFROMCAPTURE`), applied to the bar + tiles + the F4 palette +
  Settings when stealth is on.

## Security reminders

- `config.json` at `%APPDATA%\suflyor\config.json` contains live
  `groq_api_key` + `ai_bearer`. NEVER print these to chat or logs, and never
  include them in journal entries.
- `nini-context-backup.txt` (repo root) is the user's personal interview-prep
  notes — gitignored; never commit it.

## Reference

- **Methodology source:** memory `[[vpnctl-methodology]]`.
- **Project state:** memory `[[project-overlay-mvp-history]]`,
  `docs/state-and-plan.md`.
- **Visual verification:** memory `[[overlay-host-visual-verification]]`.
- **User setup:** memory `[[user-setup-monitors]]`.
