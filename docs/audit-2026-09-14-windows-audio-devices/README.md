# Windows audio device settings audit

Status: implementation and available Windows checks completed; full acceptance
is limited by hardware, renderer, independent-review and minimum-window coverage.
See [the checklist](checklist.md) for exact passes, findings and untested limits.

## Identity

- Branch: `codex/windows-audio-devices`
- Baseline commit: `2ab0dee01dda8632830aee1792543bc0d434237b`
- Baseline tree: `d3b0fb8c3e6eedbe688a116f60b53e3c2250a4ed`
- Worker: `windows-worker`
- Tested implementation: `749813a8ed368d2585b7fc86be9e6636d928bce5`
- Tested tree: `035714b1e42159f21f9cc474da9d511e440837fc`

## Baseline job plan

- Scheduled task: `SuflyorAudioDevicesBaseline20260914`
- Script: `C:\suflyor-test-evidence\audio-devices-20260914\baseline.ps1`
- Log / exit / manifest: `baseline.log`, `baseline.exit`, `baseline-manifest.json`
  under the same task evidence directory.
- Checkout: `C:\suflyor-audio-devices`, detached at the exact baseline.
- Build cache: existing `C:\suflyor\slint-experiment\target`, sequential use only.
- No existing task wrapper is replaced.

## Capture conditions

Target paired conditions: Settings 720x600, scale 1.0, same theme, language,
renderer, audio device configuration and scroll position for each before/after
pair. Record actual conditions before accepting images. Screenshots must contain
no secrets, private transcript/session titles, paths, or network endpoints.

Previous worker evidence warned that overriding APPDATA does not isolate
Windows known-folder configuration. Do not assume isolation or modify the live
profile based on an environment override. Hardware availability and production
renderer must be established; software rendering is not production acceptance.

## Baseline evidence

Baseline Cargo build completed (7m09s); a cached confirmation returned exit 0.
Binary SHA-256: `FFF8DA018A1BEF4C2076FCCBA9AB0A9324CA191CFB917B7C15C573A9C8E2A5E0`.
The first wrapper misclassified Cargo stderr as a PowerShell error; the second
waited on a console descendant after Cargo ended. Neither was accepted as a
gate. The final direct `cmd /c` invocation recorded the successful exit and hash.
Final wrapper: `baseline-r4.ps1`; final logs: `baseline-r4.log`,
`baseline-build-r4.log`, `baseline.exit`, `baseline-manifest.json`.

The default renderer failed to initialize OpenGL (`glCreateShader` missing).
Actual baseline capture: `winit-software`, English, Glacier, Settings 720x600,
scale 1.0, Audio tab at scroll top. Slint MCP returned a populated element tree;
Settings and Audio were opened through discovered element handles. The Audio
screenshot shows only a microphone selector, no system selector, and no capture
endpoints. The privacy-reviewed `before-audio-en.png` is included here
(SHA-256 `54f58530090a16b26964d7bb445dc892ddbaffe7d6eed743d1481385f70932b9`).
Raw logs and MCP trees remain in the ignored `artifacts/audio-devices/` directory.

Windows PnP reports only an unavailable Remote Audio endpoint. There is no
physical headset pair on this VM, so live default-output/headset switching
cannot be accepted from this environment. Software-renderer captures do not
establish production Skia acceptance. Baseline Russian captures were not taken;
English before/after is the comparison pair, Russian candidate is a separate
localization check.

## Candidate jobs

Candidate `00522431e0b738ee95333110d6e6664c8500e518`:
`SuflyorAudioDevicesCandidate00522431`, script `candidate.ps1 -Sha <exact SHA>`,
under the same evidence root. Logs/markers: `gate-00522431.*`,
`tests-00522431.*`, `guards-00522431.*`, `build-00522431.*`,
`candidate-00522431.*`. No existing task wrapper was replaced.
The initial task refused the wrong checkout before any build; after preserving
the task-owned formatting results, the checkout was corrected to the exact SHA.

## Check results

For `00522431`: backend fmt and Clippy passed. Backend unit tests: 723 passed,
1 failed, 1 ignored; the failure was
`ai::tests::queued_stream_stops_when_receiver_is_dropped` (semaphore reacquire).
All five `audio_route` tests passed. All six new host audio tests passed,
including UI-index rollback and complete in-memory config preservation on a
simulated save failure. All 15 focused guard tests passed: audio settings (2),
i18n (3), macOS Settings (7), transient reset (3). The MCP host build completed.
The overall runner correctly returned 1 because the backend test failed.
This evidence does not verify the later source changes.

First fully gated candidate: `7169b687c6fd5a3ee2fb47d3b1a9d2bfd7b13e10`,
tree `ad05b54af3745530f512c391977c77a50991f518`.
Final job: `SuflyorAudioDevicesCandidate7169b687`, remote script
`candidate-next.ps1 -Sha <exact SHA>`, same checkout/cache/evidence root.
Per-step logs/markers use the suffix `7169b687`. `CARGO_BUILD_JOBS=2` and
`RUST_TEST_THREADS=1` keep native work bounded and eliminate parallel-test
contention as a variable; failures remain failures and are not suppressed.

The final targeted gate returned **0**: backend fmt/Clippy and 732 tests passed
(3 ignored); Slint fmt/Clippy and 283 tests passed (3 ignored). No failed tests.
A separate host Clippy check, all six audio-controller tests, and all 15 focused
guard tests also passed on the final SHA. These counts exclude duplicate reruns.
The earlier AI-semaphore test passed when the suite ran serially; no AI source
or assertion was changed. Mechanical evidence is `gate-7169b687.log/.exit`,
`clippy-7169b687.log/.exit`, `tests-7169b687.log/.exit`, and
`guards-7169b687.log/.exit` under the recorded evidence root.

A `#[cfg(test)]` + `ui-mcp` ignored fixture constructs the real Settings window
with synthetic in-memory config and an injected save result. It never invokes
production startup, reads user config, or writes it. It permits RU/EN and
missing/loading/enumeration-error/save-error UI inspection on this endpoint-free
VM. Fixture screenshots are not evidence of physical device discovery, durable
filesystem writes, application startup, global hotkeys, or production rendering.
The test-thread event loop uses Slint's existing
[BackendSelector API](https://docs.rs/slint/1.17.1/slint/struct.BackendSelector.html#method.with_winit_event_loop_builder),
without adding dependencies. The feature-built host remains the baseline-pair
and startup audit target.

### Runtime observations on 7169b687

The production MCP host launched successfully with SHA-256
`78442EDA77B2FD4259EECFAC7FCFDF04CC2C1D1944A0DED90E3266AB1E59221B`.
The EN Audio before/after pair matches 720x600, scale 1, Glacier, software
renderer, original profile and top scroll position. Both selectors show
Windows default with honest empty notices; microphone test is disabled.
Refresh and closing/reopening Settings repopulated both defaults. Closing
actually clears the controller slot and creates a new Settings window; MCP
also listed stale deleted handles, so discovery had to skip those handles and
return to Audio. Bottom-scroll capture reaches recording and storage controls.
F1, F4 and F7 were injected through Windows input in the verified interactive
session. Distinct logs and matching Help/Palette/Archive windows confirmed each
dispatch; F1 also toggled closed. No archive contents were captured.

The ignored fixture failed immediately with `NoTranslationsBundled`, before
any interactive test: it selected a language before constructing the first
Slint component. Candidate `749813a8ed368d2585b7fc86be9e6636d928bce5` moves
component construction before language selection (test-only change). The new
native gate, host/fixture builds and available live checks all completed below.

## Final results: 749813a8

Task `SuflyorAudioDevicesCandidate749813a8`, same `candidate-next.ps1`, exact
commit and tree listed above. Runner and every step returned **0**. The targeted
gate again reports backend 732 passed / 3 ignored and Slint 283 passed / 3 ignored
(1015 passed, 0 failed). Separate host Clippy, six audio tests and 15 focused
guards also passed. The default ignored tests were not enabled by the gate.

Final host SHA-256:
`EDB94ED542601353E02DA1142491DEB515B06DE6F6520D9699FC76B07D645B7C`.
Final fixture SHA-256:
`A92339011B161766113755A74AAA42133D7FFA9B2B21F4D39B3BF7D8744CC462`.
Both were hash-checked by the interactive launch scripts. Evidence filenames
use `749813a8` under the same recorded evidence root.

Commands executed from the clean exact-SHA checkout, with
`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=1`, and
compile-time `SLINT_EMIT_DEBUG_INFO=1`:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/git-gate-native.ps1 manual -Base 2ab0dee01dda8632830aee1792543bc0d434237b
cargo clippy --locked --manifest-path slint-experiment/Cargo.toml --bin overlay-host -- -D warnings
cargo test --locked --manifest-path slint-experiment/Cargo.toml --bin overlay-host settings_audio -- --test-threads=1
cargo test --locked --manifest-path slint-experiment/Cargo.toml --test audio_settings_guard --test i18n_guard --test settings_reset_guard --test macos_settings_guard
cargo build --locked --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml
cargo test --locked --bin overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml --no-run --message-format=json
```

The final host was launched and Audio checked again: both defaults, Refresh,
close/reopen and bottom scroll passed. Its top and bottom screenshots are
byte-identical to the 7169b687 captures. F1/F4/F7 dispatch was repeated against
this final host and confirmed by distinct logs and window geometry. All ten
fixture runs (EN/RU × available, empty, loading, enumeration error, save error)
closed normally. Actual keyboard input selected USB devices, A50 and the
localized default; failed saves restored both displayed selections. Missing
rows disappeared only after successful replacement. The checklist distinguishes
in-memory fixture assertions from physical capture and disk persistence.

The MCP popup-row click did not select a row; keyboard arrows exercised the
actual ComboBox instead. No accessible-value setter was used to manufacture a
selection. A 480x320 RU check showed horizontal overflow; it is recorded as a
finding, not a pass. Its baseline was not captured, so regression status is
unknown. No shared Settings layout repair or additional build was undertaken.

## Included screenshots

All included images were inspected for private content. They show only Audio
controls and synthetic endpoint names, not credentials or archive contents.
The before/after pair uses EN, Glacier, software rendering, scale 1, 720x600,
the same real profile, and top scroll position. Fixture images are supplementary,
not matched baseline pairs.

- [Before](before-audio-en.png) / [after](after-audio-en.png), [bottom scroll](after-audio-en-bottom.png).
- Missing device: [EN](fixture-en-missing.png) / [RU](fixture-ru-missing.png).
- Default: [EN](fixture-en-default.png) / [RU](fixture-ru-default.png); [new devices](fixture-en-selected.png).
- Save failure: [EN](fixture-en-save-error.png) / [RU](fixture-ru-save-error.png).
- Loading: [EN](fixture-en-loading.png) / [RU](fixture-ru-loading.png).
- Enumeration failure: [EN](fixture-en-error.png) / [RU](fixture-ru-error.png).
- Empty list: [EN](fixture-en-empty.png) / [RU](fixture-ru-empty.png).
- Minimum-size finding: [RU top](fixture-ru-min-top.png) / [RU bottom](fixture-ru-min-bottom.png).

No release, installer, merge or infrastructure change was performed. Test
fixtures were closed and the task-owned MCP host/tunnel stopped after capture.

Native tests and live checks run on the worker, never on DSH.
Independent model review is not available through an authorized explicit-Gemini
route in this session; coordinator source review is not independent acceptance.
