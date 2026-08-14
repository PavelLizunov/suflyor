# suflyor (overlay-mvp) — agent instructions

Windows-only AI-interview overlay. **Pure Rust + Slint 1.17** (no Node, no web
engine). Read this file fully before editing; the checks below define "done".

## Project map (five standalone crates, NO root workspace)

- `slint-experiment/` — the `overlay-host` binary. UI in `ui/*.slint`
  (compiled in via build.rs). Host logic is the ~25-module DIRECTORY
  `src/bin/overlay_host/` (settings_*, tile_*, hotkeys, diagnostics, …) —
  grep the directory, not just `overlay_host.rs` (thin entrypoint).
- `overlay-backend/` — no-UI shared crate (ai, audio, bridge, config, memory,
  persistence, stt, tts, teratts_install, hermes_install, …). Most unit tests
  live here.
- `suflyor-tts/` — Piper read-aloud + diarization SIDECAR exe. Links
  sherpa-onnx ONLY and MUST stay a separate process (two onnxruntimes crash
  in one binary). Never merge it into overlay-backend. Diarization ALWAYS
  stays in this sidecar.
- `suflyor-teratts/` — experimental TeraTTSv2 read-aloud SIDECAR exe (RC17).
  Links ONNX Runtime through `ort` ONLY; same process-isolation rule as
  suflyor-tts. Never add a second ONNX Runtime to suflyor-tts instead. Its
  ~370 MB model is pinned in `suflyor-teratts/manifest/teratts-v2.json` and
  downloads on demand — NEVER bundle weights in NSIS; see
  `suflyor-teratts/NOTICE.md` for the upstream licensing release gate.
- `suflyor-wsola/` — tiny WSOLA time-stretch helper used by the transcript
  player.

Version lives in BOTH `slint-experiment/Cargo.toml` and
`scripts/slint-installer.nsi` (`PRODUCT_VERSION`) — keep in sync.
Docs/plans: `docs/goal-*.md` (task charters), `docs/retest-*.html` (tester
acceptance checklists), `docs/memory-architecture.md`. CLAUDE.md is the
Claude-Code twin of this file — same rules, different tooling notes.

## Build / test / lint (Windows; cargo at `~/.cargo/bin/cargo.exe`)

- Full gate (REQUIRED green before any commit is considered done):
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
  = fmt --check + clippy -D warnings + tests for all 5 crates + i18n_guard +
  the `ui-mcp` QA-feature compile check.
  Takes ~9 min. Run it yourself; do not declare success without it.
- Quick compile check: `cargo check --bin overlay-host --manifest-path
  slint-experiment/Cargo.toml`
- Single-crate tests: `cargo test --manifest-path overlay-backend/Cargo.toml`
- Always `set CARGO_INCREMENTAL=0` (disk-bloat policy; the gate scripts do).
- Before a release build, kill running instances:
  `taskkill /IM overlay-host.exe /F` + `taskkill /IM suflyor-tts.exe /F`.
- Release build + installer (rarely needed by agents):
  `powershell -File scripts/build-slint-release.ps1 -Installer`.

## Winbrat remote work and recovery

Before starting or resuming any Winbrat build, test, installer, or live UI
task, read [`docs/winbrat-recovery.md`](docs/winbrat-recovery.md). A lost SSH
connection does not mean a scheduled build failed: diagnose Tailscale and port
22 separately, inspect the recorded task/log/exit marker before restarting,
and recover the agent-managed Winbrat through SSH, WinRM, or its verified
console. Never mistake the owner's workstation for the test VM or run/build
Suflyor there as a fallback.

## Mandatory Slint MCP visual gate

- After any `.slint` edit or Rust change that affects visible UI, use the
  project skill `.agents/skills/slint-mcp-ui-audit/SKILL.md` before calling the
  task done, committing it, or handing a build to the user.
- Every visual fix must keep matching **before and after** screenshots of the
  same surface, size, theme, language, and UI state at a stable artifact path;
  link both from the PR. An after-only screenshot is not acceptance evidence.
- A green compile/test gate is not visual verification. The embedded Slint MCP
  server is compiled in **only** by the `ui-mcp` Cargo feature; setting the
  environment variables on a normal build does nothing. You **MUST** build the
  audited binary with `cargo build --locked --bin overlay-host --features
  ui-mcp --manifest-path slint-experiment/Cargo.toml`, then launch **that**
  binary with `SLINT_EMIT_DEBUG_INFO=1` and `SLINT_MCP_PORT=9123`, inspect
  live screenshots through the embedded Slint MCP server, and report the
  surfaces checked.
- A change to a shared Settings primitive or layout requires screenshots of all
  16 Settings tabs at 720x600. Never rely on computer-use screenshots for
  transparent-window colours.
- After the page pass, run the project skill's complete 13-shortcut global
  hotkey smoke once against the same binary. Registration logs alone do not
  prove dispatch; check the distinct result/log for every shortcut.

Git hooks: run `git config core.hooksPath .githooks` once after clone —
pre-commit runs fmt --check, pre-push runs clippy + tests (all crates).
Do NOT bypass with --no-verify.

## Hard rules

- **Publish a GitHub release or tag only after the owner explicitly
  authorizes one specific version after verified build evidence was shown.**
  Never publish on your own initiative. Direct pushes to `master` are
  forbidden; use a `codex/<task>` branch + PR.
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

## Resuming an in-flight session

**Start here:** `docs/CODEX_HANDOFF.md` — the live state (current branch, what
is done/committed, gate status) and the exact push runbook + guardrails.
`docs/state-and-plan.md` points to it. Read it before touching anything: this
checkout is SHARED with a Claude Code session, so `git status`/`git log` first.

## Agent task queue

See `docs/AGENT_TASKS.md` — self-contained tasks with acceptance criteria,
sized for one session each.
