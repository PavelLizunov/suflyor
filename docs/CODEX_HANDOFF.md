# Codex handoff — v0.35.2 release

Updated 2026-07-31. Read `AGENTS.md` before acting.

## Current state

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-qwen-0352-integration`
- Branch: `codex/owner-authorized-release`
- Base: `origin/master`
- Version: `0.35.2` in Cargo and NSIS.
- Do not merge or push `master`. Release or tag only if the owner explicitly
  authorizes this specific version after verified build evidence was shown.

## Included

- Restored explicit 4B / 12B / 26B model states and custom GGUF selection.
- Corrected the pinned 12B size (the old value was 1472 bytes short), with
  resumable downloads and a regression test.
- Added the pinned 26B `mmproj-F16.gguf` from the same immutable Hugging Face
  revision, exact size/SHA verification, on-demand install, safe model restart,
  persisted Vision state, and working F8 routing.
- Added Auto / 8K / 16K / 32K / 64K / 96K context control with cached memory
  estimates and no physical zero position.
- Fixed shared Settings scroll extent through `SettingsCard.preferred-height`;
  AI, Audio, Hermes, and the other Settings tabs no longer have artificial gaps.
- Fixed Vision setting persistence and the off-route F8 notice.
- Fixed `Shift+Alt+1`: wait for hotkey modifiers to release, then inject only
  Ctrl+C (no orphan Alt/Shift key-up events).
- Updated the project Slint MCP audit skill and v0.35.1 retest checklist.

## Verified

- Exact command passed on 2026-07-31:
  `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
- Final line: `All gating layers green.`
- Backend: 529 passed, 1 ignored. TTS: 9 passed.
- Slint MCP: Settings 720x600, all 16 tabs inspected; long AI, Audio, and
  Hermes pages reached their bottoms with no large gap or clipping.
- Live 26B Vision: projector SHA verified, 26B restarted, F8 region request
  completed successfully.
- Live `Shift+Alt+1`: copied the full 21-character disposable marker and
  restored the clipboard sentinel.

## Remaining action

Commit the release-policy and v0.35.2 retest files, push
`codex/owner-authorized-release`, and open a ready PR against `master`. After
that PR is merged, publish v0.35.2 with the verified installer: the owner
explicitly authorized this specific release after the evidence above was shown.
