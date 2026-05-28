# overlay-mvp scripts/

Workflow tooling for the Slint build + the testing methodology in `../CLAUDE.md`.

## Files

| File | Purpose |
|---|---|
| `ci.ps1` | Local CI — fmt + clippy + tests for slint-experiment + overlay-backend |
| `build-slint-release.ps1` | `-Installer` → release `overlay-host.exe` + NSIS `suflyor-slint-setup.exe` |
| `slint-installer.nsi` | NSIS script (installs to `%LOCALAPPDATA%\suflyor-slint\`) |
| `capture_window.ps1` / `capture_primary.ps1` | DPI-aware screenshots (the Slint windows are layered → PrintWindow fails; these grab the composited pixels) |
| `click_at.ps1` / `hold_at.ps1` / `send_key.ps1` / `type_text.ps1` | Synthetic input for verifying the overlay |

## Quick usage

```powershell
# Before every commit:
powershell scripts\ci.ps1                 # fmt + clippy + tests (both crates)
# (then in your Claude session)
# → Agent(subagent_type:"general-purpose", prompt = docs/REVIEW_AGENT_PROMPT.md)

# Cut a tester release:
powershell scripts\build-slint-release.ps1 -Installer
# → slint-experiment\target\release\bundle\suflyor-slint-setup.exe
```

## Why this exists

After the 2026-05-26 marathon (29 releases in 6 h → user cut most by hand),
we adopted a strict pre-commit gate (fmt + clippy -D warnings + tests, plus
a review-agent + live smoke). The `.claude/hooks/git-gate.ps1` hook enforces
the gate on every commit/push. Skipping a layer historically produced a
user-facing regression.
