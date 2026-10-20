# v0.38.1-rc.15 release evidence

Status: preparing the owner-requested experimental Windows prerelease.

Feature source was previously verified at
`460ced5ebca3e84e26ae61ee67b42778792329dd`: selected native gate passed
(800 backend unit tests, 94 Slint library tests, 153 host tests and selected
guards), as did the ui-mcp build. Matching EN/RU Vision screenshots and actual
encoder download/cancel/resume were inspected. Failed-start rollback preserved
the original run configuration and saved routes. All 13 hotkeys dispatched.

The release candidate adds synchronized rc.15 version metadata, release notes
and retest checklist. Its exact-SHA gate, QA build, installer and install smoke
are recorded before publication. Previous-SHA evidence is not substituted for
new-candidate verification.

Real Strata image inference, successful F8 activation persistence and recovery
of a previously running real text server remain unverified because suitable
GPU hardware is unavailable. The owner explicitly requested publication with
this limitation. Physical macOS and stable-only full-suite acceptance are not
claimed. The existing full overlay-bar geometry failure remains disclosed.

Private screenshots, fixtures and complete native logs stay outside Git.
