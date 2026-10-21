# Experimental Omarchy native milestone evidence

This is a development checkpoint, not Linux feature parity or a release.

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

## Still not connected or verified

PipeWire microphone/system audio, screen capture/OCR, external capture exclusion,
compositor global shortcuts, native drag/multi-monitor placement, Quattro plugin,
launcher/package/autostart and real inference are not delivered. Existing product
controls can still describe Windows-only functions; this is explicitly experimental.
No Linux release or desktop-owner deployment was performed. The private QA process
and wire fixture are task-owned services with bounded runtime. No owner desktop
configuration or heavy local build was used.

Private evidence is outside Git under the task's `omarchy` evidence directory:
matching before/after native PNGs, MCP trees, wire metadata, binary hash, worker
manifests, logs and exact exit markers. Published v0.38.1 assets are unchanged.
