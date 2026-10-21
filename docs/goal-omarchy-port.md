# Spec: fully functional Suflyor on Omarchy / Wayland

## 1. Intent & Invariants

The owner's October 21 instruction supersedes the former first-milestone scope:
deliver a working Wayland-adapted equivalent of the complete Suflyor product.
A compiling host, rendered demo, synthetic answer, green targeted gate or draft
PR is an intermediate checkpoint, not completion. Continue implementation and
actual-consumer verification until the full functional contract below is met.

Reuse the existing Rust backend, Slint UI, session and model pipelines. Do not
rewrite the product in QML. Use native PipeWire, Wayland and Hyprland mechanisms
where Windows/macOS APIs do not exist. Hardware-specific engines may use a native
functional equivalent, but a silent no-op or permanently disabled product feature
does not satisfy parity. Record unavoidable platform constraints precisely.

Preserve configuration, credentials, transcripts, download-versus-activation
semantics, model integrity checks, routing and rollback. TTS and STT ONNX runtimes
remain in separate processes. Keep Windows/macOS behavior and native gates intact.
Do not fabricate native handles, geometry, successful capture, model readiness or
capture exclusion. A configured screen-share exclusion must be verified in the
actual compositor capture; describe its observable behavior accurately.

Use the dedicated Omarchy worker and exact committed candidates. Keep the owner's
original checkout, index and unrelated files unchanged. Do not use the owner's
interactive desktop as the build or QA worker. Ordinary task-owned installation,
service lifecycle, test-device creation and integration on the dedicated worker
are part of this implementation. Stable release publication remains separate.

## 2. Interface / Data Contract

- Keep the public backend audio API: enumerate/select microphone and output;
  simultaneous 16 kHz mono PCM chunks, push-to-talk and finite diagnostics;
  bounded queues, prompt cancellation and owned capture processes.
- Keep cloud/local STT and AI request/routing contracts. Implement Linux local
  installation, validation, activation and lifecycle rather than requiring an
  externally preconfigured server as the only route.
- Keep neural TTS, selected-text reading, pause/resume/stop/rate/voice selection,
  session playback, re-transcription and diarization through native Linux output.
- Supply real screen buffers and region selection for OCR/Vision. Preserve
  cancellation, multi-monitor scaling and exclusion of Suflyor's own surfaces.
- Route compositor shortcuts, including press/release push-to-talk, into the same
  host actions as bar controls. Use a private owned IPC/portal contract, not X11
  registration misreported as Wayland support. Preserve non-US text-edit shortcuts.
- Use compositor-backed monitor/window identity, placement, drag, pinning,
  full/compact bar, tile cascade, lock modes, hide/restore and focus behavior.
- Keep all Settings tabs, profiles, providers/authentication, import/export,
  components, diagnostics, KB, history, memory, summaries and native file/URL opens.
- Deliver an installable native launch surface, owned service lifecycle,
  optional autostart and an appropriate Linux update route. Quattro integration
  is a launcher/control surface, not a replacement application implementation.
- Existing supported model engines and downloads must select Linux-compatible
  assets, executable permissions and native resource/hardware discovery. Never
  download or execute Windows installers on Linux.

Decision: Compose — existing Rust/Slint product and session pipelines with
installed PipeWire Pulse-compatible capture tools, Wayland clipboard, compositor
IPC/rules and native screenshot/OCR helpers. Reuse existing CPAL sidecar playback.
Select additional libraries only after source/API/license inspection.

## 3. Verification Checklist (Definition of Done)

Previously observed foundation, not full acceptance:
- [x] Backend and host compile on native Linux; actual Wayland windows render.
- [x] EN/RU bar, Settings, question and synthetic streamed answer inspected.
- [x] Actual clipboard copy and same-config singleton exclusion verified.
- [x] Scoped Linux driver completes: 68ed251, 236 passed, three ignored, exit 0.
- [x] Same-candidate Windows targeted gate: 1,109 passed, five ignored, exit 0.

Full outcome, all still requiring implementation or final acceptance:
- [ ] Microphone and system audio work simultaneously with selected/default
      devices; real samples, route changes, stop/restart and no orphan processes.
- [ ] Real speech reaches local/cloud STT and transcript/auto-answer paths;
      held push-to-talk and release work without premature UI success.
- [ ] Real AI text and image requests return rendered model answers; errors,
      cancellation and retry preserve routes and session consistency.
- [ ] Native screenshot, region selection, local OCR, Vision, cancellation and
      scaled/multiple monitor scenarios work; own windows are excluded.
- [ ] Every global shortcut is physically/compositor dispatched, including PTT
      press/release; non-US text editing, selection reading and clipboard work.
- [ ] Native drag, pinning, monitor placement, tile cascade, compact/full bar,
      deep/reader locks and hide/restore work without fake geometry.
- [ ] TTS voices download/activate and produce real output; read/pause/resume/
      stop/rate, transcript playback, re-transcription and diarization work.
- [ ] Managed local AI/STT installation, disk checks, resumable cancellation,
      engine/model switching, inference and process cleanup pass on Linux.
- [ ] All Settings tabs, provider sign-in, profiles/context, KB/history/memory,
      recording/summary flows, import/export and native open/reveal are exercised.
- [ ] Native installation/launcher, service stop/start, optional autostart,
      update route and uninstall preserve user configuration.
- [ ] Exact final candidate passes Linux checks and native EN/RU end-to-end QA;
      Windows/macOS regressions and PR checks are resolved, not waved away.
- [ ] Review and integrate the complete port; record installed, loaded, verified
      and published status separately. Do not close on the former first slice.

Evidence: [October 21 native audit](audit-2026-10-21-omarchy/README.md).
Draft PR #307 contains the initial milestone and remains development work.

Developer verification uses `bash scripts/check-linux-port.sh`. Build QA host with
`SLINT_EMIT_DEBUG_INFO=1 cargo build --locked --no-default-features --bin
overlay-host --features ui-mcp --manifest-path slint-experiment/Cargo.toml`.
Use one managed build at a time, one Cargo job and one test thread, disabled
incremental compilation and debug symbols, and bounded runtime/memory. The test
VM has 8 GiB RAM; inspect available memory and disk before each heavy build.
Hyprland configuration is Lua on the worker; validate any task-owned rule change
with `hyprctl reload` and `hyprctl configerrors` in the existing graphical session.
