# Suflyor v0.38.1-rc.4

This release candidate collects the fixes merged since rc.3: redaction of secrets and hosts in diagnostics and log exports, private files that only their owner can read, the move to Slint 1.18.1, and a set of interface corrections. It adds no new feature.

## Security and privacy

- **Diagnostics and log exports**: a `Bearer` token is masked in any letter case and after any separator, an `x-api-key` header and `xai-` / `nvapi-` keys are masked, and a URL loses its host, its query and its fragment. Two URLs joined by a comma or a semicolon are masked one by one.
- **Private files on macOS and Linux**: the settings export, the config backup, session journals and the Hermes `.env` are created readable by their owner only.
- **Connection tests**: a failed cloud STT test reports "Groq API unreachable" instead of the transport error text.
- **Dependencies**: `wasapi` 0.25 (advisory `RUSTSEC-2026-0332`).

## Interface

- **Slint 1.18.1**: the app starts on a machine without an OpenGL driver; the Shift+F9 row of Settings > Hotkeys no longer clips its second line.
- **Russian interface**: the bar labels and the bar's state word ("idle", "recording", "paused" and the others), the bridge status, speaker labels and the MSK suffix follow the interface language.
- **Settings**: opens on "Profile + context", the first entry of the list, instead of "Stealth".
- **Accessibility**: the toggle chips of the bar and the Settings navigation expose their state to assistive tools.
- **Tray**: a right click can no longer be swallowed.
- **Answers with formulas**: operator names, accents, determinants and vectors render.
- **Archive**: the count in the heading follows a delete.

## Reliability

- A session journal with invalid UTF-8 is indexed without the damaged line instead of being skipped.
- Read-aloud sidecar processes on Windows end together with the app.
- A new session tears down the previous capture before it starts.

## Installers

- Windows 10/11: `suflyor-slint-setup.exe`.

The Windows installer is unsigned.
