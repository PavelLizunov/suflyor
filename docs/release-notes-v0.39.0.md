# Suflyor 0.39.0

This release integrates the accumulated Omarchy port and shared Windows/native
features. The Omarchy plugin version is 0.2.0. Its interface runs inside the
existing Omarchy shell; Windows retains native Slint rather than QML.

## Highlights

- Omarchy's main panel has the flame switch in full and compact modes, and
  one-click tile timeout presets: 5, 10, 15, 30, 60 seconds or Off. The default
  remains 30 seconds; Settings supports any whole second from 1 to 600.
- Tile countdowns display tenths of a second with stable-width digits. Interaction
  resets the deadline; generation, recording, read-aloud, held menus/gestures and
  unsent edits pause it. Closing uses existing request cancellation and cleanup.
- Context-aware concise answers retain recent conversation and the last recognized
  question. With web tools enabled, fast live and automatic answers can use the
  existing search/page tools with a two-call, 15-second tool-execution budget.
- Configurable model storage reuses existing files. Existing Strata installations,
  signed official engine updates, local Vision discovery and manual model-speed
  measurements are integrated into settings.
- Nemotron 3 speaker diarization is an optional isolated native runtime, separate
  from the text model and ONNX speech processes. Windows keeps DirectML for the
  host; Linux supports CUDA-first speech with explicit CPU fallback.
- Archive playback, read-aloud, playback-only source-level balancing, native file
  dialogs, visible drag grips, tile menus and hold-to-ask help are improved.
- Omarchy installation preserves configuration, model files, recordings and bar
  placement. Updates replace the plugin and request rescan without disable/enable.

## Packages

- `suflyor-slint-setup.exe`: Windows x86_64 installer, with DirectML and isolated
  Nemotron runtime. Model weights are downloaded separately.
- `suflyor-0.39.0-omarchy-linux-x86_64.tar.zst`: native Omarchy package with the
  remote host, speech sidecars and CUDA libraries. Requires current Arch/Omarchy,
  Linux x86_64 and an AVX2 CPU; it is not a portable generic-Linux package.
- No new macOS binary is included. The existing 0.38.1 macOS package remains the
  last packaged macOS version; shared source changes are not macOS acceptance.

For Omarchy, extract the archive and run `./install.sh`. On a first installation,
run `omarchy bar put io.github.pavellizunov.suflyor`. No compiler is required.
Existing installations retain their widget placement. Do not disable/re-enable
plugins or restart the desktop just to install this update.

## Capture and verification limits

Stock Hyprland 0.56.2 black-masks excluded surfaces. The repository includes an
optional separately installed clean-capture compositor adaptation; the ordinary
plugin installer does not replace or select the system compositor. Its live
capture pixels remain unverified and mirrored outputs retain black masking.
Do not interpret an applied capture rule as a portable privacy guarantee.

The reported desktop crash was traced to expired Lua binding handles during
LAMZU PTT-button replacement, not the tile countdown. Its helper repair belongs
to the separate LAMZU/MX Ergo plugins and is not shipped in this Suflyor archive.
Hyprland's cross-device binding-removal semantics and a separate Aquamarine exit
crash remain outside this release's fixes.

Native builds and package creation do not establish end-to-end runtime acceptance.
No new visual previews, manual countdown/input acceptance, capture checks, model
inference, discretionary test suite, lint or independent review were requested
for this release. Existing repository-triggered CI reports its own results;
Windows installation, macOS runtime and full speech/AI/capture parity are not
claimed as verified.
