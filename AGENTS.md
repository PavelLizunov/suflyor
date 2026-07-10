# suflyor (overlay-mvp) — agent instructions

Windows-only AI-interview overlay. **Pure Rust + Slint 1.16** (no Node, no web
engine). Read this file fully before editing; the checks below define "done".

## Project map (three standalone crates, NO root workspace)

- `slint-experiment/` — the `overlay-host` binary. UI in `ui/*.slint`
  (compiled in via build.rs). Host logic is the ~25-module DIRECTORY
  `src/bin/overlay_host/` (settings_*, tile_*, hotkeys, diagnostics, …) —
  grep the directory, not just `overlay_host.rs` (thin entrypoint).
- `overlay-backend/` — no-UI shared crate (ai, audio, bridge, config, memory,
  persistence, stt, tts, hermes_install, …). Most unit tests live here.
- `suflyor-tts/` — read-aloud + diarization SIDECAR exe. Links sherpa-onnx
  ONLY and MUST stay a separate process (two onnxruntimes crash in one
  binary). Never merge it into overlay-backend.

Version lives in BOTH `slint-experiment/Cargo.toml` and
`scripts/slint-installer.nsi` (`PRODUCT_VERSION`) — keep in sync.
Docs/plans: `docs/goal-*.md` (task charters), `docs/retest-*.html` (tester
acceptance checklists), `docs/memory-architecture.md`. CLAUDE.md is the
Claude-Code twin of this file — same rules, different tooling notes.

## Build / test / lint (Windows; cargo at `~/.cargo/bin/cargo.exe`)

- Full gate (REQUIRED green before any commit is considered done):
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
  = fmt --check + clippy -D warnings + tests for all 3 crates + i18n_guard.
  Takes ~9 min. Run it yourself; do not declare success without it.
- Quick compile check: `cargo check --bin overlay-host --manifest-path
  slint-experiment/Cargo.toml`
- Single-crate tests: `cargo test --manifest-path overlay-backend/Cargo.toml`
- Always `set CARGO_INCREMENTAL=0` (disk-bloat policy; the gate scripts do).
- Before a release build, kill running instances:
  `taskkill /IM overlay-host.exe /F` + `taskkill /IM suflyor-tts.exe /F`.
- Release build + installer (rarely needed by agents):
  `powershell -File scripts/build-slint-release.ps1 -Installer`.

Git hooks: run `git config core.hooksPath .githooks` once after clone —
pre-commit runs fmt --check, pre-push runs clippy + tests (all crates).
Do NOT bypass with --no-verify.

## Hard rules

- **Never publish a GitHub release, never `gh release`, never push tags.**
  Releases are owner-triggered only. Pushing to master is allowed only when
  the task explicitly says so; default to a `codex/<task>` branch + PR.
- **Work on a branch `codex/<short-task-name>`**, one task = one branch =
  one coherent deliverable. Claude Code sessions share this checkout —
  branches prevent the commit races we've already been burned by.
- Both Rust crates `deny` clippy `unwrap_used` / `expect_used` / `panic` in
  production code. In `#[cfg(test)]` modules add an inner
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`;
  integration tests in `tests/` need the same file-level allow.
- **i18n:** every user-facing string in `.slint` is English `@tr("…")` and
  MUST get a matching `msgid`/`msgstr` pair in
  `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po` (the
  i18n_guard test in the gate fails otherwise). Hardcoded Cyrillic in
  `.slint` is a bug. Russian status strings built in Rust code are fine.
- **No tofu glyphs in UI text:** the skia renderer draws rare Unicode
  (warning sign, checkmark, circled digits, emoji) as squares. Use ASCII
  ([!], [ok], "1)") or the SVG icon set.
- Icons: `slint-experiment/assets/icons/*.svg`, convention 16x16 viewBox,
  stroke-width 1.6. Match it for any new icon.
- The Settings window is REUSED: every transient `*-status`/`*-result`
  Slint property must be reset in `populate_token_status`
  (settings_controller.rs) — the settings_reset_guard test enforces this.
- Secrets: `%APPDATA%\suflyor\config.json` holds live API keys — never print
  its contents. Never commit `nini-context-backup.txt`. Error strings shown
  in tiles must be generic (no URL/LAN-IP leakage; see http_log.rs).
- Don't touch `.claude/**` (Claude Code local config) or `.codex/**` unless
  the task is about them.
- Cargo.lock: `slint-experiment/Cargo.lock` and `suflyor-tts/Cargo.lock` are
  committed; `overlay-backend/Cargo.lock` is gitignored.

## Task workflow expected from agents

1. Read the task's `docs/goal-*.md` charter if referenced; keep scope to it.
2. Implement with unit tests (backend logic must be testable without UI).
3. Run the full gate (`scripts/ci.ps1`) — all layers green.
4. Commit on your `codex/<task>` branch with a descriptive message; do not
   merge to master yourself unless the task says to.
5. State in your summary: what changed, gate result, what you did NOT do.
   UI changes additionally get validated visually by the owner/tester —
   note any surface you changed so they know where to look.

## Current agent task queue

See `docs/AGENT_TASKS.md` — self-contained tasks with acceptance criteria,
sized for one session each.
