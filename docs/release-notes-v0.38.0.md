# Suflyor v0.38.0

Suflyor v0.38.0 is the first stable release with published installers for both Windows 10/11 and Apple Silicon macOS 14.2+.

## Highlights

- Native Apple Silicon application packaging with microphone and Core Audio system-capture support.
- Managed MLX text and Vision providers run through native Swift/Metal sidecars. Text inference begins prewarming in the background, and automatic answers use the verified 4096-token output budget.
- Full overlay mode reports `App RAM` / `RAM приложения` and short `MLX` memory below the model label. Per-request load, first-token, total, decode, and end-to-end measurements expire after 30 seconds; unified memory is never labelled as VRAM.
- macOS microphone and one-owner system capture stay armed through retryable device failures and recover after current-default route changes, including AirPods transitions. Microphone permission denial remains terminal.
- Live STT uses forced 10-second slices and an 800 ms silence flush to reduce delay.
- macOS Settings reports the current default input and presents Stealth as unsupported rather than promising capture exclusion.
- Windows retains capture-exclusion Stealth, DirectML local STT, the existing installer/update flow, and the established Slint overlay experience.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.
- Apple Silicon macOS 14.2 or newer: `Suflyor-0.38.0-macos-arm64.dmg`.

The Windows installer is unsigned. The macOS package is ad-hoc signed and unnotarized; follow the installation and permissions guide included in the DMG.

## Acceptance

The stable acceptance matrix is `docs/retest-v0.38.0.html`. Detailed macOS latency, memory, production-path, and AirPods evidence is recorded in `docs/audit-2026-08-31-macos-live-latency/`.
