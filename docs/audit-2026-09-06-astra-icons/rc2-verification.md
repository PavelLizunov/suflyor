# Astra icons — Mac RC2 verification

This report supersedes the incomplete historical checkpoint in README.md.
It qualifies a Mac **test prerelease**, not full product acceptance.

## Identity and scope

- Source: `8f6197cd9df14e1b2e17277811b514d092ce51e2`.
- Original baseline: `bffbfd444f6208e2a285c6129a4cc27b039cdf86`.
- PR #171 merged into master as `2c694c38c7351cb9196991a89eaaf033ceb180d7`.
- All 50 service SVG geometries directly rewritten by the lead Astra agent.
  These are coordinate-source drawings, not traced image-generation sheets.
  Earlier Gemini draft history remains disclosed in the historical checkpoint.
- Nine protected brand assets remain byte-identical to the original baseline.
  No logo or mascot regeneration. Slint UI source and action wiring unchanged.
- Follow-up dependency fix only updates Cargo-resolved der 0.8.0 to 0.8.1 and
  its checksum. Other package entries identical; cargo-deny passes again.

## Artifact

`Suflyor-0.38.1-rc.2-macos-arm64.dmg`, 55,580,159 bytes.
SHA-256: `9684eb32845b87b29d44f46fb6d22c8ee0735fac5272b042b8335f86ba617d3f`.
Normal release binary, no ui-mcp feature shipped. Apple Silicon/macOS 14.2+.
Ad-hoc signed; not notarized by Apple. Use the documented Open Anyway flow.

The existing Mac packaging scripts built Rust host, Piper/Tera sidecars and
locked Swift MLX/Metal components. ExFAT AppleDouble metadata caused initial
signature verification to fail. The same source and compiled release cache
were packaged with bundle/DMG staging on APFS; strict/deep codesign checks then
passed both before and after mounting the DMG. No signature checks bypassed.
Downloaded DMG hash independently matched the worker's hash.

A copy from the actual DMG into a private smoke Applications directory passed
signature and embedded version checks and stayed running after LaunchServices
launch. Synthetic HOME isolation was confirmed by runtime-created catalog/log
files. This did not replace the owner's installed application or credentials.

## Executed verification

On mac-worker, jobs=2, test threads=2, incremental=0, memory preflight >=40%:

```sh
cargo test --locked --manifest-path slint-experiment/Cargo.toml --test icon_guard --test version_guard --test macos_app_packaging_guard
SLINT_EMIT_DEBUG_INFO=1 cargo build --locked --manifest-path slint-experiment/Cargo.toml --bin overlay-host --features ui-mcp
bash slint-experiment/scripts/build-macos-dmg.sh
```

Guards passed: icons 2/2, version 2/2, packaging 9/9. Both builds and final
APFS packaging passed. Separate original-baseline MCP build also passed.
QA candidate binary SHA-256:
`929ad89a194caf3bcdfb2b2113cd24a96fdd486545301d8fe7369d0554d9c533`.
Baseline QA SHA-256:
`6f07254975e1ed2749daf70ba6e0c2356c2334f951b35703c84a9b05f4690c4b`.

Physical Windows selected gate passed on the same source:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\git-gate-native.ps1 manual -Base bffbfd444f6208e2a285c6129a4cc27b039cdf86
```

Formatting, Clippy and crate tests/guards passed. GitHub CI run 34154601220
and security run 34154601221 passed, including required gate and macOS job.
No branch-policy bypass was used for the owner-authorized merge.

## Live comparison and review

Native Mac MCP, fresh synthetic HOME, no copied personal configuration, neutral
endpoints, no session/AI requests. Settings 720x600, bar1280x64, scale1.
Mic toggle muted for subsequent QA; system auto-start disabled. HOME is profile
isolation, not a TCC, clipboard or network sandbox.

Independent visual review inspected 39 pairs / 78 PNGs:
- EN Glacier: all 16 Settings tabs.
- RU Glacier: all 16 Settings tabs and bar.
- EN Graphite, Obsidian, Light Frost: bar and Interface per theme.

Lead also inspected matched EN bar and additional candidate Help (640x680),
empty archive (720x540), text ask (480x112), and palette (520x420). AI providers
and Hermes scrollbars were dragged to reveal lower content. First stale frames
after tab transitions were rejected and replaced by settled second captures.
Raw images remain private: example endpoints, device/path metadata must not be
published unscreened. No observed new SVG clipping or surrounding layout shifts.
RU bar desktop y varied by32 pixels; window-local layout matched, not placement.

## Known limitations and findings

- Coaching/Diagnostics share monitor-chart imagery, reducing icon-only
  differentiation; labels preserve meaning. This is a design tradeoff.
- Matched baseline confirms existing Shift+F9 text clipping, mixed-language
  status fragments, and Alt/Option inconsistency. Not caused by SVG changes.
- F4 actual macOS input produced a dispatch log and palette window. F1/F7 input
  attempts did not establish dispatch despite registration; UI buttons opened
  corresponding windows. Unverified functional shortcuts: F1, F3, F6, F7, F8,
  Shift+F8, Ctrl+F8, F9, Shift+F9, Shift+Alt+1, Shift+Alt+2, Shift+Alt+3.
- Capture/clipboard/network/audio flows were deliberately not exercised on
  real data. Windows hotkey smoke was not repeated for this RC.
- No all-DPI, all-hover/active/busy/populated-state, full audio/AI, or complete
  matrix of every tab in every theme/language claim. User retest remains needed.
- No new Windows installer is included in this Mac-focused prerelease.

Tester form: ../retest-v0.38.1-rc.2-astra-icons.html.
