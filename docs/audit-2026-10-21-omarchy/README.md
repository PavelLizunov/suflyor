# Omarchy full-port development evidence

Full-product acceptance remains open. The owner superseded the initial scoped
milestone with the complete contract in `docs/goal-omarchy-port.md`.

## Current functional evidence, not full acceptance

### October 22 queue cancellation correction

The owner's accumulated-question report is reproduced in the real f6b6037 UI:
closing a speech-triggered auto tile leaves zero tile windows, but its silent
HTTP generation is still active after three seconds. This is D-094, a shared
host ownership defect, not a Linux-only provider problem. Previous Vision-close
acceptance did not cover auto tiles, PTT, F9 or their continuations.

- Exact candidate `2f3f2fd3ab2427452c2fa4300e52202dc5abc652` passes native
  Linux host check/build. Installed ui-mcp host SHA-256:
  `ec38f60293a85c9a3ab91c9cf4819fd0a7dedafa6b80147bfc951f913d756b36`.
  Its targeted Cargo driver exits 0: 376 passed, zero failed, five ignored
  across nine groups, including auto silent-close and independent task-owner
  regressions. The separate actual scripts/check-linux-port.sh exits 0:
  263 passed, zero failed, four ignored across six groups. These groups overlap;
  their totals are not a combined unique-test count or a complete backend run.
- Exact native Windows targeted and macOS compile-seam gates exit 0.
  CI 39790776837 and security 39790776896 pass. The Windows job wrapper is
  restored byte-for-byte; SuflyorClaudeS7Job remains disabled. The read-only
  review found no remaining source blocker and executed no checks itself.
- Actual GUI queue scenario uses the existing ui-mcp finite transcript input
  and a bounded silent HTTP provider. Two auto requests occupy both permits;
  a third visible auto tile waits. Closing the queued tile sends no HTTP.
  After a wanted typed question queues, closing one active auto tile disconnects
  only its provider request and admits the wanted request in 0.539 seconds.
  Its controlled answer renders; the other active auto remains intact.
  Close-all then retires the remaining connection, with no late tile or queued
  request appearing. This validates scheduling/cancellation, not AI semantics.
- Actual pending typed/F9 Close disconnects its provider in 0.271 seconds.
  Physical held Mic PTT, real virtual PipeWire replay and native GigaAM reach
  a pending HTTP request; its actual Close disconnects in 0.191 seconds.
  Russian pending typed-tile Close also disconnects. EN/RU pending, queued and
  completed screenshots are inspected at 460x360. No layout/translation source
  was changed, and no general physical-microphone acceptance is inferred.
- After restoring the real authenticated gateway, an actual typed answer
  renders a useful database-index definition and SQL example. Real follow-ups
  complete in about 4.6/4.3 seconds; the actual scrollbar exposes the product
  SKU unique-index/SELECT result. Initial assertions inspected only clipped
  viewport content and then required the exact phrase CREATE INDEX instead of
  valid CREATE UNIQUE INDEX; those verifier failures remain retained.
- The first combined speech/PTT queue attempt exceeds the provider fixture's
  original 100-second hold while GigaAM runs, so its saturation assertion fails.
  It is not the successful queue scenario. The later finite-transcript queue
  and separate real physical-PTT-close checks above have distinct inputs.
  Follow-up Close reaches provider cancellation in the sibling probe; clicking
  Regenerate through MCP popup text/row does not establish dispatch, so that
  native menu path and a cap-eviction stress scenario remain unaccepted.

Private receipts: native-2f3f2fd and queue-live-2f3f2fd. The temporary transcript
service override is removed; the bounded queue provider is stopped; English,
real provider and detector configuration fields are restored. The app is stopped
for subsequent checks. Original checkout HEAD, raw index and all 13 pre-existing
file hashes remain unchanged. No owner deployment, release or merge occurred.

### October 22 completed session and service checks

- Exact 658d03e complete Linux driver exits 0: 1,197 passed, zero failed,
  11 ignored across 23 Cargo groups. Exact f6b6037 affected Linux guard driver
  passes eight tests with one ignored; exact host build produces the same bytes
  as 658d03e because production Rust/UI inputs are unchanged:
  `6fde3f07a97bc95f9b24d098d55d013b7402a06d7992ead242fe777c928e0999`.
  f6b6037 native Windows/macOS gates and CI 39765001581/security 39765001611 pass.
- Exact 55943c4 both physical held Mic/System requests render useful real
  authenticated-gateway definitions/examples in approximately 7.95/7.18 seconds.
  A real session persists six transcript lines, one successful AI response and
  one tile. Pause suppresses transcript; Resume continues the same session;
  Stop finalizes it. Summary, archive indexing and manual rename/reopen pass.
  Generated-name success and summary reopen/persistence remain separate gaps.
- Exact f6b6037 actual delayed F6 followed by Stop/new session produces no late
  tile, old successful response or new-session AI rows. A pending native PTT
  cancels at the new-session boundary and shows interrupted feedback. This is
  a controlled delayed-provider lifecycle regression, not model inference.
- Two actual managed service stops remove all owned shortcuts and capture.
  A stop during active dual recording finalizes session_stop/session_summary
  and both nonempty valid WAV files; observed stop is about 10 seconds.
  The system journal shows normal stop, not a timeout kill. No-host stop,
  final update/autostart/uninstall and arbitrary-load shutdown remain separate.
- Exact f6b6037 transcript playback produces monitored sound after timecode
  seek, pause/resume and speed 1.25x/volume changes. Each measured segment has
  25,600 samples: playing RMS 1882.72, paused 0, resumed 1163.44 and changed
  controls 5176.90. The high volume clips at 32767; this is not audio-quality
  acceptance. Speed and volume return to 1x/1 afterward. Physical Ctrl+F8
  again renders exact fixture text. Three reloads preserve the host PID,
  18 owned shortcuts and a foreign conflict. Full/compact and four compact
  tray restores preserve native dimensions and positions.

### Earlier October 22 native continuation

- Exact `a07c5ac48e0d20e7de1b174c9cbcea606d9379c2` host check and ui-mcp
  build exit 0. Installed and QA executable SHA-256:
  `6947b089102d09a64182fc7545b660cf39147ae4ceffaece2184ac6e9f0bdc2c`.
  Windows targeted and macOS compile-seam gates exit 0; CI 39725243799 and
  security 39725243814 pass. This is not a current complete Linux test-driver
  run; the complete b4413d0 result below remains a distinct candidate.
- Settled compositor geometry passes full 1280x64 at (320,24), compact 680x64
  at (620,24), expansion, four compact hide/restore cycles and cold compact
  service startup. All four retained Settings remaps also preserve native
  identity and capture exclusion, including reused compositor addresses.
  The cache keeps weak native owners so hide still destroys Wayland surfaces.
- Actual GUI session start/stop records both owned virtual PipeWire streams.
  GigaAM recognizes a useful spoken database-index question on both System
  and Mic; three transcript lines and both WAV recordings persist. Archive
  opens the completed session; View transcript and Copy all consume its real
  lines. Physical microphone evidence remains unavailable, not implied.
- Actual transcript timecode playback produces 84,800 monitored samples,
  RMS 1890.181 and peak 16,306. Installing speaker models verifies both pinned
  files; native one-speaker diarization completes after 1,276.7 seconds and
  persists after closing/reopening the transcript. By voice shows the actual
  inferred speaker. Multi-speaker accuracy, performance, cancellation and the
  complete transcript-player control matrix remain open. Concurrent native
  playback/inference logged CPAL buffer underrun/overrun; this is retained.
- F1 Help, F4 Palette, F7 Archive, F3/F6 empty-state notices, typed-question Send
  and Shift+F9 unconfigured-cloud feedback are physically dispatched. A useful
  typed database-index definition renders, but later F9/F6 answers are late or
  off-topic. One F6 result arrives after session stop without an ai_response
  journal row; timely session AI/storage acceptance is not established.
- Held System PTT recognizes the complete useful spoken question. Its first
  AI attempt times out at 120 seconds. A byte-preserving local QA relay then
  confirms the real request includes enable_thinking=false and contains no
  reasoning output. The next request completes but merely repeats the question
  as transcript text. This disproves the proposed missing-no-think explanation
  for that run; it is not useful spoken-question answer acceptance.
- Physical header dragging moves the question window and saves its position.
  Reopening can add 7px Y repeatedly (D-089). A native A/B probe changes only
  the owned compositor rule from 480x112 to the rendered 480x98: baseline
  saved Y advances 606→613→620, while all three 98px-rule cycles retain 620.
  The probe restores the original installed rule and validates empty compositor
  configuration errors. The committed correction needs exact-candidate retest.
- Actual Profile + context creates, selects and saves a disposable profile;
  the question window displays that active profile. Delete removes the profile.
  Real native portal dialogs export server JSON, preview import changes and
  apply them while retaining language, devices, local GigaAM path and profile
  fields. Full backup/import and provider authentication remain separate.
- Scale-1.5 Settings renders at logical 720x600, raster 1080x900. A negative-X
  owned headless-output OCR selection does not complete and is canceled with
  Escape; all checked own windows restore capture exclusion. The owned output
  is removed and the primary output returns to scale 1. No successful scaled
  multi-output capture is claimed.

Private latest receipts: `native-a07c5ac`, `live-a07c5ac`, its `stream-relay`
subdirectory, and exact platform-gate evidence. QA audio fixtures remain owned
and loaded for continuing acceptance; final restoration/cleanup remains open.
No merge, Linux publication or owner-desktop installation occurred.

### Earlier October 22 continuation

- Exact `b4413d05bbbba3cb11cfb8eebabecd9b9fb418aa` native Linux aggregate
  driver completed with exit marker 0: 1,189 passed, zero failed, 11 ignored
  across 23 Cargo groups. This includes the complete backend (800 passed,
  four ignored), Linux port gate, selected guards, Piper, Tera and WSoLA.
  The earlier b6980fb job was OOM-killed and has no successful exit marker;
  its journal and interruption receipt remain retained, not relabeled PASS.
- Exact `03403c52ad4d149e6e1d95eed40784747761035f` host check and ui-mcp
  build passed on the dedicated worker; QA host SHA-256
  `a4fe0feaffec7ec2da6402b9f7a71433a2c0e6a345ee9b5aa4a158ccaf5aefee`.
  That QA run used the exact executable. Its Windows targeted
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
Captured data, local endpoints and paths stay outside Git. These historical
03403c5 slices do not verify every state of the later installed a07c5ac product.

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
