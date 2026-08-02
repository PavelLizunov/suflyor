# Suflyor v0.35.3

This release focuses on clearer setup and diagnostics, safer session handling,
and more predictable local speech and AI controls.

## Highlights

- Added guided speech-to-text provider and cloud Whisper model selection.
- Added a consistent drag grip to the wizard and movable auxiliary windows.
- Improved Help, Archive, empty answer tiles, Diagnostics, and audio-source
  labels so the app is easier to understand without an explanation.
- Fixed local-AI context preset selection on high-RAM systems.
- Added visible, debounced notices when the configured session cost cap is hit.

## Reliability

- Kept transcription suppressed correctly while read-aloud is playing or
  paused, then resumes it after playback.
- Moved persistence work off the UI thread and flush session journals before
  exit.
- Verifies the effective Windows capture-exclusion state and reports stealth
  failures instead of silently claiming success.
- Fixed diarization model installation and window lifecycle handling.
- Preserved journal response semantics and expanded translation guards.

## Engineering and security

- Updated Slint, base64, and TTS dependencies.
- Added regression-tested fast CI for documentation-only changes.
- Removed the Slint MCP debug server from normal production builds; UI QA can
  still enable it explicitly with the `ui-mcp` Cargo feature.

Windows 10/11 remains the supported platform. Native Linux and macOS versions
are planned, with no announced delivery date.
