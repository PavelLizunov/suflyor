# Omarchy full-port development evidence

Full-product acceptance remains open. The owner superseded the initial scoped
milestone with the complete contract in `docs/goal-omarchy-port.md`.

## Current functional evidence, not full acceptance

### October 22 continuation

- Exact `b4413d05bbbba3cb11cfb8eebabecd9b9fb418aa` native Linux aggregate
  driver completed with exit marker 0: 1,189 passed, zero failed, 11 ignored
  across 23 Cargo groups. This includes the complete backend (800 passed,
  four ignored), Linux port gate, selected guards, Piper, Tera and WSoLA.
  The earlier b6980fb job was OOM-killed and has no successful exit marker;
  its journal and interruption receipt remain retained, not relabeled PASS.
- Exact `03403c52ad4d149e6e1d95eed40784747761035f` host check and ui-mcp
  build passed on the dedicated worker; QA host SHA-256
  `a4fe0feaffec7ec2da6402b9f7a71433a2c0e6a345ee9b5aa4a158ccaf5aefee`.
  The installed QA process uses that exact executable. Its Windows targeted
  gate and macOS compile-seam gate both exit 0. GitHub CI 39706932118 and
  security 39706932177 passed. The Windows wrapper was restored byte-for-byte;
  S7 remains disabled. These checks do not cover later source candidates.
- Actual startup now completes native bar realization at (320,24), not the
  class-rule fallback (20,40). EN/RU Stealth shows verified capture exclusion.
  The before/after captures and actual compositor readbacks are retained.
  A later reused Settings attempt still logged an unresolved native handle;
  a subsequent close/reopen resolved it. Do not infer universal remap success.
- A physical/compositor F8 selection completed real Qwen3.5-0.8B image inference:
  the rendered and copied result identified the red square, blue circle and
  `HELLO SUFLYOR 123`. Request completed in 54.8 seconds. Russian grammar and
  speculative descriptions were imperfect; this is useful basic image-answer
  evidence, not general visual accuracy. Earlier native timeouts remain failures.
- Physical Ctrl+F8 on the measured text rectangle rendered exactly
  `HELLO SUFLYOR 123` through local Tesseract, then Tera read-aloud reached
  PLAYING and DONE. An earlier loosely selected crop returned inaccurate text;
  the accurate bounded selection does not establish all crop/scale scenarios.
- Closing an actual in-flight F8 tile caused llama-server to log cancellation
  of its task and slot release rather than waiting for completion. Independent
  completed Vision/OCR tiles survived until explicitly closed; live bar count
  changed 2 to 1 to 0. General concurrency, cap and AI-pulse acceptance is open.
- A real isolated session bus and disposable StatusNotifierWatcher registered
  the actual QA host, hid its bar, restored it on watcher loss, registered again
  after watcher recovery, and restored the bar through the actual SNI Activate
  method. Driver exit 0. The user's real desktop watcher was not restarted.
- Actual selected-text reading used the Wayland primary selection. Both Tera and
  Piper native tile controls passed monitored pause, resume, playback-speed and
  stop checks. Each steady-state window contains 40,000 samples at 16 kHz:
  Tera playing/resumed RMS 1727.65/1411.93, Piper 1350.07/1296.72; paused and
  stopped RMS 0. Earlier late/buffered captures failed and remain separately
  retained. No human listening verdict or full synthesis-rate comparison follows.
- Physical held microphone PTT captured the owned virtual input, transcribed
  repeated public fixture speech as `Проверка связи`, and reached a real AI tile.
  This is pipeline evidence, not physical-microphone or useful spoken-question
  acceptance. Initial attempts recorded silence; timing receipts are retained.
- All 16 EN and RU Settings tabs were captured at 720x600, top and bottom, using
  actual scrollbar-thumb drags. Four long pages in each language reached their
  continuation. Contact sheets and selected full-size provider/audio/diagnostic
  pages were inspected. This is not acceptance of all callbacks or integrations.
- Native installer lifecycle using the saved b6980fb executable and b4413d0
  installer source passed disabling autostart while retaining manual launch.
  Subsequent uninstall removed owned executables, launcher, desktop/icon and
  service, preserved 390 configuration/model files totaling 694,145,174 bytes
  byte-for-byte, and preserved unrelated compositor configuration exactly.
  The app was then reinstalled for 03403c5 QA; final-candidate cleanup is open.
- Further native UI acceptance exposed D-088: compact/full toggling changes the
  contents but leaves the old compositor size, and expansion can leave controls
  outside a 720px surface. Candidate eeca8ad adds compositor resize/readback;
  build/live compact, startup and tray-restore verification remains pending.

Private evidence roots on the worker: `final-native-b4413d0`, `native-03403c5`,
`live-03403c5`, `status-03403c5`, `pages-{en,ru}-03403c5`, `uninstall-b6980fb`.
Captured data, local endpoints and paths stay outside Git. Current source is a
later candidate than the installed/verified 03403c5 product; keep them distinct.

### Earlier independently observed slices

- `5b3b84e69025494aa1e50b117b96b985bbf542c6`: native host/library check and
  ui-mcp build passed. QA binary SHA-256
  `a283ab925078c5740557664046882289a3b3135dcc284b2707298d61426f179d`.
  Its complete managed speech driver exits 0: real managed GigaAM recognizes
  the pinned public Russian speech through both PipeWire streams and validates
  both recorded WAV channels; the previously failing short pinned-input
  recording also passes. Inputs are virtual, not a physical microphone.
- The same QA host's Settings Test voice invokes real Piper synthesis and
  produces a monitored native output waveform: 288,000 samples at 16 kHz,
  RMS 703.04 and peak 15,306. This verifies output transport, not a human
  listening verdict or all read-aloud controls.
- `2c9f2fc3b1877729d19214fefd8d491754d6bd86`: native host check/build passed;
  binary SHA-256
  `79d6fa662cdd592c2fbeaa75ba69a48a6a9a4d6b95113c3343a495127ac2dc13`.
  A real Apache-2.0 Qwen3.5-0.8B CPU server answered the typed native question
  with a correct database-index definition; the Slint tile rendered it and
  Wayland clipboard Copy returned the same text. First token 52.8 seconds,
  total 59.3 seconds on the dedicated CPU worker. Earlier SmolVLM attempts
  timed out or produced nonsense; they do not establish semantic acceptance.
- The same host fixes the first-map identity failure: bar placement is native
  at (320,24), and bar/first Settings exclusion reads true. Actual retained
  Settings reopen reads false; D-082 records the reproduced surface-recreation
  defect and subsequent source repair. The repair still needs native retest.
- A real VM input event (`qm sendkey`, F1) reaches the compositor shortcut,
  logs the distinct F1 dispatch and opens Help. Other physical shortcuts and
  held PTT remain pending. Native region selection starts slurp and Esc
  cancels it; no OCR/Vision result is claimed by this cancellation check.
- Native Windows targeted gate passes at
  `7fc843ce935d24759c00dec9c0988714b713003a`; wrapper restored byte-for-byte,
  S7 disabled. Native macOS compile-seam gates pass at
  `2c9f2fc3b1877729d19214fefd8d491754d6bd86` and
  `3b5735f82d6e1365cb3f97d0a9650172c8159df1`, both exit 0. These are distinct
  candidates; later UI changes are not covered by their evidence.
- Native Codex 0.155.1 initializes with the preserved text-only permission
  profile and reads signed-out account state under an isolated home and Unix
  desktop environment. No user login or authenticated provider response was
  performed.

Private exact identities, logs, output PCM, MCP trees and captures remain in the
worker's task-owned evidence directories. Current HEAD/gates and installed state
must be checked afresh; the historical sections below are not current parity.
No owner installation, Linux release, merge or stable publication occurred.

## Identity and baseline

- Original baseline: 8c70b6d193b3a4c84e8e20a8cc56b0681f60618c.
- Baseline backend check on omarchy-test: exit 101, four missing platform branches.
- Backend repair f5d4258f3693eb6321a1b11d20b64410538adeec: check exit 0;
  three capability tests pass.
- Native UI candidate: 5cadbb101de3077c9a8610d0190a3c3211def0ad,
  tree 570a9638eb5a4ed9c63baf5de1b4ec9255a3e35f.
- QA binary SHA-256:
  `39477eeee12ceb76fc3f239f4346bc6112682e97e5c365006162c056ce31110d`.
- The subsequent candidate 0eb2a9e792db918bbbbb7ac9a9c705465774fd10 adds
  only the checked Hyprland snippet, developer docs and serial Linux test driver;
  product Rust/UI sources match the native UI candidate. Its Windows targeted gate
  passed. Linux check passed; the complete Cargo integration invocation was stopped
  after its already-built test executable passed, because Cargo also rebuilt unused
  demo binaries. This is an interrupted gate, not a full-driver PASS.

## Native checks actually observed

- `cargo check --locked --bin overlay-host --features ui-mcp` passed on Linux.
- `cargo build --locked --bin overlay-host --features ui-mcp` passed on Linux.
- Windows targeted gate against 5cadbb1, 2bd39ad and 0eb2a9e: exit 0.
- Omarchy 4.0.4, native Wayland (`xwayland=false`), existing Hyprland session.
- Real Slint MCP captures inspected in English and Russian: bar, Settings profile
  page, typed-question window, and answer tile. No shared UI layout was edited.
- Typed question dispatch reached an isolated local OpenAI-compatible HTTP fixture,
  emitted two streaming deltas and rendered the exact synthetic answer. This is
  transport/render acceptance, not real provider or model inference.
- Tile Copy dispatched through MCP and the actual Wayland clipboard contained the
  synthetic answer. The ignored live clipboard round-trip test was explicitly run
  through the user's service manager: one passed.
- Native-boundary integration executable: one passed, one intentionally ignored
  before the explicit live run. Unsupported capture/exclusion never reports success.
- Second same-config process exited with `singleton lock busy` while the first
  remained alive. The existing Unix file-lock implementation is reused.
- The actual Hyprland client list confirms floating native windows: bar 1280x64,
  Settings 720x600, typed question 480x98, tile 460x360. No shell restart was needed.
- First runtime reproduced D-073: tiled bar/Settings, precreated capture surface and
  unsupported startup audio probe. Linux bootstrap now skips those probes; explicit
  checked Hyprland rules supply geometry. winit's fixed-size hint alone was insufficient.

## Resource and test limitations

The dedicated VM began with 4 GiB and spent substantial time swapping during the
large generated Slint host compilation. Only VM 107 was cleanly stopped and raised
to 8 GiB after checking physical-host headroom; SSH and Hyprland recovered. Builds
remain managed and memory-bounded. A two-job integration-test invocation compiled
two large binary targets concurrently and was interrupted for memory pressure;
the checked driver now serializes that step. The already-built actual test
executable and its live clipboard test passed again at 0eb2a9e, but the serialized
Cargo invocation was also stopped while rebuilding unrelated demo targets. Its
full-driver exit marker is absent; no complete Linux gate PASS is claimed.

## Completed scoped Linux driver correction

The interrupted results above remain historical failures, not acceptance. The
corrected entry point completed on the same dedicated worker without interruption:

- Candidate: `68ed25134c11f4b57ac9f0242f64a75c963b3e1b`.
- Tree: `99f2a9e50994230b4e694e9854675a7079077ddf`.
- Command: `bash -x scripts/check-linux-port.sh`; exit marker **0** and
  final `Linux port gate PASS`; worker checkout remained clean.
- Run: October 21, 2026, 06:25:29–07:50:27 +03:00; 84m58s including initial
  compilation of the requested large generated Slint test targets.
- Cargo 1.98.1; one build job, one test thread, incremental compilation disabled,
  dev debug symbols disabled, Slint debug metadata enabled. Managed task service
  used a two-hour runtime bound, MemoryHigh=6G, MemoryMax=7G, MemorySwapMax=6G.
- Host/library check passed in 9m45s. Cargo test completed in 72m24s:
  library **82 passed / one ignored**, overlay-host **146 passed / one ignored**,
  linux_platform **one passed / one ignored**.
- Backend capability tests **three passed**; Linux installer rejection **one
  passed**; update tests **three passed**. Total **236 passed, zero failed, three
  ignored across six groups**. Filters intentionally exclude the unrelated backend
  suite. The ignored tests are a microbenchmark, live audio and live clipboard.
- This is a complete scoped Cargo driver PASS, not a full Linux test suite or
  Clippy `-D warnings` pass. Existing Linux unused-code/variable and Slint warnings
  remain. No real audio, capture, shortcuts or model inference is implied.
- Windows targeted gate on the same candidate/tree: exit **0**, **1,109 passed
  across 23 groups, five ignored**. Its scheduled-task wrapper was restored
  byte-for-byte; the old S7 job remains disabled. No new macOS gate was run.

D-074 was reproduced with a tiny broken unrelated demo on the installed Cargo:
explicit `--bin` selection alone, and `cargo rustc --test`, both still compiled
the demo. A default-enabled `demo-binaries` feature preserves ordinary builds;
the Linux driver opts out with `--no-default-features`. The negative fixture then
passed library, host and platform tests without compiling that demo, while a
default invocation still selected it. The actual corrected driver ran the real
project tests, not a substitute fixture or manually invoked test executable.
A read-only independent source review found no blocking defect; it did not run
the native checks.

Private exact-SHA logs and manifests: task evidence directories
`omarchy/linux-68ed251/` and `omarchy/windows-68ed251/`. Product Rust/UI sources
still match the previously inspected native 5cadbb1 milestone. D-072, D-073 and
D-074 remain open until reviewed integration, not closed by this branch checkpoint.

## Remaining full-product acceptance

Current native sources connect audio, capture, compositor IPC, local-model setup,
Settings, native speech output and status controls. This is not full acceptance.
Still required: final-candidate detailed EN/RU audit and callbacks,
physical-microphone evidence, useful spoken-question-to-answer session, all
physical shortcuts and both PTT routes, scaled/multiple-output image capture,
retained-surface protection, native compact/full/drag/monitor/lock behavior,
synthesis-rate/seek and session audio controls, recording/history/summary/
re-transcription/diarization, profiles, provider sign-in, KB/memory/import/export,
managed AI installation/model switching, final installer/autostart/update/uninstall,
and final-candidate native regression gates. The October 22 basic image/OCR,
output-control and watcher evidence above does not replace those broader checks.
Quattro integration uses native launcher/status interfaces; no plugin rewrite is
required or implemented.
No Linux release or desktop-owner deployment was performed. The private QA process
and wire fixture are task-owned services with bounded runtime. No owner desktop
configuration or heavy local build was used.

Private evidence is outside Git under the task's `omarchy` evidence directory:
matching before/after native PNGs, MCP trees, wire metadata, binary hash, worker
manifests, logs and exact exit markers. Published v0.38.1 assets are unchanged.
