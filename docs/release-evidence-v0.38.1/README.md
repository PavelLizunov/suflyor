# Stable v0.38.1 evidence — preparation in progress

Owner reports rc.15 works and explicitly requested stable publication. This is
owner-reported acceptance, not an agent-captured real Strata inference trace.

Version metadata: Cargo manifest/lock, NSIS and both macOS Info.plist fields
are synchronized to 0.38.1. Exact final candidate is pending.

## Gates

Initial candidate 21f06cb8f2954f1c131df95f9aef6c456935e866 passed Windows
fmt and full Clippy; its complete test run is still in progress. Physical
macOS full Clippy reproduced D-066, Windows-only STRATA_FAMILY unused on macOS.
A cfg repair is prepared; no final full-gate pass is claimed yet.

D-065 is the previously demonstrated EN/RU memory-footer geometry failure.
The fixed-height footer now uses one horizontal telemetry line. The existing
strict geometry expectations are not weakened. Native matching captures and
final exact-candidate tests remain pending.

The older rc.15 GitHub CI run 39350709931 completed successfully, including
macOS. This is supporting historical evidence, not stable-candidate acceptance.

## Native consumer and artifact

Pending: exact candidate Windows MCP UI/13 shortcut dispatches, installer
build, install/start/uninstall, embedded version, size and SHA-256.
Pending: PR merge/tag tree equality, stable/Latest flags, public installer
download/hash verification and cleanup preview/apply. No stable release is
published at this preparation checkpoint.

## Limits

Real Strata inference, successful F8 activation persistence/recovery of a real
running Strata text server, physical macOS UI, audible selected-text playback
and the owner provider wire trace are not independently verified. The owner's
multi-monitor transparent rectangle remains a separate unverified hypothesis.
Private screenshots, fixtures, full logs and worker config remain outside Git.
