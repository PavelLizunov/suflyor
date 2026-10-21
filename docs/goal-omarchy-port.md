# Omarchy port: first native milestone

Status: active development; not a supported release or feature-parity claim.

## Goal

Start Suflyor on the dedicated Omarchy/Hyprland test worker using the existing
Rust backend, Slint windows and shared host orchestration. The first milestone
is a real Wayland application with Settings, typed questions and answer tiles.
Do not rewrite the UI in QML merely to obtain a shell entry point.

## Scope and reuse

- Repair the missing backend platform branches and select Linux portal dialogs.
- Reuse the existing Unix singleton lock and provide Wayland clipboard access.
- Compile the shared host on Linux. Unsupported audio, capture, local installers
  and capture exclusion must remain explicit errors or unsupported capabilities,
  not successful no-ops or Windows executables downloaded on Linux.
- Let the compositor own placement where Wayland denies absolute coordinates.
  Hyprland-specific placement, PipeWire capture, compositor global shortcuts and
  a Quattro launcher plugin follow as separate measured slices, reusing installed
  service/widget/overlay patterns and reviewed upstream licenses.

## Acceptance

- [ ] Record baseline and candidate commit/tree, worker resources and commands.
- [ ] Backend Linux check and relevant platform tests pass on the test worker.
- [ ] The shared host compiles and starts in the existing Wayland session.
- [ ] Inspect actual Slint MCP Settings, question and answer surfaces in EN/RU.
- [ ] A disposable local HTTP fixture proves typed dispatch and rendered answer;
      this does not claim a real provider or model inference.
- [ ] Clipboard copy and singleton exclusion work on the native worker.
- [ ] Windows targeted gate remains green for the exact candidate.
- [ ] Record unsupported shortcuts/audio/capture, deployment and inference gaps.

## Developer launch

On the test worker, run `bash scripts/check-linux-port.sh` and build the QA host
with `SLINT_EMIT_DEBUG_INFO=1 cargo build --locked --bin overlay-host --features
ui-mcp --manifest-path slint-experiment/Cargo.toml`. For Omarchy 4.0.4's Lua
Hyprland configuration, source the absolute checkout path to
`scripts/suflyor-hyprland.lua` from the user's `hyprland.lua`, then validate with
`hyprctl reload` and `hyprctl configerrors`. The snippet matches only Suflyor
windows; it supplies the floating geometry that Wayland clients cannot enforce.
The fixed-size winit hint alone did not make Hyprland float these windows.

Start `overlay-host` with `SLINT_BACKEND=winit-software` and
`WINIT_UNIX_BACKEND=wayland`. Use an isolated `XDG_CONFIG_HOME` for fixtures.
No autostart, system package, Quattro plugin or release artifact is installed
by these commands. Remove the explicit snippet source line to remove its rules.

## Verification environment

Use `omarchy-test`, an approved dedicated worker. Its initial 4 GiB allocation
proved insufficient for a practical full debug host build (heavy swap use).
After checking physical-host headroom, only VM 107 was shut down cleanly and
raised to 8 GiB; networking and the existing Hyprland session recovered. Use
one owned managed build at a time, two Cargo jobs, disabled incremental builds,
no debug symbols and a bounded service memory limit. Source and editing stay in
an isolated task worktree. Do not build or alter the owner's desktop shell.

Claude's scoped October 17/18 project note is reconnaissance, not current
verification: it reported four backend missing branches and 26 host errors.
The fresh baseline and exact-candidate evidence decide present status.
