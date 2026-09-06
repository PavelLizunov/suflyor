# Icon redesign — verification checkpoint

Status: **incomplete; not accepted for release**.

## Scope and protected assets

The task covers the 50 SVG interface pictograms in
`slint-experiment/assets/icons/` and their static guard. Logos, mascot artwork,
`assets/brand-*`, the root application icon, and all binary brand assets are
out of scope and must remain unchanged.

## Authorship correction

Earlier chat messages claimed that separate Astra-generated reference sheets
had been produced and used for vectorization. No such image-generation result
is present in the recorded tool history. The initial SVG drafts were written
by Gemini workers from text assignments; the lead Astra agent subsequently
edited some of them. This is not evidence that Astra personally drew all 50
icons. The user's explicit authorship requirement remains unmet. Do not label
this candidate as an exclusively Astra-drawn final set.

## Exact identities

- Original baseline: `bffbfd44` (resolve the full SHA before a baseline build).
- Current candidate: `e82c5e59b67312aec36c063cb2974d2d5d788404`.
- QA executable SHA-256:
  `40D6A9CABED75B9E8D82CCAA5259C45FB379A0FEF980167DA32304C893647BD0`.
- Branch: `codex/redraw-svg-icons`.
- Worker: `windows-worker`.

## Observed verification

The candidate's Windows run completed with exit code 0:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_BUILD_JOBS = '2'
$env:SLINT_EMIT_DEBUG_INFO = '1'
cargo build --locked --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml
```

The selected gate reported targeted scope, formatting, Clippy, and tests for
`slint-experiment`. The separate MCP build also completed successfully.
Independent source review accepted the corrected candidate; that review did
not perform visual raster verification.

Evidence root on the worker:
`%LOCALAPPDATA%\dsh-jobs\astra-icons-e82c5e59\`.
It contains `build.log`, `exit.txt`, `manifest.txt`, and `screenshots\`.
The build transcript may omit child-process output; the DSH background-job
result also records the command output.

## Actual capture conditions and limits

- Slint MCP port: **9124**, not the originally planned 9123.
- Settings: 720×600, scale factor 1.0.
- Overlay bar: 1280×64, scale factor 1.0.
- Help: 640×680, scale factor 1.0.
- Theme/language: **Glacier, English**.
- Captured and visually inspected: bar, Help's first viewport, and all 16
  Settings tabs' initial viewports.
- Default renderer launch failed with `Could not locate glCreateShader symbol`.
  Captures were made using `SLINT_BACKEND=winit-software`; they do not establish
  acceptance under the normal production renderer.
- Redirecting `APPDATA` did not prove profile isolation: startup still showed
  existing configuration/session state. Do not call the profile isolated or
  exercise data-mutating/network actions until the actual profile mechanism
  has been verified. Never copy credentials or private configuration here.
- Raw captures remain outside Git pending privacy review.
- No matching baseline captures have been made. The original baseline is
  `bffbfd44`, not the candidate's immediate parent, which already changed icons.
- No complete hotkey-dispatch pass, other-theme pass, Russian pass, scrolling
  pass, or full remaining-window/state pass has been completed.

## Safe continuation

1. Fulfil the Astra-only authorship requirement for all 50 interface SVGs;
   preserve logo/mascot assets byte-for-byte.
2. Verify the changed geometry in actual-size previews before native rebuilds.
3. Use a new immutable candidate SHA for changed SVGs; do not reuse this
   candidate's test evidence as evidence for a later source snapshot.
4. Obtain matching before/after captures from the original baseline and new
   candidate with the same renderer, language, theme, size, and data state.
5. Complete remaining state coverage and functional checks, or explicitly
   report each unverified item. Never present after-only captures as a pair.
6. Keep operations bounded, preserve a checkpoint after each completed phase,
   and do not restart DSH or repeatedly poll unchanged build state.

On the most recent read-only reconnect check, the candidate process remained
alive and the MCP listener was available. The worker checkout had one untracked
task-created launcher, `overlay-job.cmd`; tracked source remained unchanged.
