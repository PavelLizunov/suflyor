# Stable v0.38.1 release

Owner confirms the published rc.15 works and explicitly requests the stable
release. Preserve the original developer checkout and unrelated work.

Scope: synchronize 0.38.1 metadata, run the full stable Windows gate, resolve
only demonstrated release blockers, verify the native UI and installer, publish
v0.38.1 as Latest through an exact-SHA PR, verify the downloaded public asset,
and finish preview/apply release cleanup. Run the macOS full compile-seam gate
on the approved worker when available; report its actual state separately.

Do not infer individual provider/audio or multi-monitor acceptance from the
owner's general confirmation. Preserve known limits and exact evidence.

- [x] Full Windows fmt/Clippy/all-tests gate on immutable candidate.
- [x] Prior overlay-bar footer geometry failure reproduced and resolved.
- [x] macOS full compile-seam gate or exact access/resource limit recorded.
- [x] Exact-candidate native UI and 13 global-hotkey dispatch checks.
- [x] Stable installer build/install/start/uninstall and version checks.
- [x] PR merge/tag tree equality and downloaded public asset hash verified.
- [x] Release marked stable/Latest, cleanup preview/apply complete.

Published stable v0.38.1 from PR #300/tag84249fdcb967ec6415a0d62863b71dff880d2763.
Exact final evidence is in docs/release-evidence-v0.38.1/README.md.
