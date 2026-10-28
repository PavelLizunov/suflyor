# Hyprland clean capture for Suflyor

This is an opt-in local compositor build for Hyprland **0.56.2**, not a QML
plugin and not a new Suflyor or Hyprland release. It keeps protected surfaces
visible on the physical output and re-renders monitor/region capture without
them, revealing the scene underneath instead of drawing black rectangles.
Actual capture behavior remains unverified until exercised in the running build
with the intended capture application.

## Provenance

Adapted from [hyprwm/Hyprland#16061](https://github.com/hyprwm/Hyprland/pull/16061),
“screenshare: render no_screen_share views as if they did not exist”, by
richar (`ri-char`, `wxsychi@163.com`). The original head is
`eb4edf1cd20ae02cae8c83b819cd8e59100572c2`, based on
`60695e18c8cd17399651a9af5db3f42d1867e1e6`. The PR was closed unmerged by the
unvouched-author bot; this is not an upstream-supported feature.

The local target is `efb50993780079460b0cbed1363e2166a2de1d9f` (`v0.56.2`).
The adaptation retains window/layer/popup ownership and fadeout filtering. It
also considers protected fadeouts when choosing the clean-render path, uses the
0.56.2 visibility API, and never checks input regions to decide whether a visible
surface is protected. Suflyor's input-transparent bar mark is covered too.

The clean scene is rendered into a private, session-owned native-size framebuffer
before the existing export transform and crop. Local display/mirror textures are
not changed. Capture-only filtering and feedback suppression remain active while
the deferred render pass executes. The optimized display blur cache is not sampled
or overwritten by that pass. A static renderer flag avoids moving existing
renderer members used by native plugins; this does not certify plugin ABI
compatibility. Hyprland's BSD-3-Clause notice is retained in `LICENSE.hyprland`.

## Build and install

On the matching Arch/Omarchy system, with the existing C++ compiler, CMake,
Wayland/OpenGL/Hyprland development libraries, `make`, `patch`, `curl`, and `tar`:

```sh
bash scripts/build-hyprland-capture.sh
```

The script pins source/dependency downloads by SHA-256, builds only Hyprland
without tests, and installs to a patch-revisioned directory under
`~/.local/lib/suflyor-hyprland/`. It uses one compiler job by default. Build-only
scanner, protocol, udis86 and glaze sources are private to the cache; no system
package or installed Hyprland executable is replaced. Source/build dependencies
must match the installed libraries. No system upgrade is performed.

The source directory and build directory are keyed by patch hash. Change
`SUFLYOR_HYPRLAND_CACHE` or `SUFLYOR_HYPRLAND_PREFIX` to override their locations.
The source archive, udis86/protocol submodules, glaze and scanner hashes live in
`build-hyprland-capture.sh`. The scanner package is x86-64 and uses installed
pugixml. The distribution's existing `/usr/bin/start-hyprland` remains the watchdog.

## Select for the next login, without ending the current session

Pass the exact installed executable printed by the build script:

```sh
bash scripts/hyprland-capture/session.sh enable /absolute/path/to/Hyprland
```

This installs a user-owned `~/.local/share/wayland-sessions/hyprland.desktop`
override for **uwsm**'s XDG session lookup. Omarchy's existing `uwsm start ...
hyprland.desktop` launch then uses the private binary with the stock watchdog.
It does not stop or restart the current compositor, change Hyprland Lua settings,
touch the bar layout, or replace the system login-manager entry. A direct session
not launched through uwsm is not redirected by this mechanism.

Save work and finish any game before logging out and back in. A compositor change
cannot be hot-loaded with `hyprctl reload`. Installing/selecting a binary is not
evidence that the current compositor has loaded it. This explicitly requested
status command distinguishes the next-login choice from the current process:

```sh
bash scripts/hyprland-capture/session.sh status
```

A running adapted build identifies branch `suflyor-capture-pr16061` and its patch
revision in `hyprctl version`. That reports loaded code, not privacy acceptance.
Keep Suflyor stealth enabled to apply its existing `no_screen_share` layer rule.

## Rollback

From a terminal or TTY, using the same account:

```sh
bash scripts/hyprland-capture/session.sh disable
```

The command removes only this script's marked session override and leaves a
local disable marker. Even a cached uwsm command using its launcher then invokes
the stock watchdog without `--path`. The active compositor is unchanged until
the next login. The system `/usr/bin/Hyprland` and `/usr/bin/start-hyprland` remain
available; there is no package downgrade or model/configuration restoration.

## Scope and remaining limits

- Normal monitor and region capture re-render only when a protected view/fadeout
  is present. Window-only sharing retains its existing behavior and permissions.
- Mirrored outputs retain upstream **black masking**, not see-through capture.
- A clean-framebuffer allocation failure returns no clean texture rather than
  copying the protected display mirror as a silent fallback.
- Capture methods outside Hyprland's screenshare manager can ignore this rule.
- Re-rendering costs an additional scene pass while protected content is visible.
  No frame-rate, GPU, latency or power measurements have been made.
- Real screenshots/sharing, DMA-BUF versus shared-memory copies, rotated/scaled
  displays, blur, closing animations, lock screens and native plugin runtime
  compatibility have not been acceptance-tested. No tests, linters, previews,
  extra model reviews or model inference are implied by a successful build.
