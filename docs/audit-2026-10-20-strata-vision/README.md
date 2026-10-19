# Strata Vision UI audit (native checks passed; inference blocked)

Baseline: release candidate 564d9a739709b579875abd757e373eea7383334d,
product-equivalent to the current master before this feature. Windows software
renderer, light theme, scale 1, Settings 720x600, Vision tab, top scroll position.
Capture English and Russian before changing the UI, then repeat for the exact
committed candidate. Private native captures and task scripts live outside Git.

Acceptance includes download/resume/cancel and retained-window states, honest
unavailable status without a compatible Strata installation, image verification
before F8 activation, failure rollback, route persistence and hotkey dispatch.
No real Strata inference is claimed from mock fixtures or source inspection.

## Verified candidate

- Commit: `460ced5ebca3e84e26ae61ee67b42778792329dd`.
- Tree: `8df00c6a81d0ad6c789b89e24e0e2ef0e744f338`.
- Selected Windows native gate: exit 0, 800 backend unit tests, 94 Slint
  library tests, 153 host tests, selected integration/static guards.
- Additional retained-download lifecycle check: exit 0.
- `ui-mcp` native build: exit 0. QA executable SHA-256:
  `2e8cf9f02f7199317732b5ac36ad4fba337bca4340ceae8f112035242015add7`.
- Matching EN/RU unavailable/downloaded Vision cards inspected at 720x600,
  light theme, scale 1, top scroll; continuation remains reachable.
- Native download to a synthetic compatible folder: started, hidden, reopened,
  cancelled and resumed to 907,543,008 bytes. The actual downloaded encoder
  SHA-256 is `b1a82259702816a5330d7bd7607cd9676b11780e79ff7348c21103ff3ce49bd0`.
- Text/Vision/STT providers remained cloud/cloud/cloud; original Strata run
  configuration hash remained unchanged. No language weights were fetched.
- Explicit activation with a deliberately non-executable synthetic Python
  launcher failed, restored the exact original run configuration, retained
  the cloud F8 route and left the image-capability flag false. This checks
  failed-start rollback, not real encoder inference or running-server recovery.
- All 13 global hotkeys dispatched on this candidate. Selected-text dispatch
  does not prove audio playback. F8 off-route produced the expected notice.
- Independent read-only acceptance/UI source review found no blocking issue
  in the provisioning design; identified missing live inference coverage.

## Limits and cleanup

The Windows worker has 16 GiB RAM and no NVIDIA GPU. The approved GPU peer
was offline in Tailscale and SSH timed out. Successful image processing,
activation persistence and recovery of a previously running real Strata
remain unverified. No physical macOS checks or full geometry suite run is
claimed. Existing owner ghost/caption reports remain separate and open.

The worker configuration was restored byte-identically, and the normal
non-MCP rc.14 executable was restarted. The old historical scheduled build
remains disabled. The original developer checkout HEAD, raw index and all
13 unrelated files were verified unchanged. Private screenshots, logs,
fixture paths and scripts remain outside Git. This is a checkpoint, not a
release or completed feature acceptance.
