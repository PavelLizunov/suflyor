# Omarchy full-port development evidence

Full-product acceptance remains open. The owner superseded the initial scoped
milestone with the complete contract in `docs/goal-omarchy-port.md`.

## Current functional evidence, not full acceptance

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
Still required: latest EN/RU all-tab audit, physical-microphone evidence, complete
speech-to-answer session, image/OCR answer, all physical shortcuts and PTT release,
retained-surface protection, drag/multiple scaled monitors, both speech engines'
controls, recording/history/summary/re-transcription/diarization, profiles,
provider sign-in, KB/memory/import/export, managed installation/model switching,
installer/autostart/update/uninstall, and final-candidate native regression gates.
Quattro integration uses native launcher/status interfaces; no plugin rewrite is
required or implemented.
No Linux release or desktop-owner deployment was performed. The private QA process
and wire fixture are task-owned services with bounded runtime. No owner desktop
configuration or heavy local build was used.

Private evidence is outside Git under the task's `omarchy` evidence directory:
matching before/after native PNGs, MCP trees, wire metadata, binary hash, worker
manifests, logs and exact exit markers. Published v0.38.1 assets are unchanged.
