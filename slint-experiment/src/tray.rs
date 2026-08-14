//! Native Windows tray icon for the hidden-to-tray.
//!
//! The overlay bar can be hidden explicitly (bar chip or tray menu); recording,
//! hotkeys, TTS and session tasks keep running while it is hidden. The tray
//! icon is the only way back, with a right-click menu (Restore/Hide, Pause/Resume,
//! Stop, Quit) routed through the HOST's existing bar callbacks — this module
//! owns NO session logic and NO persisted state. Startup is ALWAYS visible:
//! hidden state exists only after an explicit action and dies with the process.
//!
//! Implementation: a hidden top-level message window on the Slint UI thread
//! (its wndproc is dispatched by the same message pump that runs the Slint
//! event loop) + `Shell_NotifyIconW`. Left click toggles Restore/Hide; right
//! click builds the menu fresh from a live state snapshot, so labels and
//! enabled/disabled state can never drift from the session.
//!
//! No third-party tray framework — only the already-declared Microsoft
//! `windows` crate (Win32_UI_Shell + Win32_UI_WindowsAndMessaging).

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    GetLastError, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_SETFOCUS, NIM_SETVERSION, NIN_SELECT, NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, LoadIconW, RegisterClassExW,
    RegisterWindowMessageW, HICON, IDI_APPLICATION, WM_APP, WM_CONTEXTMENU, WM_DESTROY,
    WM_RBUTTONUP, WNDCLASSEXW,
};

// ===== Pure core (unit-testable without any Win32) =====

/// WM_COMMAND ids for the tray context menu. Stable on purpose — they are the
/// routing contract between the menu builder and `TrayAction::from_menu_id`.
/// They must fit in 16 bits because Win32 delivers a menu id through the low
/// word of `WM_COMMAND.wParam`.
pub const IDM_SHOW_HIDE: u32 = 0x0201;
pub const IDM_PAUSE_RESUME: u32 = 0x0202;
pub const IDM_STOP: u32 = 0x0203;
pub const IDM_QUIT: u32 = 0x0204;

/// Actions the host can dispatch from the tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayAction {
    /// Open the themed menu at a physical screen coordinate.
    OpenMenu { x: i32, y: i32 },
    /// Restore the bar (when hidden) or hide it (when visible).
    ShowHide,
    /// Pause/Resume the RUNNING session (no-op unless one runs).
    PauseResume,
    /// Stop the RUNNING session (no-op unless one runs).
    Stop,
    /// Quit through the existing clean event-loop shutdown path.
    Quit,
}

impl TrayAction {
    #[must_use]
    pub fn from_menu_id(id: u32) -> Option<Self> {
        match id {
            IDM_SHOW_HIDE => Some(Self::ShowHide),
            IDM_PAUSE_RESUME => Some(Self::PauseResume),
            IDM_STOP => Some(Self::Stop),
            IDM_QUIT => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Live state the menu is built from (pulled fresh on every right click).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraySnapshot {
    pub bar_visible: bool,
    pub paused: bool,
    pub session_running: bool,
}

impl TraySnapshot {
    /// Startup is ALWAYS visible — hidden state is never persisted, so every
    /// launch (fresh, legacy config, migrated config, relaunch) starts shown.
    #[must_use]
    pub fn startup() -> Self {
        Self {
            bar_visible: true,
            paused: false,
            session_running: false,
        }
    }
}

/// One context-menu row: `label` is RU or EN per `is_ru`, `enabled`/`checked`
/// drive MF_GRAYED / MF_CHECKED.
pub struct TrayMenuEntry {
    pub id: u32,
    pub label: &'static str,
    pub enabled: bool,
    pub checked: bool,
}

/// Build the four tray menu rows for the given live state + language.
/// Order is fixed: Restore/Hide, Pause/Resume, Stop, Quit.
#[must_use]
pub fn menu_entries(snap: &TraySnapshot, ru: bool) -> Vec<TrayMenuEntry> {
    vec![
        TrayMenuEntry {
            id: IDM_SHOW_HIDE,
            label: if snap.bar_visible {
                if ru {
                    "Скрыть"
                } else {
                    "Hide"
                }
            } else if ru {
                "Восстановить"
            } else {
                "Restore"
            },
            enabled: true,
            checked: false,
        },
        TrayMenuEntry {
            id: IDM_PAUSE_RESUME,
            label: if snap.paused {
                if ru {
                    "Продолжить"
                } else {
                    "Resume"
                }
            } else if ru {
                "Пауза"
            } else {
                "Pause"
            },
            enabled: snap.session_running,
            checked: snap.paused,
        },
        TrayMenuEntry {
            id: IDM_STOP,
            label: if ru { "Стоп" } else { "Stop" },
            enabled: snap.session_running,
            checked: false,
        },
        TrayMenuEntry {
            id: IDM_QUIT,
            label: if ru { "Выход" } else { "Quit" },
            enabled: true,
            checked: false,
        },
    ]
}

// ===== Win32 implementation (UI thread only) =====

const TRAY_ICON_ID: u32 = 1;
const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;

/// Single-instance guard for the ICON: one tray icon per process. `install`
/// claims it; the `TrayHandle` drop releases it (and removes the icon).
static INSTALL_SLOT: AtomicBool = AtomicBool::new(false);

/// Explorer broadcasts this registered message after recreating the taskbar.
/// Re-adding the icon is essential if the bar happened to be hidden then.
static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);

/// The hidden message window outlives the notification icon. The icon itself
/// exists only while the bar is hidden, so restoring the bar removes it from
/// the notification area immediately.
static TRAY_HWND: AtomicIsize = AtomicIsize::new(0);
static TRAY_ICON_VISIBLE: AtomicBool = AtomicBool::new(false);

fn claim_install_slot(slot: &AtomicBool) -> Result<(), String> {
    slot.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map(|_| ())
        .map_err(|_| "a tray icon is already installed in this process".to_string())
}

thread_local! {
    /// UI-thread-only callbacks installed by `install`. The wndproc runs on the
    /// same thread (the Slint/winit message pump dispatches our hidden window).
    static TRAY_CTX: RefCell<Option<TrayCtx>> = const { RefCell::new(None) };
}

struct TrayCtx {
    dispatch: Box<dyn Fn(TrayAction)>,
    availability: Box<dyn Fn(bool)>,
}

/// Owns the tray icon for the process lifetime; drop removes the icon
/// (`Shell_NotifyIconW(NIM_DELETE)`) and destroys the hidden window — the
/// clean-shutdown path relies on this.
pub struct TrayHandle {
    hwnd: HWND,
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        let data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        // SAFETY: `data` describes the icon this handle added; NIM_DELETE only
        // removes it. DestroyWindow targets our own hidden message window.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            let _ = DestroyWindow(self.hwnd);
        }
        TRAY_ICON_VISIBLE.store(false, Ordering::SeqCst);
        TRAY_HWND.store(0, Ordering::SeqCst);
        INSTALL_SLOT.store(false, Ordering::SeqCst);
        TRAY_CTX.with(|c| *c.borrow_mut() = None);
    }
}

/// Install the tray message window on the CURRENT (UI) thread. The notification
/// icon is added only when the bar hides and removed again on restore.
///
/// Errors are non-fatal for the host — the app keeps running without a tray.
pub fn install(
    dispatch: impl Fn(TrayAction) + 'static,
    availability: impl Fn(bool) + 'static,
) -> Result<TrayHandle, String> {
    claim_install_slot(&INSTALL_SLOT)?;
    TRAY_CTX.with(|c| {
        *c.borrow_mut() = Some(TrayCtx {
            dispatch: Box::new(dispatch),
            availability: Box::new(availability),
        });
    });
    match install_win32() {
        Ok(handle) => {
            publish_availability(true);
            Ok(handle)
        }
        Err(e) => {
            publish_availability(false);
            TRAY_CTX.with(|c| *c.borrow_mut() = None);
            INSTALL_SLOT.store(false, Ordering::SeqCst);
            Err(e)
        }
    }
}

fn install_win32() -> Result<TrayHandle, String> {
    let class_name: PCWSTR = windows::core::w!("suflyor_tray_message_window");
    let window_title: PCWSTR = windows::core::w!("suflyor tray");
    // SAFETY: classic RegisterClassExW/CreateWindowExW/Shell_NotifyIconW
    // sequence for a notification icon; every handle is either process-owned
    // or checked, and the window is never shown (message-only role).
    unsafe {
        let module = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {e}"))?;
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(tray_wndproc),
            hInstance: module.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            let err = GetLastError();
            // A previous install in THIS process (dropped handle, class kept)
            // is fine — same wndproc; anything else is a real failure.
            if err != ERROR_CLASS_ALREADY_EXISTS {
                return Err(format!("RegisterClassExW failed: {err:?}"));
            }
        }
        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            window_title,
            Default::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )
        .map_err(|e| format!("CreateWindowExW: {e}"))?;
        let taskbar_created = RegisterWindowMessageW(windows::core::w!("TaskbarCreated"));
        if taskbar_created == 0 {
            let err = GetLastError();
            let _ = DestroyWindow(hwnd);
            return Err(format!("RegisterWindowMessageW failed: {err:?}"));
        }
        TASKBAR_CREATED_MESSAGE.store(taskbar_created, Ordering::Relaxed);

        TRAY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
        TRAY_ICON_VISIBLE.store(false, Ordering::SeqCst);
        Ok(TrayHandle { hwnd })
    }
}

/// Add the notification icon before hiding the bar. The bar must stay visible
/// when this fails, otherwise there would be no restore surface.
pub fn show_icon() -> Result<(), String> {
    if TRAY_ICON_VISIBLE.load(Ordering::SeqCst) {
        return Ok(());
    }
    let raw = TRAY_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return Err("tray message window is unavailable".to_string());
    }
    let hwnd = HWND(raw as *mut std::ffi::c_void);
    // SAFETY: the stored HWND belongs to the current process and remains alive
    // until TrayHandle::drop clears TRAY_HWND.
    let module = unsafe { GetModuleHandleW(None) }.map_err(|e| format!("GetModuleHandleW: {e}"))?;
    match unsafe { add_notify_icon(hwnd, module) } {
        Ok(()) => {
            TRAY_ICON_VISIBLE.store(true, Ordering::SeqCst);
            publish_availability(true);
            Ok(())
        }
        Err(e) => {
            publish_availability(false);
            Err(e)
        }
    }
}

/// Remove the notification icon once the bar is visible again. The hidden
/// message window stays alive so a later explicit hide can add the icon anew.
pub fn hide_icon() {
    if !TRAY_ICON_VISIBLE.swap(false, Ordering::SeqCst) {
        return;
    }
    let raw = TRAY_HWND.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(raw as *mut std::ffi::c_void),
        uID: TRAY_ICON_ID,
        ..Default::default()
    };
    // SAFETY: data identifies only this process' notification icon.
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

/// Add the notification icon and opt into the current accessible interaction
/// contract. `NIM_SETVERSION` is required after every `NIM_ADD`; version 4
/// enables keyboard selection/context-menu events as well as mouse events.
unsafe fn add_notify_icon(
    hwnd: HWND,
    module: windows::Win32::Foundation::HMODULE,
) -> Result<(), String> {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: TRAY_CALLBACK_MESSAGE,
        hIcon: unsafe { load_tray_icon(module) },
        ..Default::default()
    };
    fill_tip(&mut data.szTip, "suflyor");

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        let err = unsafe { GetLastError() };
        return Err(format!("Shell_NotifyIconW(NIM_ADD) failed: {err:?}"));
    }
    data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) }.as_bool() {
        let err = unsafe { GetLastError() };
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
        return Err(format!("Shell_NotifyIconW(NIM_SETVERSION) failed: {err:?}"));
    }
    Ok(())
}

/// The embedded app icon (winresource puts `assets/icon.ico` into the exe as
/// icon resource 1); generic application icon as a last resort so the tray
/// entry still exists if the exe carries no icon resource. `LoadIconW` returns
/// shared handles, so repeated Explorer recovery does not leak an `HICON`.
///
/// # Safety
/// `module` is the live process module; both loaders are read-only.
unsafe fn load_tray_icon(module: windows::Win32::Foundation::HMODULE) -> HICON {
    LoadIconW(
        Some(module.into()),
        PCWSTR(std::ptr::without_provenance::<u16>(1)),
    )
    .or_else(|_| LoadIconW(None, IDI_APPLICATION))
    .unwrap_or(HICON(std::ptr::null_mut()))
}

fn fill_tip(dst: &mut [u16; 128], text: &str) {
    let mut units = text.encode_utf16().take(dst.len() - 1);
    for slot in dst.iter_mut() {
        *slot = units.next().unwrap_or(0);
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_created = TASKBAR_CREATED_MESSAGE.load(Ordering::Relaxed);
    if taskbar_created != 0 && msg == taskbar_created {
        // Explorer restarted and discarded every notification icon. Re-add
        // ours only when the bar is hidden; visible mode deliberately has no
        // persistent tray entry.
        if TRAY_ICON_VISIBLE.load(Ordering::SeqCst) {
            match unsafe { GetModuleHandleW(None) }
                .map_err(|e| format!("GetModuleHandleW: {e}"))
                .and_then(|module| unsafe { add_notify_icon(hwnd, module) })
            {
                Ok(()) => {
                    publish_availability(true);
                    eprintln!("[overlay-host] tray icon restored after Explorer restart");
                }
                Err(e) => {
                    TRAY_ICON_VISIBLE.store(false, Ordering::SeqCst);
                    publish_availability(false);
                    eprintln!("[overlay-host] tray icon restore failed: {e}");
                }
            }
        } else {
            publish_availability(true);
        }
        return LRESULT(0);
    }
    if msg == TRAY_CALLBACK_MESSAGE {
        // With NOTIFYICON_VERSION_4, LOWORD(lparam) is the mouse/keyboard
        // event. Mouse right-click and keyboard context-menu requests are
        // separate Shell events; both use the current cursor position so an
        // undefined WM_CONTEXTMENU wparam can never anchor at (1, 0).
        match (lparam.0 as u32) & 0xFFFF {
            // v4 emits NIN_SELECT for mouse activation. Handling WM_LBUTTONUP
            // as well toggles twice on affected shells (restore then hide).
            NIN_SELECT | NIN_KEYSELECT => dispatch_from_ctx(TrayAction::ShowHide),
            WM_RBUTTONUP | WM_CONTEXTMENU => request_tray_menu(hwnd),
            _ => {}
        }
        return LRESULT(0);
    }
    if msg == WM_DESTROY {
        return LRESULT(0);
    }
    // SAFETY: default processing for everything else on our own window.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn dispatch_from_ctx(action: TrayAction) {
    TRAY_CTX.with(|c| {
        if let Ok(ctx) = c.try_borrow() {
            if let Some(ctx) = ctx.as_ref() {
                (ctx.dispatch)(action);
            }
        }
    });
}

fn publish_availability(available: bool) {
    TRAY_CTX.with(|c| {
        if let Ok(ctx) = c.try_borrow() {
            if let Some(ctx) = ctx.as_ref() {
                (ctx.availability)(available);
            }
        }
    });
}

fn request_tray_menu(hwnd: HWND) {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return;
    }
    dispatch_from_ctx(TrayAction::OpenMenu {
        x: point.x,
        y: point.y,
    });
    let focus_data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    };
    // Return keyboard focus bookkeeping to the notification icon after the
    // host has opened its own styled window.
    unsafe {
        let _ = Shell_NotifyIconW(NIM_SETFOCUS, &focus_data);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn startup_snapshot_is_visible_and_idle() {
        let snap = TraySnapshot::startup();
        assert!(snap.bar_visible, "startup must ALWAYS be visible");
        assert!(!snap.paused);
        assert!(!snap.session_running);
    }

    #[test]
    fn menu_ids_are_unique_and_round_trip() {
        let ids = [IDM_SHOW_HIDE, IDM_PAUSE_RESUME, IDM_STOP, IDM_QUIT];
        let distinct: std::collections::HashSet<u32> = ids.iter().copied().collect();
        assert_eq!(distinct.len(), ids.len());
        for id in ids {
            assert!(id <= u32::from(u16::MAX));
            assert_eq!(
                TrayAction::from_menu_id(id & u32::from(u16::MAX)),
                TrayAction::from_menu_id(id)
            );
        }
        assert_eq!(
            TrayAction::from_menu_id(IDM_SHOW_HIDE),
            Some(TrayAction::ShowHide)
        );
        assert_eq!(
            TrayAction::from_menu_id(IDM_PAUSE_RESUME),
            Some(TrayAction::PauseResume)
        );
        assert_eq!(TrayAction::from_menu_id(IDM_STOP), Some(TrayAction::Stop));
        assert_eq!(TrayAction::from_menu_id(IDM_QUIT), Some(TrayAction::Quit));
        assert_eq!(TrayAction::from_menu_id(0), None);
    }

    #[test]
    fn menu_routing_reflects_state() {
        // Idle (bar visible, no session): Hide offered, session items disabled.
        let idle = menu_entries(&TraySnapshot::startup(), false);
        assert_eq!(idle.len(), 4);
        assert_eq!(idle[0].label, "Hide");
        assert!(idle[0].enabled);
        assert_eq!(idle[1].label, "Pause");
        assert!(!idle[1].enabled);
        assert!(!idle[1].checked);
        assert!(!idle[2].enabled, "Stop needs a running session");
        assert!(idle[3].enabled, "Quit is always available");

        // Hidden + running + paused: Restore, Resume checked, Stop enabled.
        let snap = TraySnapshot {
            bar_visible: false,
            paused: true,
            session_running: true,
        };
        let running = menu_entries(&snap, false);
        assert_eq!(running[0].label, "Restore");
        assert_eq!(running[1].label, "Resume");
        assert!(running[1].enabled);
        assert!(running[1].checked);
        assert!(running[2].enabled);

        // Running, not paused: Pause enabled + unchecked.
        let snap = TraySnapshot {
            bar_visible: true,
            paused: false,
            session_running: true,
        };
        let running = menu_entries(&snap, false);
        assert_eq!(running[1].label, "Pause");
        assert!(running[1].enabled);
        assert!(!running[1].checked);
    }

    #[test]
    fn menu_labels_follow_language() {
        let snap = TraySnapshot {
            bar_visible: false,
            paused: true,
            session_running: true,
        };
        let ru = menu_entries(&snap, true);
        assert_eq!(
            [ru[0].label, ru[1].label, ru[2].label, ru[3].label],
            ["Восстановить", "Продолжить", "Стоп", "Выход"]
        );
        let en = menu_entries(&TraySnapshot::startup(), false);
        assert_eq!(
            [en[0].label, en[1].label, en[2].label, en[3].label],
            ["Hide", "Pause", "Stop", "Quit"]
        );
        // Every label is non-empty in BOTH languages.
        for ru in menu_entries(&snap, true) {
            assert!(!ru.label.is_empty());
        }
        for en in menu_entries(&snap, false) {
            assert!(!en.label.is_empty());
        }
    }

    #[test]
    fn install_slot_rejects_duplicates_and_releases() {
        let slot = AtomicBool::new(false);
        assert!(claim_install_slot(&slot).is_ok());
        let dup = claim_install_slot(&slot);
        assert!(
            dup.is_err(),
            "second install must be refused (no duplicate icons)"
        );
        slot.store(false, Ordering::SeqCst);
        assert!(
            claim_install_slot(&slot).is_ok(),
            "released slot is reusable"
        );
    }
}
