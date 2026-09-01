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

## [ ] T5 — finish common mathematical notation rendering

**Priority:** backlog after the macOS port.
**Problem:** v0.37.0 renders the main matrix/fraction/root cases, but some
generated notation is still shown as raw TeX, including `\operatorname{...}`,
`\bar{...}`, `\overline{...}`, and related inline commands.
**Do:** extend the existing math normalizer/renderer without adding a second
rendering stack. Preserve stable streaming layout and ordinary prose/code.
**Accept:** focused parser tests cover operator names, accents, determinants,
and mixed prose; the full gate is green; an owner screenshot contains no raw
supported TeX commands and no streaming text jumps.

## [ ] T6 — restore the custom tray context menu on owner Windows

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
