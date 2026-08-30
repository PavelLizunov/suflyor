# Suflyor v0.38.0-rc.1

This release candidate brings the current Windows application and the active
macOS port together for final acceptance before v0.38.0.

## Local speech recognition

- macOS GigaAM now defaults to CPU; Core ML remains an explicit opt-in because
  it was slower and used more memory on the measured short-utterance fixture.
- Validation, live transcription, and push-to-talk share one process-wide
  GigaAM model instead of retaining duplicate model owners.
- Stopping a session or changing the model/provider releases the shared cache
  without invalidating in-flight work.
- Windows keeps DirectML as its default. The native gate now stages the matching
  DirectML runtime for backend tests on older Windows 10 systems.

## Runtime reliability

- The overlay host's large runtime modules were split without changing the
  visible UI or hotkey contracts.
- Managed-MLX non-streaming responses now report server completion usage and
  timing data; stream cancellation and terminal-event guards remain covered by
  the native gates.

## Verification

- Exact-SHA macOS and Windows full gates passed for the STT candidate.
- GigaAM CPU warm median was 75.6 ms on the recorded fixture; the duplicate
  live-model RSS delta fell from about 445 MiB to 2.4 MiB.
- A bounded macOS concurrency run completed five GigaAM lifecycle cycles while
  Piper TTS and the pinned MLX text model were resident and serving production
  requests. Peak combined RSS was 3961 MiB, with zero swap and no thermal or
  performance warning.
- EN/RU Settings screenshots and detailed STT measurements are recorded in
  `docs/audit-2026-08-29-stt-macos/`; the three-process resource screen is in
  `docs/audit-2026-08-30-stt-mlx-tts-concurrency/`.

## Known acceptance item

The unattended macOS worker had authorized TCC database records for microphone
and audio capture, but a freshly signed GUI test bundle did not progress from
session-journal creation into an established audio route, and injected playback
produced no non-silent samples. The RC checklist therefore keeps real microphone
and system-audio transcription as an explicit manual acceptance item; synthetic
audio exercised the same production STT pipeline.

## Release-candidate note

This is a prerelease test build. Use the accompanying v0.38.0-rc.1 retest
checklist before approving the stable v0.38.0 release.
