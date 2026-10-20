# v0.38.1-rc.15 release evidence

Status: published experimental Windows prerelease at the owner's explicit request.

## Source and artifact identity

- Product PR: [#298](https://github.com/PavelLizunov/suflyor/pull/298).
- Exact tested candidate: `91a2fbf65da775530caef623fd48608a09f9f761`.
- Merge/tag commit: `b041f6462ba99f3785e9a2b4c4a0818fa49232ec`.
- Candidate and merge share tree `8b818cb2009ae0e7d045ffb856e5d6b335088979`.
- [Published prerelease](https://github.com/PavelLizunov/suflyor/releases/tag/v0.38.1-rc.15).
- [Windows installer](https://github.com/PavelLizunov/suflyor/releases/download/v0.38.1-rc.15/suflyor-slint-setup.exe): 38,380,312 bytes.
- Installer SHA-256: `91b01ea6ce8a0775d77229aa7d6cabdc9ebfba2d8b4681110ba17a60f0da42f1`.
- The published installer was downloaded independently and matched the exact size/hash.
- Release is non-draft and prerelease; stable Latest remains v0.38.0.

## Exact-candidate Windows checks

- Selected native gate: exit 0; 800 backend unit, 94 Slint library and 153 host
  tests passed, alongside all selected static/integration guards. No stable-only
  full-suite or full overlay geometry pass is claimed.
- `ui-mcp` build: exit 0. QA executable SHA-256:
  `06a29dd2b5d5a575940a251c21220e07c2adb568c0edba331bd697465b95c323`.
- EN/RU Vision page inspected at 720x600, light theme, scale 1: unavailable,
  downloaded, downloading, retained reopen and cancellation. Bottom options
  remain reachable. The downloaded caption explains the text-route prerequisite.
- Real encoder transfer into a synthetic compatible folder: cancel and resume
  reached 907,543,008 bytes, SHA-256
  `b1a82259702816a5330d7bd7607cd9676b11780e79ff7348c21103ff3ce49bd0`.
- Download left text/Vision/STT providers cloud/cloud/cloud and the Strata run
  configuration byte-identical. No language weights were downloaded or started.
- Explicit activation cancellation reports activation cancellation, not download
  cancellation. A deliberately failing synthetic launcher restored the original
  run configuration, preserved the cloud F8 route and kept image capability false.
- All 13 global shortcuts registered and dispatched on the QA candidate. F8's
  saved off-route produced its expected notice. Selected-text dispatch is not
  proof of actual audio playback.
- Installer build: exit 0. Install, installed executable startup and uninstall:
  exit 0; executable and registry version 0.38.1-rc.15; 13 registered hotkeys,
  zero registration failures/panics; configuration restored byte-identically.
- Normal non-MCP release was left running on the worker after the smoke test.

## Supporting review and CI

Independent read-only acceptance/UI source review identified prerequisite and
cancellation copy defects. They were corrected in the tested candidate and
confirmed through native EN/RU text and activation-cancel checks (D-064).

GitHub PR #298 required gate, Windows Rust, validation, gitleaks and all three
cargo-deny checks passed. Advisory macOS CI was still in progress at publication;
no physical macOS UI verification is claimed.

## Publication cleanup

Post-release cleanup preview and apply both passed. Old rc.14 release/tag and
the merged task remote branch were removed. Five non-ready unrelated PRs
#205, #286, #288, #289 and #296 and unproven branches were preserved. GitHub
`delete_branch_on_merge` remains enabled. The active worker checkout/targets
were retained; no inactive worker worktree target was eligible for removal.

The original developer checkout HEAD, raw index and 13 unrelated files were
verified unchanged. Private screenshots, fixture paths and complete logs remain
outside Git. The historical scheduled build remains disabled.

## Explicit experimental limits

Real Strata image inference, successful F8 activation persistence and recovery
of a previously running real text server remain unverified: the Windows worker
has 16 GiB RAM and no NVIDIA GPU, while the approved GPU peer was offline.
The owner requested this prerelease after that limitation was disclosed. Real
encoder downloads and synthetic failed-start checks do not establish inference.

Existing owner transparent-rectangle/caption reports and the previously disclosed
full overlay-bar geometry failure remain separate. Physical macOS, actual speech
and full stable-suite acceptance are not claimed. Existing macOS MLX is unchanged.
