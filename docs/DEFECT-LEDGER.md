# Defect ledger

Every finding of an audit, review or investigation is recorded here before it is fixed or deferred. Line numbers refer to master 8a38baccc38ec317693be4dde376e958e8f3c242 unless an entry says otherwise.

Rules: a severity mark means the defect was confirmed in source; bold P0 or P1 also means it was demonstrated by a failing test or a reproduced run. A suspicion that was not demonstrated is an `UNVERIFIED IMPORTED HYPOTHESIS` and carries no severity mark. An entry is closed by the pull request that fixed it, after that pull request is merged. Refuted claims are not entries.

Severity: P0 data loss, security exposure or total failure; P1 wrong behaviour in common use; P2 rare cases or degraded behaviour; P3 minor.

## Entries

### D-081 Linux installer omits compositor rules and singleton activation   Severity: P2   Status: Open
Evidence: source review of install-linux.sh shows no Hyprland snippet deployment/source, despite D-073 proving the first bar tiles without rules. Russian tiles translate their title to `Тайл` but the tile-specific rule matches only `tile`. The desktop entry directly launches a second process, which exits at the singleton lock instead of restoring the existing hidden bar. Custom XDG_CONFIG_HOME is not recorded in the service, so its IPC identity can differ from the launcher.
Fix: install the checked rules/icon/native launch wrapper, source only the task-owned rules from the existing user Hyprland config, persist the same config identity for service and launcher, and activate the existing process before starting a new service. Keep uninstall limited to owned files/block and preserve configuration/models. Verify native install, mapped geometry, second launch, autostart and uninstall. The tested shared target directory contains all binaries; no missing shared library is established by current ldd output.
Closed by:

### D-080 Native self-capture retains black exclusion rectangles   Severity: P2   Status: Open
Evidence: actual Hyprland 0.56.2 grim before/after captures of a disposable native window show `no_screen_share=1` replaces window contents with a black rectangle, including when all opacity properties are zero. Thus setting opacity alone cannot remove Suflyor rectangles from its own OCR/Vision image while exclusion is enabled. Captures were inspected; this is compositor behavior, not transparent desktop recovery.
Fix: while own surfaces are fully transparent, temporarily suspend their exclusion for the owned screenshot, then restore exclusion before restoring saved opacity. Keep external capture content protected and describe black-rectangle behavior accurately. Verify actual screenshot pixels and failure/cancel restoration.
Closed by:

### D-079 Hyprland Lua shortcut ownership is lost on application restart   Severity: P2   Status: Open
Evidence: native Hyprland 0.56.2 probe registers a described Lua bind and reads `dispatcher=__lua`, `arg=13`, with the description preserved. The adapter instead recognizes ownership by searching `arg` for the command, so after a restart it treats its own keys as foreign conflicts. Action commands also stop the drain after one datagram, delaying rapid PTT release. Source review found temporary identity titles left visible after compositor resolution.
Fix: mark bindings with a stable per-config description, compare that description and remove only owned bindings on teardown. Windows native 700ce89 tray_guard catches the IPC hide path bypassing the guarded explicit bar callback; route that command through the existing callback, retaining the guard assertion. Continue the bounded input drain for action commands, and restore original titles after identity resolves. Verify real relaunch and PTT press/release. Reviewer's claim that opacity_inactive and opacity_fullscreen do not exist was refuted by actual getprop values; preserve all three opacity values.
Closed by:

### D-078 Linux system capture cannot resolve an input offered by Settings   Severity: P2   Status: Open
Evidence: Linux Settings intentionally offers output sinks plus capture inputs, matching the shared backend contract. `audio_linux::resolve(System, selected)` enumerates only sinks, so selecting an input or headset mix fails as unavailable despite appearing selectable. Source-confirmed during full-port review; live pinned-input regression remains required.
Fix: resolve a pinned system choice against sink names first and then input sources without silently falling back to a default. Preserve ambiguity rejection and route identity. Exercise a real PipeWire input choice and cancellation. Native 3633059 managed GigaAM/speech test passes for both streams and recorded WAVs, but a 500 ms pinned-input probe returns empty PCM because finite duration starts before PipeWire opens the stream. Start finite recording time at first PCM, bound startup separately, and preserve immediate stop cancellation; retain the failing live assertion.
Closed by:

### D-077 Linux ONNX runtime aborts before main on a virtual CPU without AVX   Severity: P2   Status: Open
Evidence: native candidate 697ceeb builds, but the backend test driver exits 101 with SIGILL before any test starts. The core identifies protobuf static initialization inside the linked ONNX runtime; disassembly shows `vxorps` at the faulting instruction. The dedicated VM exposes x86-64-v2-AES without AVX, while its physical Intel N150 supports AVX/AVX2. This is a native runtime requirement, not a failing asset-picker assertion or memory exhaustion.
Fix: expose the physical worker's supported x86-64-v3 CPU baseline to the dedicated test VM, verify the same executable starts and completes through Cargo, and enforce/document the runtime CPU requirement for Linux installation. Verification: the same 697ceeb Cargo driver now exits 0: nine asset-picker, one installer rejection, 12 audio/session, one console guard, 21 Piper and 74 Tera tests pass (118 total, two live audio tests ignored). Real inference and playback are separate acceptance paths. Keep unsupported hardware from silently appearing supported. No owner desktop configuration changes.
Closed by:

### D-076 In-flight Linux native adapters need lifecycle and compositor correctness fixes   Severity: P3   Status: Open
Evidence: source review of the development candidates finds interactive slurp running synchronously on the Slint thread, a fresh generated title on every native identity miss, incorrect `dispatch setprop` spelling, IPC paths depending on migrating data roots, no PTT release action, and CPAL render-start failures reaching a completion callback without a failure outcome. The first native Linux compile also rejects an ambiguous window-geometry error type at 210cb31. A reviewer claim that Hyprland 0.56.2 lacks Lua/eval was refuted by actual installed-worker `hyprctl eval` bind/unbind success; no downgrade to obsolete syntax is warranted. The short-lived-thread PDEATHSIG concern applies to persistent local AI servers, not finite audio children owned until capture returns.
Fix: keep a stable mapped compositor identity, use the installed CLI contract, resolve IPC from stable config identity, dispatch capture asynchronously, own press/release actions, propagate actual playback failures and keep persistent process ownership on a long-lived spawn thread. Source audit also found Windows-only gates disconnecting Linux setup/audio/local-model/Strata/update settings; extend the relevant modules and all their helpers together. Native macOS and CI 39620875878 at 700ce89 reproduce E0425 after a cleanup incorrectly narrows whisper_cublas_version_of to Windows/test even though the cfg!-based macOS picker also compiles its fallback call. Match the helper cfg to its actual non-Linux caller and rerun the native gate. Native CI at ccf521a stops on formatting before compilation; apply native rustfmt and rerun, rather than counting that as a platform compile failure. Validate with exact candidate builds and real compositor/audio consumers; these are development candidates, not deployed full-port acceptance.
Closed by:

### D-075 macOS window guard assumes macOS-only spelling for a shared non-Windows branch   Severity: P3   Status: Open
Evidence: PR #307 CI 39572451692 macOS job fails at native_macos_window_guard.rs:78. At 6569979, CI 39611578690 compiles/runs Windows tests, then two macos_settings_guard assertions still demand Windows-only setup and pre-Linux STT cfg spelling. The macOS job was cancelled after the Windows failure, not proven failed in compilation. Native ce89dee later reaches macos_app_packaging_guard and finds the same stale Windows-only cfg assertion for the updater; update the expected shared Windows/Linux scope while retaining macOS exclusion. Native 7fc843c then reaches macos_global_hotkeys_guard, whose receiver assertion still points to the old inline host location; assert the host-to-next_event-to-platform-receiver chain rather than remove polling coverage. The production lock-menu scale branch was correctly extended to all non-Windows targets, but the source-shape guard still requires the former macOS-only cfg text. The native AppKit, placement and scaling assertions remain relevant and must stay.
Fix: align only the cfg spelling assertion with the shared non-Windows branch; retain the actual macOS bridge and geometry invariants and rerun native guards.
Closed by:

### D-074 Linux test driver rebuilds unrelated demonstration binaries   Severity: P3   Status: Open
Evidence: Linux drivers at 2bd39ad and 0eb2a9e were interrupted while Cargo built additional large binary targets. A minimal reproduction on the same Cargo 1.98.1 worker confirms that `cargo test --lib --bin overlay-host --test linux_platform` and `cargo rustc --test linux_platform` still build unrelated binaries for integration tests. Serialization alone does not restrict that target set.
Fix: put the three non-product binaries behind a default-enabled `demo-binaries` feature, preserving normal developer/Windows behavior. Disable package defaults only in the Linux driver and explicitly select library, overlay-host and linux_platform tests. Keep one build job for tests and run the relevant backend capability, installer and update regressions. Require a complete exact-SHA driver exit, not direct executable results.
Verification: candidate 68ed25134c11f4b57ac9f0242f64a75c963b3e1b, tree 99f2a9e50994230b4e694e9854675a7079077ddf: complete scoped Linux driver exit 0, 236 passed / three ignored across six groups; Windows targeted gate exit 0, 1,109 passed / five ignored across 23 groups. Default-enabled demo gating preserves ordinary builds; a native negative fixture verifies the target-selection cause. Remains open until integration.
Closed by:

### D-073 First Wayland runtime tiles the bar and probes unsupported startup capture   Severity: P3   Status: Open
Evidence: exact 143a21ce5fd9f9b736833c19041a3074f04c5a79 builds and starts as a native Wayland client (`xwayland=false`). Hyprland reports the bar tiled at 1896x1030 instead of its preferred 1280x64; opening Settings splits it to 941x1030. The Windows off-screen capture-overlay precreation maps an unnecessary 1920x1080 surface, and automatic system-audio startup logs an unsupported capture error.
Fix: give Linux windows a stable application identity and disable compositor tiling through winit's fixed-size window contract; skip Windows/macOS capture precreation and startup audio probing on Linux. Rebuild and inspect actual native windows before claiming acceptance.
Closed by:

### D-072 Linux backend has missing platform return branches   Severity: P3   Status: Open
Evidence: fresh `cargo check --manifest-path overlay-backend/Cargo.toml` on omarchy-test at 8c70b6d193b3a4c84e8e20a8cc56b0681f60618c exits 101 with four errors: platform_capability, detect_gpu, detected_hardware_model_profile and pick_llama lack non-Windows/non-macOS return paths. This reproduces Claude's earlier scoped probe; Linux is not yet a supported product target.
Fix: add explicit unsupported capability/installer results and conservative unknown hardware fallbacks, retaining existing Windows/macOS behavior. Verify the backend on Linux and Windows against the exact candidate.
Closed by:

### D-071 New TLS trust-anchor advisories block the post-release checks   Severity: P2   Status: Closed
Evidence: security run 39500531432 rejects slint-experiment webpki-root-certs 1.0.7 (RUSTSEC-2026-0383) and suflyor-tts rustls-webpki 0.103.15 / webpki-roots 1.0.8 (RUSTSEC-2026-0382/0381). These advisories appeared after the stable release gate. No attack or runtime compromise was observed.
Fix: update only those locked packages to webpki-root-certs 1.0.10, rustls-webpki 0.103.16 and webpki-roots 1.0.10. The remaining webpki-roots 0.26.11 is a compatibility shim whose entire implementation re-exports `parent::TLS_SERVER_ROOTS` from patched v1, not an old certificate store. Except only that shim: the installed cargo-deny does not support consumer-scoped advisory exceptions, so pair the documented advisory exception with explicit bans on all webpki-roots versions except 0.26.11 and >=1.0.10. An affected v1 store or pre-shim 0.26 release must still fail. Exercise both allowed versions and both rejected intervals with real cargo-deny fixtures. Verify the native targeted gate and exact-candidate security jobs. Published v0.38.1 assets are not silently replaced by a source fix.
Closed by: PR #304, merge 457c92b135a8a33e1c250c9b43f3dc6e2dc9fe1a; exact candidate 272f395ce048feba0bf01f2d584d32d3441c2ace passed the Windows native targeted gate (exit 0), CI 39505366383 (Windows/macOS) and security 39505366378 (all three crates). Real registry fixtures accepted 0.26.11/1.0.10 and rejected 0.26.10/1.0.9.

### D-070 Release documentation workflow opens reviews for an unchanged README   Severity: P3   Status: Closed
Evidence: run 39441520345 on v0.38.1 reports `nothing to commit, working tree clean` but pushes an identical automation branch and opens #301 after PR creation is denied. `if (git diff --quiet ...)` tests empty stdout rather than the native exit code in PowerShell. Earlier #197, #257 and #292 are the same fallback review family. The current README already names v0.38.1.
Fix: interpret exit 0 as no change, 1 as a diff and any other exit as failure; test the actual workflow block. Keep workflow permissions explicit and repair the repository PR-creation setting without broadening default token permissions. Re-run the live no-change consumer before closing superseded review issues.
Closed by: PR #304, merge 457c92b135a8a33e1c250c9b43f3dc6e2dc9fe1a. Live master workflow 39513282128 passed all three guard fixtures, preserved the already-current v0.38.1 README and stopped before branch/commit/PR/issue mutations. Superseded issues #301, #292, #257 and #197 closed after that replay. README local-AI and CONTRIBUTING gate/MCP/release facts were corrected; SECURITY was reviewed without a factual change. Showcase images remain illustrative, not exact-v0.38.1 acceptance evidence; current native captures are recorded in docs/release-evidence-v0.38.1/README.md.

### D-069 Stable CI rejects late initialization in local-engine switch   Severity: P3   Status: Closed
Evidence: full GitHub Windows run 39428900019 at d5968aa fails Clippy 1.99 `needless_late_init` at settings_local_ai.rs:2561. Both branches assign the final status message.
Fix: bind the message to the existing if-expression without changing branch effects or copy. Verify full CI and the native worker on the final candidate.
Verified: final candidate 77708af4caf3ba7d4413097c4221cf5075de4557 passes the full native Windows/macOS gates and full GitHub run 39431248172.
Closed by: #300, stable v0.38.1, merge 84249fdcb967ec6415a0d62863b71dff880d2763.

### D-068 Full bar geometry test uses obsolete Russian control labels   Severity: P3   Status: Closed
Evidence: Windows geometry replay at dfca6f2 no longer reaches the old memory overflow; it fails finding `Скриншот в vision`. Current bundled PO translates `Screenshot to vision` as `Скриншот области → ответ AI`.
The next replay at e0240d4 also exposed the obsolete expand label `Развернуть бар`; the bundled label is `Развернуть панель`. All English/Russian fixture labels were compared with the current PO.
Fix: match the current bundled accessibility labels in the regression fixture; retain every geometry assertion unchanged.
Verified: final candidate 77708af4caf3ba7d4413097c4221cf5075de4557 passes the full native Windows/macOS gates and full GitHub run 39431248172.
Closed by: #300, stable v0.38.1, merge 84249fdcb967ec6415a0d62863b71dff880d2763.

### D-067 Cancelled macOS push-to-talk still starts native capture   Severity: P2   Status: Closed
Evidence: physical macOS full gate at dfca6f2 hangs in `record_source_until_stop_supported_shape` with stop already true. Native process sample traces `record_source_until_stop` into `mic_capture_start` and the permission/start path before cancellation is inspected. The source unconditionally opens microphone/system capture before its stop loop.
Fix: return an empty recording before native startup when stop is already set. Strengthen the existing supported-shape test to assert this for both sources. No permission reset or grant is needed.
Verified: final candidate 77708af4caf3ba7d4413097c4221cf5075de4557 passes the full native Windows/macOS gates and full GitHub run 39431248172.
Closed by: #300, stable v0.38.1, merge 84249fdcb967ec6415a0d62863b71dff880d2763.

### D-066 Windows-only Strata setup constant fails macOS full Clippy   Severity: P2   Status: Closed
Evidence: exact candidate 21f06cb8f2954f1c131df95f9aef6c456935e866, physical macOS full gate exits 101: `STRATA_FAMILY` is never used. Its only consumer is the Windows-only `run_setup`.
Fix: compile the constant only on Windows, matching its consumer. Verify full macOS and Windows gates on the final candidate.
Verified: final candidate 77708af4caf3ba7d4413097c4221cf5075de4557 passes the full native Windows/macOS gates and full GitHub run 39431248172.
Closed by: #300, stable v0.38.1, merge 84249fdcb967ec6415a0d62863b71dff880d2763.

### D-065 Memory telemetry overflows the fixed-height full bar footer   Severity: P2   Status: Closed
Evidence: prior Windows full-suite failure in overlay_bar_geometry::open_tile_controls_stay_inside_1280_in_english_and_russian: app memory y=56,height=12 exceeds the64px window. Current overlay_bar.slint places model and memory readouts in a two-line VerticalLayout; action row,padding and spacing leave only14px of footer height. Stable candidate 21f06cb full-suite replay reproduced this exact failure. Final EN/RU regression passes without weakened assertions; native idle bar captures inspected. Memory-visible permutations are headless, not real Windows telemetry.
Fix: keep model and memory on one horizontal footer line without changing native1280x64/680x64 window sizes or weakening geometry expectations. Verify the existing EN/RU memory permutations and native bar presentation.
Verified: final candidate 77708af4caf3ba7d4413097c4221cf5075de4557 passes the full native Windows/macOS gates and full GitHub run 39431248172.
Closed by: #300, stable v0.38.1, merge 84249fdcb967ec6415a0d62863b71dff880d2763.

### D-064 Strata Vision download/activation copy omits prerequisites and conflates cancellation   Severity: P3   Status: Closed
Evidence: candidate d91dae4, the downloaded status asks to enable F8 while its button is disabled when Strata is not the active local text model. The activation-cancel result shares the download-cancel wording and asks to download again. Confirmed by source review; successful real activation remains unverified.
Verified: candidate 91a2fbf shows the text-route prerequisite in EN/RU downloaded captions; native cancellation during activation reports activation cancellation and preserves F8. Selected native gate passed. Real image inference remains an explicit experimental limitation, not implied by this copy repair.
Closed by: #298, published v0.38.1-rc.15, merge b041f6462ba99f3785e9a2b4c4a0818fa49232ec.

### D-063 Deferred native presentation can re-show a retained window closed before reveal
UNVERIFIED IMPORTED HYPOTHESIS. Status: Open
Evidence: Settings now retains its instance after close; present_window_stealth_aware_at defers native decoration/reveal without testing Slint visibility. set_skip_taskbar can call SW_SHOWNOACTIVATE. Tile native setup has the same delayed visibility gap. Settings close omits the existing force_hide fallback used by tile close.
Scenario: an early close can leave a native frame after the Slint content is hidden; multi-monitor compositor residue is reported by the owner. The exact owner ghost has not been reproduced on the single-monitor software-rendered worker. Fast-empty auto-tile baseline did fully remove the placeholder.
Applied by #295: hidden-window guards,Settings native force-hide and recovery of an early-cancelled parked reveal. Native Settings close/reopen and download-state regression pass. Exact multi-monitor compositor symptom and the before33ms sequence remain unverified; this hypothesis stays open.
Closed by:

### D-062 Buffered Responses answers are discarded as empty and close auto tiles   Severity: **P1**   Status: Closed
Evidence: native Windows c39142d QA binary, loopback Responses SSE sends response.created and response.completed containing output text, without delta events. Log records reason=stop deltas=0; the placeholder appears then disappears. The completed response contains a synthetic non-empty answer.
Scenario: a bridge buffers successful Responses output; delta-streamed refusal text remains visible while successful buffered answers disappear. This reproduces the owner symptom with a fixture, not a trace of the owner endpoint.
Verified: native EN/RU buffered and normal streamed answers at564d9a739709b579875abd757e373eea7383334d remain during10s observation and render exactly once; explicit close removes them. Empty response leaves count0. Backend gate passes completed-only,streamed,empty and incomplete fixtures. Owner endpoint wire trace remains unverified.
Closed by: #295, published v0.38.1-rc.14, merge92438f4eb9bc7e0957dc5128706a0df729009de0.

### D-061 OpenAI provider block overlaps the download card after text wrapping   Severity: **P1**   Status: Closed
Evidence: native Windows MCP at the rc.13 source tree, RU 720x600: OpenAI conditional VerticalLayout y144/h296, test button y431/h32, next card y448. The next card overlaps the button by15px.
Scenario: AI providers with OpenAI selected; its reasoning help wraps to three lines but the conditional group reports a shorter preferred height.
Verified: existing WrapBlock restores8px separation in native EN/RU720px captures; selected native gate includes the passing720/600/480px regression.
Closed by: #295, published v0.38.1-rc.14, merge92438f4eb9bc7e0957dc5128706a0df729009de0.

### D-056 Closing Settings disconnects an active local-model download from its UI   Severity: **P1**   Status: Open
Evidence: at 3b00939d3927e9baa58d58e42b5453e63f7dec21, `settings_controller.rs` removes the Settings instance in `on_close_clicked`; local-AI progress/completion callbacks hold only that instance's weak handle.
Scenario: start the built-in download, close Settings, then reopen it. The transfer continues but progress and Cancel disappear; the new window presents idle download actions.
Demonstrated on the Windows worker: the first Settings handle showed Gemma 298 / 6405 MB and Cancel; after close/reopen the handle changed and Download model returned while the original curl process remained active. Private captures are retained outside Git.
Fix proposed: retain the hidden Settings instance and preserve all job-source/shared status text during active work; idle reopen still clears completed transient results. Candidate branch: codex/rc13-checkpoint. Required acceptance: exact-candidate close/reopen/cancel and resumed full download, plus idle reset.
Closed by:

### D-057 Download-only completion rewrites an inherited local Vision route   Severity: P1   Status: Open
Evidence: `apply_download_result` at a5abc4a replaces local answer URL/model; `Config::vision_endpoint` inherits those fields for local Vision when specific fields are blank.
Scenario: cloud answers plus an external local Vision route silently switch screenshots to the managed server after an asset-only download, although that server was not started.
Fix proposed: discover downloaded models from disk; preserve saved endpoints/model choices until explicit successful activation. Regression compares the resolved Vision endpoint before and after application.
Closed by:

### D-058 Cloud boot may load downloaded weights through automatic engine maintenance   Severity: P1   Status: Open
Evidence: the boot engine-update condition at a5abc4a checks deep lock, Strata and throttle, but not the active answer provider; an update verifies its engine by launching installed weights.
Scenario: downloading ahead of time while using cloud answers can cause a later boot update to load a local model without Use.
Fix proposed: run model-loading boot maintenance only for the active managed llama.cpp answer route.
Closed by:

### D-059 CPU-first partial releases can displace complete accelerated builds   Severity: P2   Status: Open
Evidence: release discovery accepts the first release for which the GPU picker can fall back to CPU; equal build tags suppress a later update after accelerated assets finish uploading.
Scenario: the newest release has only Windows CPU assets while the preceding release has the complete matching CUDA or Vulkan bundle.
Fix proposed: prefer a complete hardware-matching release before CPU fallback; keep fallback when none of the recent releases supports that GPU. Offline fixtures cover NVIDIA, Vulkan and CPU.
Closed by:

### D-060 Cloud download-card Use selects a saved endpoint instead of activating its model   Severity: P1   Status: Open
Evidence: the new footer at a5abc4a invokes only provider selection, unlike the existing built-in activation callback. Switching the provider before async activation can also replace cloud answers when activation fails.
Scenario: a saved external local endpoint wins over the downloaded model; Now answering stays stale. A ready fallback may otherwise be mistaken for success of the requested model.
Fix proposed: every card Use action invokes built-in activation; switch cloud answers only after the requested choice succeeds. Existing local fallback must persist the model actually serving.
Closed by:

### D-001 Secret redaction of diagnostics and log exports is bypassed by case and delimiters   Severity: **P1**   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/diagnostics.rs:150 - `if rest.starts_with("Bearer ") {`
Scenario: a log line holding `authorization: bearer <token>`, `Bearer<tab><token>`, `Bearer  <token>` (two spaces), `Bearer: <token>`, an `x-api-key: <value>` header, or an `xai-` or `nvapi-` key is copied into the "Copy report" clipboard text and the "Collect logs" export with the credential intact.
Found by: review of the 14 open Sentinel pull requests at 8a38bac; confirmed by seven failing tests in the Windows CI job at 530881781c552a569444df4e854a21300281e230 (121 passed, 7 failed).
Fix plan: match the label in any ASCII case after any run of spaces, tabs, `:`, `=` or quotes; add the header and the two prefixes; keep a word boundary so the app's own `ai_bearer ...` log lines stay readable.
Closed by: #221, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-002 Private files are created readable by every local user on macOS and Linux   Severity: **P2**   Status: Closed
Evidence: overlay-backend/src/config.rs:1651 - `std::fs::write(path, bytes).context("write export")?;` (also the server-settings export at :1796, `config.json.bak` at :1611, the session journal at journal/writer.rs:77-81, the Hermes `.env` at hermes_install.rs:96)
Scenario: with the default umask, a full settings export (API keys and bearers), the config backup, every session journal (transcripts, prompts, answers) and the Hermes `.env` (bridge token) get mode 0644, and the sessions directory 0755. Windows is not affected: access there follows the profile directory ACL.
Found by: review of the 8 open Sentinel permission pull requests at 8a38bac; confirmed by four failing tests in the macOS CI job at 719f80c86753b6135ffe71cf6794a6e4f894b25e (modes 0644 and 0755 observed).
Fix plan: one set of helpers that creates such files 0600 and the directory 0700 on Unix.
Closed by: #223, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-003 cargo-deny fails on every code change: wasapi 0.23 has an unsound advisory   Severity: **P2**   Status: Closed
Evidence: overlay-backend/Cargo.toml:72 - `wasapi = "0.23"` (also suflyor-tts/Cargo.toml:30 and suflyor-teratts/Cargo.toml:30, and three committed lockfiles)
Scenario: RUSTSEC-2026-0332 (`WaveFormat::parse` reads past the end of a `WAVEFORMATEX`, fixed in 0.25.0) makes the `cargo-deny` jobs fail for overlay-backend and suflyor-tts on any pull request that is not docs-only. No call to `WaveFormat::parse` exists in this repository, so the unsound function is not reached by project code; the damage is a red security check that hides real results. Open pull request #219 silences it by adding the advisory to the ignore list.
Found by: the first run of the security workflow on pull request #221; confirmed in the job log and against the advisory text.
Fix plan: bump the three manifests and the three lockfiles to 0.25.0, whose dependency requirements are identical.
Closed by: #222, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

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

### D-006 The packaging workflow names its artifact after an old version   Severity: P3   Status: Closed
Evidence: .github/workflows/package-windows.yml:45 - `name: suflyor-v0.38.0-windows-${{ github.sha }}`
Scenario: the version in `slint-experiment/Cargo.toml` is 0.38.1-rc.3; an installer built today is uploaded under a 0.38.0 name.
Found by: reading the workflows at 8a38bac.
Fix plan: read the version from `Cargo.toml` in the workflow.
Fix proposed: pull request #232 (commit 041bb97): the workflow reads the version from `slint-experiment/Cargo.toml`. The workflow is manual and was not dispatched.
Closed by: #232, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-007 A test of `append_bookmark` tests a copy of the code, and the function has no caller   Severity: P3   Status: Closed
Evidence: overlay-backend/src/journal/tests.rs:1043 - `fn append_bookmark_creates_file_with_header_then_appends_entries() {` (its body re-implements the append logic inline and never calls `append_bookmark`)
Scenario: the test stays green whatever happens to the function. The function itself has had no caller since the bookmark chip was removed.
Found by: the dead-code scan and a read of the test at 8a38bac.
Fix plan: remove the function and the stand-in test.
Closed by: #225, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-008 A tray right click can be dropped forever after one menu fails to report completion
UNVERIFIED IMPORTED HYPOTHESIS. Status: Open
Evidence: slint-experiment/src/tray.rs:180 - `static TRAY_MENU_REQUEST_PENDING: AtomicBool = AtomicBool::new(false);` (set by the first context event, cleared only by `return_focus()` or `hide_icon()`)
Scenario: the styled menu is dismissed only after it gained focus and lost it, on Escape, or on an action. If it is shown but never receives focus, nothing clears the flag and every later right click is ignored until restart. This matches task T6 ("right-clicking the tray icon can produce no menu").
What would confirm it: on the affected installation, the log line `tray menu shown` followed by right clicks that produce no further `tray menu` line.
Found by: reading the code for task T6 at 8a38bac. Not reproduced on windows-worker on 2026-10-09: with the tray callback messages injected as Explorer sends them (twin events, a second click with the menu still open, five more clicks), the build of master plus the wasapi bump (2d55575) and the build with the change (8cf4904) both opened seven menus for seven clicks. The stuck state needs a menu that never receives focus, which did not happen there.
Fix plan: tell the twin events of one click apart by time instead of a completion flag.
Status note 2026-10-10: the change of #227 is on master through #239. The entry stays open: the reported stuck state was never reproduced, so the fix is not confirmed.
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
Measured on windows-worker at 0e16f620ce433f5c4e0c04b4e37a33715b2f1900 on 2026-10-10: `rustfmt --edition 2021 --check` on each of the 41 files reports that 17 would change (list in the campaign evidence, `d010-files.txt`).
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

### D-013 The targeted gate cannot run backend tests on the Windows worker   Severity: P2   Status: Closed
Evidence: scripts/git-gate-native.ps1:178 - `& $cargo test --manifest-path $manifest` (no step stages `DirectML.dll`; scripts/ci.ps1:102-113 has the step "stage DirectML for backend tests")
Scenario: on Winbrat (Windows 10 Enterprise LTSC, build 17763) the system `DirectML.dll` is version 10.0.17763 and lacks an export that the `ort` build (DirectML 1.15.4) needs. The Full gate copies the matching DLL next to the test executables; the targeted gate, which `AGENTS.md` prescribes for every normal change, does not. `scripts\git-gate-native.ps1 push` then fails at "overlay-backend test": the test executable exits with 0xc0000138 (STATUS_ORDINAL_NOT_FOUND) before a single test runs. GitHub's Windows image has a newer system DLL, so CI does not show it.
Found by: the first targeted gate run on windows-worker at 2d55575248c5ea1c7e3008554325b98edbf13002 on 2026-10-08 (exit 1 after 2.1 minutes; the same at five other commits).
Fix plan: move the staging step of ci.ps1 into a function both gates call, for overlay-backend and slint-experiment.
Fix proposed: pull request #234 (commit 0e16f620ce433f5c4e0c04b4e37a33715b2f1900). Verified on windows-worker at that commit on 2026-10-10: `scripts\git-gate-native.ps1 push -Base origin/master` now runs the backend tests (726 passed, 1 failed: the test of D-014) where it exited with 0xc0000138 before.
Closed by: #234, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-014 A backend test passes alone and fails inside the full suite on the Windows worker   Severity: P3   Status: Open
Evidence: overlay-backend/src/ai/tests.rs:75 - `"a queued stream kept a permit after its receiver was dropped"` (test `ai::tests::queued_stream_stops_when_receiver_is_dropped`; it takes two permits of the process-wide `AI_SEMAPHORE` and waits at most five seconds to get them back)
Scenario: on windows-worker, `cargo test --manifest-path overlay-backend\Cargo.toml` with four test threads fails this one test on every run (six runs at three commits, 726 or 724 others passing), while the same test alone with one thread passes five times out of five. GitHub CI passes it. The semaphore is shared by every test in the binary, so the outcome depends on what else runs beside it and how fast the machine is. The code under test may be correct; the test cannot tell.
Found by: the first backend test run on windows-worker at 2d55575248c5ea1c7e3008554325b98edbf13002 on 2026-10-08. Not investigated further.
Fix plan: give the test its own semaphore, or hold the lock that serialises the tests which use the shared one.
Investigated on windows-worker at 0e16f620 on 2026-10-10: the lib suite with four test threads fails this test in five runs out of five (726 passed, about 11 seconds); with one test thread 727 pass; the test passes alone, with the whole `ai` module, and paired with each of the other 43 test modules one at a time. No single module is the cause.
Fix proposed: pull request #237 (commit a67564b1e59e6c43a8e5445d9458893e42f94060): the deadline of the test becomes thirty seconds. Verified on windows-worker at that commit: five runs of the lib suite with four test threads, 727 passed each time. The test still shares the semaphore with its neighbours.
Status note 2026-10-10: #237 is on master through #239 and the test passes on windows-worker. The entry stays open: the test still shares the process-wide semaphore with its neighbours.
Closed by:

### D-015 Russian text appears in the English interface   Severity: P3   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/settings_hermes.rs:38 - `"выключен".to_string()` (Settings, Hermes: "Status: выключен"); slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:1256 - `"Микрофон"` (speaker labels "Микрофон" and "Система" in the transcript window); overlay-backend/src/journal/time.rs:29 - the `(МСК)` suffix of every session date in the archive
Scenario: with the interface language set to English these strings stay Russian. `AGENTS.md` calls a hardcoded Cyrillic literal without `@tr()` a bug; these three are built in Rust, where the `.slint` i18n guard does not look.
Found by: the Slint MCP audit on windows-worker on 2026-10-09 (builds of 8cf4904 and 77ad476, English interface): element labels read through MCP and screenshots.
Fix plan: route the three strings through the translation table the Rust side already uses for other labels.
Fix proposed: pull request #236 (commit 7262b9f993b11a859e78da151d27ee6fe13212ed) for the three places named above and the bridge error texts. Verified live on windows-worker at that commit in both languages ("Status: off", "Microphone", "System", "(MSK)"; Russian unchanged). Still Russian after it: `"Удаление не удалось (файл занят?) — повторите"` in aux_windows/archive.rs, and the class is wider (a script counts 829 lines with Cyrillic in the host Rust sources, comments included).
Closed by: #236 (the places named in the entry; the wider class stays open as noted), merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-016 Settings shows a hard-coded knowledge-base size that differs from the parser's count   Severity: P3   Status: Closed
Evidence: slint-experiment/ui/settings_panel.slint:1997 - `Embedded knowledge base — 1,696 entries`
Scenario: the number is a literal in the UI text. A replica of `kb::parse` over the three knowledge files at 8a38bac counts 1652 entries. Whichever is right, the literal goes stale with every edit of the knowledge files.
Found by: the Slint MCP audit of the Settings tabs on 2026-10-09 against the count made for docs/architecture.md. The exact figure was not taken from a run of the Rust code.
Fix plan: show `kb::all().len()`.
Fix proposed: pull request #232 (commit 4ea3f2d): both labels lose the number. The literal was 1643 knowledge entries plus 53 snippets in June 2026.
Closed by: #232, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-017 Several controls are not exposed as controls to accessibility and to the MCP audit   Severity: P3   Status: Open
Evidence: slint-experiment/ui/settings_panel.slint (the 16 navigation entries), slint-experiment/ui/archive.slint (the "Cancel" and "Delete" choices of the delete confirmation), slint-experiment/ui/overlay_bar.slint (toggle chips)
Scenario: read through Slint MCP, the Settings navigation entries and the two buttons of the delete confirmation are plain `Text` elements with no role, and the bar's toggle chips (stealth, auto-tile, lock, audio) report no checked state. A screen reader cannot tell them from labels, and an automated audit can click them only by position and cannot read whether a toggle is on.
Found by: the Slint MCP audit on windows-worker on 2026-10-09: element roles per window (the bar exposes 19 buttons; a Settings page exposes only its content controls and "Close").
Fix plan: give the entries `accessible-role: button` or `tab`, and the chips `accessible-checked`.
Addition 2026-10-10: in the Russian interface the bar's accessibility labels are English ("Session archive", "Settings", "Help"): slint-experiment/ui/overlay_bar.slint uses plain strings for `a11y:`, not `@tr()`. The Settings language combobox does not react to an MCP click, an accessibility action or a set value on its list item.
Partly fixed by pull request #240, on master through #243 (checkable state for the toggle chips, a role and state for the Settings navigation entries, the bar labels through `@tr`). The delete confirmation buttons of the archive are not covered.
Closed by:

### D-018 The app exits at startup on a machine without OpenGL   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host_windows.rs:722 - `let _ = slint::BackendSelector::new()` with `.backend_name("winit".into())` and no renderer fallback
Scenario: on Winbrat (a virtual machine without a GL driver) `overlay-host.exe` logs its startup, registers hotkeys and then ends with "Error: Failed to initialize OpenGL driver: Could not locate glCreateShader symbol". No window appears and nothing tells the user why. With the environment variable `SLINT_BACKEND=winit-software` the same binary starts and works. A user on a virtual desktop or with a broken GPU driver gets a program that silently does not open.
Found by: the first start of the QA build of 8cf4904 on windows-worker on 2026-10-09; the software renderer was confirmed as the workaround in the same session.
Fix plan: on that error, retry with the software renderer and log the fallback.
Fixed by pull request #241 (Slint 1.18.1). Seen on windows-worker: the 1.18.1 build starts without `SLINT_BACKEND=winit-software`, where the 1.17.1 build exits.
Closed by: #241, merged through #243, master 4fc4fe286ce531525f4ccdf0ec5b48a856e56f83
Reopened 2026-10-12: Slint is back at 1.17.1 because 1.18.1 cuts off the end of a scrolled Settings tab (the content height of a conditional block with word-wrapped text is under-reported; upstream names this limit in slint-ui/slint pull request 13124). With the same code, the real mouse wheel reaches the last control of "AI providers" on 1.17.1 and stops 63 px short on 1.18.1. The fix plan above (retry with the software renderer) is open again.

### D-019 `mask_host` keeps query values, fragments and a host behind extra slashes   Severity: P2   Status: Closed
Evidence: overlay-backend/src/config.rs:1927 - `format!("{scheme}***{port}{path}")` (and overlay-backend/src/config/tests.rs:1744 - a test that expects `"http://***:18902?token=secret123"`)
Scenario: the function blanks the host and returns the rest of the URL unchanged. It is used for the server-settings import preview and by the diagnostics redactor, so `http://host:18902/v1?token=SECRET` keeps its token, and `//192.168.0.142:18902/v1` or `http:///192.168.0.142/v1` keep the address. It needs a base URL that carries a token in its query string, or a malformed URL, to matter.
Found by: the Grok audit on master 37c1230 (docs/audit-grok/wave1_worker3_config.md finding 2 and wave4_worker1_privacy.md finding 2); confirmed in source by the lead and demonstrated by three failing tests in CI at da058978a30acba39336851e9c4185c1273a0e46.
Fix proposed: pull request #233 (commit 612a770a08ae72b7bff49fb6795ca23d42ef41ce): query and fragment become `?***` and `#***`, leading slashes are kept and the host behind them is masked.
Closed by: #233, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-020 The archive heading keeps the old session count after a delete   Severity: P3   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:158 - `p.set_summary(SharedString::from(total.to_string()));` (the only place that sets the count; the delete handler rebuilds the rows only)
Scenario: open the archive with four sessions, delete one: the heading still reads "Session archive (4)" above three rows until the window is opened again.
Found by: the Slint MCP audit on windows-worker on 2026-10-10 (build of 7262b9f, a synthetic session deleted with the archive's own button); then read in source.
Fixed by pull request #238 (commit a0c8edf9bae5a71acf106d5483ea057580934182): the delete handler re-reads the count.
Closed by: #238, merged through #239, master f2122d087e43ddeaea3f5df78d846ce73d883780

### D-021 `redact_urls` passes several URLs to `mask_host` as one when they are not separated by whitespace   Severity: P3   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/diagnostics.rs:134 - `let end = tail.find(char::is_whitespace).unwrap_or(tail.len());`
Scenario: a log line such as `http://first.internal/v1,http://user:secret@second.internal/v1` is cut only at whitespace, so the second URL is treated as part of the first one's path and stays as it is. It needs two URLs joined by punctuation in a log line or a report.
Found by: independent review of pull request #233 (pi, gpt-sol-latest) on 2026-10-10; read in source by the lead. Not demonstrated by a test.
Fix plan: end a URL at the characters that cannot belong to it (comma, semicolon, quotes, angle brackets), or scan the remainder for a further scheme.
Closed by: #246, master 449054081e700b730426d5cb24b685c7d27d5d48, released in v0.38.1-rc.4

### D-022 Settings opens on the thirteenth tab   Severity: P3   Status: Closed
Evidence: slint-experiment/ui/settings_panel.slint:92 - `in-out property <int> active-tab: 0;` (and line 764: `label: @tr("Stealth"); ... tab-index: 0;`)
Scenario: the default tab index is 0 and index 0 belongs to "Stealth", the thirteenth entry of the navigation list. A first open of Settings lands on Stealth instead of the first entry, "Profile + context".
Found by: the Slint MCP check of the combined build on windows-worker on 2026-10-10 (the checked navigation entry right after opening Settings); then read in source. It may be deliberate; no comment says so.
Fix plan: decide the intended first tab, then set the default to its index.
Closed by: #246, master 449054081e700b730426d5cb24b685c7d27d5d48, released in v0.38.1-rc.4

### D-023 The bar's state word stays English in the Russian interface   Severity: P3   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/tile_controller.rs:850 - `o.set_status_text(SharedString::from("recording"));` (also "idle" at lines 855 and 916, and overlay_host_windows.rs:777)
Scenario: with the interface in Russian the first pill of the bar reads "recording" or "idle" while its neighbours read "спросить", "захватить", "старт". Same class as D-015: text built in Rust.
Found by: screenshots of the Russian bar on windows-worker on 2026-10-10.
Fix plan: check first whether the word is also used as a key (the tray uses the same word for its state); if so, translate at the point of display only.
Closed by: #246, master 449054081e700b730426d5cb24b685c7d27d5d48, released in v0.38.1-rc.4

### D-024 A silent uninstall waits forever on a question whose default answer deletes the user's data   Severity: P2   Status: Closed
Evidence: scripts/slint-installer.nsi:125 - `MessageBox MB_YESNO|MB_ICONQUESTION "Удалить также ваши данные (настройки, история сессий, записи) и скачанные модели ИИ?...` (no `/SD`)
Scenario: `uninstall.exe /S` removes the program files, then shows this box although it runs silent, because the box has no silent default. A script or a package manager that uninstalls silently hangs with the registry key still present. The default button is Yes, which removes `%APPDATA%\suflyor` and the downloaded models.
Found by: the install smoke of v0.38.1-rc.4 on windows-worker on 2026-10-10: the uninstaller was still running after five minutes; it was ended without an answer and the data was intact.
Fix plan: `/SD IDNO` on the box, and `MB_DEFBUTTON2` so that Enter keeps the data.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5. Checked on windows-worker: the silent uninstall of the built installer ended by itself and left the user data in place.

### D-025 Texts that stay English in the Russian interface   Severity: P3   Status: Closed
Evidence: slint-experiment/ui/archive.slint:381 - `a11y: "Rename session";` (also "Regenerate name from transcript", "Delete session", "View transcript" at lines 386 to 396); slint-experiment/src/bin/overlay_host/tile_ask.rs:296 - `"F9 ask · live"` and :311 `"F9 manual ask"`; slint-experiment/src/app_state.rs:266 - `"AI bridge call failed (see overlay-host.log for diagnostic)"`
Scenario: with the interface in Russian the four row buttons of the archive announce English names, a tile opened with F9 has an English title, badge and state word ("error"), and its error text is English. Same class as D-015 and D-023: text that does not go through `@tr`, so the i18n guard does not see it.
Found by: the Slint MCP pass of v0.38.1-rc.4 in Russian on windows-worker on 2026-10-10 (rule "untranslated" of the audit script, then read in source).
Fix plan: `@tr` for the four archive names; the tile title, badge and error text through the language-aware helpers that the bar status already uses.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5, for the texts named here. The wider class (text built in Rust in one language) is D-030.

### D-026 F3 does nothing after an answer to F9 or to a typed question   Severity: P2   Status: Closed
Evidence: slint-experiment/src/bin/overlay_host/tile_ask.rs:119 - `s.last_question = Some(out.display_question);` (line numbers of master 03fc755; the only writers of `last_question` were the auto-tile path, F6 and F3 itself; the streaming ask of F9 never wrote it)
Scenario: the user presses F9, reads the answer, presses F3 to refine it. The log says `tile:error: {"message":"Reask: no previous Q/A yet — ask AI first."}` and nothing appears, because `tile:error` has no UI.
Found by: live run against a stand-in AI endpoint on windows-worker on 2026-10-11 (first run of an answered tile on the worker), then read in source.
Fix plan: remember a finished manual ask; show a notice tile when there is nothing to re-ask.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5

### D-027 Five windows show std-widgets in the light default on a dark window   Severity: P3   Status: Closed
Evidence: slint-experiment/ui/text_ask.slint:15 - `import { LineEdit, Button } from "std-widgets.slint";` with no `Palette.color-scheme` in the file (also wizard.slint, recover_offer.slint, help.slint, macos_ai_setup.slint; master 03fc755)
Scenario: the "write a question" window has a white field and a saturated blue button in the three dark schemes; the wizard has light grey buttons. Every other window sets the palette from `Theme.is-light`.
Found by: captures of the question window and of the seven wizard steps on windows-worker on 2026-10-11.
Fix plan: the same `widgets-light` lines as the other windows, and a test that requires them.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5

### D-028 A tinted pill fails the contrast rule against its own tint   Severity: P2   Status: Closed
Evidence: slint-experiment/ui/tile.slint:616 - `background: root.trigger-color.with-alpha(0.18);` with the text in the same colour (master 03fc755); the colour came from Rust as an RGB literal (bar_tray.rs:45, tile_ask.rs:306 and 37 more)
Scenario: light scheme: the bar's state word measured 1.60:1, the tile badge "F9 manual ask" 2.22:1, "AI streaming" 2.26:1. Dark schemes: danger, accent and muted on their own 18 percent tint compute to 4.0 to 4.4:1.
Found by: measured contrast in the capture matrix of v0.38.1-rc.4 (2026-10-10) and of the tile captures (2026-10-11); the dark values are computed from the theme tokens.
Fix plan: the host names a tone, the theme owns the colours, and one pair `tint` / `on-tint` defines a tinted pill.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5

### D-029 A request that the server never answers is reported as a refused connection   Severity: P3   Status: Closed
Evidence: overlay-backend/src/ai/stream.rs:212 - `return Err(anyhow!("AI connection error"));` (master 03fc755), which `classify_ai_error` reads as "AI bridge unreachable (connection refused)"
Scenario: the endpoint accepts the request and sends nothing. The tile shows "Asking AI…" for 120 seconds and then says the connection was refused, while the log says `transport error (timeout)`.
Found by: live run against the stand-in endpoint in its "never answer" mode on windows-worker on 2026-10-11.
Fix plan: keep the failure kind, which has no address in it, in the message.
Closed by: #249, master 1161c72e96bd37ab276585325b9bd919e6ead89f, released in v0.38.1-rc.5

### D-030 Text that the host builds in Rust exists in one language only   Severity: P3   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/settings_updates.rs:47 - `set_update_status` with `"Проверка GitHub…"`; slint-experiment/src/bin/overlay_host/settings_import_export.rs:103 - `"import cancelled"` (master 1161c72; a script over the sources counts 62 Russian and 44 English string literals passed straight to a Slint setter, not counting the 12 state words that the bar translates itself)
Scenario: the English interface shows Russian status lines in Updates, local AI, Hermes, diagnostics, memory and the transcript window; the Russian interface shows English ones in the connection tests, import and export, and the wizard (`[err] no Groq API key set`). A comment in transcript.rs says Rust-built Russian is allowed, so part of this is a recorded choice, not an oversight.
Found by: script over the sources on 2026-10-11; the wizard and tile cases were also seen live.
Fix plan: one helper per module in the style of `status_copy.rs` and `tile_text.rs`, and a guard test that fails on a literal with letters passed to a setter. The same pass can replace the 118 `[ok]` / `[err]` / `[--]` markers by a state the UI draws.
Addition 2026-10-11 (T6/R5/A6): the code-level map of the tile, transcript and archive windows finds the same defect; a second independent reading confirms it. These additional cases have NOT been reproduced live. Evidence at master 2a7c08d3165243093c15c637d2c9cd19f59be4d7: slint-experiment/src/bin/overlay_host/tile_followup.rs:235,267,275,282,696 sets status literals without a locale check; slint-experiment/src/bin/overlay_host_windows.rs:3102 and slint-experiment/src/bin/overlay_host/tile_controller.rs:849 do the same, and slint-experiment/ui/tile.slint:395 displays `root.source-label` unchanged. slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:542,574 sets Russian speaker labels, and :658,707,1092,1100 sets speaker-detection status or error text without a locale check. slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:573,594,626-629,726 builds summary status, error text and titles in one language. The selected interface language does not change these strings. Extend the module language helpers; the transcript error styling is a separate defect, D-049.

### D-031 Shift+Alt+1 with no selected text does nothing visible   Severity: P3   Status: Closed
Evidence: log of the run: `Shift+Alt+1 — read selection (clipboard)` then `sa1: copied 0 chars from selection`; no window, no status change
Scenario: the user presses the read-aloud key with nothing selected (or in an application that does not answer Ctrl+C) and cannot tell whether the key works.
Found by: the 13-key pass on windows-worker on 2026-10-11.
Fix plan: a short status on the bar, as the other keys that cannot act already do.
Closed by: #251, master 84a4173939f2f165d6e99e389dd4eb4dfdff1550; not released yet

### D-032 `cargo fmt --check` does not see the modules of `overlay_host/`   Severity: P3   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/status_copy.rs:196 - a 109-character function signature, `pub(super) fn manual_tile_placeholder(deep_locked: bool, has_transcript: bool, is_ru: bool) -> &'static str {`, which rustfmt would wrap (master 1161c72); slint-experiment/src/bin/overlay_host.rs:19 - `include!("overlay_host_windows.rs");`
Scenario: the host's modules are declared with `#[path]` inside a file that is pulled in with `include!`, and rustfmt does not follow `include!`. The format step of the gate therefore passes whatever those 36 files look like.
Found by: the worker's `cargo fmt` on 2026-10-11 changed nothing in a new file with an over-long line under `overlay_host/`, while it reformatted files outside it.
Fix plan: a gate step that runs `rustfmt --check --edition 2021` on the files under `src/bin/overlay_host/`, after one formatting commit.

### D-033 Four messages of the session archive are written to a line that is not drawn   Severity: P3   Status: Closed
Evidence: slint-experiment/ui/archive.slint:277 - `if root.retranscribe-busy : Text {` is the only place that shows `retranscribe-status`; slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:372 - `p.set_retranscribe_status(SharedString::from("Активную сессию удалить нельзя"));` with `retranscribe-busy` false (also :425 a failed delete, :527 not enough data for a summary, :695 an empty saved summary; master 4134f93)
Scenario: the user deletes the running session, or a delete fails, or asks for a summary that cannot be built: the handler returns after setting a text nobody sees, and the button looks dead. "Regenerate name" has three more silent returns (no transcript, a cloud provider, no name from the AI).
Found by: read in source on 2026-10-11 after "Regenerate name" on a session without a transcript showed nothing in a live run on windows-worker. The four messages were not reproduced live.
Fix plan: a notice line of its own, shown when no job runs, in both languages.
Closed by: #251, master 84a4173939f2f165d6e99e389dd4eb4dfdff1550; not released yet

### D-034 Failed memory saves dismiss the tile and transcript editors   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/tile_copy.rs:324 - `let _ = store_note(verbatim, None, "none");` discards the failure returned at :351-360; :450-458 clears marks after saving, and :563-565 clears pending capture and text regardless of success. slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:149-175,221-223 calls the same helper and clears the editor state. slint-experiment/ui/tile.slint:898 shows the editor only while a mark or pending capture exists.
Scenario: the user approves an edited memory note while the store cannot open or write. The save fails, but the editor closes and the approved edits are no longer available there for retry.
Found by: code-level map of the tile and transcript windows on 2026-10-11 (T2/R3); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: return save success from the shared helper and clear marks, pending capture and edited text only after success in both windows.

### D-035 A failed initial push-to-talk transcription leaves the tile busy   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/tile_ptt.rs:144 - `tile.set_followup_busy(true);`; :311-344 calls `ptt_tile_error` and returns for empty recognition or an STT error, but the helper at :54-71 does not clear busy. The per-tile stream sink is constructed only at :398. slint-experiment/src/bin/overlay_host/tile_controller.rs:821-851 clears only the shared streaming slot on session stop.
Scenario: the first push-to-talk request recognizes no speech or fails transcription. The tile shows an error but follow-up remains disabled; stopping the session does not repair this independent tile through the shared-slot handler.
Found by: code-level map of the tile window on 2026-10-11 (T4); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: clear follow-up busy in `ptt_tile_error` on every transcription failure.

### D-036 Copy all omits transcript utterances beyond the display limit   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:453 - `const TRANSCRIPT_DISPLAY_CAP: usize = 2000;`; :1249 applies the cap before :1260 builds `utts_display`, which :1301,1338 passes to the action handlers. The copy handler at :275-289 formats only that vector. slint-experiment/ui/transcript.slint:846 says to use Copy all for the full transcript.
Scenario: a session contains more than 2,000 utterances. Copy all exports only the display subset, despite the footer promising the full transcript. The remaining utterances stay in storage but are omitted from this export.
Found by: code-level map of the transcript window on 2026-10-11 (R1); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: pass the full utterance vector separately to Copy all while retaining the display-vector mapping for checked rows and playback.

### D-037 Direct tile creation bypasses the sixteen-window limit   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host_windows.rs:1922,1925-1929 defines the sixteen-tile cap and applies it only after receiving a queued request. slint-experiment/src/bin/overlay_host/tile_ask.rs:541, slint-experiment/src/bin/overlay_host/read_aloud.rs:163 and slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:932 push tiles directly. slint-experiment/src/bin/overlay_host/window_lifecycle.rs:439-449 updates the count without eviction.
Scenario: the user opens tiles through direct creation paths. The live window count can exceed sixteen without evicting the oldest tile. A freeze is not established.
Found by: code-level map of the tile window on 2026-10-11 (T1); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: apply the existing eviction policy at every tile registration through a shared registration path.

### D-038 Busy tile menu actions close the menu without acting   Severity: P2   Status: Open
Evidence: slint-experiment/ui/tile.slint:60-92 gives menu rows no disabled state; :495-518 shows Regenerate and Smart cloud but suppresses their callbacks while `followup-busy`, then closes the popup. slint-experiment/src/bin/overlay_host/tile_ask.rs:424,506 and slint-experiment/src/bin/overlay_host/tile_followup.rs:353 allow busy tiles to expose these actions.
Scenario: the user opens the menu while an answer is in progress and clicks Regenerate or Smart cloud. The row looks actionable, but the popup closes without dispatching the request.
Found by: code-level map of the tile window on 2026-10-11 (T3); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: bind disabled appearance and interaction to follow-up busy instead of silently accepting the click.

### D-039 Whole-answer selection cannot request memory capture   Severity: P2   Status: Open
Evidence: slint-experiment/ui/tile.slint:877-892 uses a read-only `TextEdit` with copy and select-all handling but no capture callback. slint-experiment/ui/controls.slint:131-139 connects Add selection to memory only in `SelectableText`. slint-experiment/src/bin/overlay_host/tile_copy.rs:482-484 supports the whole-answer buffer through a negative block index.
Scenario: the user selects text across answer blocks in whole-answer selection mode. The selected text can be copied, but the field has no action that sends it to the memory approval editor.
Found by: code-level map of the tile window on 2026-10-11 (T5); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: connect the field's selection offsets to the existing capture callback with the whole-answer block index.

### D-040 Tile views hide wide code and the tail of selection text   Severity: P2   Status: Open
Evidence: slint-experiment/ui/tile.slint:781 clips the code container and :833 uses `wrap: no-wrap`; slint-experiment/src/bin/overlay_host/tile_window.rs:161 expands the tile to 800 pixels wide. slint-experiment/src/bin/overlay_host/tile_copy.rs:626 sets a 12,000-byte limit, applied at :648-652. The separate code-copy handler at :290-317 copies the complete code string.
Scenario: code lines exceed the expanded tile width, or the whole-answer selection buffer exceeds 12,000 bytes. The views omit the code's right edge or the selection tail. The selection buffer appends an ellipsis, but does not state its limit; complete code remains available through code-block copy. Permanent source-data loss is not established.
Found by: code-level map of the tile window on 2026-10-11 (T8); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: add horizontal code scrolling and state the selection-buffer limit without truncating the underlying answer.

### D-041 Long transcript utterances have no expansion action   Severity: P2   Status: Open
Evidence: slint-experiment/ui/transcript.slint:698-699 caps each row at 120 pixels and clips it; :754-797 provides text selection and marking but no expansion action. slint-experiment/ui/controls.slint:118 sizes `SelectableText` to its preferred text height. slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:1260,1272 retains the utterance text for the display and copy paths.
Scenario: an utterance needs more than 120 pixels of height. Its ending is hidden in the transcript row, with no way to expand that row, although the text remains copyable.
Found by: code-level map of the transcript window on 2026-10-11 (R2); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: offer row expansion or a scrollable utterance detail view.

### D-042 Transcript audio controls silently reject loading and initial seeking   Severity: P2   Status: Open
Evidence: overlay-backend/src/session_audio.rs:124 checks file existence, while :89-93 requires at least one readable recording channel. slint-experiment/src/bin/overlay_host/transcript_player.rs:438 returns false on load failure. slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:373-380 has no failure branch for Play, and :398 wires seeking directly to `seek_fraction`. slint-experiment/src/bin/overlay_host/transcript_player.rs:475-482 seeks only when a player already exists.
Scenario: recording files exist but neither channel is readable, or the user seeks before the player loads. Play or seek does nothing without an explanation; the Play failure does not clear the audio-available state. One readable channel is enough for loading, so a single unreadable channel does not establish this failure.
Found by: code-level map of the transcript window on 2026-10-11 (R4); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: show a load failure and ensure the session player exists before seeking.

### D-043 Transcript copy feedback acknowledges both actions   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:32 sets one `copied` flag and :36 clears it. slint-experiment/ui/transcript.slint:330,355 uses that flag for both Copy all and Copy selected. slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:310-311 returns without feedback when no rows are checked.
Scenario: either copy action succeeds and both buttons say Copied, including the action the user did not perform. Copy selected remains clickable with no checked rows and then gives no feedback.
Found by: code-level map of the transcript window on 2026-10-11 (R6); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: track acknowledgment per copy action and disable Copy selected when no rows are checked.

### D-044 Archive query failures appear as empty results   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:137,187,198 converts list and search failures to empty vectors with `unwrap_or_default()`. slint-experiment/ui/archive.slint:87,95 starts with empty results and unavailable false; :325-328 displays the empty-state message. slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:154-155 separately marks a failed store open unavailable.
Scenario: an archive list or search query fails, or the initial list is still loading. The window reports no archived sessions or no matches rather than a query error or loading state. A store-open failure is handled separately and is not part of this claim.
Found by: code-level map of the archive window on 2026-10-11 (A1); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: distinguish loading, query failure and successful empty results.

### D-045 Archive browsing stops at 300 sessions without a notice   Severity: P2   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:107 sets `ARCHIVE_LIST_CAP` to 300; :138 counts every session, :144 caps displayed rows, and :158 sets the heading to the full count. :187-191 reapplies the cap when the search field is empty. slint-experiment/ui/archive.slint:318-552 has no overflow notice or paging action.
Scenario: the archive contains more than 300 sessions. The heading counts them all, but browsing shows only the first 300 and gives no explanation for the missing rows. Search remains available.
Found by: code-level map of the archive window on 2026-10-11 (A2); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: disclose the browsing limit and offer paging or guidance to find older sessions with search.

### D-046 Archive name generation allows overlapping jobs   Severity: P2   Status: Open
Evidence: slint-experiment/ui/archive.slint:396-400 leaves Regenerate name enabled during generation. slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:346-347 spawns a request on every eligible click and :361-365 saves its result. slint-experiment/src/session_namer.rs:187-211 runs the completion without a generation latch.
Scenario: the user requests a new name more than once before a prior request finishes. Jobs overlap and their completion order determines the saved name, so an earlier request can overwrite a later result.
Found by: code-level map of the archive window on 2026-10-11 (A3, generation); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: latch name generation for the session and disable its action until that job ends.

### D-047 A failed archive rename dismisses the editor   Severity: P2   Status: Open
Evidence: overlay-backend/src/session_names.rs:95-101 logs persistence failure without returning it. slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:278-288 calls that helper, sets `renaming-index` to -1 and refetches the list. :1063 reads the displayed name back from storage.
Scenario: the user submits a typed session name and the write fails. The editor closes without reporting failure and the row reloads its stored name, leaving no editing state for retry.
Found by: code-level map of the archive window on 2026-10-11 (A3, rename); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: propagate persistence failure and retain the rename editor and typed name until saving succeeds.

### D-048 Archive retranscription does not update the transcript view   Severity: P2   Status: Open
Evidence: overlay-backend/src/re_transcribe.rs:277-281 returns fresh transcript lines, and :298-305 passes them to summary generation. overlay-backend/src/runtime.rs:275-359 persists summary sources in a conspect, not catalog utterances. slint-experiment/src/bin/overlay_host/aux_windows/archive.rs:651 refreshes archive rows, but :503-507 still opens the transcript from catalog utterances.
Scenario: retranscription builds a summary from fresh recognition. Opening the session transcript afterward still shows the prior catalog utterances, so the summary and transcript view use different sources. This does not establish that the summary is incorrect.
Found by: code-level map of the archive window on 2026-10-11 (A5); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: preserve and expose a versioned retranscript without overwriting the original live timestamps.

### D-049 Transcript speaker errors use the progress text style   Severity: P3   Status: Open
Evidence: slint-experiment/src/bin/overlay_host/aux_windows/transcript.rs:658,707 sends speaker-detection failures to `diar-status`. slint-experiment/ui/transcript.slint:557-562 draws the same status property for progress and errors with `color: Theme.text-muted` and no failure state.
Scenario: speaker detection fails or its result cannot be saved. The error appears with the same style as progress rather than a failure state. The untranslated text is part of D-030, not a second localization entry.
Found by: code-level map of the transcript window on 2026-10-11 (R5, styling); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: expose a failure state separately from progress and bind its text style to the theme's failure role.

### D-050 Archive Delete text bypasses theme foreground roles   Severity: P3   Status: Open
Evidence: slint-experiment/ui/archive.slint:625-627 uses `Theme.danger` for the Delete background, but :635 sets its text to `#ffffff`. slint-experiment/ui/theme.slint:48 changes the danger colour by scheme.
Scenario: the user changes the theme scheme. Delete keeps white text on every danger fill instead of using a theme-owned foreground role. A contrast failure is not measured.
Found by: code-level map of the archive window on 2026-10-11 (A6, colour); confirmed by a second independent reading and citation checks against master 2a7c08d3165243093c15c637d2c9cd19f59be4d7. NOT reproduced live.
Fix plan: use a reviewed danger-foreground role for the button text.

### D-051 macOS MLX installed state and buttons can refer to another selected model   Severity: P1   Status: Open
Evidence: current tree 26928b6, slint-experiment/src/bin/overlay_host/settings_controller.rs:156 calls `populate_token_status(&win, cfg)` before :477 calls `settings_mlx::wire(&win, cfg)`; :1615 calls `settings_mlx::populate(win)`. slint-experiment/ui/settings_panel.slint:212,226 defaults both model indices to 0. slint-experiment/src/bin/overlay_host/settings_mlx.rs:281-282 snapshots those indices, :312-318 starts their checks, and :209-216 later loads the saved indices. :192-199 checks `installed_snapshot(role.model())` and applies the result without comparing the current selection; :35-48 writes shared per-role booleans regardless of model index. slint-experiment/ui/settings_panel.slint:2177-2187,3169-3179 leaves both model selectors enabled during jobs; :2238,2247,3228,3237 enables buttons from the shared installed/busy/checking state. slint-experiment/src/bin/overlay_host/settings_mlx.rs:235 dispatches Enable for text using the current index, while :112 applies an install result for the captured role without a selection guard.
Scenario: on macOS, save Gemma as the text model, install LFM but not Gemma, restart, then open Settings > AI providers. The selector shows Gemma, but the initial LFM check can mark it Installed, disable Download / Resume and enable Enable for text for the absent Gemma. Changing either role's model while a check or download is pending also allows an earlier model's completion to overwrite the selected model's state.
Found by: Known, not fixed here in rc.7 release notes and regress-ui finding 1; re-read the initialization, asynchronous completion and button paths at 26928b6. NOT reproduced live.
Fix plan: load saved model indices before the first populate; bind asynchronous state and progress updates to the selected model and request generation, rejecting stale completions; disable role selection during install/activation. Add coverage for first open with differing installation states and out-of-order completions for both roles.

### D-052 macOS Enable for text overwrites an independently selected Vision provider   Severity: P1   Status: Open
Evidence: current tree 26928b6, slint-experiment/ui/settings_panel.slint:2246-2248 dispatches `mlx-text-enable()` from Enable for text. slint-experiment/src/bin/overlay_host/settings_mlx.rs:233-236 passes the selected text role to `enable`; :143-147 sets `config.vision_provider = "same".into()` whenever that role is Gemma, without checking the prior Vision choice. :157 saves the configuration; :173-179 sets `window.set_vision_provider_index(1)` on success. slint-experiment/src/bin/overlay_host/settings_vision.rs:27-28 maps `"off" => 0` and `"same" => 1`.
Scenario: on macOS, choose OpenAI or Off for Vision, select Gemma in the managed MLX text card and click Enable for text. If activation and saving succeed, the persisted Vision provider and its displayed selection become Same as text model instead of preserving the independent choice.
Found by: Known, not fixed here in rc.7 release notes and regress-ui finding 2; re-read the callback, configuration save and UI completion at 26928b6. Actual inference NOT run.
Fix plan: remove Gemma's forced Vision-provider and displayed-index assignments; update image-input availability without changing an independently chosen provider. Preserve the existing rejection of Same as text model for text-only models; add coverage that enabling Gemma preserves Off and a separate provider.

### D-053 Auto-tiles drop short question clauses after a statement   Severity: P2   Status: Open
Evidence: current tree 26928b6, overlay-backend/src/runtime/trigger_detect.rs:225 requires an alphanumeric ratio of at least 0.60; :230 retains only tokens of at least 3 characters and :232-233 rejects fewer than 2 such tokens. :497,505 applies the speech filter to the full line before extracting a question. :393-396 splits at question marks and periods followed by whitespace; :425-426 reapplies the speech filter per clause before checking the leading interrogatives at :430-435 (including `"как "` at :299). :468-479 scans clauses backwards and returns None if none qualifies; :515,521-525 only falls back to configured keywords.
Scenario: normal auto-tiles, no matching keyword, one transcript line `Мы обсудили архитектуру. Как же?`. Step 1: the full line has 26 alphanumeric characters out of 32 (above 0.60) and 3 distinct qualifying tokens: `обсудили`, `архитектуру`, `Как`, so it passes. Step 2: the period and question mark split it into `Мы обсудили архитектуру.` and `Как же?`. Step 3: the question has 5 alphanumeric characters out of 7 (above 0.60), but `Как` has 3 characters and `же` only 2; the filter keeps one token and rejects the clause before the `как ` prefix is checked. Step 4: the statement has neither a leading interrogative nor a final question mark; with no matching keyword detection returns None. `Что это?` is not dropped by this condition: both words have 3 characters, its ratio is 6/8, and the `что ` prefix at :298 qualifies it.
Found by: Known, not fixed here in rc.7 release notes and regress-core finding 4; re-read the detector at 26928b6. The earlier report's `Что это?` example is refuted by the current code; no tests or runtime checks were run.
Fix plan: after the full utterance passes the speech filter, allow recognized question clauses without requiring two tokens of at least three characters; retain explicit acknowledgement/repeated-noise exclusions. Add coverage for the statement plus `Как же?`, the accepted `Что это?` control and noise cases.

### D-054 Profile + context title and description have excess height on screen   Severity: Unassigned   Status: Open (unconfirmed in source)
Evidence: current tree 26928b6, slint-experiment/ui/settings_panel.slint:2020-2023 nests the Profile + context SettingsCard in a VerticalLayout with `spacing: 10px`; :2024-2029 defines its description Text with `font-size: Metrics.font-label` and `wrap: word-wrap`, without a height constraint. :2093-2096 defines the context TextEdit with `height: 220px`. slint-experiment/ui/settings_controls.slint:101-105 binds the card's minimum and preferred heights to `card-content.preferred-height`, with padding and spacing tokens; :106-111 defines the title Text without a height constraint, followed by `@children` at :112. slint-experiment/ui/metrics.slint:31-32 defines spacing 8px and padding 12px; :70-71 defines description/title font sizes 11px/12px. These bindings do not establish the reported 36px/48px Text heights or their cause; source confirmation of the screen geometry is unavailable.
Scenario: the reported live Windows measurement at ru, 720x600 gives the title Text a height of 36px and the description Text 48px where 16px and 26px are needed, producing uneven gaps around the description. Setting `vertical-stretch: 0` on both Texts reportedly changed nothing. These measurements and the ineffective change are supplied live evidence, not measurements performed in this source-only task.
Found by: Known, not fixed here in rc.8 release notes and the supplied Windows measurements; layout facts re-read at 26928b6. Cause not established in source; severity remains unassigned pending confirmation.
Fix plan: inspect allocated card/layout/Text bounds and width-dependent text metrics in Slint 1.17.1 at the reported locale and size; establish where extra height is assigned before changing constraints. If confirmed, constrain title/description sizing at the owning layout while preserving wrapping and the 220px editor; compare before/after geometry at ru 720x600 and smaller sizes. Do not repeat the ineffective vertical-stretch-only change or impose 26px on descriptions at every width.

### D-055 Local model status line is Russian in the English interface   Severity: P3   Status: Open
Evidence: overlay-backend/src/local_ai/model_state.rs:365 builds the status text `[!] Gemma 12B QAT fallback: {:.1} GiB на диске. Матрица 8 ГБ VRAM / 16-31 ГБ RAM подтверждена владельцем. Память для vision: неизвестно.` in the backend, outside the `@tr` strings of the `.slint` sources.
Scenario: the interface language is English, the AI provider is the local model. Settings > AI providers shows this line in Russian between English texts.
Found by: live layout check of Settings in English on the Windows worker on 2026-10-12 (capture en-local-same-gigaam-AI_providers-3.png).
Fix plan: have the backend return the state and the numbers, and build the sentence in the UI from a translated string; check the other status texts of model_state.rs for the same pattern.

Needs a live check (not defect entries): T7 - render minimum-size tiles in both locales at high DPI with editor, player and dialog visible, then exercise the controls; R7 - render the transcript at 480 by 320 in both locales at high DPI with eight speakers, warning, status, editor and player, then check scrolling and hit targets; A7 - start a disposable session during pending reindex and inspect whether its active row and stale delete guard become reachable, without deleting it; A8 - render the archive at 480-pixel width in both locales at high DPI with all row actions and a long confirmation title, then check the action hit targets.

## Imported from the Grok audit (docs/audit-grok/, master 37c1230)

The 14 reports hold 119 numbered findings. Each was checked against the source at 37c1230 on 2026-10-10 by a second model (pi, gpt-sol-latest, read-only tools, one run per report) with the instruction to quote the line it read; a script then checked that each quoted line exists at the cited path (91 of the first 115 rows matched on both citations, 21 on one, 2 on none, 1 had no citation). Result: 42 confirmed, 64 partial (the code is as described, the consequence is overstated or mitigated), 8 refuted, 5 need a run of the program.

The table lists the 42 confirmed ones. Evidence level: read in source by a second model; the severity is that model's, not the lead's. The lead re-read three of them: `mask_host` (now D-019), the credentials map and the audio tail. The audio tail is rated P1 by the second model; the lead reads it as at most 200 ms of audio at a stop, which is P2 or P3. None of these rows was demonstrated by a test or a run, except D-019. Partial and refuted findings are not listed, by the rule at the top of this file.

| Source | Sev | Defect | Evidence | Scenario and limits |
|---|---|---|---|---|
| wave1_worker1_persistence#1 | P2 | The pre-migration copy is not guaranteed to be consistent. | `overlay-backend/src/persistence/sqlite_store.rs:60`: `let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");` | The checkpoint result, including SQLite's busy indication, is ignored, and only the main file is copied without protection from concurrent changes. This establishes a missing consistency guarantee, not damage to a particular `.bak` file. |
| wave1_worker1_persistence#3 | P2 | Invalid UTF-8 prevents all events in a journal file from being indexed. | `overlay-backend/src/persistence/indexer.rs:41–42`: `let content = std::fs::read_to_string(path)` / `.with_context( || format!("read journal {}", path.display()))?;` | Any invalid UTF-8 sequence stops reading before individual lines are parsed, and recovery uses the same reading method. The existing test covers invalid JSON in valid UTF-8, not invalid UTF-8. |
| wave1_worker1_persistence#6 | P2 | Journal openings in the same millisecond can append to the same file. | `overlay-backend/src/journal/writer.rs:74–75`: `let rand: u32 = (now_unix_ms() & 0xFFFFFF) as u32;` / `let path = dir.join(format!("{stamp}_{rand:06x}.jsonl"));` | Two openings with the same stamp and millisecond share a file because creation is not exclusive and collisions are not handled. Indexing combines their events under the same filename stem, but `skip_active` excludes the entire shared file rather than being bypassed. |
| wave1_worker1_persistence#10 | P2 | Deleting the SQLite database loses user-maintained memory and diarization data. | `overlay-backend/migrations/0003_memory.sql:3–5`: `-- memory items. Unlike sessions/utterances/ai_turns (a rebuildable projection of` / `-- the JSONL journals), these rows are NOT derived from the journals — they are` / `-- the user's curation, so they MUST survive a catalog re-index.` | The journals cannot restore these tables, despite Store documentation promising lossless recovery. Normal reindexing preserves them and maintenance uses backups and repair without deletion, so loss requires a manual wipe or separate damage. |
| wave1_worker1_persistence#16 | P2 | The journal queue has no size limit and ignores failed sends. | `overlay-backend/src/journal/writer.rs:95`: `let (tx, rx) = mpsc::unbounded_channel::<WriterCmd>();` | A slow disk can grow memory use, and counters updated before an ignored send failure can disagree with the journal. Shutdown reports errors and timeouts and normal stop logs them, but the queue remains unlimited and a timeout releases the task handle without waiting for completion. |
| wave1_worker2_memory#3 | P2 | The validator counts Russian negations without checking their position and does not protect English negation. | `overlay-backend/src/memory/normalize.rs:375–386`: `const NEGATIONS: &[&str] = &["не", "нет", "ни", "нельзя", "без", "никак", "никогда", "ничего", "никто", "никакой"];` | Both cited rewrites pass the checks because Russian negation can move and English `not` can be omitted as an ordinary content word. The model is asked to preserve negation, but its result uses this same validator, and existing tests cover adding or removing Russian negation rather than moving it. |
| wave1_worker2_memory#4 | P2 | Prefix matching treats different words and names as matches. | `overlay-backend/src/memory/normalize.rs:399–402`: `let need = a.chars().count().min(b.chars().count()).min(4); need > 0 && common_prefix_len(a, b) >= need` | The rule used for source checks and search allows “компания” to be replaced by “компьютером” and Vladimir to match Vladislav. Word-order and number checks do not prevent this; the diminutive-name test confirms intentionally approximate search, not the correctness of these false matches. |
| wave1_worker2_memory#5 | P2 | Allowed omissions and text truncation can change meaning. | `overlay-backend/src/memory/normalize.rs:410–418`: `match src_words[from..].iter().position(\ | s\ | words_match(w, s)) { Some(off) => from += off + 1, None => return false, }` | “Production is stable and the test environment is up” can become “production is up” while passing order, negation-count, and number checks, and truncation in context and Summary can remove an important qualification. Q/A candidates keep complete answers without a maximum length, but distortion of every long answer is not established and the existing test only prevents word reordering. |
| wave1_worker3_config#1 | P2 | Reading credentials can change which data directory is selected. | `overlay-backend/src/credentials.rs:143–144`: `.join("suflyor"); ensure_dir_permissions(&dir)?;` | Reading credentials creates the brand directory, causing the resolver to stop selecting the legacy directory. Normal startup migrates first, but if migration fails, old data becomes inaccessible through the resolver without being deleted. |
| wave1_worker3_config#2 | P2 | `mask_host` leaves URL tokens and some address forms visible. | `overlay-backend/src/config.rs:1927`: `format!("{scheme}***{port}{path}")` | Query strings and fragments are retained verbatim, and extra leading slashes can leave the address in the retained remainder. The function serves UI previews and export redaction; additional IPv4 masking in exports does not hide arbitrary query tokens. |
| wave1_worker3_config#7 | P2 | Updating one credential can overwrite other recoverable credentials. | `overlay-backend/src/credentials.rs:155`: `serde_json::from_slice(&bytes).unwrap_or_default()` | An unreadable or damaged map silently becomes empty before a slot is written, and there is no lock across reading and writing; the shared temporary file also allows a race. Current UI callbacks are synchronous, so concurrent UI and background writers are not established, but the API does not protect concurrent calls. |
| wave2_worker1_audio#1 | P2 | Audio resampling discards fractional progress and leftover samples between chunks. | `overlay-backend/src/audio.rs:807`: `let out_len = (input.len() as f64 / ratio).floor() as usize;` | Both implementations lose fractional progress and leftovers, the 3:1 branch processes only complete triples, and averaging does not fully filter frequencies that would distort resampled audio. WSOLA exists only in playback and tests do not cover successive chunks with non-integer conversion lengths, so the degree of speech-recognition degradation has not been established by running it. |
| wave2_worker1_audio#3 | P2 | Audio capture does not recover when events and notifications disappear. | `overlay-backend/src/audio.rs:410–422`: `if event.wait_for_event(250).is_err() { … continue; }` | Without events or notifications, the main thread neither reads the client nor starts recovery, and heartbeat does not change this, although stop is still checked. RouteWatcher and read-error handling protect normal live capture when signals arrive, but push-to-talk ignores read errors and has no watcher; a particular Bluetooth driver's behavior requires a live test. |
| wave2_worker1_audio#8 | P1 | Stopping or recovering capture discards the last incomplete audio chunk. | `overlay-backend/src/audio.rs:447–449,485–487`: `if f32_buf.len() >= chunk_samples_target { … f32_buf.clear(); … } … let _ = client.stop_stream(); … Ok(CaptureExit::Stopped)` | Windows and macOS do not send the accumulated incomplete chunk or drain the native buffer on stop or recovery, so normal recording completion can lose the end of speech without a guaranteed 200 ms limit. WAV padding is budget-limited, speech recognition keeps its own timestamps, and diarization warns about substantially shortened WAV files, but none restores the lost ending. |
| wave2_worker2_stt#1 | P2 | The speech-recognition queue can hold an unlimited number of tasks and their audio. | `overlay-backend/src/stt.rs:513–519`: `tokio::spawn(async move {` … `let _permit = match sem.acquire_owned().await {` | Both branches create tasks holding PCM before acquiring permission, so six active workers and limited voice-detection buffers do not limit waiting tasks; result delivery also holds permission, and `transcribe_once` is outside this limit. Memory can grow during sustained delays, but gigabyte estimates were not measured, a slow UI need not delay the separate event consumer, and closing the semaphore is not an implemented stop path here. |
| wave2_worker4_local_ai#1 | P2 | An unrelated HTTP server on port 8080 can be accepted as the model server. | `overlay-backend/src/local_ai.rs:915`: `&["-s", "-o", dev_null(), "--max-time", "2", url],` | HTTP 404 and 503 count as reachable, allowing an early return without checking ownership, model, or answer generation; `ensure_servers_for_route` similarly skips startup. Installation and restart have stricter checks, but an already responding unrelated server never reaches them. |
| wave2_worker4_local_ai#2 | P2 | Engine verification can accept an unrelated response on port 8077. | `overlay-backend/src/local_ai.rs:1761`: `let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);` | Failure to free the port is ignored, and readiness accepts a successful curl exit without checking HTTP status, model, or whether the child process is alive, allowing an update replacement. A file that cannot launch returns `false`, so false success requires a process that starts but does not provide a working server. |
| wave2_worker4_local_ai#5 | P2 | Some model launch paths accept files without checking their content hash. | `overlay-backend/src/local_ai/model_state.rs:402–406`: `ManagedModel::Fallback12B => pick_llama_gguf(` … `file_has_expected_size(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE),` | The 12B and legacy 4B models are selected by size, and the projector by size and build compatibility, which cannot detect same-size content damage. Installation hashes and the metadata-cached 26B hash check reduce exposure but do not protect these launch paths, and the old 4B size is explicitly exempt from the download hash check. |
| wave2_worker4_local_ai#6 | P2 | Binary archives are extracted without a content hash or signature check. | `overlay-backend/src/local_ai.rs:2466–2467`: `curl_resumable(url, &zip, size, label, cancel, on)?;` / `extract_zip(&zip, dest_dir)?;` | Downloads may exceed the expected size, and curl follows redirects without a separate final-host restriction before extraction. Three allowed initial GitHub hosts, HTTPS, and extraction path checks provide protection but do not establish executable-content integrity; source or redirect compromise is an unusual condition, not a proven leak. |
| wave2_worker4_local_ai#7 | P2 | Installed model files can be reused without detecting local damage or replacement. | `overlay-backend/src/local_ai.rs:561`: `verify_sha256(&whisper_dest, WHISPER_SHA256, "Whisper model")?;` | Whisper, the 12B projector, and GigaAM use `/main/` URLs, but pinned installation hashes prevent silently accepting upstream changes. After installation, Whisper checks existence, the projector checks size, and `GigaAMModel::load` has no own hash check, leaving local damage or replacement undetected at these points. |
| wave2_worker4_local_ai#11 | P2 | Whisper readiness accepts HTTP responses without verifying the intended server or model. | `overlay-backend/src/local_ai.rs:2027`: `if want_whisper && !is_reachable(&format!("{WHISPER_BASE_URL}/models")) {` | Boot skips Whisper startup after any HTTP response on port 8081, while installation readiness accepts HTTP errors without checking process identity, a live child, or the model. Installation stops if the port cannot be safely freed, ruling out an ignored stop error there but not weak readiness or boot's trust in an unrelated listener. |
| wave3_worker1_bridge#3 | P2 | The session-generation check does not protect the subsequent updates from a session change. | `slint-experiment/src/slint_session.rs:1283–1286`: `if lock(&rt).session_gen != session_gen {` … `return;` | Stop/Start between the unlocked check and later write lets an old result change the new session, and cost accounting and tile creation are likewise not protected together with the check. There is no check before the AI call, cache hits bypass it, and successful answers update health before it. |
| wave3_worker2_window#1 | P2 | A window can be moved onscreen after capture exclusion fails. | `slint-experiment/src/bin/overlay_host/window_lifecycle.rs:176–179`: `if let Err(e) = set_stealth(hwnd, on) { diag!("[overlay-host] stealth apply failed (window stays capturable): {e}"); }` | Failure to exclude an auxiliary window from capture does not prevent its onscreen move, and the separate tile path also only logs failure. The fallback keeps a window parked when its native handle is absent, but not when capture exclusion fails for an existing handle. |
| wave3_worker2_window#3 | P2 | The panel fallback moves it onscreen without confirmed capture exclusion. | `slint-experiment/src/bin/overlay_host/bar_tray.rs:460–464`: `set_global_stealth_effective(false);` … `if stealth_supported() && global_stealth() { surface_stealth_unavailable(o); }` | After retries are exhausted, the panel appears onscreen even with stealth enabled and no confirmed capture exclusion. The warning prevents a false success indication but cannot keep its pixels out of capture; failure to obtain the native window handle is unusual. |
| wave3_worker2_window#5 | P2 | Changing window style can reveal a panel that was hidden. | `slint-experiment/src/win32.rs:393–399`: `if after == before { return Ok(()); }` … `ShowWindow(hwnd, SW_HIDE);` … `ShowWindow(hwnd, SW_SHOWNOACTIVATE);` | When style bits differ, the function shows the window regardless of previous visibility or `BAR_TRAY_HIDDEN`, and the stealth handler calls it without checking tray state. Unchanged bits avoid the issue, and successful capture exclusion before restyling limits the leak claim, but unexpected panel visibility remains a defect. |
| wave3_worker2_window#11 | P2 | The effective-stealth indicators report the panel's result rather than protection for all windows. | `slint-experiment/src/bin/overlay_host/window_lifecycle.rs:553–560`: `let effective = global_stealth_effective(); sw.set_stealth_effective(effective);` … `sw.set_diag_stealth_on(effective);` | Settings and diagnostics can indicate success while a tile remains capturable because other windows' failures are only logged. The platform constant indicates mechanism availability rather than checking the Windows version, and reading back the panel's state protects its own indication but provides no guarantee for all windows. |
| wave3_worker3_tile#1 | P2 | Old streamed results can affect a new answer or restore a closed conversation. | `slint-experiment/src/bin/overlay_host/tile_controller.rs:163–165`: `if self.my_gen == self.current.load(Ordering::SeqCst) { self.inner.emit(channel, payload); }` | The generation check is not protected together with slot replacement, and synchronous `SlintEvents` forwarding reaches a handler that takes the current slot without rechecking, allowing concurrent requests to damage a new answer. Push-to-talk is independent of `ai_task`, F9 deletes the conversation without canceling its stream and later `Done` saves it again, while a shared body-and-window lock prevents a different mismatch but not this race. |
| wave3_worker4_settings#1 | P2 | Settings does not refresh all displayed values after import or reopening. | `slint-experiment/src/bin/overlay_host/settings_controller.rs:180–186`: `win.set_coaching_debrief(snap.post_meeting_debrief_enabled);` … `win.set_trigger_keywords_input(SharedString::from(snap.trigger_keywords.as_str()));` | Coaching, Auto-tiles, and retention are initialized only when the window is created, and import refreshes only the values handled by `populate_token_status`, leaving context and profiles unchanged. Reopening does refresh profiles, but not the listed switches. |
| wave3_worker4_settings#2 | P2 | Reopening Settings clears installation-running flags while background work continues. | `slint-experiment/src/bin/overlay_host/settings_controller.rs:1853–1857`: `win.set_tts_installing(false);` … `win.set_tera_installing(false);` | Reopening resets TTS, Tera, OCR, diarization, updater, and Components flags without stopping their threads, and TTS's duplicate-start protection depends on that flag. GigaAM preserves state through a guard, and Tera generations reject old callbacks, but these do not prevent the reset and another start. |
| wave3_worker4_settings#3 | P2 | An old model-list response can overwrite a newer list and selection. | `slint-experiment/src/bin/overlay_host/settings_ai.rs:479–484`: `let _ = slint::invoke_from_event_loop(move || {` … `let mut list = models;` | Overlapping requests can finish out of order because the callback checks only that the window exists, not request generation or the current endpoint and model. The callback itself does not save configuration. |
| wave3_worker4_settings#5 | P2 | Failed saves leave changed in-memory configuration without applying the matching runtime settings. | `slint-experiment/src/bin/overlay_host/settings_ai.rs:730–737`: `c.ai_prompt_cache = on;` / `if let Err(e) = overlay_backend::config::save(&c) {` … `return;` … `overlay_backend::ai::set_prompt_cache(on);` | On a save error, `SharedConfig` retains changes while runtime application is skipped, causing prompt-cache state to differ between memory, the static setting, and disk. Speech recognition locally compensates through `update_cloud_model` and restoring the selection, but that does not protect the other checked handlers. |
| wave3_worker4_settings#6 | P2 | Save failures can leave the UI, runtime, and disk with different settings. | `slint-experiment/src/bin/overlay_host/settings_controller.rs:425–434`: `st.stealth = on;` … `set_global_stealth(on);` … `c.stealth_enabled = on;` / `let _ = config::save(&c);` | Language and monitor changes apply before saving, stealth ignores save errors, and TTS voice or engine changes skip runtime application when saving fails. The defect requires a write failure; `stealth_effective` separately reports actual capture exclusion and does not mistake intent for successful protection. |
| wave3_worker4_settings#10 | P3 | Saving an empty value cannot remove either stored API key. | `slint-experiment/src/bin/overlay_host/settings_ai.rs:548–552`: `let trimmed = new_value.trim().to_string();` / `if trimmed.is_empty() {` / `eprintln!("[overlay-host] ai_bearer save skipped: empty input");` / `return;` | Both handlers reject empty values, and Settings has no separate deletion action for these keys. The field is intended for a new key and clears after Save while status still indicates a stored secret, so the claim that the UI shows “unset” is incorrect. |
| wave4_worker1_privacy#1 | P2 | Home-path redaction misses alternative path forms and standalone usernames. | `slint-experiment/src/bin/overlay_host/diagnostics.rs:216–219`: `let r = redact_home_in(s, home);` / `let r = redact_home_in(&r, &home.replace('\\', "\\\\"));` / `redact_home_in(&r, &home.replace('\\', "/"))` | Export replaces only the full home path in three forms, not usernames separately, short names, or mixed separators. It checks `USERPROFILE`, `HOME`, and `dirs::home_dir()` without ASCII case sensitivity, so a leak requires an alternative path or name representation in the original log. |
| wave4_worker1_privacy#2 | P2 | URL redaction leaves query secrets and some address forms visible. | `slint-experiment/src/bin/overlay_host/diagnostics.rs:127–130`: `[hay.find("http://"), hay.find("https://")].into_iter().flatten().min()` | Queries and fragments remain, while other schemes, standalone DNS names, and IPv6 addresses bypass URL masking, and later passes do not remove arbitrary secrets such as `token=secret123`. HTTP(S) user credentials, including mixed-case schemes, and standalone IPv4 addresses are protected; an actual leak depends on such data appearing in the log. |
| wave4_worker1_privacy#3 | P2 | Secret redaction recognizes only a limited set of token formats. | `slint-experiment/src/bin/overlay_host/diagnostics.rs:150–154`: `if rest.starts_with("Bearer ") {` / `rest = &rest[7..];` / `let tok_len = rest.find(char::is_whitespace).unwrap_or(rest.len());` | Different case, a tab, or a double space leaves arbitrary bearer tokens visible, and GitHub or Hugging Face prefixes and Basic authentication are not handled. The `sk-` branch covers `sk-ant-…`, `gsk_` has no preceding-character check, and the cited read-aloud sites log character counts rather than text, so secret presence in an exported log is not established. |
| wave4_worker2_installers#2 | P2 | Files can be replaced between verification and use because those operations rely on paths. | `overlay-backend/src/ocr_install.rs:60–63`: `verify_sha256(&tarball, BUNDLE_SHA256, "OCR")?; … extract_tar_bz2(&tarball, &root)` | Verification and use do not retain a protected open file; TeraTTS similarly checks `.part` before separately renaming it, and installed model files, including the sidecar, are checked only by size. Hash checks and atomic publication protect against damaged downloads, but not concurrent replacement by a process with write access, which is an unusual condition rather than proven privilege escalation. |
| wave4_worker3_cicd#1.1 | P2 | The local gate treats embedded Markdown inputs as documentation and skips Cargo checks. | `scripts/git-gate-native.ps1:74–76`: `$_ -notmatch '(^ | /)[^/]+\.(md | html | txt)$'` | Changes to embedded Markdown can pass the local push classification without Cargo. GitHub's classifier separately excludes `overlay-backend/knowledge/*`, and `.html` and `.txt` files outside `docs/` also fall outside its documentation-only rules. |
| wave4_worker3_cicd#1.4 | P2 | The local gate's fallback checks only the last commit rather than the pushed range. | `scripts/git-gate-native.ps1:43–45`: `& git.exe rev-parse --verify $Base *> $null` … `if ($LASTEXITCODE -ne 0) { $Base = 'HEAD~1' }` … `$diffArguments = @("$Base...HEAD")` | When the base reference is missing in ordinary linear history, a final documentation commit can hide an earlier Rust commit because the hook does not pass the actual base of the pushed references. GitHub independently classifies its own range. |
| wave4_worker3_cicd#1.5 | P2 | Git-quoted filenames can prevent the local gate from recognizing a Rust package. | `scripts/git-gate-native.ps1:55,60`: `$changedInput = @(Invoke-Git (@('diff') + $diffArguments + @('--name-only', '--diff-filter=ACMRD')))` … `ForEach-Object { $_.Replace('\', '/') }` | Quoted or escaped Git filenames are not decoded, so a leading quote prevents package-prefix recognition and can cause Cargo checks to be skipped. Ordinary spaces alone do not cause this, and GitHub's classifier uses NUL separators. |
| wave4_worker3_cicd#3.2 | P2 | The CI gate trusts a classifier that the pull request can modify. | `.github/workflows/ci.yml:53–55`: `result="$(bash .github/scripts/is-docs-only.sh "$BASE_SHA" "$HEAD_SHA")"` … `true)  echo "rust=false" >> "$GITHUB_OUTPUT" ;;` | Checkout supplies the classifier from the tree being checked, so replacing it with output `true` skips Rust and satisfies the gate, which does not depend on validate or macos. Classification-error checks do not verify the result's honesty, and whether merging is possible depends on branch protection settings not established by the source. |
| wave2_worker3_tts#1 | P2 | A read-aloud request has no size limit on either engine. | `overlay-backend/src/tts.rs:1256–1259`: `let b64 = base64::engine::general_purpose::STANDARD.encode(&spoken);` … `self.send_tera_speak(&format!("SPEAK {b64}"), spoken.chars().count())` | The whole text is encoded and sent as one line and split only after decoding; the command, job and PCM queues are unbounded as well. It needs a very large selection to matter. |
