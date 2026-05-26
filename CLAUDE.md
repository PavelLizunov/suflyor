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
- `.claude/autonomous_active` — ISO 8601 deadline. Presence = mode armed.
  Do NOT delete this file from inside an autonomous run (that defeats
  the whole point).
- `.claude/_progress_counter` — internal, managed by hooks. Don't touch.

## Project conventions

- Frontend: React 19 + Vite + plain CSS in `src/styles.css` (no Tailwind).
  Run with `npm run dev` (vite only) or `npm run tauri dev` (full app).
- Backend: Rust + Tauri 2 in `src-tauri/`. Two binaries — `default-run`
  in `Cargo.toml` is `overlay-mvp`. Build via `npm run tauri build` (NOT
  `cargo build` — that bypasses the vite frontend bundle).
- Tests: `cargo test --lib` (255 tests, <1s) — the `--bin overlay-mvp`
  variant in older docs runs zero tests (the binary itself has none —
  all unit tests live in the library target). `cargo clippy --all-targets
  -- -D warnings` for strict lint covers lib + journal-eval CLI.
- Cargo path issue: `cargo` is at `~/.cargo/bin/cargo.exe`. Git Bash
  doesn't always pick it up — prepend
  `export PATH="/c/Users/x3d_mutant/.cargo/bin:$PATH"`.

## Release verification — MANDATORY (after v0.0.34 P0 incident)

See `RELEASE_CHECKLIST.md` for the canonical 6-gate methodology.
Established 2026-05-26 after v0.0.34 shipped an infinite-overlay-grow
bug that all static checks passed but no one launched the binary.

**Every release MUST pass all 6 gates before `git push`:**

1. **Static checks** (`cargo test --lib`, `cargo clippy --all-targets
   -- -D warnings`, `npx tsc --noEmit`)
2. **Build** (`npm run tauri build -- --bundles nsis`)
3. **Install** via the NSIS installer with `/S` silent flag and verify
   `LastWriteTime` of `%LOCALAPPDATA%\suflyor\overlay-mvp.exe` updated
4. **Smoke test via computer-use** — `mcp__computer-use__screenshot` after
   `open_application "suflyor"`. Bar must:
   - render at sane size (520-1000 px wide), correct position (top)
   - NOT extend past screen edges
   - have stable dimensions over a 5-second wait (catches feedback loops
     like v0.0.34's infinite-grow)
5. **Feature verification** of the changed surface — for layout changes,
   drag-resize the bar; for Settings changes, navigate panels + click
   save; for hotkey changes, trigger the hotkey
6. **Quit cleanly** — `⚙ → ✕ Выйти → «Выйти» confirm` then check
   `(Get-Process overlay-mvp -ErrorAction SilentlyContinue).Count == 0`

Static checks are necessary but NOT sufficient. They don't see runtime
layout feedback loops. See `POST_MORTEM_v0034.md` for the full incident
analysis.

## i18n (RU + EN, since v0.0.42)

Typed-strings module at `src/i18n.ts`. ~212 keys × 2 langs. Pattern:
```ts
import { t, resolveLang, type Lang } from "./i18n";
const lang: Lang = resolveLang(cfg?.ui_language); // "ru" default
<button title={t("settings.save", lang)}>{t("settings.save", lang)}</button>
```
`{placeholder}` interpolation via `.replace("{token}", value)` — no
helper. Adding a new string: append to the const map in `i18n.ts`
(TS will type-check usage at call sites). Adding a new component:
load `cfg.ui_language` from `get_config` on mount (overlay/settings/
replay all in the overlay window can do this). Tile windows can't
call `get_config` (gated by `assert_overlay`) — `tile.rs` bakes
`&lang=ru|en` into the spawn URL via `app.try_state::<SharedConfig>()`.

## Knowledge base

Embedded reference at `src-tauri/knowledge/{glossary,commands,patterns}.md`
(~1700 entries). Exposed via Tauri commands `kb_search`, `kb_get`,
`kb_stats`, `kb_spawn`. Settings UI has a search panel + F4 inline palette
in the overlay. Hyphenated keys (`kubectl-debug`) match correctly via
`kb_key_matches_trigger` token-set check.

## Voice coach (live + retrospective)

Two coaching surfaces:
- **Live pill** in overlay bar: WPM + filler density over rolling 60s
  mic-only window. Backend emits `speech:coach` event every 2s. Pace
  buckets: low/<150 · ok/150-180 · fast/>200 · idle.
- **Post-meeting debrief**: opt-in via Settings → 🎯 Coaching. On
  `stop_session`, mic transcript + 3-point Sonnet ask → tile labeled
  "🎯 Debrief: what to improve". Skip conditions: <30s session, <5 mic
  lines, empty AI bearer. ~$0.005 per session when enabled.

## Security boundaries

- **Caller-window guard** (`assert_overlay`): all sensitive Tauri commands
  reject calls from non-overlay (e.g. tile-*) windows. Applied to
  config/session/screenshot/mic/spawn_tile/expand_snippet/kb_spawn.
- **Capability split**: `capabilities/default.json` (overlay-only) +
  `capabilities/tile.json` (tile-* with narrow perms; no opener,
  no global-shortcut, no set-position/size).
- **KB query cap**: `kb::search` clamps query to 200 chars to prevent
  DoS via huge paste.

## Active background processes

If `tauri dev` is running, modifying Rust files triggers ~5-10s rebuild +
overlay relaunch. Modifying TS triggers vite HMR (no relaunch). Prefer
TS-only changes during interactive sessions; batch Rust changes.

## Security reminders

- `config.json` at `%APPDATA%\overlay-mvp\config.json` contains live
  `groq_api_key` + `ai_bearer`. NEVER print these to chat or logs.
- DevTools is debug-only (release build excludes the auto-open call).
  In dev (`tauri dev`), DevTools opens automatically — secrets are
  visible to anyone with physical access. Treat dev box accordingly.
