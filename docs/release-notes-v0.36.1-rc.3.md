# Suflyor v0.36.1-rc.3

This release candidate fixes the deep-lock chip alignment regression found in
v0.36.1-rc.2.

## Fixes since rc.2

- The deep-lock icon is vertically centred beside its permanent `Local AI
  unloaded` / `Локальный ИИ выгружен` label in both full and compact bars.
- A UI layout guard now requires explicit vertical centring for that icon, so
  the same regression fails automated checks before a release.

## Included from rc.2

- F6 and `+ tile` report a managed local AI deep lock before an empty
  transcript.
- The persistent three-state lock and bundled DirectML runtime for older
  supported Windows 10 systems.

## Release-candidate note

This is a test build. Use the accompanying rc.3 retest checklist before
approving a final release.
