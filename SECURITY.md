# Security Policy

suflyor handles local API keys (Groq / AI bearer), captures the screen for the
Vision feature, and ships an auto-updater that downloads and runs an installer —
so security reports matter.

## Reporting a vulnerability

Please report **privately**, not via a public issue:

- Open a private GitHub security advisory:
  <https://github.com/PavelLizunov/suflyor/security/advisories/new>

Include repro steps and the affected version (the installer filename, e.g.
`suflyor-slint-setup.exe`, or Settings → About). You'll get an acknowledgement
within a few days.

## In scope

- Secret leakage — API keys/bearers reaching logs, journals, the copied
  diagnostic report, or an AI/STT/vision error tile.
- The auto-updater download-and-execute path (artifact verification).
- The local AI / STT server network surface.
- Stealth / screen-capture egress.

## Out of scope

- Issues requiring physical access to an already-unlocked machine.
- The bundled third-party model weights / inference engines (report upstream).
