# Windows audio device selection

Approved in DSH on 2026-09-14. Task branch: `codex/windows-audio-devices`.
Base: `2ab0dee01dda8632830aee1792543bc0d434237b`.

## Intent and invariants

Expose the existing Windows system-audio capture setting in Settings > Audio.
Both capture selectors offer Windows default, preserve explicit device choices,
and show an unavailable saved endpoint rather than silently selecting another.
Refresh devices on Settings reopen and on request, off the UI thread.

Keep the existing nullable `mic_device` and `system_audio_device` schema:
null follows the OS default; a name pins that endpoint. Preserve capture-side
mixed sources such as A50 Stream Out. Do not change the audio engine, macOS
capture behavior, TTS playback routing, dependencies, or unrelated settings.
Device changes apply to the next capture; explain that an active session must
be restarted. A failed save must leave the prior config and selection intact.
Enumeration placeholders and errors must never become persisted device names.

## Acceptance

- [x] Microphone and system selectors offer default and available endpoints (synthetic endpoints; real host empty/default checked).
- [x] Missing saved device is visible; default/new device replaces its binding (tests and EN/RU fixture).
- [x] Reopen/refresh obtains a new list without restarting the application (final host; physical hotplug not tested).
- [x] Loading, empty, unavailable, enumeration failure, and save failure are honest (EN/RU fixture and regression tests).
- [x] Regression tests cover mapping, special capture sources, and failed saves.
- [x] Exact candidate Windows targeted gate passes.
- [ ] Paired baseline/candidate Slint MCP captures cover affected states, RU/EN.
- [x] Functional hotkey pass records each dispatch or an explicit limitation (F1/F4/F7 passed; ten remaining keys named in checklist).
- [ ] Live default-output switching during capture is checked on Windows.

## Evidence

See [audio device audit](audit-2026-09-14-windows-audio-devices/README.md).
No stable/RC publication, installer, or merge is part of this task.
