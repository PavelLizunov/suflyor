# Codex handoff — v0.35.3 preparation

Updated 2026-08-02. Read `AGENTS.md` before acting.

## Current state

- Worktree: `C:\Users\x3d_mutant\Natively\suflyor-release-0353`
- Branch: `codex/release-0.35.3`
- Base: `origin/master` at `5bcf3c4`
- Version: `0.35.3` in Cargo, Cargo.lock, and NSIS.
- Never push directly to `master`.
- Do not create a tag or publish the release until the owner has seen the
  verified v0.35.3 build evidence and explicitly authorizes this exact version.

## Included since v0.35.2

- Fixed local-AI context preset selection on high-RAM systems.
- Kept TTS suppression aligned with playback and pause/resume.
- Added visible, debounced session cost-cap notices.
- Moved persistence work off the UI thread and flush session data on exit.
- Verified effective stealth state and surfaced capture-exclusion failures.
- Fixed diarization installation and auxiliary-window lifecycle.
- Added guided STT provider and cloud-model selection.
- Improved Help, Archive, empty-tile, Diagnostics, and audio-source clarity.
- Added a consistent drag grip to movable auxiliary windows and the wizard.
- Hardened journal semantics, i18n coverage, CI, release documentation, and
  the opt-in Slint MCP QA path.
- Updated Slint, base64, and TTS dependencies.

## Verification state

- Exact v0.35.3 full gate passed with `All gating layers green`.
- Exact `ui-mcp` QA build passed the live Slint MCP visual audit at 720x600.
  English before/after evidence shows Updates changing from 0.35.2 to 0.35.3;
  fresh English Help, wizard, and STT-model screenshots were inspected.
- All 13 global shortcuts reached distinct handlers. The debug-only QA build
  could not launch `suflyor-tts.exe`; the complete sidecar was subsequently
  built into the release bundle.
- Release binaries and NSIS installer built successfully. `overlay-host.exe`
  reports product/file version 0.35.3; the installer is 23,535,506 bytes with
  SHA-256 `957F6853832079D3955F7E6333A2601CCC36F5E530E6BBDD04C81BC2BDCDA0D9`.
- Installed runtime checks and owner visual acceptance are still pending.
- Qwen `qwen3.8-max-preview` release audit: `READY`, no code blockers.

## Remaining action

1. Install and verify version, cost-cap notice, stealth, TTS/STT suppression,
   diarization lifecycle, STT model persistence, context presets, drag grips,
   audio-source labels, journal flush, and clean exit.
2. Push this branch and open a PR against `master`; do not push `master`.
3. Show the evidence to the owner and wait for explicit post-evidence approval
   of v0.35.3 before merge, tag, or GitHub release publication.
4. After publication, review and merge the automation PR that updates the
   README latest-release marker to v0.35.3.
