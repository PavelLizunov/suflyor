# Suflyor — DSH agent instructions

Suflyor is a native meeting/interview assistant built with Rust and Slint. The
production code supports Windows and an active macOS port; Linux is not a
product target. This repository has five standalone crates and no root Cargo
workspace.

This file is the repository-wide contract for agents working through DeepSeek
Harness (DSH). System, developer, direct user, and global DSH instructions
remain higher priority. Within repository instructions, more specific
`AGENTS.md` files apply inside their directories and the nearest file wins.

## DSH is the control plane

- Development is orchestrated from DSH on `harness-test`. Use DSH tools for
  repository inspection, editing, planning, delegation, and evidence capture.
- Do not turn `harness-test` into a Windows/macOS builder and do not install
  platform SDKs there. Native compilation, tests, installers, and live UI QA run
  on homelab workers.
- Before any homelab, remote-worker, Tailscale, VM, build, or recovery action,
  load and follow the global `homelab` skill. The canonical infrastructure docs
  live outside this repository in `/var/lib/dsh/Project/homelab-infra-docs/`.
- Use trusted SSH aliases, not embedded IP addresses:
  - `windows-worker` / `windows-brat`: Windows Rust builds, native gate,
    installer, Windows runtime and Slint MCP QA.
  - `mac-worker` / `mac-mini`: macOS Rust/Swift builds, package checks, and
    physical macOS QA.
  - `linux-worker` / `debian-xfce`: Git/Python-only support tasks. It currently
    has no Rust toolchain; do not provision one as part of an unrelated task.
- Remote builds use an immutable candidate commit: commit on the task branch,
  make the worker fetch and check out that exact SHA in its own clean checkout,
  then record the SHA, command, exit code, and artifact/evidence path. Never
  build from a shared mutable source directory or claim that a different SHA
  verifies the current change.
- Before Windows work, read `docs/winbrat-recovery.md`. A lost SSH session is not
  proof that a job failed: inspect its task, log, and exit marker before retrying.
- Read-only documentation checks such as `git diff --cached --check` may run on
  the DSH control plane. Cargo, native packaging, and live application checks do
  not.

## Start every task here

1. Run `git status --short --branch` and inspect recent `git log`; do not trust a
   handoff document over the actual checkout.
2. Read this file and every nested `AGENTS.md` governing the files to change.
3. Read a referenced `docs/goal-*.md` charter when one exists. Treat
   `docs/CODEX_HANDOFF.md` as an in-flight handoff for the branch/worktree it
   names, not as a universal description of `master`.
4. Keep scope narrow. Do not overwrite, stage, revert, or clean unrelated work.
5. For a capability, dependency, integration, reusable utility, or architecture,
   use the global `search-first` discipline. For non-trivial implementation,
   use the global `sdd` workflow and wait for approval of its Micro-Spec.
6. Use `gemini-swarm` for substantial independent mechanical assignments; the
   lead agent owns decomposition, integration, architectural decisions, and
   final acceptance.

## Project map

- `slint-experiment/`: `overlay-host`, Slint UI, host orchestration, native
  Windows/macOS adapters, translations, assets, and integration guards.
- `overlay-backend/`: UI-free domain crate: AI, audio, STT, config, journal,
  SQLite persistence, KB/RAG, personal memory, installers, and sidecar control.
- `suflyor-tts/`: Piper read-aloud and diarization sidecar. Links sherpa-onnx
  only and remains a separate process.
- `suflyor-teratts/`: experimental TeraTTSv2 sidecar. Links ONNX Runtime through
  `ort` only; model assets download on demand and are not bundled.
- `suflyor-wsola/`: pitch-preserving time-stretch helper used by playback.
- `scripts/`: gate, build, installer, release, maintenance, and QA tooling.
- `.github/`: CI, security, packaging, and release-document automation.
- `.agents/skills/`: project-owned DSH procedures. Load a matching skill rather
  than copying its procedure into task prompts.
- `docs/`: current architecture and operational docs plus historical plans and
  evidence. Read `docs/AGENTS.md` before changing them.
- `experiments/`: non-production feasibility spikes; never import them into a
  production crate.

The thin binary entrypoint is `slint-experiment/src/bin/overlay_host.rs`. The
canonical shared Windows/macOS runtime root is
`slint-experiment/src/bin/overlay_host_windows.rs`; host subsystems live in the
`slint-experiment/src/bin/overlay_host/` directory.

## Sources of truth

For factual claims about the implementation, use the narrowest executable
source before prose; code does not override higher-priority instructions:

1. Current code, manifests, migrations, tests, and gate scripts.
2. This root file and the nearest nested `AGENTS.md`.
3. Active task charter and exact-SHA verification evidence.
4. Architecture/status documents explicitly marked current.
5. Historical plans, audits, and release evidence only for provenance.

`CLAUDE.md` is legacy tool-specific context. It may contain useful history, but
it is not the DSH operating contract and must not override current code,
`AGENTS.md`, or direct instructions. If a lasting rule changes, update the
relevant `AGENTS.md`; do not maintain two competing copies.

## Verification routing

The diff classifier is `scripts/git-gate-native.ps1`. Run it on
`windows-worker` against the exact candidate SHA for Windows-targeted work.

- **Docs:** Markdown/HTML/text-only changes. Before commit run
  `git diff --cached --check`; after commit use `git show --check <SHA>`. No
  Cargo build. The compiled KB inputs `overlay-backend/knowledge/glossary.md`,
  `commands.md`, and `patterns.md` require backend checks; a local
  `knowledge/AGENTS.md` edit remains documentation. GitHub's separate fast-path
  classifier also treats paths under `docs/` as docs-only.
- **Targeted:** one crate or one isolated UI surface. Run fmt, Clippy, and tests
  for the affected crate on the appropriate native worker.
- **Full:** multiple crates, dependencies/lockfiles, build/installer/CI/gate
  infrastructure, audio capture/routing, persistence/recovery, credentials,
  security, networking/update, or similarly cross-cutting runtime work. Run
  `scripts/ci.ps1` on `windows-worker`; run the macOS gate on `mac-worker` when
  the changed seam is compiled or exercised there.
- Set `CARGO_INCREMENTAL=0`; gate scripts already do this. Inspect free space
  before heavy builds and follow homelab cache-hygiene rules. Do not clean
  caches or targets merely to be tidy.
- **Hard mac-worker memory rule (16 GiB):** before any Cargo/Swift build or gate,
  verify that no other Cargo, Rust, Swift, Suflyor host, or local-model process is
  active and that `memory_pressure` reports at least 40% system-wide free memory.
  Always use `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`; never raise either
  limit on this worker. The macOS gate enforces the same preflight. If macOS
  shows a memory-pressure/Force Quit warning, stop the owned build immediately,
  verify its processes exited, and report the interrupted gate; never close
  unrelated user applications to make room.
- GitHub Actions is independent supporting evidence, not a substitute for a
  required physical-worker or live UI check.

### Visible UI changes

For any `.slint` edit or Rust change that affects visible UI, load and follow
`slint-mcp-ui-audit`; it is the procedural source of truth. Non-negotiable
acceptance evidence is the exact candidate SHA, matching before/after captures,
and functional hotkey dispatch where required. A green compile or registration
log is not visual or functional acceptance.

## Repository-wide invariants

- Production Rust denies `clippy::unwrap_used`, `expect_used`, and `panic`.
  Test modules and integration tests may add the documented local allow.
- User-facing `.slint` strings are English `@tr("...")` source strings with an
  exact Russian `msgid`/`msgstr` pair in
  `slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po`.
- Avoid rare Unicode/emoji in UI text because Skia may render tofu. Use ASCII or
  the SVG icon set. New stroke icons use a 16x16 viewBox and stroke width 1.6.
- The Settings window is reused. Every transient `*-status`/`*-result` property
  must be reset by `populate_token_status`; never show optimistic success before
  an asynchronous operation confirms it.
- `suflyor-tts` and `suflyor-teratts` remain separate processes. Never combine
  their ONNX runtimes with each other or with in-process STT.
- Version metadata lives in both `slint-experiment/Cargo.toml` and
  `scripts/slint-installer.nsi` (`PRODUCT_VERSION`). Keep them synchronized.
- Committed lockfiles: `slint-experiment/Cargo.lock` and
  `suflyor-tts/Cargo.lock`. `overlay-backend/Cargo.lock` is ignored.

## Security and privacy

- Never print or commit `%APPDATA%\suflyor\config.json`, credentials, tokens,
  private transcripts, personal prep notes, or `nini-context-backup.txt`.
- Keep screenshot-visible errors generic. Redact local paths, usernames, URLs,
  hostnames, and LAN addresses from logs, diagnostics, docs, and evidence.
- Do not bypass SSH host-key checking or weaken platform security/TCC controls.
- Do not touch `.claude/**` or `.codex/**` unless the task explicitly owns those
  files.

## Git and delivery

- Work on `codex/<short-task-name>`; one task, one branch, one coherent change.
  Never push directly to `master` and never merge your own PR unless explicitly
  instructed.
- Enable `.githooks` in native worker/developer clones that provide
  `powershell.exe`. Do not enable the Windows-only hook in the Linux DSH
  control-plane checkout.
- A DSH candidate commit may use `--no-verify` only because the selected native
  gate is deliberately moved to a worker testing that exact SHA (or for a
  docs-only commit after `git diff --cached --check`). Push only the task branch
  from DSH so the worker can fetch that SHA; do not open/merge a PR or release
  until its required evidence is green. If verification requires a code change,
  create and push a new candidate commit and rerun the affected evidence; never
  reuse evidence from the old SHA.
- Commit only task-owned, verified files. In the final summary state what
  changed, exact verification evidence, and what was not run.
- RC publication follows the project `source-command-release` skill. Stable
  tags/releases require explicit owner authorization. Publishing is not done
  until the skill's post-release cleanup preview and apply phases complete.
