# Agent task queue

Self-contained tasks for delegated agents (Codex etc.). Rules of engagement:
AGENTS.md (read it first). One task = one `codex/<name>` branch = one session.
Definition of done for EVERY task: the docs/targeted gate selected by
`scripts/git-gate-native.ps1` is green, plus the task's own acceptance bullets.
Full `scripts/ci.ps1` is reserved for publishing an owner-authorized stable
release, never normal development or a prerelease. Do NOT publish a stable
release, push a stable tag, or merge to master unless the task says so.

Status legend: [ ] open · [~] claimed (write your agent/branch) · [x] done.

---

## [~] T9 — preserve corrupt configuration with private permissions

**Claimed:** Codex, branch `codex/private-config-backup-20261117`; scheduled
bot maintenance authorizes delivery through merge.
**Problem:** a renamed malformed config retains its original POSIX mode. PR
#340 tightens it only after rename and tolerates a permission failure.
**Scope:** use the existing permission helper before rename; if restriction
fails, keep the original intact without creating a permissive backup. Preserve
exact bytes, timestamp naming, Windows behavior and default-load recovery.
Use a narrow injected restriction seam for deterministic failure coverage.
**Accept:** production-entrypoint tests cover fresh/existing 0644 and 0664
fixtures and verbatim backup bytes; a synthetic restriction failure leaves the
original bytes/name intact and creates no backup. Exact-SHA Windows targeted
gate, physical macOS backend tests/security checks, independent Codex review,
required PR checks and post-merge main checks pass. No real config, secret,
dependency, UI, production or release changes.
**Unknowns:** platform verification and worker admission remain to be checked.

## [x] T8 — restrict both new and existing TTS sidecar logs

**Delivered:** PR #343, master `6f4287b07c0da4bbbe14632d99464c86c11345df`.
Complete post-merge CI run 43840996679 passed, including macOS; verified on
2026-11-17. Source #335 was closed as superseded.
**Claimed:** Codex, branch `codex/private-tts-logs-20261116`; authorized by
scheduled bot maintenance on 2026-11-16.
**Problem:** `sidecar_stderr` opens both engine logs using ordinary append mode.
PR #335 fixes new-file creation but leaves pre-existing 0644/0664 logs readable.
**Scope:** use the existing private append helper and restrict the opened file
handle to 0600 on Unix before passing it to a child process. Keep log contents,
append semantics, Windows behavior and the existing null-stderr error fallback.
No UI, dependency, advisory-policy or unrelated bot-journal edits.
**Accept:** synthetic tests cover both log names, new files and existing wide
files; check permissions before appending and preserve previous bytes. Permission
errors must propagate before the handle can reach a sidecar. Exact-SHA native
Windows targeted gate and physical macOS backend tests pass, followed by
independent Codex review, required CI, expected-head merge and master checks.
**Unknowns:** worker availability and native tests determine platform behavior;
no live user log or configuration is inspected.

## [x] T7 — deliver bot dependency security remediation

**Delivered:** PR #341, master `25b9f15f4f45dfb2ce9e3d24dd669593e64de3dd`.
Post-merge CI run 43711534558 (including macOS) and security run 43711534664
completed successfully; verified on 2026-11-16.
**Claimed:** Codex, branch `codex/sqlite-security-20261115`; owner requested
completion through verified merge on 2026-11-15.
**Problem:** the application lockfile carries affected bundled SQLite and
Markdown releases, a yanked executor, and an archived pinning build dependency.
**Scope:** adopt rusqlite 0.41.1 / bundled SQLite 3.54.1, pulldown-cmark 0.13.6
and async-executor 1.14.1; regenerate the consumer lockfile. Preserve the
published spin_on 0.1.1 executor and its no_std API as a small vendored build
helper, replacing pin-utils::pin_mut! with core::pin::pin! and removing that
dependency. Preserve upstream licenses and record archive provenance.
**Invariants:** no security-policy weakening; no schema, UI, audio or executor
polling changes; preserve unrelated work. Never use old-SHA check evidence.
**Accept:** a !Unpin pending future retains its address through repeated polls;
existing helper tests pass; cargo-deny advisories/bans/sources/licenses pass
unchanged; exact-SHA native Windows/macOS targeted checks and independent Codex
review pass; required GitHub checks pass before expected-head merge, followed
by main-branch CI verification.
**CI follow-up:** Rust/Cargo 1.100 changed its internal build layout and breaks
ort-sys rc.12 DLL staging (D-096). Pin CI's toolchain action to the verified
native Windows Rust 1.97.1 release. Keep all tests/security gates and restore a
newer toolchain only with a compatible ORT/native-staging migration.
**Verification:** affected workflow shell guards, exact-SHA native gates,
independent review and required GitHub CI must pass before merge.

## [~] T5 — finish common mathematical notation rendering

**Claimed:** Claude Code, branch `claude/t5-math-notation`. Parser tests are in the
PR; the owner screenshot and the streaming check are still open.

**Priority:** backlog after the macOS port.
**Problem:** v0.37.0 renders the main matrix/fraction/root cases, but some
generated notation is still shown as raw TeX, including `\operatorname{...}`,
`\bar{...}`, `\overline{...}`, and related inline commands.
**Do:** extend the existing math normalizer/renderer without adding a second
rendering stack. Preserve stable streaming layout and ordinary prose/code.
**Accept:** focused parser tests cover operator names, accents, determinants,
and mixed prose; the full gate is green; an owner screenshot contains no raw
supported TeX commands and no streaming text jumps.

## [~] T6 — restore the custom tray context menu on owner Windows

**Claimed:** Claude Code, branch `claude/t6-tray-menu`. Event-routing coverage is
in the PR; owner verification on the affected installation is still open.

**Priority:** backlog after the macOS port.
**Problem:** left-click restore works in v0.37.0, but right-clicking the tray
icon can produce no menu on the owner's Windows installation.
**Do:** trace both legacy `WM_RBUTTONUP` and notification-icon v4
`WM_CONTEXTMENU` delivery, including Explorer/taskbar recreation. Keep the
project-styled menu above the taskbar; do not fall back to a native popup.
**Accept:** automated event-routing coverage plus owner verification that one
right-click opens exactly one styled menu in front of the taskbar.

---

## Completed Tasks

- [x] **T1 — TTS number normalization** (`a13f671f`): Implemented in `overlay-backend/src/tts_normalize.rs` (`normalize_for_speech`) and integrated into `overlay-backend/src/tts.rs` with 15+ unit tests.
- [x] **T2 — Diarization segment post-merge** (`c7c9b952`): Implemented in `suflyor-tts/src/diar.rs` (`postprocess` function with 4 unit tests covering gap merging, fragment attachment, fragment dropping, and sorting).
- [x] **T3 — Icon guard test + star.svg regridding** (`f5eb3fb6`): Redrawn `slint-experiment/assets/icons/star.svg` on 16x16 grid (stroke 1.6) and added `slint-experiment/tests/icon_guard.rs` static guard test.
- [x] **T4 — deps: global-hotkey 0.6 → 0.8** (`af224317`): Updated `global-hotkey` dependency to `0.8` in `slint-experiment/Cargo.toml` and fixed API drift in `src/bin/overlay_host/hotkeys.rs`.
