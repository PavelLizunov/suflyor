# Defect ledger

Every finding of an audit, review or investigation is recorded here before it is fixed or deferred. Line numbers refer to master 8a38baccc38ec317693be4dde376e958e8f3c242 unless an entry says otherwise.

Rules: a severity mark means the defect was confirmed in source; bold P0 or P1 also means it was demonstrated by a failing test or a reproduced run. A suspicion that was not demonstrated is an `UNVERIFIED IMPORTED HYPOTHESIS` and carries no severity mark. An entry is closed by the pull request that fixed it, after that pull request is merged. Refuted claims are not entries.

Severity: P0 data loss, security exposure or total failure; P1 wrong behaviour in common use; P2 rare cases or degraded behaviour; P3 minor.

## Entries

### D-001 Secret redaction of diagnostics and log exports is bypassed by case and delimiters   Severity: **P1**   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/diagnostics.rs:150 - `if rest.starts_with("Bearer ") {`
Scenario: a log line holding `authorization: bearer <token>`, `Bearer<tab><token>`, `Bearer  <token>` (two spaces), `Bearer: <token>`, an `x-api-key: <value>` header, or an `xai-` or `nvapi-` key is copied into the "Copy report" clipboard text and the "Collect logs" export with the credential intact.
Found by: review of the 14 open Sentinel pull requests at 8a38bac; confirmed by seven failing tests in the Windows CI job at 530881781c552a569444df4e854a21300281e230 (121 passed, 7 failed).
Fix plan: match the label in any ASCII case after any run of spaces, tabs, `:`, `=` or quotes; add the header and the two prefixes; keep a word boundary so the app's own `ai_bearer ...` log lines stay readable.
Closed by: (open; fix proposed in pull request #221)

### D-002 Private files are created readable by every local user on macOS and Linux   Severity: **P2**   Status: Open
Evidence: overlay-backend/src/config.rs:1651 - `std::fs::write(path, bytes).context("write export")?;` (also the server-settings export at :1796, `config.json.bak` at :1611, the session journal at journal/writer.rs:77-81, the Hermes `.env` at hermes_install.rs:96)
Scenario: with the default umask, a full settings export (API keys and bearers), the config backup, every session journal (transcripts, prompts, answers) and the Hermes `.env` (bridge token) get mode 0644, and the sessions directory 0755. Windows is not affected: access there follows the profile directory ACL.
Found by: review of the 8 open Sentinel permission pull requests at 8a38bac; confirmed by four failing tests in the macOS CI job at 719f80c86753b6135ffe71cf6794a6e4f894b25e (modes 0644 and 0755 observed).
Fix plan: one set of helpers that creates such files 0600 and the directory 0700 on Unix.
Closed by: (open; fix proposed in pull request #223)

### D-003 cargo-deny fails on every code change: wasapi 0.23 has an unsound advisory   Severity: **P2**   Status: Open
Evidence: overlay-backend/Cargo.toml:72 - `wasapi = "0.23"` (also suflyor-tts/Cargo.toml:30 and suflyor-teratts/Cargo.toml:30, and three committed lockfiles)
Scenario: RUSTSEC-2026-0332 (`WaveFormat::parse` reads past the end of a `WAVEFORMATEX`, fixed in 0.25.0) makes the `cargo-deny` jobs fail for overlay-backend and suflyor-tts on any pull request that is not docs-only. No call to `WaveFormat::parse` exists in this repository, so the unsound function is not reached by project code; the damage is a red security check that hides real results. Open pull request #219 silences it by adding the advisory to the ignore list.
Found by: the first run of the security workflow on pull request #221; confirmed in the job log and against the advisory text.
Fix plan: bump the three manifests and the three lockfiles to 0.25.0, whose dependency requirements are identical.
Closed by: (open; fix proposed in pull request #222)

### D-004 The Hermes `config.yaml` that receives a generated API key is written with default permissions   Severity: P2   Status: Open
Evidence: overlay-backend/src/hermes_install.rs:145 - `std::fs::write(&cfg_path, text).map_err(|e| format!("запись config.yaml: {e}"))?;`
Scenario: `ensure_api_server` generates an API server key and writes it into the Hermes `config.yaml`; on macOS and Linux a newly created file is 0644. Same class as D-002, not covered by pull request #223 because the file belongs to Hermes and may already exist with a mode its owner chose.
Found by: reading the code next to the `.env` fix at 8a38bac. Not demonstrated by a test.
Fix plan: decide with the owner whether Suflyor should tighten a file that Hermes owns; if yes, reuse the helper from D-002.
Closed by:

### D-005 Two shipped crates are never compiled or dependency-checked for Windows in CI   Severity: P2   Status: Open
Evidence: .github/workflows/ci.yml - the `rust` job names only `overlay-backend`, `slint-experiment` and `suflyor-tts`; .github/workflows/security.yml - `crate: [overlay-backend, slint-experiment, suflyor-tts]`
Scenario: `suflyor-teratts` (a sidecar the installer ships, 4658 lines, uses wasapi on Windows) and `suflyor-wsola` are built and tested only by the advisory macOS job, where the Windows-only code is compiled out. A change that breaks the Windows build of the sidecar, or a new advisory in its lockfile, reaches a release unnoticed until the manual packaging run.
Found by: reading the workflows at 8a38bac.
Fix plan: add both crates to the Windows job and to the cargo-deny matrix. This changes CI and needs the owner's decision.
Closed by:

### D-006 The packaging workflow names its artifact after an old version   Severity: P3   Status: Open
Evidence: .github/workflows/package-windows.yml:45 - `name: suflyor-v0.38.0-windows-${{ github.sha }}`
Scenario: the version in `slint-experiment/Cargo.toml` is 0.38.1-rc.3; an installer built today is uploaded under a 0.38.0 name.
Found by: reading the workflows at 8a38bac.
Fix plan: read the version from `Cargo.toml` in the workflow.
Closed by:

### D-007 A test of `append_bookmark` tests a copy of the code, and the function has no caller   Severity: P3   Status: Open
Evidence: overlay-backend/src/journal/tests.rs:1043 - `fn append_bookmark_creates_file_with_header_then_appends_entries() {` (its body re-implements the append logic inline and never calls `append_bookmark`)
Scenario: the test stays green whatever happens to the function. The function itself has had no caller since the bookmark chip was removed.
Found by: the dead-code scan and a read of the test at 8a38bac.
Fix plan: remove the function and the stand-in test.
Closed by: (open; removal proposed in pull request #225, together with four other unused functions)

### D-008 A tray right click can be dropped forever after one menu fails to report completion
UNVERIFIED IMPORTED HYPOTHESIS. Status: Open
Evidence: slint-experiment/src/tray.rs:180 - `static TRAY_MENU_REQUEST_PENDING: AtomicBool = AtomicBool::new(false);` (set by the first context event, cleared only by `return_focus()` or `hide_icon()`)
Scenario: the styled menu is dismissed only after it gained focus and lost it, on Escape, or on an action. If it is shown but never receives focus, nothing clears the flag and every later right click is ignored until restart. This matches task T6 ("right-clicking the tray icon can produce no menu").
What would confirm it: on the affected installation, the log line `tray menu shown` followed by right clicks that produce no further `tray menu` line.
Found by: reading the code for task T6 at 8a38bac. Not reproduced: no Windows machine was reachable.
Fix plan: tell the twin events of one click apart by time instead of a completion flag.
Closed by: (open; change proposed in pull request #227)

### D-009 A secret stored under a JSON key is not redacted unless it carries a known prefix
UNVERIFIED IMPORTED HYPOTHESIS. Status: Open
Evidence: slint-experiment/src/bin/overlay_host/diagnostics.rs:146 - `pub(crate) fn redact_secrets(s: &str) -> String {` (matches `Bearer`, `gsk_` and `sk-` only)
Scenario: if any log line ever contains serialized configuration such as `"ai_bearer":"<value>"` with a token that does not start with `sk-` or `gsk_`, the value passes through. Pull request #221 does not change this: it leaves labels inside identifiers alone on purpose.
What would confirm it: a log line produced by the app that contains a config key with its value.
Found by: design of the fix for D-001. No such log line was found by grep of the log calls.
Fix plan: none until confirmed.
Closed by:

### D-010 `cargo fmt --check` never reads the host modules   Severity: P3   Status: Open
Evidence: slint-experiment/src/bin/overlay_host.rs:19 - `include!("overlay_host_windows.rs");`
Scenario: rustfmt follows `mod` declarations but not `include!`. `overlay_host_windows.rs` and the 40 files under `src/bin/overlay_host/` (28134 lines, most of the host) are declared inside the included file, so the format check of the required CI job passes whatever their layout is. Formatting drift there is never reported.
Found by: preparing changes in `diagnostics.rs` at 8a38bac; confirmed on windows-worker at 2d55575248c5ea1c7e3008554325b98edbf13002: `cargo fmt --manifest-path slint-experiment\Cargo.toml --all -- --check -v` exits 0 and lists 155 files, among them `src\bin\overlay_host.rs` and none under `src\bin\overlay_host\`.
Fix plan: pass those files to rustfmt explicitly in the gate, or replace the include with a module declaration. The first run will report existing drift, so it needs its own formatting-only pull request.
Closed by:

### D-011 The sherpa-onnx patch bump does not compile   Severity: P3   Status: Open
Evidence: pull request #199 (dependabot, sherpa-onnx 1.13.5 to 1.13.8), Windows CI job - `error[E0063]: missing field window_shift_ratio in initializer of sherpa_onnx::OfflineSpeakerSegmentationPyannoteModelConfig` and `missing field compute_confidence in initializer of sherpa_onnx::FastClusteringConfig`
Scenario: the dependency cannot be updated without a code change in the diarization setup of `suflyor-tts`; the automated pull request stays red.
Found by: the dependabot report at 8a38bac.
Fix plan: add the two fields with their upstream defaults in a pull request that also carries the bump, then listen to read-aloud and run a diarization.
Closed by:

### D-012 Three tests are compiled out on every platform that CI runs   Severity: P3   Status: Open
Evidence: overlay-backend/src/stt.rs:1618 - `#[cfg(not(any(windows, target_os = "macos")))]` on `gigaam_shim_load_unsupported_off_windows`, and the same attribute on `validate_gigaam_dir_unsupported_off_windows` and `configure_gigaam_accelerator_honest_noop_off_windows`
Scenario: CI tests on Windows and macOS only. These three tests never run, so the "unsupported platform" fallbacks they describe are untested in practice.
Found by: a script count of test attributes at 8a38bac (1241 test functions; the 21 Windows-only, 9 macOS-only and 3 Unix-only ones do have a job that runs them).
Fix plan: decide whether the off-platform fallbacks are still wanted (Linux is not a product target); if not, remove them with their tests.
Closed by:

### D-013 The targeted gate cannot run backend tests on the Windows worker   Severity: P2   Status: Open
Evidence: scripts/git-gate-native.ps1:176 - `& $cargo test --manifest-path $manifest` (no step stages `DirectML.dll`; scripts/ci.ps1:102-113 has the step "stage DirectML for backend tests")
Scenario: on Winbrat (Windows 10 Enterprise LTSC, build 17763) the system `DirectML.dll` is version 10.0.17763 and lacks an export that the `ort` build (DirectML 1.15.4) needs. The Full gate copies the matching DLL next to the test executables; the targeted gate, which `AGENTS.md` prescribes for every normal change, does not. `scripts\git-gate-native.ps1 push` then fails at "overlay-backend test": the test executable exits with 0xc0000138 (STATUS_ORDINAL_NOT_FOUND) before a single test runs. GitHub's Windows image has a newer system DLL, so CI does not show it.
Found by: the first targeted gate run on windows-worker at 2d55575248c5ea1c7e3008554325b98edbf13002 on 2026-10-08 (exit 1 after 2.1 minutes; the same at five other commits).
Fix plan: move the staging step of ci.ps1 into a function both gates call, for overlay-backend and slint-experiment.
Closed by:
