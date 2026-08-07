# Suflyor v0.36.1-rc.2

This release candidate fixes two deep-lock defects found while validating
v0.36.1-rc.1.

## Fixes since rc.1

- F6 with an empty transcript now reports that the managed local AI is
  deep-locked/unloaded. The explicit deep lock takes precedence over transcript
  and endpoint preconditions.
- The separate `+ tile` manual path follows the same ordering and notice.
- Cloud AI and external local endpoints retain their existing empty-transcript
  behavior and two-state lock.
- The deep-lock chip now permanently says `Local AI unloaded` (localized in
  Russian) instead of relying on a red icon alone.
- Full and compact bar layouts were rechecked in English and Russian, including
  the running/open-tile worst case. The compact bar is 680 px wide so the
  complete state label, session controls, and Expand do not overlap.

## Included from rc.1

- The persistent three-state lock for Suflyor's managed local AI.
- Live EN/RU Settings localization fixes.
- The matching bundled DirectML runtime for supported older Windows 10
  systems.

## Release-candidate note

This is a test build. Use the accompanying rc.2 retest checklist before
approving a final release.
