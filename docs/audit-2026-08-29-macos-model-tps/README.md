# macOS model-state and TPS audit

- Date: 2026-08-29
- Environment: physical Apple Silicon `mac-worker`; native Rust, Slint, AppKit, and Swift MLX paths
- Baseline: `1fa75a0e59694bf2fa75c2fae52e837e1958f568`
- Code candidate: `c8fbc58389a283b8c7ef061f7c3e4c3b6d764230`

## Privacy and comparison conditions

- UI checks use an isolated temporary home, synthetic model IDs, a loopback-only mock model catalog, and no real credentials, endpoints, transcripts, or meeting context.
- Before/after captures use the same 720 x 600 Settings window size, language, theme, provider, model catalog, and interaction sequence.
- The TPS screen uses the installed pinned managed-text model and the exact candidate sidecar, but records only public model/revision identifiers, token counts, timing values, finish reasons, and binary/model hashes. Generated text, bearer values, local paths, and private configuration are not retained.
- Auto/F6 and F9 are compared only when model identity, prompt payload, sampling, output cap, resident sidecar, and warm state match. Decode TPS and TTFT remain separate metrics.

## Production macOS path map

- **Process assembly:** `slint-experiment/src/bin/overlay_host.rs` delegates to the shared `overlay_host_windows.rs` runtime root; `src/native/` supplies AppKit/window seams and the `overlay_host/` directory owns hotkeys, Settings, tiles, capture, diagnostics, and MLX lifecycle.
- **Settings and visible state:** `ui/index.slint` compiles `ui/settings_panel.slint`; `settings_controller.rs` seeds the reused window; `settings_ai.rs` owns cloud/generic-local provider and model callbacks; `settings_mlx.rs` owns managed MLX role actions.
- **AI request paths:** `slint_session.rs` dispatches Auto tiles, `overlay-backend/src/runtime.rs` dispatches F6, and `overlay_host/tile_ask.rs` dispatches streaming F9. All resolve through `overlay-backend/src/ai.rs` and `ai/provider.rs`.
- **Managed MLX:** `overlay-backend/src/mlx_install.rs` pins and verifies model snapshots, `mlx_runtime.rs` owns the single resident child, and `suflyor-mlx/Sources/SuflyorMLXCore/` implements the authenticated loopback OpenAI-compatible sidecar.
- **Native media:** `overlay-backend/src/audio_macos.rs`, Slint native capture adapters, AppKit window helpers, TCC/capture watchdog code, and the macOS playback modules in the TTS/TeraTTS sidecars cover the remaining platform-specific production seams.
- **Packaging and gates:** `slint-experiment/scripts/build-macos-app.sh` packages/signs the app and sidecars; `scripts/git-gate-macos.sh` is the full Apple Silicon compile/test gate.

## Oversized-file assessment

| Source | Approx. lines at audit | Decision |
|---|---:|---|
| `overlay_host_windows.rs` | 5,897 | Further extraction is justified, but startup/timer/event-loop groups need a dedicated behavior-preserving change and full hotkey/window smoke. |
| `overlay-backend/src/local_ai.rs` | 4,332 | Catalog, context-profile, and process-repair seams are candidates; no mechanical move was mixed into this macOS fix because this is a high-risk Windows process manager. |
| `ui/settings_panel.slint` | 4,263 | Per-tab component extraction is justified but requires a dedicated full Settings visual matrix. |
| `overlay-backend/src/runtime.rs` | 2,682 | Manual/session flows can be split by domain after contract tests; not required for either root cause. |
| `overlay-backend/src/ai.rs` | 2,130 after change | TPS telemetry was a small self-contained seam and was extracted now to `ai/tps.rs`; endpoint/request orchestration stays in the facade. |
| `settings_controller.rs` / `settings_ai.rs` | 1,968 / 1,553 | Additional provider/population splits are possible, but overlap with reused-window behavior makes them a separate UI task. |
| `audio_macos.rs` | 1,045 | Size alone is insufficient reason to split a permission/capture state machine; retain until a tested ownership boundary is identified. |

The similarly named `slint-experiment/src/settings_panel.slint` is not imported by the production `ui/index.slint` compilation root and was excluded from production decomposition decisions.

## Source-backed findings

1. The cloud and generic-local model ComboBoxes persisted the selected model string but did not copy `self.current-index` back to their separate root index properties. Recreating a conditional provider pane could therefore display the old index even though configuration contained the newly selected model.
2. Managed MLX non-streaming responses omitted OpenAI-compatible `usage` and `timings`, so automatic/F6 requests could not update TPS. The streaming F9 path instead estimated tokens from SSE chunk count, which is provider-dependent and not comparable to decode tokens per second.
3. The production Auto tile and F6 paths both use the same non-streaming completion function. F9 uses streaming. There was no source-level inference configuration split that would by itself explain lower decode throughput.
4. `overlay-backend/src/ai.rs` remained oversized. The process-wide EWMA and stream-metric selection were mechanically extracted to `overlay-backend/src/ai/tps.rs`, while the existing `overlay_backend::ai::{avg_tps, record_tps}` public paths remain unchanged.

## Verification so far

- Baseline macOS gate: `1fa75a0e59694bf2fa75c2fae52e837e1958f568`, exit 0, all Apple Silicon compile-seam layers green.
- Earlier full candidate macOS gate: `b6e569742ec3b151c879b19b7527106f8d1bd866`, exit 0. A later independent review found two terminal-event edge cases; both were corrected and regression-tested in the final code candidate.
- Swift MLX tests after the cancellation correction: `b39079e809f8e357610e20c62183dd4e9f810d8c`, 10 tests passed. The subsequent `c8fbc583` commit changes only Rust formatting.
- Final Windows full five-crate gate: exact clean `c8fbc58389a283b8c7ef061f7c3e4c3b6d764230`, exit 0, including Slint UI-MCP feature check and the new stream-terminal regression test.
- Final macOS gate: exact clean `c8fbc58389a283b8c7ef061f7c3e4c3b6d764230` (tree `827046bc422511d51ee48ae4f80bae18d7bbdd8a`), exit 0, terminal `All macOS compile-seam layers green.`
- Two independent read-only review passes report no remaining blocker, security, public-API, or Windows-compatibility findings.
- The earlier disconnected final-gate attempt had no surviving marker or build process. After worker recovery, one resilient rerun produced the verified result above. A bounded paired TPS comparison was not started because both metric smokes hit the predeclared truncation stop condition.

## TPS iteration checkpoint

- Production release sidecar, pinned text-model revision, authenticated health, and `/v1/models` identity checks succeeded.
- The non-streaming smoke returned positive completion-token and server decode-TPS metrics, proving the new envelope is populated at runtime.
- Both bounded prompts ended with `finish_reason=length` (first at 160 output tokens, corrected prompt at 256). This hit the predeclared truncation stop condition.
- The paired non-streaming/streaming screen was therefore not started and no parity or speed decision is claimed. The two generation smokes each settled inside the three-minute wait; all candidate processes exited and the worker returned to 83% free memory.
- Coverage: non-streaming metric smoke `tested`; paired screen `invalid/incomplete`; streaming metric parity `deferred` pending a separately approved fixed-length methodology.

## Acceptance evidence

- `windows-gate-summary.txt` records the sanitized final exact-SHA Windows gate result and the marker-recovery condition.
- `mac-gate-summary.txt` records the green baseline, earlier-candidate, and final exact-SHA macOS gates.
- `review-summary.txt` records the final independent correctness, security, API, and compatibility verdicts.

No paired TPS rows are recorded because the truncation stop condition made that comparison invalid. The unverified paired and streaming-parity states remain explicit above.
