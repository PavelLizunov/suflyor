# Taskbar window audit — 2026-08-03

## Report

The overlay bar did not create a running taskbar button, while Tile and
Settings windows did. The defect was in the shared transparent-window Win32
style path, not in the Slint layouts.

## Before

The old binary produced this live Tile state after the deferred decoration:

| Window | Ex-style | `WS_EX_TOOLWINDOW` | `WS_EX_APPWINDOW` | Owner |
| --- | ---: | --- | --- | ---: |
| Bar | `0x1b8` | true | false | `0` |
| Tile | `0x40198` | true | **true** | `0` |

`apply_transparency` added `TOOLWINDOW` but preserved winit's `APPWINDOW`.
With both flags present, Explorer kept the taskbar button. Settings used the
same `make_transparent_tile` path.

Safe visual evidence: [old bar](before-old-bar.png) and
[taskbar before](before-taskbar.png). Full-desktop captures were deliberately
removed because they contained private desktop content.

## Fix

`apply_transparency` now reuses the existing `set_skip_taskbar` /
`skip_taskbar_exstyle` transition. It clears `APPWINDOW`, forces `TOOLWINDOW`,
performs the required hide/restyle/show-no-activate shell refresh, then applies
the existing click-through or interactive transparency bit.

A pure regression test covers both the bar and interactive Tile/Settings
style variants while preserving unrelated extended-style flags.

## After

Live probe against QA binary SHA-256
`CBF43BA80136CC09ACED4F67D76FDD06C2BD68DC7F2A3CA27A3C58E3FAA981CB`:

| Window | Ex-style | `WS_EX_TOOLWINDOW` | `WS_EX_APPWINDOW` |
| --- | ---: | --- | --- |
| Bar | `0x1b8` | true | false |
| Tile | `0x198` | true | false |
| Settings | `0x198` | true | false |
| Help | `0x198` | true | false |
| F4 palette | `0x198` | true | false |
| Session archive | `0x198` | true | false |

After the complete hotkey smoke, all 11 visible process HWNDs had
`TOOLWINDOW=true`; zero had `APPWINDOW=true`. Windows UI Automation reported
only the pinned `suflyor (Slint)` launcher, with no running Tile or Settings
taskbar button.

Slint MCP captures: [bar](after-bar-mcp.png),
[Settings](after-settings-mcp.png), [Tile](after-tile-mcp.png), and
[taskbar after](after-taskbar.png). The Settings and Tile surfaces remained
painted and interactive after the hide/restyle/show transition.

## Verification

- Targeted unit test:
  `win32::tests::transparent_windows_clear_appwindow_without_losing_other_flags`
- Slint MCP server: `slint-mcp-embedded`, 14 tools.
- MCP surfaces inspected: bar, Settings at 720x600, Tile at 460x360.
- Global shortcuts dispatched distinctly on the same binary: `F1`, `F3`,
  `F4`, `F6`, `F7`, `F8`, `Shift+F8`, `Ctrl+F8`, `F9`, `Shift+F9`,
  `Shift+Alt+1`, `Shift+Alt+2`, `Shift+Alt+3`.
- Capture surfaces were cancelled with `Esc`.
- `Shift+Alt+1` copied 29 characters from a disposable Notepad selection.
- Qwen `qwen3.8-max-preview`: final `APPROVE`; two earlier read-only attempts
  timed out and were recorded rather than treated as reviews.
- Full repository gate: `scripts/ci.ps1` ended with
  `All gating layers green` (Slint, backend, TTS, and i18n checks).
- One earlier backend run hit the existing timing-sensitive local-AI readiness
  test while port 8080 was transiently active; the test passed alone and the
  unchanged full gate then passed completely.
