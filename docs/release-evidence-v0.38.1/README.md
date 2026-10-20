# Stable v0.38.1 — published verification

Owner reports rc.15 works and explicitly requested stable publication. That is
owner-reported acceptance, not an agent-captured real Strata inference trace.

## Immutable identity

- Tested/built candidate: `77708af4caf3ba7d4413097c4221cf5075de4557`.
- Product PR: #300. Merge and release tag: `84249fdcb967ec6415a0d62863b71dff880d2763`.
- Candidate, merge and tag share tree: `30515c632df4322628107358f488b15cb6cc9b5e`.
- Cargo manifest/lock, NSIS and both macOS plist version fields: `0.38.1`.
- Release: https://github.com/PavelLizunov/suflyor/releases/tag/v0.38.1
- Non-draft, non-prerelease; GitHub Latest resolves to `v0.38.1`.

## Full gates and demonstrated repairs

Windows worker `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
exited 0 on the exact candidate. All five crates passed fmt/Clippy/tests;
1,252 tests passed across 49 result groups, with 6 explicitly ignored.
The QA-only feature check and build passed. GitHub manual full run
`39431248172` completed successfully, including Windows rust, macOS and gate.
All required PR checks and the three cargo-deny jobs passed.

Physical Apple Silicon `bash scripts/git-gate-macos.sh --full` exited 0 on the
exact candidate: 1,233 Rust tests across 42 groups, 5 explicitly ignored,
Swift MLX tests/release build, Metal library and all Rust sidecars passed.
Preflight passed, Cargo jobs/test threads remained 2. An external-volume open
stall was diagnosed from process samples; task-owned APFS output was used
without clearing shared caches. Physical macOS UI is not implied by this gate.

Initial candidate `21f06cb8f2954f1c131df95f9aef6c456935e866` reproduced the
full-bar footer failure: memory y=56, height=12 in a 64px window. D-065 moves
telemetry to one line; the original strict geometry assertions pass EN/RU.
D-068 synchronizes two obsolete Russian fixture labels with the bundled PO,
without relaxing geometry assertions. D-066 gates a Windows-only setup constant.
D-067 returns before native macOS capture when already cancelled; the prior
physical test hung in native startup, the strengthened both-source test passes.
D-069 removes late initialization rejected by Clippy 1.99 without changing
local-engine switch side effects or status copy. Independent source/UI reviews
found no demonstrated blocking issue; tests and live evidence remain separate.

## Native Windows UI and shortcuts

QA binary SHA-256: `47d1ff8793655fdc1f40aa3c344d61c28c74f1acc2760fe65c86256acc37dddc`.
Matching EN/RU before/after full bars were inspected at 1280x64, scale 1,
Light Frost, position 0,24, idle state. Compact 680x64, Screenshots/Strata
card, lower continuation, Settings close/reopen and Diagnostics were inspected.
No shared Settings primitive was changed. The memory-visible permutations are
covered by the strict headless EN/RU regression; Windows normally reports no
MLX telemetry, so the native Windows captures do not prove a real memory sample.

All 13 global hotkeys registered and distinctly dispatched through native input.
F8, Shift+F8 and Ctrl+F8 followed the saved off route and displayed its notice.
Selected-text dispatch copied 0 characters in the worker; audible playback is
not accepted. System audio reported the worker's missing default device;
no successful audio capture is claimed. Screenshots/full logs remain private.

## Downloadable artifacts and installed consumers

Windows `scripts/build-slint-release.ps1 -Installer` exited 0.

- Asset: `suflyor-slint-setup.exe`.
- Bytes: `38380653`.
- SHA-256: `4300fc1bd00083db3cda8cc4afae0b5cd6841e0c70b619c11448c60d27c7d5b8`.
- Install/start/uninstall passed; embedded and registry version `0.38.1`.
- Installed process alive, 13 registrations, zero registration failures/panics.
- Config preserved and restored byte-identically. Normal non-MCP release left running.

macOS `slint-experiment/scripts/build-macos-dmg.sh` exited 0.

- Asset: `Suflyor-0.38.1-macos-arm64.dmg`.
- Bytes: `56341703`.
- SHA-256: `02dd40ae57c510c689eabed8d64230339299d7fed3520eb8ad69972fccbd6958`.
- DMG mounted, expected layout/installation guide checked, all four executables
  thin arm64, signatures verified, MLX packaged startup check passed.
- Exact packaged app copy/start/remove passed in a task-owned isolated HOME;
  alive for 10 seconds, version `0.38.1`, owner config unchanged. Existing
  `/Applications/Suflyor.app` was not replaced. Capture permissions and physical
  UI were not exercised. Package is ad-hoc signed, not notarized.

Both public assets were independently downloaded from the release and matched
these exact sizes and SHA-256 digests. Tag commit and tree were verified.

## Cleanup and preservation

Release cleanup preview and apply completed successfully. The merged automation
README branch was removed. Five unrelated non-ready PRs #205, #286, #288, #289
and #296 and unproven branches were preserved. The newest prerelease rc.15 was
retained; stable releases were untouched. Original developer HEAD, raw index
and all 13 unrelated changed/untracked files remained byte-identical. Historical
`SuflyorClaudeS7Job` stayed disabled. A documentation-only closeout records the
final archive and cleanup evidence separately from the installer build identity.

## Remaining limits

Real Strata inference, successful enabled-F8 persistence/recovery of a previously
running real text server, physical macOS UI, audible selected-text playback and
the owner's provider wire trace remain independently unverified. The optional
Strata add-on remains experimental. The owner's multi-monitor transparent
rectangle remains D-063, a separate unverified hypothesis. General owner
acceptance does not silently close these gaps or mark the active Vision charter
complete. No credentials, fixture models or complete native logs are committed.
