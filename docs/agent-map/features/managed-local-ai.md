# Managed local AI: source-linked lifecycle contract

**Evidence:** principal source paths inspected at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. No models downloaded, binaries executed, endpoints contacted or hardware benchmarked. Managed local lifecycle is not equivalent to arbitrary OpenAI-compatible endpoint reachability.

## Ownership and selection

[Managed endpoint predicate](../../../overlay-backend/src/local_ai/model_choice.rs#L9-L34) accepts only HTTP loopback host, port 8080 and `/v1` path. LAN/other-port endpoints are not ours to terminate/relabel. [ManagedLlamaChoice](../../../overlay-backend/src/local_ai/model_choice.rs#L240-L307) distinguishes pinned 4B/12B/26B choices, user-selected custom GGUF and explicit context preset.

[Context presets](../../../overlay-backend/src/local_ai/model_choice.rs#L36-L119) include auto/8K/16K/32K/64K/96K but clamp against hardware profile ceilings. [Hardware profile](../../../overlay-backend/src/local_ai/hardware_profile.rs#L6-L109) recognizes a bounded owner-supplied matrix; unknown hardware remains Unknown, not an inferred stronger profile. Context preset ceilings are source policy, not a measured RAM/VRAM feasibility proof.

[UI local controls](../../../slint-experiment/src/bin/overlay_host/settings_local_ai.rs#L26-L105) seed selected model/context/vision capability and live labels; [runtime config resolver](../../../overlay-backend/src/config.rs#L746-L776) supplies the local endpoint/model. [Model config repair](../../../overlay-backend/src/local_ai/model_state.rs#L175-L228) conservatively repairs stale managed selection. Arbitrary endpoint/model text must not silently be treated as an app-managed verified asset.

## On-demand install and integrity

[Installer preflight](../../../overlay-backend/src/local_ai.rs#L377-L420) computes disk needs for selected assets. [Model pins](../../../overlay-backend/src/local_ai.rs#L35-L112) define URLs/bytes/digests; [resumable download](../../../overlay-backend/src/local_ai.rs#L2854-L3000) verifies size and SHA for pinned models before use and clears mismatched downloads. Some URLs use upstream mutable main paths but pinned SHA fails closed on changed content.

[Managed model download](../../../overlay-backend/src/local_ai.rs#L2339-L2410) allows explicit choice regardless of hardware warning and does not restart a live server automatically. [Cached hash verification](../../../overlay-backend/src/local_ai.rs#L2784-L2838) memoizes size/mtime/file stamp checks and avoids duplicate full scans; preserving same metadata despite changed bytes is a local writable-path threat still requiring a stated precondition.

[Installed presence](../../../overlay-backend/src/local_ai/model_state.rs#L303-L340) primarily uses expected sizes. [Launch selection](../../../overlay-backend/src/local_ai/model_state.rs#L385-L430) hashes verified 26B but chooses fallback/legacy by completeness rules; [projector selection](../../../overlay-backend/src/local_ai/model_state.rs#L479-L497) uses exact size/build support. Do not describe every startup sentinel as a new full-file hash audit. Runtime and model licenses have independent provenance; no automatic clearance follows from process isolation.

## Start, switch and readiness

[Boot ensure_servers](../../../overlay-backend/src/local_ai.rs#L1949-L2034) starts missing app-managed llama/Whisper routes and returns child handles; early liveness shortcuts are not ownership/model-identity proof. [is_reachable](../../../overlay-backend/src/local_ai.rs#L910-L928) intentionally accepts any successful curl transport even an HTTP error page.

[ensure_llama_serving](../../../overlay-backend/src/local_ai.rs#L1417-L1445) leaves reachable endpoint alone. This is conservative about not killing warming/foreign servers, but can report success without an exact alias/completion probe. Contrast [switch_local_model](../../../overlay-backend/src/local_ai.rs#L1324-L1405): serialized destructive lifecycle, owned port reclamation, target launch, strict model/completion readiness, rollback/recovery statuses. Statuses `Switched`, `RolledBack`, `FallbackStarted`, `FailedToStart` are distinct; only confirmed target success should commit requested choice.

[Port reclaim](../../../overlay-backend/src/local_ai.rs#L1212-L1276) identifies listeners and checks executable under managed root on Windows; unknown/foreign listener fails safe. POSIX reclaim path does not invent ownership by killing arbitrary PIDs. [Launch args](../../../overlay-backend/src/local_ai.rs#L2057-L2108) apply known offload/context/sampler policy or explicit CPU override. Unknown hardware behavior also depends on external llama.cpp auto-fit/defaults.

## Engine update and lifetime

[Engine update](../../../overlay-backend/src/local_ai.rs#L1603-L1729) downloads selected upstream binary archive, stages, performs a readiness check, owner-aware stops, swaps binary files with backups and attempts relaunch. [Swap/recovery](../../../overlay-backend/src/local_ai.rs#L1735-L1872) are separate operations: they minimize partial installs but do not establish power-loss atomicity across every file or fresh readiness on a stale scratch listener.

[Hidden spawn](../../../overlay-backend/src/local_ai.rs#L3062-L3088) applies no-console flags. [Windows JobObject](../../../overlay-backend/src/local_ai.rs#L3091-L3145) sets kill-on-close and attaches managed children; errors are logged best effort, not proof assignment always succeeds. The same helper is explicitly called by [TTS sidecar spawn](../../../overlay-backend/src/tts.rs#L1594-L1602) and Nemotron runner; earlier no-JobObject TTS claim is false for Windows source.

## Deep lock and MLX

[deep_lock](../../../overlay-backend/src/deep_lock.rs#L1-L71) distinguishes listening/suppress-tiles from unloading managed local model. Its guard blocks owned llama or MLX endpoint while active; external/cloud/Whisper STT/TTS/OCR are not blocked by that predicate. Model lifecycle permits explicit unlock only for owner-controlled routes; successful readiness precedes clearing lock intent.

[MLX resolver](../../../overlay-backend/src/ai/control.rs#L105-L128) checks deep lock, acquires an owned request lease and substitutes actual model/endpoint. [Host MLX route](../../../slint-experiment/src/bin/overlay_host/mlx_lifecycle.rs#L86-L140) may prepare runtime off-thread while checking chosen model/generation. This small contract does not exhaust Swift MLX sidecar, account/provider or platform memory behavior.

## Evidence and open checks

[Model tests](../../../overlay-backend/src/local_ai/tests.rs) declare choice/identity/readiness/staging/rollback cases; [deep-lock tests](../../../overlay-backend/src/deep_lock.rs#L215-L417) declare transition/ownership guards. They were not compiled or run here. [Grok local-AI candidates](../reconciliation/candidates.json) retain 11 original concerns, including liveness shortcuts, scratch-port false readiness, boot race, asset launch integrity and unknown hardware feasibility.

Acceptance requires exact-SHA Windows/macOS model switch/failure rollback, process attach/forced-exit cleanup, owned versus foreign listener fixtures, repeated lock/unlock under active request, UI state/disk save failure and actual hardware memory measurements under platform resource rules. No engine upgrade/restart/release is authorized by this research document.
