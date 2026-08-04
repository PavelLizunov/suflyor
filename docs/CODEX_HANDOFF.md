# Codex handoff — v0.36.0 preparation

Updated 2026-08-04. Read `AGENTS.md` before acting.

## Current state

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-release-0360`
- Branch: `codex/release-0.36.0`
- Base: `origin/master` at `7fe425c`
- Version: `0.36.0` in Cargo, Cargo.lock, and NSIS.
- Never push directly to `master`.
- Do not create a tag or publish the release until the owner has seen the
  verified v0.36.0 build evidence and explicitly authorizes this exact version.

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

1. Push this branch, open a PR against `master`, wait for fresh GitHub CI, and
   merge only through the PR.
2. Show the verified evidence to the owner and wait for explicit authorization
   of `v0.36.0` before creating the tag or GitHub Release.
3. After publication, review the automation PR that updates the README latest
   release marker to v0.36.0.
