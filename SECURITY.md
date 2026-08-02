# Security Policy

suflyor handles local API keys (Groq / AI bearer), captures the screen for the
Vision feature, and ships an auto-updater that downloads and runs an installer —
so security reports matter.

## Supported versions

Only the latest published release is supported. Upgrade through the in-app
updater or download the newest installer from
[GitHub Releases](https://github.com/PavelLizunov/suflyor/releases).

## Reporting a vulnerability

Please report **privately**, not via a public issue:

- Open a private GitHub security advisory:
  <https://github.com/PavelLizunov/suflyor/security/advisories/new>

Include repro steps and the affected version (the installer filename, e.g.
`suflyor-slint-setup.exe`, or Settings → Updates). We aim to acknowledge a
report within a few days and provide a fix or mitigation timeline after triage.
This project does not currently run a bug-bounty program.

## In scope

- Secret leakage — API keys/bearers reaching logs, journals, the copied
  diagnostic report, or an AI/STT/vision error tile.
- The auto-updater download-and-execute path. It restricts release assets to
  GitHub hosts and fail-closes unless the installer SHA-256 matches the digest
  in GitHub release metadata; the installer is not Authenticode-signed.
- The local AI / STT server network surface, and the token-protected Hermes API
  when explicitly bound beyond loopback (for example to Tailscale or `0.0.0.0`).
- Stealth / screen-capture egress.
- Exposure of captured audio, transcripts, screenshots, summaries, or local
  paths through logs, diagnostics, UI errors, or exported data.

## Out of scope

- Issues requiring physical access to an already-unlocked machine.
- The bundled third-party model weights / inference engines (report upstream).
