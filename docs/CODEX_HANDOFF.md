# Codex handoff — v0.36.0 released

Updated 2026-08-04. Read `AGENTS.md` before acting.

Operational note: all Winbrat jobs and connection incidents follow
[`docs/winbrat-recovery.md`](winbrat-recovery.md). Read it before remote work;
SSH loss never authorizes a duplicate build or an unrequested screen-control
fallback.

## Current state

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-post-release-0360`
- Branch: `codex/post-release-0.36.0`
- Base: `origin/master` at `6e2fe92`.
- Version `v0.36.0` is published at
  <https://github.com/PavelLizunov/suflyor/releases/tag/v0.36.0>.
- The release tag points to merge commit `bb9c42a`; `master` additionally
  contains the post-release README sync in `6e2fe92`.
- Never push directly to `master`.

## Included since v0.35.3

- Excluded auxiliary windows from the Windows taskbar before their first
  visible frame and hardened the shared Win32 tool-window fallback.
- Preserved explicitly approved memory verbatim, including newlines and table
  rows, and removed automatic background rewrites.
- Added an explicit Restore original action for eligible legacy memory rows;
  manual edits remain protected from restore.
- Made deterministic tile notices, statuses, errors, and chrome follow the
  selected interface language.
- Polished Archive alignment, row actions, and scrollbar spacing.
- Replaced stale or Russian showcase images with current privacy-safe English
  captures that identify the active local models.
- Extended the visual regression methodology and stabilized CI resource use.

## Verification state

- The merged feature PRs passed their full repository gates, pre-push hooks,
  Qwen reviews, and focused Slint MCP audits.
- Post-merge smoke on `7fe425c` inspected Help, Knowledge palette, Archive,
  Settings > Memory, and ten auxiliary tiles; no Suflyor taskbar target was
  exposed and the Memory surface remained unchanged while idle.
- Exact v0.36.0 full gate ended with `All gating layers green`.
- Exact release binaries and NSIS installer built successfully. The installer
  is 23,522,696 bytes with SHA-256
  `2E6A161DD86D19F4BFCD3694EAC42182D4AC041596D986255A2DA2E779AB16C8`.
- Silent installation succeeded. The installed executable reports file and
  product version 0.36.0 and has SHA-256
  `368E2EAFD58995C78F23B799BE3AADA626124FCD1F122362CC12B5A23C60AA99`.
- Exact `ui-mcp` audit inspected Settings > Updates at 720x600 in English and
  Russian, then restored English. F1, F4, and F7 dispatched distinctly; the
  other ten unchanged shortcuts were not repeated on the version-only branch.
- The installed normal binary is running without an MCP listener. Its bar and
  Settings windows have `TOOLWINDOW=true` and `APPWINDOW=false`.
- Installed bar geometry stayed 1200x64 at `(360, 24)` after five seconds; the
  two-step Quit action stopped host and TTS in 1,145 ms before the final
  normal-binary relaunch.
- Qwen `qwen3.8-max-preview` release-scope audit: v0.36.0 justified, confidence
  0.90. Final release-file review: `APPROVE`, no blockers, confidence 0.92.

## Remaining action

No v0.36.0 release work remains. Start the next product change from current
`origin/master` in a new `codex/<task>` branch and worktree. Do not retag or
replace the published release assets.
