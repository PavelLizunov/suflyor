# Native Platform Adapters & Window Management (`slint-experiment/src/native`)

Local guide for native OS integration adapters under `slint-experiment/src/native/` and platform window management in `slint-experiment/src/win32.rs`.

---

## 1. Scope & Architecture

Native platform adapters isolate OS-specific FFI, window manipulation, single-instance process locking, clipboard integration, screen capture, and native menu bar/tray behaviors from host orchestration code.

### Adapter Module Map
- **Windows Implementation:**
  - `src/native/windows/clipboard.rs`: Wraps `clipboard-win` (`read_text`, `set_text`, `write_text`, `clear`).
  - `src/native/windows/lifecycle.rs`: Win32 named mutex (`Global\suflyor-overlay-singleton`) via `CreateMutexW` / `WaitForSingleObject` to coordinate emergency relaunch.
  - `src/native/windows/screen.rs`: GDI `BitBlt` virtual-screen capture (`GetDC`, `CreateCompatibleDC`, `GetDIBits` with negative height for top-down BGRA).
  - `src/win32.rs`: Win32 HWND manipulation, DWM frame extension (`DwmExtendFrameIntoClientArea`), blur-behind (`DwmEnableBlurBehindWindow`), click-through (`WS_EX_TRANSPARENT`), taskbar toolwindow styling (`WS_EX_TOOLWINDOW`), topmost (`HWND_TOPMOST`), stealth (`WDA_EXCLUDEFROMCAPTURE` with readback validation `GetWindowDisplayAffinity`), manual window drag delta tracking, and `EnumDisplayMonitors` monitor placement (`pick_monitor`).
- **macOS Implementation:**
  - `src/native/macos/clipboard.rs` & `clipboard.m`: AppKit pasteboard integration (`NSPasteboard`), selection copy modifier check, synthetic Command+C via `CGEventPost` (guarded by `AXIsProcessTrusted`).
  - `src/native/macos/lifecycle.rs`: Per-user file lock (`suflyor-overlay-singleton.lock` under data root) using `File::try_lock()` with polling retry for relaunching instances.
  - `src/native/macos/screen.rs` & `screen.m`: CoreGraphics display list enumeration, ScreenCaptureKit screenshot capture (`SCScreenshotManager` excluding Suflyor windows/app), Screen Recording permission preflight (`CGPreflightScreenCaptureAccess` / `CGRequestScreenCaptureAccess`), and Apple Vision OCR (`VNRecognizeTextRequest`).
  - `src/native/macos/status.rs` & `status.m`: Native AppKit menu bar status item (`NSStatusItem`, `NSMenu`), toggle overlay visibility, and quit callback through the event loop.
  - `src/native/macos/window.rs` & `window.m`: AppKit window level configuration (`NSPopUpMenuWindowLevel`), space collection behavior (`CanJoinAllSpaces`), transparency, window activation (`activateIgnoringOtherApps:YES`), and drag delegation (`performWindowDragWithEvent`).
- **Platform Seams & Selection:**
  - `src/native/mod.rs`: Conditional module paths using `#[cfg(windows)]` and `#[cfg(target_os = "macos")]`.
  - `src/win32.rs`: Exposes `windows_impl` on Windows, and `posix_stubs` on non-Windows (delegating to macOS native calls when `target_os = "macos"` or returning safe stubs for unsupported platforms).

---

## 2. Platform Responsibilities: Windows vs macOS

| Domain | Windows (`windows/` + `win32.rs`) | macOS (`macos/`) |
|---|---|---|
| **Role & Priority** | Primary target platform (native overlay product). | Supported secondary desktop target platform. |
| **Window Level / Topmost** | `SetWindowPos` with `HWND_TOPMOST` / `HWND_NOTOPMOST`. | `NSPopUpMenuWindowLevel`, `CanJoinAllSpaces`, `orderFrontRegardless`. |
| **Transparency & Styling** | DWM frame extension + blur-behind, `WS_EX_TRANSPARENT` / `WS_EX_TOOLWINDOW`. | AppKit `opaque = NO`, `backgroundColor = [NSColor clearColor]`. |
| **Stealth / Capture Exclusion**| Win32 `SetWindowDisplayAffinity` (`WDA_EXCLUDEFROMCAPTURE`). | ScreenCaptureKit `SCContentFilter` excluding Suflyor process (`self_pid` / `own_windows`). |
| **Screen Capture** | GDI `BitBlt` + `GetDIBits` from desktop DC (requires hiding overlay windows first via `hide_own_windows` + `DwmFlush`). | ScreenCaptureKit `SCScreenshotManager captureImageWithFilter`. |
| **OCR Engine** | Tesseract OCR (via `overlay-backend::ocr`). | Apple Vision Framework (`VNRecognizeTextRequest` off-thread). |
| **Single-Instance Mutex** | Win32 named mutex `Global\suflyor-overlay-singleton`. | File lock `suflyor-overlay-singleton.lock` via `try_lock()`. |
| **Clipboard & Copy** | `clipboard-win` crate + synthetic `SendInput` Ctrl+C (with hardware scan codes). | `NSPasteboard` FFI + synthetic `CGEventPost` Command+C (`AXIsProcessTrusted`). |
| **Status Bar / Tray** | Windows Notification Area Tray (`Shell_NotifyIconW`). | AppKit `NSStatusItem` status item + menu delegate. |
| **Linux Support** | **Explicitly unsupported**. `posix_stubs` provides fallback defaults; do not invent Linux support. | N/A |

---

## 3. Unsafe & FFI Boundaries

1. **Windows Win32 FFI:**
   - Managed via Microsoft's `windows` crate (v0.62) features (`Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Dwm`, `Win32_Graphics_Gdi`, `Win32_System_Threading`, etc.).
   - GDI BitBlt bitmap cleanup must release/delete GDI objects (`DeleteObject`, `DeleteDC`, `ReleaseDC`) on **all** exit paths.
   - `GetDIBits` requires the bitmap handle to be deselected from the DC prior to retrieving bits.
   - Window subclassing (`SetWindowSubclass`) for stealth cursor override handles `WM_SETCURSOR` and `WM_TIMER`.
2. **macOS C / Objective-C FFI (`extern "C"`):**
   - Native C/Objective-C bridge functions compiled via `cc` in `build.rs` (`window.m`, `status.m`, `clipboard.m`, `screen.m`) into `libsuflyor_appkit.a`.
   - Payload strings passed across FFI use explicit byte lengths or duplicate C-strings (`strdup`) requiring matching free functions (`suflyor_macos_free_string`, `suflyor_macos_free_screenshot_buffer`).
   - AppKit UI calls must execute on the **main thread** (`[NSThread isMainThread]`).
   - Objective-C memory management uses ARC (`-fobjc-arc`) with explicit `@autoreleasepool` blocks around Vision/ScreenCaptureKit image processing.
3. **Slint Raw Handles:**
   - Slint window extraction uses `raw_window_handle` (0.6). `grab_hwnd` / `view_id` must be called after the window has been realized in the event loop tick.

---

## 4. Platform Build Seams

- **`build.rs` Seams:**
  - `#[cfg(target_os = "macos")]`: Compiles `src/native/macos/*.m` files using `cc::Build` (`-fobjc-arc`, `-fblocks`) and links frameworks (`AppKit`, `ApplicationServices`, `ScreenCaptureKit`, `CoreGraphics`, `Vision`).
  - `#[cfg(windows)]`: Embeds `assets/icon.ico` using `winresource`.
- **`Cargo.toml` Conditioning:**
  - `[target.'cfg(windows)'.dependencies]`: Includes `windows` crate features and `clipboard-win`.
  - `[target.'cfg(target_os = "macos")'.build-dependencies]`: Includes `cc`.

---

## 5. Key Invariants

1. **No Linux Support:** Linux is not a supported target. Non-Windows and non-macOS platforms fall into default `posix_stubs` in `src/win32.rs`. Do not add Linux-specific native code.
2. **Effective Stealth Verification (I1):** Windows stealth (`set_stealth`) requires readback confirmation via `GetWindowDisplayAffinity`. `presentable_stealth` returns `true` only when the exclusion is confirmed by OS readback.
3. **Taskbar Baseline Standard (I2):** `skip_taskbar_exstyle` forces `WS_EX_TOOLWINDOW` on and `WS_EX_APPWINDOW` off in both directions, preventing winit re-shows from corrupting taskbar state.
4. **GDI Capture Window Hiding:** Windows GDI `BitBlt` ignores `WDA_EXCLUDEFROMCAPTURE`. Before capturing screen rectangles, all application overlay windows must be hidden via `hide_own_windows()` and flushed with `DwmFlush()`, then restored with `show_windows()`.
5. **Main Thread Maintained for AppKit:** macOS AppKit status item creation and window configuration must run on the main event loop thread.
6. **Off-Thread Processing for Heavy Operations:** ScreenCaptureKit screenshots and Apple Vision OCR requests execute off the UI thread via `tokio::task::spawn_blocking`.
7. **Single-Instance Mutex Ownership:** Singleton mutex/lock acquisition is isolated strictly to the platform lifecycle adapters (`src/native/windows/lifecycle.rs` and `src/native/macos/lifecycle.rs`).

---

## 6. Verification & Test Routing

Native platform adapters are verified through dedicated guard integration tests in `slint-experiment/tests/`:

- **Process Lifecycle Guard:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test native_lifecycle_guard
  ```
  *Verifies:* Win32 `CreateMutexW` single-instance isolation and macOS file-lock implementation.

- **Screen Capture & OCR Guard:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test native_screen_guard
  ```
  *Verifies:* Windows GDI `GetDIBits` isolation, ScreenCaptureKit FFI integration, permission handling, and local OCR routing.

- **macOS Window Guard:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test native_macos_window_guard
  ```
  *Verifies:* AppKit floating window bridge calls, level configuration, and coordinate space transformations.

- **macOS Status Item Guard:**
  ```powershell
  cargo test --manifest-path slint-experiment/Cargo.toml --test native_macos_status_guard
  ```
  *Verifies:* AppKit status item creation, main thread ownership, and drop-triggered removal.

- **Format & Syntax Check for AGENTS.md:**
  ```bash
  git diff --check -- slint-experiment/src/native/AGENTS.md
  ```
