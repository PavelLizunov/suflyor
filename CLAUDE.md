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

## Methodology — six-layer testing (adopted from vpnctl, 2026-05-26)

**Why this exists:** v0.0.67 → v0.1.2 attempt was a 33-release marathon
where layers 1-2 (clippy + cargo test) passed every release but the user
caught regressions live in WebView2 layout, focus races, multi-monitor
geometry, and i18n drift. User cut 64 of 68 GitHub releases by hand
(`«удалить большую часть релизов, зачем они там?»`) and asked for the
methodology from their `vpnctl` Rust project where there were no large
bugs. This section is that methodology, adapted to Tauri/WebView2.

### The six layers (UI), three (logic-only)

Each layer catches a strict subset the others miss. **Do not skip.**
Every skip in the past produced a user-facing regression.

| # | Layer | Tool | Catches | Misses |
|---|---|---|---|---|
| 1 | clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | API misuse, dead code, `unwrap`/`expect`/`panic` outside `#[cfg(test)]` | CSS, HTML, runtime behaviour |
| 2 | cargo test | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | Rust unit + integration (260+ tests) | TS code, WebView paint, layout |
| 3 | Copy-contract | `cargo test --test copy_contract` (and `npx tsc --noEmit` for TS) | Drift in canonical i18n strings, error message formats | Style of NEW text — additive: pin it the same commit it lands |
| 4 | review-agent | `Agent(subagent_type:general-purpose, prompt=docs/REVIEW_AGENT_PROMPT.md)` BEFORE commit | Logic bugs, security, library misuse, codebase duplicates | Whether the WebView paints correctly |
| 5 | Live install + smoke | `scripts/ci.ps1 && scripts/visual_check.ps1` | Runtime crashes, infinite-grow loops, missing assets in bundle | Cross-display-config edge cases (need user's monitor) |
| 6 | Visual check | `scripts/visual_check.ps1` saves PNG → Claude reads it | Floating panels covering chrome, tile chrome overflow, font fallback, transparency paint glitches | Quirks specific to user's display setup |

Logic-only changes (e.g. detector regex, kb parser) need 1, 2, 4, 5.
Anything that touches Tauri windows, React, or CSS needs all six.

### Blocking workflow before every commit

```
1. review-agent      (independent — paste full diff + invariants, do NOT
                      reference "the discussion above")
2. npm run ci        = scripts/ci.ps1 (fmt-check + clippy + tests + tsc)
3. scripts/visual_check.ps1 → Read the PNG it saves
4. git commit / push (auto-gated by .claude/hooks/git-gate.ps1 — BLOCKS
                      if fmt/clippy/test/tsc fail; --no-verify bypasses)
```

The git-gate.ps1 PreToolUse hook is the ONLY piece that genuinely
BLOCKS bad commits — without it, the methodology relies on discipline
(remembering to run `npm run ci`). Setup:
- `.claude/settings.json` registers the hook against `Bash` matcher
- `.claude/hooks/git-gate.ps1` runs `cargo fmt --check + clippy` on
  every `git commit`, plus `cargo test --lib + --test copy_contract +
  npx tsc --noEmit` on every `git push`
- `--no-verify` in the git command bypasses with a WARN line (rare;
  hotfix only per the short-circuit below)
- Required toolchain component: `rustup component add rustfmt`
- After editing the hook script OR settings.json: RESTART Claude Code
  (settings watcher does not pick up changes mid-session)
- Pipe-test before trusting:
  ```pwsh
  '{"tool_input":{"command":"git commit -m x"}}' | powershell -NoProfile -ExecutionPolicy Bypass -File .claude\hooks\git-gate.ps1
  ```

**Hotfix-only short-circuit** (review-agent skippable ONLY if ALL THREE):
- impl ≤ 5 lines
- touches exactly ONE surface (e.g. only a CSS rule, or only a single
  Tauri command body)
- changes no string pinned by `tests/copy_contract.rs`

### Concrete test patterns

- **`src-tauri/tests/copy_contract.rs`** — pins canonical Russian + English
  UI strings (`settings.save`, error format `tile spawn failed: {reason}`,
  the F4 palette placeholder, etc.). Any drift fails the test in the same
  commit. Adding a new visible string = add it to the contract test in
  the same commit that adds the string. See [Copy contract](#copy-contract)
  below for full coverage list.
- **`src-tauri/tests/spec_*.rs`** — independent contract tests for new
  public Rust functions. Written by `test-writer-agent` (Agent tool)
  BEFORE the agent sees impl. Per-file feature scope. `#![allow(clippy::unwrap_used)]`
  on module level. Each test = own tempdir / fresh state. Tests that
  fail = either impl wrong or spec ambiguous — DO NOT weaken the test.
- **Existing 260 `cargo test --lib` suite** — keep green at all times.
  WAV roundtrip, decimator signal, detector regex, tile slot math,
  config schema migration. Many of these are the most valuable bug-catchers
  the project has.
- **"Inverted impl" smell** — if `fn foo() -> Vec<T>` returned `Vec::new()`
  and a test still passed, that test is useless. Every test must check
  observable behaviour against the spec, not "didn't panic".

### Operational rules

- **`cargo fmt --all`** (NOT `--check`) BEFORE running tests. If it
  changes anything, include in the same commit. fmt drift is the most
  common CI killer.
- **Build = `npm run tauri build`** (NOT `cargo build`). `cargo build`
  bypasses the vite frontend bundle and ships a dev URL in release.
  See the v0.0.95 P0 ROOT CAUSE incident.
- **After Rust backend change** → mandatory NSIS install + visual check.
  Don't trust "cargo test green = production sees new code". Verify
  the installed binary timestamp updated and the new code path runs.
- **Sub-agent isolation** — review-agent and test-writer-agent see only
  what's in the prompt. Brief like a new colleague. Paste the full diff
  and the architectural invariants. Don't reference earlier conversation.

### Visual check (layer 6) for overlay-mvp

`scripts/visual_check.ps1 [-Open]`:
1. Kill any running overlay-mvp.
2. (`-Open` only) Run the NSIS installer silently and confirm timestamp.
3. Launch overlay via `Start-Process "$env:LOCALAPPDATA\suflyor\overlay-mvp.exe"`.
4. Wait 2 s for WebView2 paint.
5. Use `Add-Type` + System.Windows.Forms to grab a screenshot of the
   primary display and save to `target/visual/overlay-{ts}.png`.
6. (in future Claude session) `Read` that PNG to verify bar geometry,
   chip overflow, content rendering.

The script does the boring infra — the actual eyeball gate is Claude
reading the PNG via the `Read` tool. Without that final read, layer 6
is just bookkeeping. See `scripts/visual_check.ps1`.

### Tier 2 status — strict TS + ESLint adopted 2026-05-27

**TypeScript:** all flags from the suflyor spec ARE enabled
(`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`,
`noImplicitOverride`, `noImplicitReturns`, `noPropertyAccessFromIndexSignature`,
`useUnknownInCatchVariables`, `allowUnreachableCode: false`,
`allowUnusedLabels: false`). `tsc --noEmit` is GREEN.

**ESLint:** `eslint.config.js` extends `tseslint.configs.strictTypeChecked`
+ `stylisticTypeChecked` + `react-hooks`. Spec-required rules ON:
`no-explicit-any`, `no-non-null-assertion`, `consistent-type-assertions:
never`, `switch-exhaustiveness-check`, `react-hooks/exhaustive-deps`,
plus `no-restricted-syntax` for `Date.now()` / `Math.random()` (use
`src/clock.ts` instead).

**Pragmatic downgrades** (documented in `eslint.config.js`):
`restrict-template-expressions`, `no-unnecessary-condition`,
`no-empty-function`, `prefer-nullish-coalescing`, `no-invalid-void-type`
— off because the overlay marathon left a long tail of idiom-mismatch
that aren't safety issues. Pin these in the methodology backlog.

**Residual error count:** 34. These are real (floating promises, type
assertions, plus-coercion bugs). Backlog — fix as the surrounding code
is touched. NOT YET wired into `git-gate.ps1` because gating on 34
existing errors would block all commits. Re-evaluate Tier 2.5: once
the residual drops to 0, add `npx eslint src` to git-gate's push gate
alongside `tsc --noEmit`.

Run `npm run lint` to see current state. `npm run lint:fix` for the
~117 auto-fixable subset.

### Lessons learned (the "we got burned" list)

1. **Don't skip a layer.** Every time I did during the marathon,
   regressions reached the user. v0.0.85 reload+translate dead for 7
   releases because layer 4 was skipped. v0.1.2 palette restore race
   because layer 6 was skipped.
2. **Don't run "fix waves"** when something's broken. Roll back to the
   last known-good state FIRST, then fix with the full layer cake. The
   marathon-block-5 hotfix waves compounded regressions because every
   "fix" landed against an already-broken base.
3. **Static checks are necessary, not sufficient.** clippy + cargo test +
   tsc can all pass while the WebView shows a blank window. Treat them
   as a sanity gate, not a release gate.
4. **The user has 1 portrait secondary** (1200×1920 at x=-1200) + 1
   landscape primary (1920×1080). Any default that depends on monitor
   orientation needs both orientations live-tested. See
   `tile.rs::pick_monitor` history.
5. **WebView2 transparency + `always_on_top` is paint-flaky** on Windows
   DWM. Always opaque-ish backgrounds (`rgba(20, 22, 30, 0.92)` not
   fully transparent) to avoid "tile created but invisible" bugs.
6. **No marathons.** User explicitly asked never to run another 29-release
   sprint. Fewer, better-verified releases. See memory
   `[[no-marathon-releases]]`.

### Reference

- **Methodology source:** memory `[[vpnctl-methodology]]` (full vpnctl
  6-layer doc with example agent prompts).
- **Project state:** memory `[[project-overlay-mvp-history]]`.
- **User setup:** memory `[[user-setup-monitors]]`.

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
