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

### Session history and summaries (section 7 item 2)

Live on Omarchy, October 23, QA build 1b1da11, English UI, cloud gateway route,
virtual speech fixture; driven through Slint MCP and checked against files.
Scripts and receipts: `~/suflyor-evidence/session-history/` (phase1, phase1b,
arch, throwaway).

- Real session `2026-10-23_07-45-09_b88922`: two transcript lines (mic and
  system), F6 answer 443 chars, capture stopped 1.2 s after the timer chip.
- Debrief enabled for the test: with one user line it correctly reports "Not
  enough of your speech in this session to build a debrief." A debrief with
  enough user speech is not yet observed.
- Archive "Generate": re-transcription of both WAVs with progress in the archive
  (mic 4 chunks in 103 s on this VM), summary landed after about 5 minutes; the
  row then offers "View". "View" opens the saved summary with no AI call, also
  after a service restart.
- Rename persists to `session_names.json` and survives restart. "Regenerate name"
  on the cloud route shows "Names are generated by the local AI only. Choose a
  local provider in Settings → AI." The automatic namer is local-only as well;
  both are still to be accepted with a local model (item 3).
- Transcript window: both tracks, search, player, speaker count, session name.
- Delete, twice, on throwaway sessions: the confirmation names the session; the
  journal, recordings directory and conspect are removed; other sessions stay.
- SIGKILL mid-session: systemd restarted the service (NRestarts=1), no orphan
  `parec`, the archive marks the session "Interrupted" with its line kept, both
  crash-truncated WAVs were repaired at startup, and "Generate" built a summary
  from them. The recovery offer window is gated off by `SLINT_OVERLAY_RECOVERY`
  on every platform and was not accepted.
- Fixture restored afterwards: config byte-identical to the backup (debrief off
  again), installed host `ec38f602…`, service inactive, zero owned bindings.

### Managed local AI on the test VM (section 7 item 3, in progress)

Gates for 4f6aba7 passed: Windows backend 818/0, host 97 + 159; Linux host 158/0, linux_platform 8/0, backend local_ai 95/0, fmt clean. Live with the 4f6aba7 QA build (English UI), Install now shows "Not enough disk space: about 9 GB needed, 7 GB free. Free up space, then press Install again." The worker was restored afterwards (binary ec38f602, config unchanged, service stopped).

Live, October 23, QA build 1b1da11, English UI. Settings ▸ Components lists the
llama.cpp engine and local model as optional; "Open" jumps to AI providers.
The first MCP click on a standard Button in this window is sometimes lost; a
second click works. Slint 1.17 has no MCP scroll: drag the real scrollbar thumb
with QEMU `mouse_button` plus `hyprctl` cursor moves (wheel events from the PS/2,
tablet and vmmouse devices did not scroll).

- On this VM (8 GiB RAM, 4 vCPU, no GPU, 7.8 GiB free) the profile is Unknown
  and the only offer is "Built-in engine + Gemma 4 12B — about 7 GB with speech
  components". Unknown maps to the 12B fallback on every platform; a CPU-only
  8 GiB machine cannot run it usefully. Product decision, not changed here.
- The disk pre-check refused correctly: about 9 GB needed (12B, projector,
  binaries allowance, GigaAM), 7 GB free. The row's "about 7 GB" understates it.
- The UI then showed "Ошибка установки локального AI. Подробности в журнале." in
  the English UI: wrong language and no cause or next step. 49 status lines in
  `settings_local_ai.rs` were Russian-only on every platform.
- 4f6aba7 fixes both: a typed `DiskSpaceError` lets Settings say "Not enough
  disk space: about 9 GB needed, 7 GB free. Free up space, then press Install
  again."; other failures name a recognised cause (network, integrity,
  unpacking, no engine build, server start) and the button, never the raw
  chain (it can carry a home path or URL); the remaining local-AI statuses
  follow the UI language. Gates for 4f6aba7 are listed below when done.
- There is no engine-only install: the disk pre-check always counts the 12B
  model, and "Choose GGUF…" appears only once the engine is installed. A small
  CPU-only machine cannot reach a small custom model through the managed path.
  Product gap on every platform, not changed here.
- The worker cannot pass the pre-check without about 1.2 GiB more free space.
  Superseded task QA binaries free about 0.9 GiB; the rest is Codex evidence
  (keep) and the unrelated `vpnrouter-omarchy-checks` tree. Growing VM 107's disk
  or removing other data is the owner's decision.
- Still open for this item: engine download, cancel and resume, custom GGUF
  through the native file chooser (rfd), activation, switching, inference and
  cleanup. The small models already on the worker
  (`~/suflyor-evidence/real-inference/`: Qwen3.5 0.8B, SmolVLM 500M, Qwen2.5
  1.5B) are the intended custom-GGUF fixtures. The local namer and a debrief
  with enough user speech (item 2 leftovers) need the same local model.

### Settings tabs in EN and RU (section 7 item 5, layout pass)

Live, October 23, QA build 4f6aba7 at 720x600. A script
(`tabs.py` in the private evidence folder) opens every tab and pages through it by
dragging the content scrollbar thumb with the MCP `drag_element` tool, which
does exist in Slint 1.17, so the QEMU thumb drag described above is no longer
needed. EN: 22 pages, RU: 22 pages, each tab read to its
end. The sidebar has 17 items: "Memory" is the last one and sits below the
600 px window, so the MCP tree (which omits clipped elements) left it out of
this pass. It is covered separately below. No Russian text in the English UI and no English in the Russian UI.

Port defects found and fixed:

- On Linux `Theme.font-sans` ("Segoe UI") falls back to the wider Liberation
  Sans, so two Russian labels ran past their equal-width buttons:
  "Структурировать (AI)" (Profile) and "Взять ключ из локального Hermes"
  (Hermes). 192834c shortens them to "Упорядочить (AI)" and "Ключ из
  локального Hermes".
- Diagnostics showed "Global hotkeys: registered: 50". The row counts
  space-separated keys, and the Wayland labels such as "SHIFT + ALT + 1" contain
  spaces. 1c94292 compacts the labels; the row should read 18 (the compositor
  bindings, including push-to-talk press and release).

Shared, not port (also visible on Windows by code):

- English copy uses Russian guillemets: «Off», «Model default», «Determine
  speakers».
- Auto-tiles: the "Fast answers" hint is elided ("…open their d…").
- RU AI providers: the bearer-token placeholder is cut off ("…нажмите «Сохрани").
- Diagnostics: the two red "Clear …" buttons use a smaller font than the other
  buttons.

Callbacks, QA build 1c94292, EN and RU (`callbacks.py`): 13 safe buttons per
language, each answered in the UI language.

- Profile "Structure via AI" with no context: "Nothing to process yet".
- AI providers and Screenshots connection tests: "Connection works".
- STT test: "The model loaded; recognition is ready".
- Hermes "Check" (no Hermes on the worker): "API: The server does not answer".
  This names the cause but not the next step (shared copy, not port).
- Audio microphone test (no microphone in the VM): "Too quiet (-inf dBFS) —
  speak or check the microphone".
- Updates: "No newer stable release (latest: 0.38.1)".
- Diagnostics "Check all": every row Ready, test tone captured at -12 dBFS,
  "Global hotkeys: registered: 18" (the 1c94292 fix, seen live).
- "Check & repair database": "Database healthy. Backup: …". The line sits
  below the window edge; scroll to read it.
- Components "Open" switches to AI providers.
- "Test voice" plays through Piper (journal: sidecar started and finished); "refresh" and
  "Refresh devices" change no text, as designed.

The MCP element tree leaves out elements clipped by a ScrollView, so a probe
must scroll before it reads a status line.

The RU run still showed "Структурировать (AI)": `build.rs` did not report the
`.po` files to cargo, so the cached worker build kept the old strings. 8632229
adds `rerun-if-changed=translations`. With the 8632229 QA build both Russian
labels fit (Profile and Hermes screenshots) and "Упорядочить (AI)" answers
"Пусто — нечего обрабатывать".

Gates for 8632229: Windows backend 818/0, host 97 + 159. Linux host build
passed; the first test run was OOM-killed while the app was running (5.3 GB
peak), and the rerun with the app stopped passed: i18n_guard 3/0, host
158/0, linux_platform 8/0.

Worker restored after this pass: binary ec38f602, config identical to the
saved copy (English UI), service stopped, the four database backups made by
"Check & repair database" removed.

Profiles, QA build 8632229, English UI (`profile.py`): "Add profile" with
"QA profile" selects the new profile; "Rename" to "QA renamed" updates the
list; "Delete" returns to "none yet — name one and Add" and the config holds
`context_profiles: []`. "Add profile" with an empty name adds nothing, and
"Rename" stays disabled while the name field is empty. The app keeps a rolling
`config.json.bak`; after the run it held the test profile, so it was reset to
the saved original config.

Not yet done for item 5: file dialogs (export/import), Hermes install, setup
wizard, and the destructive "Clear …" buttons.

### Knowledge palette and personal memory (section 7 item 4, part)

Live, October 23, QA build 8632229, English UI (`kbmem.py` and `tile.py` in
the private evidence folder). F4 is pressed on the VM keyboard
(`qm sendkey 107 f4`).

- F4 opens the "Knowledge palette". "kubectl" lists glossary and command
  entries with source chips; "zzqxv" shows `No results for "zzqxv".`; Enter
  closes the palette and opens a read-only tile "kubectl" (`kb · glossary`)
  with the full entry; the tile's Close works. The MCP `dispatch_key_event`
  for Enter reports "Attempting to access deleted window" because the palette
  closes while the key is delivered; the action itself succeeds.
- Memory (reached by dragging the sidebar thumb): Add, Edit and Delete of one
  fact work and the list returns to "No approved memory yet."

Defects found and fixed in 55a7a64 and accf626 (shared code, visible on
Windows too):

- The Memory statuses were hard-coded Russian, so the English UI showed
  "➕ факт добавлен", "⏳ извлекаю…" and a bare "➕ 19" after Extract.
  `wire_memory` now takes the shared config and picks the language at click
  time; Extract reports "New suggestions: N" or "No new suggestions"; a
  failed save says the local memory database is unavailable and to press Add
  again.
- Palette previews were cut at the first dot, so `kubectl apply -f
  manifest.yaml` showed as "kubectl apply -f manifest". The preview now ends at
  ". " on the first body line.

accf626 drops the emoji from these new statuses: see the next point. In the
accf626 Russian run the longer "no new suggestions" status widened the row
past the card (a horizontal scrollbar appeared and "Добавить" moved to the
edge), because the status Text did not wrap. 1a89391 wraps it and shortens the
extract statuses.

Russian live check on the 1a89391 QA build (`memstatus.py`): Add, Edit and
Delete work; Extract on an empty candidate list shows "Новых предложений нет"
with no horizontal scrollbar and "Добавить" stays at x=603. With the catalog
file made read-only for the test, Add shows "Факт не сохранён: локальная база
памяти недоступна. Нажмите «Добавить» ещё раз." on two lines inside the card
(journal: "attempt to write a readonly database"). File mode, config and
catalog were restored afterwards and compared byte for byte with the copies.

Port finding, not changed: Linux draws no emoji in the software renderer. The
memory rows lose their kind glyph (📌, 📝) and a suggestion's reason "📝 415"
reads as a bare "415". The glyphs come from `kind_glyph` and the backend
candidate reason; replacing them with SVG icons or words is a design change.

Shared, not changed: a tile opened from the palette keeps the English source
chip "kb · glossary" in the Russian UI (`help_palette.rs`, `format!("kb · {}",
…)`), while the palette itself shows "глоссарий".

Shared, not changed: "Extract from recent sessions" turns every stored AI turn
into a suggestion whose text is the whole prompt the app sent ("Последние
реплики разговора …" plus the transcript), because `ai_turns.question` stores
the built prompt, not the user's question. 19 such suggestions appeared on the
worker. The catalog database was restored from a byte copy afterwards.

### Other findings, not port defects

All are shared host or backend behaviour, seen on Linux:

- The Russian `DEFAULT_VISION_PROMPT` (`overlay-backend/src/vision.rs:18`) shows
  as the user's own turn in the English UI.
- The saved-summary tile title is hard-coded "Сводка · …"
  (`aux_windows/archive.rs`, `on_view_summary_requested`) in the English UI.
- Summary markdown `**Heading**\nvalue` renders as one line ("Decisions No
  decisions recorded"): the soft break becomes a space and the bold is not
  visibly distinct.
- While a session runs, the stop chip's accessible label is the timer text
  ("01:22"), not "stop".
- Re-transcription of a short crash-truncated recording was worse than the live
  transcript ("Иапределиение один" vs "Дай краткое определение и один").
- In archive rows with a summary, the row icons sit further left than in other
  rows because "View" and ↻ take the space.

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
- accf626: Windows targeted gate exit 0 (backend 818/0, host 97 + 161); Linux
  host 160, linux_platform 8, i18n guard 3.
- 1a89391: Windows targeted gate exit 0 (backend 818/0, host 97 + 161). Linux
  QA build passed; the combined build-and-test unit was killed by
  systemd-oomd again (5 GiB memory peak, 3.7 GiB swap peak, 1 h 14 min CPU in
  2 h 25 min). The tests-only rerun with the app stopped passed i18n guard 3,
  host 160 (1 ignored) and linux_platform 8 (1 ignored).
- A Linux slint test build takes hours on this VM (f2df413: 22:53 to 01:07) and
  swaps at about 4.8 GiB for the library test crate. Wait on the unit state, not
  only on the EXIT marker: an OOM kill leaves none.
- VM 107 has 4 vCPU (all four cores of the host's Intel N150) and 8 GiB; the
  idle desktop already holds a load average near 9 because Hyprland and
  quickshell render in software. More CPU is not available. 12 to 16 GiB of
  memory would remove the swap stalls; building on a stronger Linux node and
  copying only the QA binary here would cut the build from hours to minutes.
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
