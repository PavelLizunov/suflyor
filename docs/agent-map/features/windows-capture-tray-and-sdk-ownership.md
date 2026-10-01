# Windows capture, affinity, tray and SDK ownership: bounded source contract

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`; selected executable sources, no Win32 calls/device/screenshot/clipboard/UI/SDK build or native fault test. [SDK name census](<../native/windows-sdk-name-edges.md>) is syntax candidate navigation, not compiler-resolved Windows graph. Extends the earlier hotkeys/capture contract with ownership/partial-failure detail.

## GDI rectangle ownership

[Capture adapter](<../../../slint-experiment/src/native/windows/screen.rs#L7-L87>) rejects nonpositive w/h, gets screen DC, memory DC, bitmap, cleaning earlier acquisitions on later acquisition failure. It selects bitmap into mem DC, BitBlts physical rectangle and deselects **before** GetDIBits. Ordinary Result exits delete bitmap/DC/release screen; cleanup results ignored. Bitmap old selection isn't explicitly validated, and allocation/panic paths aren't caught by this manual cleanup (no RAII). Native GDI misuse/leak fault scenarios untested.

BGRA is top-down via negative biHeight. Buffer length `(w as usize) * (h as usize) * 4` is unchecked multiply and allocates after DC/bitmap acquisition. GetDIBits error when `lines == 0`; a nonzero partial scanline result is **not required equal h**, so success doesn't prove all rows populated. Geometry comes from OS capture facade; malicious/huge w/h preconditions not demonstrated. No unconditional complete-image/cleanup guarantee from Result success.

[Facade](<../../../slint-experiment/src/capture.rs#L18-L46>) uses primary monitor; [virtual desktop](<../../../slint-experiment/src/capture.rs#L109-L129>) uses full physical bounds including negative origins. [Monitor enumeration/bounds](<../../../slint-experiment/src/win32.rs#L468-L530>) queries monitor rectangles and virtual metrics; per-monitor DPI/deselection/partial scanlines are untested.

## Own-window hide/flush/restore versus WDA

[GDI hide](<../../../slint-experiment/src/win32.rs#L545-L616>) enumerates top-level windows in own process, ignores others, remembers visibility/topmost, hides; DwmFlush before capture. Restoration uses HWND values later, shows NOACTIVATE and resets TOPMOST when remembered. Handles/EnumWindows/ShowWindow/DWM results best effort; no RAII guard restores after arbitrary panic. Cross-process windows aren't hidden by this path.

[F8 caller](<../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L214-L234>) hides → freezes screenshot → shows again → matches error and generic tile. Ordinary capture Err is restored before error dispatch. It is synchronous on event-loop callback; native capture delay/large allocation may stall UI. No screenshots/native error/focus/topmost acceptance.

[Stealth](<../../../slint-experiment/src/win32.rs#L325-L379>) sets WDA_EXCLUDEFROMCAPTURE/NONE, ignores cosmetic cursor-guard install failure, then reads GetWindowDisplayAffinity and returns Err on mismatch. `presentable_stealth` requires intent + successful effective affinity. WDA is separate from internal GDI self-exclusion; source comments say BitBlt ignores WDA, so hiding is required. Apply/readback proves requested OS flag source contract, not every external conferencing/capture application's screenshot behavior.

[Taskbar style](<../../../slint-experiment/src/win32.rs#L389-L434>) temporarily hides visible window, resets TOOLWINDOW/no APPWINDOW, frame changes, shows NOACTIVATE. Its extended-style set is unchecked and `skip` doesn't reverse TOOLWINDOW baseline. [HWND borrow](<../../../slint-experiment/src/win32.rs#L122-L151>) requires realized Slint raw handle; force-hide falls back no-op if unavailable. Native presentation/layout/race completeness remains open.

## Singleton guard and clipboard/input

[Mutex](<../../../slint-experiment/src/native/windows/lifecycle.rs#L14-L56>) CreateMutex initial_owner=false, waits, handles OBJECT_0/ABANDONED as ownership success, closes on other wait result. Drop ReleaseMutex+CloseHandle ignore errors; main lifetime guard came from preflight contract. Windows mutex ownership is thread-sensitive; source guard doesn't itself document a type-enforced UI-thread marker here, and no cross-thread misuse tested.

[Clipboard](<../../../slint-experiment/src/native/windows/clipboard.rs#L3-L23>) reads empty-filtered string, result-aware set uses category error without text payload; best-effort write/clear discard result. [Ctrl+C](<../../../slint-experiment/src/win32.rs#L890-L954>) maps scan codes, emits Ctrl/C down+up sequence, returns exact SendInput count==4. Partial native insertion can return false; it doesn't separately repair outstanding injected modifiers. Modifier guard checks Alt/Ctrl/Shift hardware states, not arbitrary external clipboard semantics. No clipboard contents touched or input injected by research.

## Tray lifetime, restoration and callbacks

[Install slot/context/Drop](<../../../slint-experiment/src/tray.rs#L182-L255>) uses atomic one-per-process claim plus thread_local callback boxes. `TrayHandle` contains HWND; no Rc PhantomData thread marker exists in this selected struct. Contract says install current UI thread; Drop assumes same-context use, deletes notify icon/window, clears atomics/context. This is source/caller lifetime evidence, not a compiler-enforced all-thread guarantee.

[Native installation](<../../../slint-experiment/src/tray.rs#L260-L327>) registers class, loads icon with IDI_APPLICATION fallback, creates hidden popup message HWND (not message-only), registers Explorer restart message. Notify icon is **not** added on startup; slot/window permit later hide. [set_visible](<../../../slint-experiment/src/tray.rs#L377-L413>) adds icon on hide, marks false/publishes failure on NIM_ADD Err, deletes on restore; SETVERSION/NIM_DELETE results best effort.

[Wndproc](<../../../slint-experiment/src/tray.rs#L437-L507>) re-adds on Explorer restart only when icon-visible flag true, publishes availability/failure, NIN_SELECT/KEYSELECT dispatch show-hide; context requests deduplicated by atomic. Direct thread-local callback invokes host; no catch_unwind in inspected dispatch across extern system boundary, panic behavior untested. [Host install](<../../../slint-experiment/src/bin/overlay_host_windows.rs#L4775-L4813>) retains guard, availability callback restores bar if tray disappears while hidden. [Slint tray menu](<../../../slint-experiment/src/bin/overlay_host/bar_tray.rs#L170-L241>) localizes menu from current config, uses monitor work-area and scale, native return-focus on dismiss. None tested by registration source alone.

## SDK census and acceptance boundary

Four files yield 180 Windows SDK import-name records and 121 direct/qualified call syntax candidates (including types/constructors), with source signatures/ranges/function container. Alias names mapped syntactically; lexical shadowing, function pointers/callbacks/macros/cfg/Posix stubs remain unresolved. Not backend JobObject/audio/credential/GDI exhaustive SDK inventory. Seven research fixtures verify extraction and selected source-order facts; no Windows SDK/parser guard execution, external screen-share, resource-pressure/fault UI or independent acceptance. Original 39/75/5 classifications unchanged.
