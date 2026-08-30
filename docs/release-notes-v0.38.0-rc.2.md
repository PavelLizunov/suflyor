# Suflyor v0.38.0-rc.2

This release candidate fixes the macOS Settings regressions found during RC1 acceptance and corrects the compact read-aloud tile layout.

## Fixes

- macOS now shows the running application version in **Settings → Backup**, where the platform-neutral maintenance controls live.
- Selecting **Managed MLX (Apple silicon)** for text immediately reveals its model details and Enable action without activating or persisting the provider prematurely.
- The equivalent Managed MLX Vision selection now reveals the Vision model configuration and action immediately.
- A short selected-text/read-aloud user bubble now follows its content height instead of stretching across the full tile.

## Installers

- macOS Apple Silicon, macOS 14.2 or newer: `Suflyor-0.38.0-rc.2-macos-arm64.dmg`
  - SHA-256: `3fc4aa760a45012452165cf61a315b038651e8084fc3427fec954ca07b4f543b`
  - The package is ad-hoc signed and not notarized; follow the installation and permissions guide included in the DMG.
- Windows: `suflyor-slint-setup.exe`
  - SHA-256: `356891ae6f85dac9ac66771001f928856cc5e2334bdd957b1cc36cd71fb6cec2`

## Verification

Candidate `7679a4cce2ccc609af7c8e690d5b42cdb5337de4` passed exact-SHA formatting, the macOS tile-layout guard and QA build, both native release builds, macOS DMG structure/signature/mount checks and mounted launch smoke, plus Windows silent-install, `DisplayVersion`, component-presence, and installed-binary launch smoke with the software renderer required by the worker's virtual display adapter. The unchanged i18n, Settings, tile, and version guard set had passed on both native workers immediately before the measured one-line tile-height correction.

The fast visual audit covered the four affected Russian states through Slint MCP. It measured the corrected read-aloud bubble at 54 px inside a 360 px tile, and an independent visual review passed all four captures. Evidence is in `docs/audit-2026-08-30-macos-settings-rc2/` and the manual acceptance list is `docs/retest-v0.38.0-rc.2-macos-settings.html`.

At the owner's request, this fast RC pass intentionally omitted the full 16-tab, English, baseline, global-hotkey, and prolonged verification matrices.

## Release-candidate note

This is a prerelease test build for acceptance before stable v0.38.0.
