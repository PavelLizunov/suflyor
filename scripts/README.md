# overlay-mvp scripts/

Workflow tooling for the Slint build + the testing methodology in `../CLAUDE.md`.

## Files

| File | Purpose |
|---|---|
| `git-gate-native.ps1` | Selects docs, targeted, or full verification from the diff |
| `ci.ps1` | Full CI — fmt + clippy + tests for all five crates + UI-MCP compile |
| `post-release-cleanup.ps1` | Preview/apply safe PR, prerelease, branch, worktree, and target cleanup |
| `build-slint-release.ps1` | `-Installer` → release `overlay-host.exe` + NSIS `suflyor-slint-setup.exe` |
| `slint-installer.nsi` | NSIS script (installs to `%LOCALAPPDATA%\suflyor-slint\`) |
| `slint-experiment/scripts/capture_window.ps1` / `capture_primary.ps1` | DPI-aware screenshots (the Slint windows are layered → PrintWindow fails; these grab the composited pixels) |
| `click_at.ps1` / `hold_at.ps1` / `send_key.ps1` / `type_text.ps1` | Synthetic input for verifying the overlay |

## Quick usage

```powershell
# Inspect/run the gate selected for the current diff:
powershell scripts\git-gate-native.ps1 manual

# Force the full five-crate gate when risk warrants it:
powershell scripts\git-gate-native.ps1 manual -Full

# Cut a tester release:
powershell scripts\build-slint-release.ps1 -Installer
# → slint-experiment\target\release\bundle\suflyor-slint-setup.exe

# After publishing: preview, then apply repository/disk hygiene:
powershell scripts\post-release-cleanup.ps1
powershell scripts\post-release-cleanup.ps1 -Apply
```

## Why this exists

After the 2026-05-26 marathon, the project adopted layered verification. Since
2026-08-15 the native Git hooks select the smallest safe tier: docs validation,
one affected component, or full CI for cross-cutting/high-risk work. Visual UI
evidence remains mandatory independently of cargo scope. Releases also finish
with `post-release-cleanup.ps1`, so merged branches, obsolete prereleases,
completed worktrees, and rebuildable caches do not accumulate again.
