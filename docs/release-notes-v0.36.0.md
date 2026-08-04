# Suflyor v0.36.0

This release keeps saved memory exactly as approved, removes Suflyor's
auxiliary windows from the Windows taskbar, and makes interface language more
consistent.

## Highlights

- Explicitly saved memory from tiles and transcripts is now stored verbatim,
  preserving internal whitespace, newlines, table rows, identifiers, and
  repetitions. Approved memory is no longer rewritten in the background.
- Added an explicit **Restore original** action for legacy memory entries that
  still have their original source. Manual edits disable restoration so newer
  user changes cannot be overwritten.
- Auxiliary windows no longer create or briefly flash a Windows taskbar
  button. Taskbar exclusion is requested before the first visible frame, with
  a shared hardened Win32 tool-window fallback.
- Deterministic tile chrome, notices, statuses, and errors now follow the
  selected interface language; generated answers continue to follow the
  separate AI response-language setting.
- Polished Session Archive alignment, row actions, and scrollbar spacing, and
  refreshed the public screenshots with the current privacy-safe English UI.

## Compatibility

- Legacy memory entries can be restored only when an older version retained
  their original source; already-condensed entries without that provenance
  cannot be reconstructed automatically.
- Windows 10/11 remains the supported platform. Native Linux and macOS versions
  are planned, with no announced delivery date.
