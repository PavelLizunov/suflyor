# Hotkeys, window presentation and capture: source-linked contract

**Evidence:** source inspection at `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; no global-key injection, native screenshots, monitor/DPI/affinity/clipboard test executed. This is registration/dispatch and privacy presentation, not functional UI acceptance.

## Global keys: registration is not execution

[register_hotkeys](../../../slint-experiment/src/bin/overlay_host/hotkeys.rs#L74-L231) builds 13 keys and records successes/failures. Returned manager must stay alive; dropping it unregisters bindings. [Registry diagnostics](../../../slint-experiment/src/bin/overlay_host/hotkeys.rs#L30-L64) is a startup outcome snapshot, not proof the correct handler later dispatched.

| Key | Actual reviewed dispatcher behavior |
| --- | --- |
| F1 | Help window |
| F3 | Re-ask previous question |
| F4 | KB palette |
| F6 | Manual text ask tile |
| F7 | Session archive |
| F9 | Main Text-route ask |
| Shift+F9 | One-shot Cloud-route override without changing persisted provider |
| F8 | Region capture using Describe or configured TestPractice mode |
| Shift+F8 | Region capture Translate mode |
| Ctrl+F8 | Region OCR/read-aloud, local platform engine |
| Shift+Alt+1 | Native copy of current foreground selection, text-only clipboard restore, then read-aloud tile |
| Shift+Alt+2 | Region OCR/read-aloud |
| Shift+Alt+3 | Pause/resume current speech target and synchronize matching tile indicator |

[Host dispatcher](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2519-L2819) is authoritative. Earlier prose describing F8 as full-monitor capture and Shift+F8 as drag-only variant is historical; both reviewed paths use the region-selection overlay with different modes. Platform capture extent also differs (Windows virtual desktop versus macOS display under cursor).

[Read-selection branch](../../../slint-experiment/src/bin/overlay_host_windows.rs#L2701-L2787) saves clipboard text, polls modifier release, clears clipboard, sends native copy, waits 140 ms, reads returned text and restores original text. This is not preserving rich/image clipboard formats or reading clipboard history. Failures use a visible generic error; arbitrary selected sensitive text can intentionally enter a tile/TTS.

## Window identity, realization and input

[Windows HWND extraction](../../../slint-experiment/src/win32.rs#L127-L135) depends on realized raw handle; [macOS view-id seam](../../../slint-experiment/src/win32.rs#L1060-L1069) is distinct, unsupported platforms use harmless stubs, not product support.

[Presentation helper](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L285-L378) parks auxiliary windows off-screen, shows them, retries handle extraction, decorates and repositions. [Tile presentation](../../../slint-experiment/src/bin/overlay_host/tile_window.rs#L259-L318) has its own path and applies native styling/monitor placement later. Missing realization, first-frame visibility and Winit size timing require exact live capture; no guarantee follows from source comments.

[Native transparency](../../../slint-experiment/src/win32.rs#L196-L274) configures DWM/style flags and deliberately clears accidental click-through for interactive surfaces. [Taskbar style](../../../slint-experiment/src/win32.rs#L389-L434) forces TOOLWINDOW/no APPWINDOW baseline in both directions; hide/restyle/show is a separate visibility effect. [Input shortcut filters](../../../slint-experiment/src/bin/overlay_host/kbd_shortcuts.rs) handle editing per window, not global ask/vision hotkeys.

[Tile monitor choice](../../../slint-experiment/src/win32.rs#L858-L876) requires a primary and chooses a nonprimary only when landscape and sufficiently wide. [Placement](../../../slint-experiment/src/bin/overlay_host/tile_window.rs#L374-L450) uses work area/DPI/clamping and explicitly ignores too-small fallback geometry. Coordinate validity, negative origin, mixed-DPI bounds and bottom-right overflow remain native measurements.

## Stealth intent versus effective result

[global intent/effective flags](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L54-L99) distinguish requested stealth and bar's last verified capture-affinity result. [Bar apply](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L122-L175) checks native set/readback and shows fault rather than optimistic success. **Effective is bar-only**, not a verified all-window aggregate.

[Auxiliary apply](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L176-L188) logs failure without returning success; [reveal](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L315-L354) still moves on-screen after it. This is not fail-closed window privacy; actual leak when native affinity fails remains a high-priority test. [WindowRegistry](../../../slint-experiment/src/bin/overlay_host/window_lifecycle.rs#L493-L589) reapplies some settings to windows/tiles; tray menu has its separate gate/lifetime.

[Windows set_stealth](../../../slint-experiment/src/win32.rs#L323-L359) sets WDA_EXCLUDEFROMCAPTURE and reads it back. [macOS global capability](../../../slint-experiment/src/win32.rs#L1097-L1113) is false and enabled setting returns unsupported. Internal self-exclusion below must not be advertised as OS-wide exclusion from external capture applications.

## Screenshot source and own-window exclusion

[Host freeze](../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L192-L244) checks permissions, hides own windows, captures native desktop, restores before match/error and creates Slint image. Return/error is not the same as panic/forced-termination cleanup; no general RAII restoration is claimed.

[Windows hide/show](../../../slint-experiment/src/win32.rs#L551-L612) enumerate own visible windows, hide and compositor-flush before GDI capture, then restore appropriate visibility. [GDI readback](../../../slint-experiment/src/native/windows/screen.rs#L3-L87) captures a physical rectangle, deselects bitmap before GetDIBits and releases normal/error resources. WDA exclusion does not protect own-window pixels from this GDI path, hence explicit hide is necessary.

[macOS screenshot](../../../slint-experiment/src/native/macos/screen.m#L112-L182) uses ScreenCaptureKit filter excluding own app or own windows and returns failure if no self-exclusion identity is available. Capture controller/Objective-C callback lifetime, permissions, timeout and restored UI are separate native checks. OCR uses platform-local processing; vision mode encodes/transmits image per [AI/vision contract](ai-and-vision-routing.md).

## Acceptance boundaries

Source-declared window/style/native guards and hotkey startup logs are useful static evidence, not screenshot/function passes. Exact candidate QA must exercise all 13 registrations/handlers, modifiers/keyboard layouts, selection-copy restore, region modes, focus/tray reopen, mixed DPI, negative monitor origin and actual WDA failure/screen-share result. Changes to visible behavior require the owning Slint-MCP procedure, not this source document as a substitute.
