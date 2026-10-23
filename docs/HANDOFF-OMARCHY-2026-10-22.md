# Claude Code handoff: full Omarchy port, 2026-10-22

Status: development and acceptance in progress, **not a completed port**.
The owner requested this written handoff on October 22. Stop this coordinator's
implementation here; the next agent continues the full still-open outcome.

## 1. Recover the actual goal and checkout

Owner's controlling requirement: full Suflyor functionality adapted to
Omarchy/Hyprland/Wayland, not just compilation, rendered fixtures or green tests.
The explicit goal is in [goal-omarchy-port.md](goal-omarchy-port.md). Its unchecked
acceptance rows remain open. The latest queue malfunction is an additional shared
product defect, not a replacement for the full port.

- Task worktree: `~/Work/sufloyr-omarchy`, branch `codex/omarchy-port`.
- Repository: <https://github.com/PavelLizunov/suflyor>.
- Draft PR: <https://github.com/PavelLizunov/suflyor/pull/307>, OPEN, unmerged.
- Last code candidate: `2f3f2fd3ab2427452c2fa4300e52202dc5abc652` (Codex); see section 10 for later commits.
- Evidence checkpoint before this handoff: `5d739d9652ecddc3c25b0e19d45bc8680b93ec72`.
  This handoff is a later documentation-only commit; obtain its SHA from Git.
- Integration base used for native gates:
  `6a5f0f53a6d1fb1a67547c817c9920acec55a29c`.
- No merge, Linux release, owner-desktop deployment or stable-asset replacement
  was performed. Do not interpret a development checkpoint as those actions.
- The existing `docs/CODEX_HANDOFF.md` says master and contains older master
  history. It is not this branch's current operational state. It was preserved,
  not overwritten. This file is the current branch-specific handoff.

First inspect `git status --short --branch` and recent `git log`; then read root
and relevant nested AGENTS, the goal, [the current audit](audit-2026-10-21-omarchy/README.md)
and [the defect ledger](DEFECT-LEDGER.md). The repository has five independent
Rust crates, no root Cargo workspace.

## 2. Preserve the original checkout

The owner's original `~/Work/sufloyr` checkout is NOT the task worktree.

- Original HEAD: `a15a25817a5e61d69274a8c2da743bc5ecb98afa`.
- Original branch: `claude/integration-1013`, former upstream gone.
- Raw index SHA-256:
  `386387f61228f4d870ef015edf98eaa3b011f1fb5d7eb6caa11542a69837b9ab`.
- Thirteen pre-existing modified/untracked files remain outside the task.
  They include instruction files and `docs/ADR-003-local-model-harness.md`.
- Preservation manifest:
  `~/.hygiene/sufloyr/rc13-pi/publish-original-state.json`.
- At handoff preparation, HEAD, raw index and all thirteen file hashes were
  checked against that manifest and match. Do not stage, reset, stash, clean,
  revert or overwrite this unrelated work.

## 3. Latest owner-reported queue bug: D-094

The owner reports that many questions accumulate, an important answer waits for
unwanted ones, and closing a tile apparently does not stop generation.

### Actual cause and baseline

The shared AI semaphore permits two requests. Most tile close handlers removed
the window but did not cancel the task holding the AI receiver. Backend receiver
closure cancellation already worked, but these detached callers kept receivers
alive. Initial Vision and +tile had task registration; auto tiles, PTT, F9 and
continuations generally did not. Prompt-building callbacks could also start
network work after the tile closed. This affects shared Windows/macOS/Linux host
code, not just Wayland.

Exact f6b6037 native reproduction: real speech creates an auto placeholder;
actual Close removes the window, but its silent HTTP generation remains active
three seconds later. The baseline and an earlier failed speech-detection setup
are retained separately under private `queue-baseline-f6b6037` evidence.

### Code correction in 2f3f2fd

- Every tile gets a synchronous tile-id when common tile wiring runs.
- The existing task registry owns an empty preparation entry before the task
  exists. Close removes it; registering a late task for a closed owner aborts
  that task instead of recreating ownership.
- F9, PTT, follow-up and regeneration register their task abort handles.
  Close, close-all and cap eviction share cancellation; Vision and read-tile
  close paths also retire the same owner.
- Auto placeholder labels map to owners in a synchronized bridge map. The
  auto stream checks this ownership during waiting/retries and returns on
  cancellation without caching, billing or completing the closed tile.
- Close-all increments the spawn generation under the same lock; pending
  old-generation spawns are ignored. Queued auto deltas recheck ownership on
  the UI thread and cannot repopulate early buffers after close.
- Shared stream sinks release only their own pulse on Done/Error/Drop.
  Latest-wins F9/follow-up behavior was preserved, not redesigned into a new
  scheduling policy. Increasing concurrency was not used to mask orphan work.
- Added actual behavioral regressions for silent auto cancellation and
  independent active/queued/late task registration.

Owning files: `overlay-backend/src/events.rs`; `slint-experiment/src/slint_events.rs`,
`slint_session.rs`, `bin/overlay_host_windows.rs`, and
`bin/overlay_host/{tile_window,tile_controller,tile_ask,tile_ptt,tile_followup,vision_capture,read_aloud}.rs`
plus the archive content-close path.

### Exact verification, not a full-port verdict

- Linux host check/build passed. Installed/QA executable SHA-256:
  `ec38f60293a85c9a3ab91c9cf4819fd0a7dedafa6b80147bfc951f913d756b36`.
- Targeted Linux Cargo driver: exit 0, 376 passed, zero failed, five ignored
  across nine groups. These include 93 library, 155 host, eight Linux-platform
  and 120 filtered backend AI tests; remaining groups run zero matching tests.
- Separate actual `scripts/check-linux-port.sh`: exit 0, 263 passed, zero failed,
  four ignored across six groups. It overlaps the preceding groups. Do not sum
  them as unique tests or call this a complete backend run.
- Native Windows targeted gate and macOS compile-seam gate: exit 0.
- CI 39790776837 and security 39790776896: success on exact 2f3f2fd.
- Read-only independent review found no remaining source blocker. The reviewer
  did not execute checks. Several initial review allegations were refuted by
  the actual UI-callback placement and preserved latest-wins policy.
- Actual saturated GUI queue: two silent active auto requests, one queued auto.
  Closing the queued tile sends no HTTP. Closing an active auto disconnects
  only its request and admits a wanted typed request in 0.539 seconds. The
  unrelated active auto remains. Close-all disconnects the remainder with no
  late tile. This scenario uses finite QA transcript input and a controlled
  provider; it measures scheduling/cancellation, not model inference.
- Actual pending typed/F9 Close disconnects in 0.271 seconds. Actual physical
  held Mic PTT with virtual PipeWire speech/GigaAM disconnects in 0.191 seconds.
  Russian pending typed Close also passes. Physical microphone remains unknown.
- Actual follow-up Close disconnects in the sibling probe. That probe later
  fails at the MCP popup interaction; do not discard the earlier logged
  follow-up cancellation or call the whole probe a pass.
- Later physical QEMU pointer input on the measured Regenerate popup row
  actually dispatches regeneration and renders a useful real gateway answer.
  Against the delayed provider, physical Regenerate then Close disconnects;
  its exact receipt is `regenerate-close-result.json`. The script's printed
  elapsed value includes subsequent MCP work, so use the receipt for timing.
- Actual 16-tile cap evicts the oldest streaming auto and its provider request
  disconnects. Initial enumeration sees a legitimately destroyed handle while
  eviction runs. A later settled readback confirms 16 remaining tiles and the
  canceled owner, followed by close-all. Receipt: `cap-result.json`.
- Restored real authenticated gateway returns useful typed SQL answers and
  continuations. Initial continuation assertions inspected only the clipped
  viewport, then rejected valid CREATE UNIQUE INDEX syntax. Scrolling the
  actual tile exposes the useful product-SKU example; failures are retained
  as verifier errors, not relabeled product failures.

UI captures were inspected in English/Russian at 460x360. There was no Slint
layout or translation-source change. The fix is installed only on the dedicated
worker and pushed in the draft development branch. **The owner's installed
v0.38.1 does not contain it.** A release/backport/delivery decision is still
separate; do not tell the owner their production installation was repaired.

## 4. Port implementation already present

The branch reuses Rust/Slint, not a speculative QML rewrite/plugin.

- Native PipeWire simultaneous microphone/system PCM, selected routes,
  diagnostics, finite recording, held press/release PTT and owned child cleanup.
- Linux ONNX GigaAM CPU STT. Dedicated VM CPU exposes x86-64-v3/AVX2 because
  linked ONNX initialization crashes before main on the old non-AVX VM CPU.
- Piper and Tera remain separate processes, with native CPAL output; voice
  downloads, activation and actual output have been exercised in narrower runs.
- Hyprland compositor identities, floating geometry, placement, drag/pin,
  full/compact bar, stable application ID and first-frame capture protection.
- Weak native window identities expire on hide/remap. Never retain a strong
  winit Arc across hide: Wayland hide must destroy the native window.
- Native shortcut registration, private configuration-owned IPC, clipboard and
  primary selection; file/URL opens, grim/slurp and Tesseract OCR.
- Native setup/resource/update adapters, architecture-compatible asset choice,
  checksum verification, safe extraction, transactional activation/rollback and
  ownership-aware server stopping. Manual fixtures do not prove the full
  managed installer lifecycle.
- StatusNotifierItem with watcher-loss restoration, desktop/CLI launcher,
  managed service, optional autostart and preservation-oriented uninstall.
- Codex Unix executable/environment discovery and text/image route support;
  native signed-out initialize/account-read verified, authenticated login not.
- Linux updater validates the official URL and opens the release page. It does
  not falsely claim an automatic Linux package update was installed.

Important earlier fixes: D-078 pinned audio routes; D-079 binding ownership;
D-080 capture transparency/exclusion order; D-081 install/manual launch;
D-082/D-088 weak remap identity and native compact sizing; D-084 stable close
sequence; D-083 provider discovery/managed restart; D-085 watcher recovery;
D-086 silent Vision cancellation; D-087 native
installer test fixtures; D-089 rendered question height 98px instead of112px;
D-090 reload subscription/re-registration; D-091 SUPER-first unbind spelling;
D-092 session-boundary request cancellation; D-093 graceful managed stop.
Ledger entries remain Open until verified reviewed integration, not merely a
checkpoint push.

## 5. Candidate-specific acceptance already observed

Do not silently transfer a historical result to the current executable.

| Candidate | Actual scope and result |
| --- | --- |
| b4413d05bbbba3cb11cfb8eebabecd9b9fb418aa | Complete Linux driver: 1,189 passed, zero failed,11 ignored,23 groups. Earlier b698 OOM run is interrupted, not PASS. |
| 03403c52ad4d149e6e1d95eed40784747761035f | Useful real Qwen Vision red-square/blue-circle/text answer; accurate physical Ctrl+F8 OCR; Vision-close server retirement; Piper/Tera pause/resume/rate/stop PCM; real isolated watcher recovery; all16 EN/RU Settings captures top/bottom, not all callbacks. |
| a07c5ac48e0d20e7de1b174c9cbcea606d9379c2 | Settled compact/full/cold/remaps; real dual speech session/WAV/archive/transcript; one-speaker diarization1276.7s and persistence; profiles/context and native server import/export. Slow/off-topic local-model answers remain failures. |
| 55943c49f6725324edfdeb118a11060bef191a8e | Complete Linux driver1194/0/11ignored/23groups. Both physical held Mic/System with virtual replay produce useful authenticated-gateway answers, AI phases7.95/7.18s. Real session six transcript lines,one successful durable AI response; pause/resume; Stop; summary/archive/manual rename. Three real reloads/18bindings/foreign conflict, five settled question restores. |
| 658d03e1176c9d905fb491698909282077082d2e | Complete Linux driver1197/0/11ignored/23groups, including D092 semantic tests; exact host build and Win/macOS/CI pass. |
| f6b6037428cc9302fd9dab188eec5619a9ebb56b | Exact build/affected guards/Win/macOS/CI pass. Actual delayed F6 Stop/new-session cancellation and PTT interruption; two managed stops; active dual-WAV/journal finalization. Transcript seek/pause/resume/speed1.25x/volume PCM; physicalOCR/reload/compact remaps. |
| 2f3f2fd3ab2427452c2fa4300e52202dc5abc652 | Current code, targeted gates and D094 queue/close/current real answer evidence described above. Not a new complete1197-test run. |

The exact658 and f6 host builds share hash
`6fde3f07a97bc95f9b24d098d55d013b7402a06d7992ead242fe777c928e0999`
because f6 changes installer/guard, not production Rust/UI. Both were actually
built. Superseded790 Linux check passed but aggregate/build was interrupted;
keep its interruption receipt. Physical microphone, multi-speaker accuracy and
arbitrary model semantic/performance quality are not implied by virtual replay.

## 6. Worker state verified for handoff

Fresh readback: October22, approximately19:52MSK. Recheck before starting work.
Exact private operational paths, access helpers and restoration receipts are in
`~/.hygiene/sufloyr/omarchy/CLAUDE-CODE-HANDOFF.md` on the control machine.

- Omarchy checkout: detached exact2f3f2fd, clean. Local task branch additionally
  contains documentation checkpoints. Do not switch the worker during a job.
- Installed host is exact2f3f2fd, hash above. Service inactive, MainPID0.
  No Cargo/rustc/host/capture/local model/TTS process is active.
- Eighteen owned shortcuts were removed by shutdown; fresh readback is zero.
  Primary monitor restored to1920x1080,scale1,origin0; no extra output.
  `hyprctl configerrors` is empty.
- A final attempted monitor command used `dispatch hl.monitor(...)`; the Lua
  side effect applied scale1.5 before dispatch rejected its return value.
  This was discovered during handoff inventory, restored with native `eval`,
  then reload/readback verified scale1 and no configuration errors. This was
  not a successful scaled capture and is not left applied.
- Controlled queue provider stopped. Temporary QA transcript service drop-in
  removed. Actual cloud gateway bearer retained privately; cloud route,
  English, full bar, stealth enabled, fast answers enabled, auto-tiles disabled,
  no trigger keywords and no tile suppression are restored.
- Owned virtual audio fixture remains loaded: sink suflyor_ui_speech and source
  suflyor_ui_input are still defaults. This is intentional resumable QA state,
  not final cleanup. Original defaults/module IDs are in private receipts.
- Disk about8.6GiB free of48GiB. VM8GiB RAM, task-local serial build settings.
- Windows SuflyorPostReleaseDocs Ready; SuflyorClaudeS7Job Disabled. Original
  wrapper restored with SHA-256
  `972DC1462BFEF4F84FABA784B4D6E702A3390147FD42E5D93F3B3949A5EBF728`.
  No Windows builders. Latest Windows2f3 gate exit0.
- macOS2f3 compile-seam exit0; no Cargo/rustc process. Its dedicated launch-agent
  definition remains installed, job completed. No new builds were started for
  the handoff.
- Two historical failed transient QA units may still appear in list-units;
  they have no running process and are not failed builds. Keep evidence.

## 7. Remaining full-product work

Do not turn the following into a claim that every item is broken. They are
unfinished implementation or missing actual-consumer acceptance.

1. Successful scaled/multiple-output capture and placement, including negative
   coordinates; initial centering before settled placement; pin/locks/cascade;
   full physical shortcut/non-US editing/release/concurrency coverage.
2. Final current-candidate session/history: summary reopen/persistence, debrief,
   generated name, re-transcription, deletion/recovery; multi-speaker diarization
   accuracy/cancellation/errors/performance and broader player controls.
3. Managed Linux AI installer through the actual UI: verified engine/model
   download, cancellation/resume, custom GGUF/projector, activation/switch,
   rollback, route preservation and unrelated-server protection. The manual
   small llama.cpp/Qwen fixtures are not managed-install acceptance. The worker
   last had no managed local-AI root; check current disk before a large download.
4. Actual provider/account authentication, including Codex login; KB/memory,
   full backup/import and Hermes installation/integration. Server-settings
   transfer and profile/context are already narrowly verified.
5. Current EN/RU all-tab/callback coverage. Historical034 screenshots are not
   current callback evidence. Final exact-candidate native gates and review.
6. Final install/update/autostart/uninstall lifecycle and configuration/model
   preservation. Earlier successful uninstall preserved390files/694145174bytes,
   but it used an earlier host/installer pair.
7. Finish owned fixture cleanup: restore original audio defaults, unload only
   owned audio modules, remove temporary QA overrides/credential fixture and
   agents when no longer needed, preserve required models/recordings/config and
   all durable evidence. Do not kill unrelated servers or desktop processes.
8. Owner production delivery of D094 is not done. Do not assume full-port
   development authority authorizes a stable release or master merge.

No permanent infrastructure blocker is established. Physical microphone is
unavailable in the VM. Cold small local CPU models have demonstrated120s
failures or irrelevant answers; warmed Qwen2.5System answered in60.54s. The
working authenticated gateway proves the general speech-to-AI path but does
not certify all providers or local performance.

## 8. Native workflow and evidence discipline

- Load the actual homelab skill before remote/build/recovery work. Canonical
  infrastructure docs path was unavailable; the existing homelab-infra-docs
  clone supplies dated operational context. Check actual tools/host state.
- No heavy Rust/Slint builds on the owner's interactive desktop. Use dedicated
  native workers, exact committed SHA, managed bounded jobs and resource preflight.
- One owned build/installer/UI action per worker at a time. An SSH timeout is
  not proof the job stopped. Inspect unit/task, process, journal and durable
  EXIT marker before restarting or changing checkout.
- Linux task build environment: CARGO_INCREMENTAL=0,CARGO_BUILD_JOBS=1,
  RUST_TEST_THREADS=1,CARGO_PROFILE_DEV_DEBUG=0,CARGO_PROFILE_TEST_DEBUG=0,
  SLINT_EMIT_DEBUG_INFO=1. Existing shared target is recorded privately.
- macOS preflight: no other Cargo/Rust/Swift/host/model process; at least40%
  system-wide free memory; jobs2/testthreads2 maximum.
- Do not enable/restart SuflyorClaudeS7Job. Use the existing approved Windows
  job and restore its wrapper byte-for-byte. Read winbrat-recovery first.
- UI verification requires the exact ui-mcp build with compiled Slint metadata,
  actual screenshots/interactions and relevant physical shortcut delivery.
- Current Hyprland uses Lua: `hyprctl eval` executes configuration code;
  `hyprctl dispatch` expects a dispatcher. Read installed API/help first.
- Wait for settled lifecycle/geometry. Fixed2s checks saw legitimate centering;
  native key delivery via QEMU can be delayed. Replays should follow actual
  capture arrival, not an assumed immediate physical key event.
- MCP trees contain only visible/available viewport content. Scroll the real
  thumb before claiming a missing continuation. Popup element coordinates in
  Slint1.17 were not reliable for Regenerate; measured native pointer input
  dispatched it. Do not change working product code to fix a verifier.
- Record failures and read-only review limits. Do not relabel ignored tests,
  direct test-executable runs or unloaded transient-unit defaults as a passed
  Cargo driver. Keep real exit markers/journals, including OOM interruptions.
- Keep captures, credentials, endpoint/config files and raw manifests outside
  Git. The private handoff has paths to fixtures, never bearer values.

## 9. Previous completed work and separate unresolved topics

Stable v0.38.1 was published before this port: tag
`84249fdcb967ec6415a0d62863b71dff880d2763`, tested tree
`30515c632df4322628107358f488b15cb6cc9b5e`. Both downloaded stable assets were
hash verified. Later docs/TLS repair merged through PR304 and evidence PR306;
issues301/292/257/197 were closed. Stable binaries were not rebuilt for that
later TLS source correction. Prior rc13/14/15 work and stable release evidence
remain historical; do not redo publication as part of this handoff.

Separate unresolved topics are D063 multi-monitor ghost, independent Strata
inference and the owner's historical graphical-session OOM growth trigger.
The kernel killed quickshell, not a demonstrated machine reboot. They are not
silently resolved by this port's gates. Current local self-improvement H03/H07
records are evidence, not instructions to perform unrelated tasks.

## 10. Continuation by Claude Code, October 22–23

The owner asked to fix the queue, optimize its algorithm, then continue the
port. Sections 1–9 above are the Codex handoff and stay as written; this
section supersedes their "last candidate" and worker-state lines.

### Queue redesign (D-094 follow-up)

The two-slot first-come semaphore became a priority queue in
`overlay-backend/src/ai/queue.rs`:

- Interactive requests (F9, typed ask, PTT, Vision, continuations) are admitted
  before Background ones (session naming, debrief, summaries, memory).
- Background work holds at most one of the two slots.
- A newer waiting AutoTile supersedes older waiting ones; the superseded
  placeholder is removed silently (`"cancelled"`, no retry or error tile).
- Exclusive (managed local-server switch) waits for zero active requests and
  blocks new admissions; an abandoned switch wakes the queue again (659d415).
- The idle prompt prime has its own slot and never takes a background one.

Master-based branch `claude/ai-queue-priority` @ `50ae142`, draft PR
<https://github.com/PavelLizunov/suflyor/pull/309>. The same commits are on this
branch through `659d415`. Evidence:

- Windows targeted gate on 50ae142: exit 0, backend 809 passed, host 154, all
  eight queue tests.
- Linux `ai::` tests at 659d415: 176 passed, 0 failed.
- Live Omarchy run with a controlled provider (QA build f1d0412): one active
  auto tile, the middle waiting auto superseded, a typed question admitted
  without waiting behind autos, the newest auto admitted about 10 ms after the
  active one was closed, no late tiles after close-all.
- Independent gpt-sol review found the abandoned-switch wake-up defect (fixed in
  659d415). Two pre-existing findings are left open: the Codex subscription
  route bypasses the queue, and two tests race on `LOCAL_NO_THINK`.

### Scaled and multi-output capture (section 7 item 1)

A headless output `SUFTEST` (1200x1920, scale 1.5, at -800,0) was attached for
the test and removed afterwards. Physical F8 via QEMU, slurp region drag; the
frame sent to a capturing provider was compared with `grim` of the same area:

| Region | grim | sent | RMSE |
| --- | --- | --- | --- |
| inside the scaled output | 680x500 | 681x501 | 3.4% (JPEG) |
| across both outputs | 500x200 | 501x201 | 2.7% (JPEG) |

The extra pixel is slurp including the selection edge. Tiles open on the focused
output, which is where the capture was made.

### Port defects found and fixed

- f2df413: the bar landed at x=960 instead of 320 on the first start with a
  second output, because the compositor width read failed and 0 was used. It now
  falls back to the window's own logical width. Rechecked live: (320, 24).
- f2df413: service stop hit the 10 s systemd timeout once. `close()` removed the
  18 owned bindings with 18 separate `hyprctl` processes; they now go in one
  `eval`, like installation. Rechecked live: stop 4.0 s, result success, on a
  compositor answering in about 0.5 s per request.
- 1b1da11: a new tile sat centred for 4–10 s before moving to its slot. Each
  realization attempt made about eleven sequential `hyprctl` calls with the move
  last (ownership re-checks, pin, set/readback exclusion, rect, monitors, move).
  Move, pin and capture exclusion now go in one Lua eval after one ownership
  check. The ownership check stays: a dispatcher on an unknown address still
  answers "ok", so it cannot replace it. Rechecked live on the scaled output
  with QA build 1b1da11 (host SHA-256 `07667c93…f8746`): F6 to placement 2.95 s
  instead of 5.3 s, visibly centred under 1.5 s instead of about 4 s; the tile is
  pinned and `no_screen_share` reads back true (stealth on); stop 1.6 s, success.

Hyprland latency matters for every Linux window operation. On the test VM it was
about 40 ms per request when idle and 200–600 ms with Hyprland at 100–200% CPU
(software rendering, RustDesk capture, 14% steal). Fixed-delay checks on this VM
are therefore unreliable; wait for settled geometry.

### Other findings, not port defects

- The Russian `DEFAULT_VISION_PROMPT` (`overlay-backend/src/vision.rs:18`) shows
  as the user's own turn in the English UI on every platform.

### Gates and worker state at the end of this pass

- f2df413: Windows targeted gate exit 0 (backend 818/0, host 97 + 156, guards,
  TTS 21, Tera 73); Linux host build plus lib 94, host 155, linux_platform 8.
- 1b1da11: Windows targeted gate exit 0 (backend 818/0, host 97 + 156). Linux
  host build passed; the first test build was killed by systemd-oomd at the 6 GiB
  unit limit (no EXIT marker, `Result=oom-kill`). The tests-only rerun passed lib
  95 and host 155 but failed one `linux_platform` guard that looked for the old
  per-property stealth call in `tile_window.rs`.
- 9bc8814 updates that guard to the batched placement path (`arrange(...,
  stealth)` plus `no_screen_share` in the Linux window module). Linux
  `fmt --check` and `--test linux_platform`: 8 passed, 0 failed, 1 ignored
  (live clipboard). Production code is unchanged from 1b1da11.
- A Linux slint test build takes hours on this VM (f2df413: 22:53 to 01:07) and
  swaps at about 4.8 GiB for the library test crate. Wait on the unit state, not
  only on the EXIT marker: an OOM kill leaves none.
- Omarchy fixture restored after each live run: one output (Virtual-1,
  1920x1080, scale 1), no configuration errors, zero owned bindings, config
  restored, installed host back to `ec38f602…` (2f3f2fd), service inactive.
  Scripts live in `~/suflyor-evidence/scaled-capture/` (setup, teardown, probe,
  settle, stop-probe, hyprlat).
- Another session was working on the same VM at the same time: quickshell exited
  "due to IPC request" four times while reloading the local plugin
  `io.github.pavellizunov.vpnrouter`. It is not ours; leave it alone, and expect
  the extra load in timings.
- Windows wrapper restored after both gates, SHA-256 `972DC146…F728`;
  PostReleaseDocs Ready, S7 Disabled.
