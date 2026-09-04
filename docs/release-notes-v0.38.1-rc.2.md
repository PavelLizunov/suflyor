# Suflyor v0.38.1-rc.2

This release candidate fixes delayed system-audio transcription and contaminated automatic-tile questions observed while testing v0.38.1-rc.1 on Apple Silicon.

## Fixes

- Continuous system audio is submitted to STT after at most five seconds instead of ten. Microphone utterances retain the existing ten-second safety cap, and natural 800 ms silence still flushes either source earlier.
- Automatic tiles extract the final real question from a multi-sentence transcript segment instead of sending the preceding statement as part of the question.
- Aggressive every-line mode rejects one-word and repeated-word recognition noise while preserving substantive non-question lines.
- The managed MLX sidecar, compact prompt, 384-token automatic-tile budget, and single-flight policy from v0.38.1-rc.1 are unchanged.

## Installers

- Apple Silicon macOS 14.2 or newer: `Suflyor-0.38.1-rc.2-macos-arm64.dmg`.
- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned. The macOS package is ad-hoc signed and unnotarized; follow the installation and permissions guide included in the DMG.

## Known limitation

This RC improves transcript timing and question boundaries; it does not replace or retune the managed LFM model. Local-model factual mistakes remain a separate evaluation item.
