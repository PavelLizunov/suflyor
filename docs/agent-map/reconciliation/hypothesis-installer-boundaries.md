# Original CI C06/C07: bounded installer evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_installer_boundary_hypotheses.py) inspect frozen NSIS/PowerShell source. They do not compile NSIS, launch an installer, stop a process, or delete files. C06/C07 remain hypotheses.

## C06 — per-user install has no signature or ACL tightening

[Installer header](<../../../scripts/slint-installer.nsi#L14-L31>) requests user execution, defaults to `$LOCALAPPDATA\suflyor-slint`, and still exposes a directory page. The file contains no `!uninstfinalize`, AccessControl plugin call, or explicit overwrite policy. Registry uninstall strings are quoted and stay under HKCU.

## C07 — stop command and recursive cleanup

[Process stop](<../../../scripts/slint-installer.nsi#L33-L55>) copies the helper to `$PLUGINSDIR` and interpolates `$INSTDIR` inside quotes without a NSIS quote escape. [Helper](<../../../scripts/stop-installed-suflyor.ps1#L1-L47>) compares full executable paths and returns distinct check/stop codes. This source boundary is not a Windows command-line injection execution.

[Uninstall](<../../../scripts/slint-installer.nsi#L119-L159>) asks for confirmation and then recursively removes `$APPDATA\suflyor`, `$APPDATA\overlay-mvp`, and `$PROFILE\suflyor-local-ai`. No deletion was performed.

## Limits

No makensis, installer, process enumeration, or filesystem cleanup was run. Original statuses and 39/75/5 remain unchanged.
