//! Window lifecycle + stealth/theme registry (Phase 1 of the
//! `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.1).
//!
//! This module owns the process-global stealth / colour-scheme / tile-opacity
//! state, the stealth-aware presentation helper, the per-window `Theme.scheme`
//! appliers, and a single [`WindowRegistry`] so stealth + theme are applied to
//! ALL open windows through ONE path instead of three hand-maintained loops
//! (the bug class where a new window — Help, the recover-offer — was forgotten
//! in one of the loops and leaked into a screen-share).
//!
//! The persistent, pre-stealthed capture overlay is deliberately NOT part of
//! the registry: it is realized once and WDA-excluded from its first frame, so
//! it must not be re-driven on the same rules as the on-demand windows.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    enum_monitors, get_mic_active, get_sys_active, get_window_rect, grab_hwnd,
    move_window_pos_only, pick_monitor, refresh_status, set_stealth, stealth_supported, ui,
    ArchiveWindow, ComponentHandle, Duration, HelpWindow, LockModeMenuWindow, OverlayBarWindow,
    PaletteWindow, Rc,
    RecoverOfferWindow, RefCell, SettingsWindow, TextAskWindow, TileWindow, TileWindows, Timer,
    TranscriptWindow, WizardWindow, HWND_GRAB_DELAY_MS, HWND_REVEAL_FAST_MS,
};

/// Phase E6 v36 — process-global tile body opacity (raw f32 bits in an
/// AtomicU32 so it stays lock-free). EVERY tile-spawn path reads this via
/// `apply_tile_hwnd_with_monitor` so a tile spawned before Settings is ever
/// opened still honours the saved transparency. Seeded from config at startup,
/// updated live by the Settings slider.
static TILE_BODY_OPACITY_BITS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x3F80_0000); // 1.0_f32

/// Place a shared Slint window using the coordinate units returned by the
/// platform geometry adapter: physical pixels on Windows, logical points on macOS.
pub(crate) fn set_platform_window_position(window: &slint::Window, x: i32, y: i32) {
    #[cfg(windows)]
    window.set_position(slint::PhysicalPosition::new(x, y));
    #[cfg(target_os = "macos")]
    window.set_position(slint::LogicalPosition::new(x as f32, y as f32));
}

/// Store the current global tile body opacity (clamped 0.5..=1.0).
pub(crate) fn set_global_tile_opacity(value: f32) {
    let clamped = value.clamp(0.5, 1.0);
    TILE_BODY_OPACITY_BITS.store(clamped.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// Read the current global tile body opacity (defaults to 1.0).
pub(crate) fn global_tile_opacity() -> f32 {
    f32::from_bits(TILE_BODY_OPACITY_BITS.load(std::sync::atomic::Ordering::Relaxed))
}

/// #111 — process-global stealth (WDA_EXCLUDEFROMCAPTURE) state.
///
/// The stealth toggle only ever flipped the bar + already-open tiles, so any
/// window created WHILE stealth was on (the F4 KB palette, the Settings
/// window, freshly-spawned tiles) never received the capture-exclusion flag
/// and leaked the overlay into screen-share / recording. Mirror of
/// `global_tile_opacity`: one lock-free flag every window-realize path
/// consults so new windows inherit stealth. Flipped by both stealth toggles.
static STEALTH_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Store the current global stealth state.
pub(crate) fn set_global_stealth(on: bool) {
    STEALTH_ON.store(
        on && stealth_supported(),
        std::sync::atomic::Ordering::Relaxed,
    );
}

/// Read the current global stealth state (defaults to off).
pub(crate) fn global_stealth() -> bool {
    STEALTH_ON.load(std::sync::atomic::Ordering::Relaxed)
}

/// I1 — process-global EFFECTIVE stealth: the bar's last VERIFIED
/// capture-exclusion state (apply + `GetWindowDisplayAffinity` readback).
/// Distinct from `STEALTH_ON` (the config INTENT, which is preserved so the
/// next apply retries): this flag is the honest source for every visible
/// success indicator — the bar chip, the Settings Stealth-tab status line,
/// and the Diagnostics row. Starts false: until the bar's HWND is realized
/// and the exclusion is read back, stealth cannot be claimed.
/// LIMITATION (kept explicit so the indicators never overclaim): the global
/// verifies the BAR's WDA only. Per-window (tile / registry / capture-overlay)
/// exclusion failures are logged (`apply_stealth_one`, the F8 path) but NOT
/// aggregated here — a tile whose exclusion failed stays capturable while this
/// flag can still read true; the log is the diagnostic channel for those.
static STEALTH_EFFECTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Store the verified effective stealth state (bar apply + readback outcome).
pub(crate) fn set_global_stealth_effective(on: bool) {
    STEALTH_EFFECTIVE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Read the verified effective stealth state (defaults to false).
pub(crate) fn global_stealth_effective() -> bool {
    STEALTH_EFFECTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

/// I1 — surface a stealth apply/readback failure on the bar's status pill:
/// raise the `stealth-fault` flag so the pill's text binding swaps in the
/// TRANSLATED generic failure message (overlay_bar.slint `@tr`), and tint the
/// pill amber (colours are not translated, so Rust keeps owning them).
/// Deliberately generic + secret/path-free; the caller's log line carries the
/// Win32 detail. `apply_bar_stealth` clears the flag once a later apply
/// verifies, restoring the mic/sys truth.
pub(crate) fn surface_stealth_unavailable(bar: &OverlayBarWindow) {
    bar.set_stealth_fault(true);
    bar.set_status_color(slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24));
}

/// Apply a stealth toggle to the BAR — the one window outside the
/// `WindowRegistry` (it also carries the taskbar-button side effect). I1: the
/// WDA apply + readback decide the EFFECTIVE state; the chip, the status pill,
/// and the process-global effective flag all follow the readback, never the
/// intent alone. Config intent is preserved by the caller regardless, so the
/// next toggle/realize retries. Returns the effective state so callers can
/// echo it (Settings status line). Shared by all three stealth-toggle paths
/// (bar / Settings / wizard) so their semantics cannot drift.
pub(crate) fn apply_bar_stealth(
    bar: &OverlayBarWindow,
    state: &slint_replay::app_state::SharedState,
    on: bool,
) -> bool {
    if !stealth_supported() {
        set_global_stealth_effective(false);
        bar.set_stealth_active(false);
        if bar.get_stealth_fault() {
            bar.set_stealth_fault(false);
            refresh_status(bar, get_mic_active(state), get_sys_active(state));
        }
        return false;
    }

    let effective = match grab_hwnd(bar.window()) {
        Ok(hwnd) => {
            let applied = set_stealth(hwnd, on);
            let effective = slint_replay::win32::presentable_stealth(on, &applied);
            if let Err(e) = &applied {
                diag!(
                    "[overlay-host] bar stealth apply failed (effective=off, config intent \
                     preserved for retry): {e}"
                );
            }
            // I2 — the taskbar style follows the EFFECTIVE state; both
            // directions force the TOOLWINDOW baseline + clear APPWINDOW, so
            // the bar can never become a taskbar-eligible APPWINDOW.
            if let Err(e) = slint_replay::win32::set_skip_taskbar(hwnd, effective) {
                diag!("[overlay-host] bar skip-taskbar failed: {e}");
            }
            effective
        }
        Err(e) => {
            diag!("[overlay-host] bar stealth: HWND not realized (effective=off): {e}");
            false
        }
    };
    set_global_stealth_effective(effective);
    bar.set_stealth_active(effective);
    if on && !effective {
        surface_stealth_unavailable(bar);
    } else if bar.get_stealth_fault() {
        // A previous failure banner is stale — clear the flag and restore the
        // mic/sys truth on the pill.
        bar.set_stealth_fault(false);
        refresh_status(bar, get_mic_active(state), get_sys_active(state));
    }
    effective
}

/// Apply WDA to a single registry/realize window, logging (never swallowing)
/// a failure (I1): a window whose exclusion failed stays capturable, and the
/// user must be able to diagnose it from the log.
fn apply_stealth_one(hwnd: slint_replay::win32::HWND, on: bool) {
    if let Err(e) = set_stealth(hwnd, on) {
        diag!("[overlay-host] stealth apply failed (window stays capturable): {e}");
    }
}

/// Process-global tile-monitor PIN — the `(left, top)` of the display the user
/// chose for new tiles, packed `(left << 32) | (top as u32)` into an AtomicI64
/// (lock-free, mirror of `global_tile_opacity`). `TILE_MONITOR_AUTO` (the
/// sentinel) means "auto" → `apply_tile_hwnd_with_monitor` falls back to
/// `pick_monitor`. Seeded from `cfg.tile_monitor_name` at startup, updated live
/// by the Settings monitor dropdown. Matching by top-left (not an index)
/// survives a monitor reorder, and an unplugged pinned monitor simply isn't
/// found in `enum_monitors()` → auto fallback (never an off-screen tile).
const TILE_MONITOR_AUTO: i64 = i64::MIN;
static TILE_MONITOR_PIN: std::sync::atomic::AtomicI64 =
    std::sync::atomic::AtomicI64::new(TILE_MONITOR_AUTO);

/// Pin new tiles to the monitor whose top-left is `(left, top)`, or `None` for
/// auto (let `pick_monitor` decide).
pub(crate) fn set_global_tile_monitor(pin: Option<(i32, i32)>) {
    let packed = match pin {
        Some((left, top)) => (i64::from(left) << 32) | i64::from(top as u32),
        None => TILE_MONITOR_AUTO,
    };
    TILE_MONITOR_PIN.store(packed, std::sync::atomic::Ordering::Relaxed);
}

/// Read the tile-monitor pin as `(left, top)`, or `None` when auto.
pub(crate) fn global_tile_monitor() -> Option<(i32, i32)> {
    let packed = TILE_MONITOR_PIN.load(std::sync::atomic::Ordering::Relaxed);
    if packed == TILE_MONITOR_AUTO {
        None
    } else {
        Some(((packed >> 32) as i32, (packed & 0xFFFF_FFFF) as u32 as i32))
    }
}

/// Parse a `cfg.tile_monitor_name` pin string (`"{left},{top}"`) into coords;
/// empty / malformed → `None` (auto). Shared by the startup seed and the
/// Settings dropdown handler so the encode/decode lives in one place.
pub(crate) fn parse_tile_monitor_pin(s: &str) -> Option<(i32, i32)> {
    let (l, t) = s.split_once(',')?;
    Some((l.trim().parse().ok()?, t.trim().parse().ok()?))
}

/// Process-global colour scheme (0=Glacier..3=Light Frost), mirror of
/// `global_stealth`: tiles are spawned from 5 scattered sites and are
/// ephemeral, so rather than thread the value through every call site we
/// keep one lock-free copy that each tile-realize path consults. The
/// Settings scheme handler updates it (so future tiles inherit the choice)
/// AND walks the live tile list to re-skin existing ones. Seeded from
/// config at startup.
static COLOR_SCHEME: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Store the current global colour scheme (clamped 0..=3).
pub(crate) fn set_global_scheme(scheme: i32) {
    COLOR_SCHEME.store(clamp_scheme(scheme), std::sync::atomic::Ordering::Relaxed);
}

/// Read the current global colour scheme (defaults to 0=Glacier).
pub(crate) fn global_scheme() -> i32 {
    COLOR_SCHEME.load(std::sync::atomic::Ordering::Relaxed)
}

/// Clamp a persisted `color_scheme` to the 4 schemes `theme.slint` defines
/// (0=Glacier, 1=Graphite, 2=Obsidian, 3=Light Frost). A corrupt/out-of-range
/// value falls back to Glacier rather than rendering an all-default (black)
/// theme.
pub(crate) fn clamp_scheme(n: i32) -> i32 {
    if (0..=3).contains(&n) {
        n
    } else {
        0
    }
}

/// Stealth-aware presentation for the auxiliary windows (F4 palette, Settings)
/// that otherwise rely on winit's default centering. Mirrors
/// `present_tile_window` + `apply_tile_hwnd_with_monitor` (review M1): park the
/// window OFF the virtual desktop BEFORE its first frame so it's never composited
/// onto a real monitor, then — once winit realizes the HWND — run `decorate`
/// (e.g. Settings' DWM transparency for rounded corners), apply WDA when stealth
/// is on, and move it to the centre of the target monitor. The first ON-SCREEN
/// frame is therefore already fully painted + decorated (+ stealth-excluded).
/// NOTE: parking is now UNCONDITIONAL (was stealth-only). A non-stealth window
/// used to be shown immediately and only decorated ~1-2 frames later, which the
/// user saw as a bare outline / black rounded corners flashing before the content
/// composited. Parking always closes that gap for stealth-off windows too.
pub(crate) fn present_window_stealth_aware<W, F>(win: &W, decorate: F)
where
    W: slint::ComponentHandle + 'static,
    F: Fn(slint_replay::win32::HWND) + 'static,
{
    present_window_stealth_aware_at(win, None, decorate);
}

/// ТЗ 2026-07-06 (C) — true if a saved top-left position still lands on a
/// visible monitor (with a small slack so a title bar dragged flush to an edge
/// still counts). A stale position from an unplugged monitor fails → the
/// caller centers instead. Pure — unit-tested below.
fn pos_on_visible_monitor(pos: (i32, i32), monitors: &[slint_replay::win32::MonitorRect]) -> bool {
    const SLACK: i32 = 8;
    monitors.iter().any(|m| {
        pos.0 >= m.left - SLACK
            && pos.0 < m.right - SLACK
            && pos.1 >= m.top - SLACK
            && pos.1 < m.bottom - SLACK
    })
}

/// `present_window_stealth_aware` with an optional RESTORED position: `Some` +
/// still-visible → reveal there instead of centering (both the native reveal
/// and the degraded Slint fallback honor it). Everything else is identical —
/// callers without a saved position use the plain wrapper above.
pub(crate) fn present_window_stealth_aware_at<W, F>(
    win: &W,
    saved_pos: Option<(i32, i32)>,
    decorate: F,
) where
    W: slint::ComponentHandle + 'static,
    F: Fn(slint_replay::win32::HWND) + 'static,
{
    // G1 — layout-independent Ctrl+C/V/X/A/Z/Y for every editable field on this window
    // (winit key filter; idempotent). Covers Settings / palette / text_ask / wizard /
    // help / archive / transcript — all the aux windows funnel through here.
    crate::kbd_shortcuts::install(win.window());
    // Park off-screen BEFORE the first frame (always — see fn doc). The reveal
    // tick decorates + (under stealth) WDAs, then moves it on-screen, so the
    // first visible frame is complete. Unconditional so a stealth toggle
    // mid-realize can't strand the window off the desktop either.
    set_platform_window_position(win.window(), -32000, -32000);
    let _ = win.show();
    // V0.8.4 — reveal as soon as the HWND realizes (~1-2 frames) instead of a
    // fixed 200ms blind wait, so on-demand windows (Settings/help/palette/wizard/
    // tiles) pop nearly instantly. A fast attempt covers the common case; if the
    // HWND isn't grabbable yet, a bounded set of conservative retries keeps a slow
    // first-realize safe (no window stranded off-screen). Stealth-safe: in EVERY
    // native path WDA is applied BEFORE a parked window is moved on-screen.
    let do_reveal: Rc<dyn Fn(&W) -> bool> = Rc::new(move |w: &W| -> bool {
        let Ok(hwnd) = grab_hwnd(w.window()) else {
            return false;
        };
        #[cfg(target_os = "macos")]
        if let Err(error) = slint_replay::native::window::configure_floating(w.window()) {
            diag!("[overlay-host] macOS floating-window configuration failed: {error}");
        }
        decorate(hwnd);
        if global_stealth() {
            // I1 — a failed exclusion is logged, never silently swallowed.
            apply_stealth_one(hwnd, true);
        }
        // The off-screen frame is now painted + decorated (+ WDA under stealth):
        // reveal it at the RESTORED position when one is saved and still visible
        // (ТЗ 2026-07-06 C), else centered on the picked monitor using the real
        // HiDPI-aware size, so the first ON-SCREEN frame is already complete.
        let (_x, _y, w_px, h_px) = get_window_rect(hwnd).unwrap_or((0, 0, 460, 360));
        let monitors = enum_monitors();
        if let Some((sx, sy)) = saved_pos.filter(|p| pos_on_visible_monitor(*p, &monitors)) {
            let _ = move_window_pos_only(hwnd, sx, sy);
            set_platform_window_position(w.window(), sx, sy);
        } else if let Some(mon) = pick_monitor(&monitors) {
            let cx = (mon.left + (mon.width() - w_px) / 2).max(mon.left + 8);
            let cy = (mon.top + (mon.height() - h_px) / 2).max(mon.top + 8);
            let _ = move_window_pos_only(hwnd, cx, cy);
            set_platform_window_position(w.window(), cx, cy);
        } else {
            let _ = move_window_pos_only(hwnd, 100, 100);
            set_platform_window_position(w.window(), 100, 100);
        }
        #[cfg(target_os = "macos")]
        if let Err(error) = slint_replay::native::window::raise_key_front(w.window()) {
            diag!("[overlay-host] macOS window raise failed: {error}");
        }
        true
    });
    // Last-ditch reveal when the HWND NEVER becomes grabbable after every retry —
    // otherwise the window is stranded invisibly parked at (-32000,-32000) with no
    // trace, and the user opens Settings/help and "nothing happens". Warn-log it so
    // it surfaces in the boot-smoke log + diagnostics. STEALTH-SAFE: only bring the
    // window on-screen (via Slint's own positioning — no native HWND needed) when
    // stealth is OFF; revealing without WDA under stealth would leak it into a
    // screen-share, so under stealth it stays parked + warn-logged and the user can
    // reopen to retry. Native decoration (rounded corners) is skipped in this
    // degraded path — a plain but VISIBLE window beats an invisible one.
    let fallback_reveal: Rc<dyn Fn(&W)> = Rc::new(move |w: &W| {
        if global_stealth() {
            diag!(
                "[overlay-host] present: HWND never realized after retries; window kept \
                 parked off-screen under stealth (reopen to retry)"
            );
            return;
        }
        diag!(
            "[overlay-host] present: HWND never realized after retries; revealing via Slint \
             fallback (no native decorate/center)"
        );
        let monitors = enum_monitors();
        if let Some((sx, sy)) = saved_pos.filter(|p| pos_on_visible_monitor(*p, &monitors)) {
            set_platform_window_position(w.window(), sx, sy);
        } else if let Some(mon) = pick_monitor(&monitors) {
            let cx = (mon.left + (mon.width() - 460) / 2).max(mon.left + 8);
            let cy = (mon.top + (mon.height() - 360) / 2).max(mon.top + 8);
            set_platform_window_position(w.window(), cx, cy);
        } else {
            set_platform_window_position(w.window(), 100, 100);
        }
    });
    realize_with_retries(win, do_reveal, fallback_reveal);
}

/// The ONE realize-then-reveal retry schedule (I3): a fast attempt once the
/// HWND is usually realized (~1-2 frames), two conservative retries, then the
/// caller's fallback so no window is stranded off-screen after a `grab_hwnd`
/// miss. `attempt` returns true once it has revealed the window. Shared by
/// `present_window_stealth_aware_at` (aux windows) AND the bar realization in
/// `overlay_host.rs` — a slow first-realize (heavy paint / busy compositor)
/// no longer gives up after a single miss, and there is exactly one retry
/// implementation to maintain.
pub(crate) fn realize_with_retries<W>(
    win: &W,
    attempt: Rc<dyn Fn(&W) -> bool>,
    fallback: Rc<dyn Fn(&W)>,
) where
    W: slint::ComponentHandle + 'static,
{
    let weak = win.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_REVEAL_FAST_MS), move || {
        let Some(w) = weak.upgrade() else { return };
        if attempt(&w) {
            return;
        }
        // Retry #1 SOON (80ms, not the full 200ms) — the heavy Settings window
        // often isn't HWND-realized by the 33ms fast attempt, so the old 200ms
        // gap was the "Settings opens with a delay" the user saw. Still only
        // reveals once grab_hwnd succeeds (window painted off-screen → no flash).
        let weak2 = w.as_weak();
        let attempt2 = attempt.clone();
        let fallback2 = fallback.clone();
        Timer::single_shot(Duration::from_millis(80), move || {
            let Some(w) = weak2.upgrade() else { return };
            if attempt2(&w) {
                return;
            }
            // Retry #2 (final) at a longer delay; on a final miss, run the fallback.
            let weak3 = w.as_weak();
            let attempt3 = attempt2.clone();
            let fallback3 = fallback2.clone();
            Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS * 2), move || {
                let Some(w) = weak3.upgrade() else { return };
                if !attempt3(&w) {
                    fallback3(&w);
                }
            });
        });
    });
}

/// #B1 — push the LIVE open-tile count to the bar's `open-tiles` property so
/// the "+ tile (N)" label and the "close all" chip reflect reality. Call this
/// after EVERY `tiles.push(...)` and EVERY close-handler `tiles.retain(...)`
/// (and in the close-all handler). Distinct from `tiles_spawned`, which is a
/// monotonic display counter for the per-tile #N badge and must not change.
pub(crate) fn refresh_open_tiles(weak: &slint::Weak<OverlayBarWindow>, tiles: &TileWindows) {
    let n = tiles.borrow().len();
    if let Some(o) = weak.upgrade() {
        o.set_open_tiles(n as i32);
    }
    // When the screen is cleared, reset the cascade-placement counter so the
    // NEXT tile starts from the top-right cluster again instead of marching
    // further left on every close-all -> respawn cycle (stress-test bug).
    if n == 0 {
        super::tile_window::TILE_SLOT_COUNTER.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

// `Theme` is a Slint GLOBAL, but globals are scoped to each window-component
// INSTANCE — every window (bar, settings, each tile, palette) owns its own
// copy. So switching the scheme means setting it on EVERY live window, and
// every freshly-created window must be seeded at construction. These tiny
// per-type helpers centralise the `global::<Theme>().set_scheme(..)` call so
// the clamp + access pattern lives in one place.
pub(crate) fn apply_scheme_bar(w: &OverlayBarWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_lock_menu(w: &LockModeMenuWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_tile(w: &TileWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_settings(w: &SettingsWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_palette(w: &PaletteWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_text_ask(w: &TextAskWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_wizard(w: &WizardWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_help(w: &HelpWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_recover_offer(w: &RecoverOfferWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_transcript(w: &TranscriptWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}
pub(crate) fn apply_scheme_archive(w: &ArchiveWindow, scheme: i32) {
    w.global::<ui::Theme>().set_scheme(clamp_scheme(scheme));
}

/// Single owner of the on-demand overlay windows whose stealth + theme must
/// stay in lock-step (§5.1). Every field is an `Rc<RefCell<…>>` clone of the
/// slot created in `main`, so the whole struct is cheap to `clone()` into each
/// handler closure. The bar (`OverlayBarWindow`) is intentionally NOT a field:
/// it is the composition root with bespoke stealth side effects (the taskbar
/// button + the `stealth-active` chip); all three stealth-toggle handlers
/// drive it through the ONE shared `apply_bar_stealth` (I1: chip/pill follow
/// the verified WDA readback; I2: the taskbar baseline can never drift to
/// APPWINDOW). The persistent capture overlay is likewise excluded (§5.1) —
/// it re-applies + verifies WDA on every show (I4, vision_capture.rs).
#[derive(Clone)]
pub(crate) struct WindowRegistry {
    pub tiles: TileWindows,
    pub settings: Rc<RefCell<Option<SettingsWindow>>>,
    pub palette: Rc<RefCell<Option<PaletteWindow>>>,
    pub text_ask: Rc<RefCell<Option<TextAskWindow>>>,
    pub wizard: Rc<RefCell<Option<WizardWindow>>>,
    pub help: Rc<RefCell<Option<HelpWindow>>>,
    pub recover_offer: Rc<RefCell<Option<RecoverOfferWindow>>>,
    /// ТЗ1 — the read-only transcript viewer (opened from the archive). The most
    /// sensitive surface (every utterance verbatim), so it MUST re-stealth on a
    /// live toggle like every other on-demand window.
    pub transcript: Rc<RefCell<Option<TranscriptWindow>>>,
    /// 🗄 Session-archive browser (F7 / 🗄 chip). Shows session titles + FTS
    /// search snippets that include transcript text, so — like the transcript
    /// viewer — it MUST re-stealth on an OFF→ON toggle; otherwise an archive
    /// opened while stealth was off stays captured after stealth is turned on.
    pub archive: Rc<RefCell<Option<ArchiveWindow>>>,
    /// Transient lock-mode menu. Although short-lived, it can reveal the local
    /// AI state, so a live stealth/theme switch must reach it too.
    pub lock_menu: Rc<RefCell<Option<Rc<LockModeMenuWindow>>>>,
}

impl WindowRegistry {
    /// Apply the WDA_EXCLUDEFROMCAPTURE flag to EVERY open registry window
    /// (tiles + Settings + palette + text-ask + wizard + Help + recover-offer +
    /// transcript + archive) in one call. This replaces the three near-identical
    /// hand-written loops
    /// in the bar / wizard / Settings stealth handlers; the per-window blocks
    /// below mirror those loops exactly (same `grab_hwnd` + `set_stealth`
    /// pattern, same UI-property echoes), so the only behavioural change is
    /// that Help + the recover-offer can never again be forgotten in one loop.
    /// The caller drives the bar itself through `apply_bar_stealth` BEFORE
    /// this walk, so the Settings status line seeded below already reflects
    /// the verified outcome (I1). The capture overlay is excluded — it
    /// re-applies + verifies WDA on every show (I4, vision_capture.rs).
    pub(crate) fn apply_stealth(&self, on: bool) {
        let on = on && stealth_supported();
        // All tiles.
        for t in self.tiles.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // Settings — also reflect the new state in its in-window Switch + the
        // EFFECTIVE-state status line (I1). The caller drove the bar through
        // `apply_bar_stealth` BEFORE this registry walk, so the effective
        // global below is already the outcome of the verified apply.
        if let Some(sw) = self.settings.borrow().as_ref() {
            sw.set_stealth_toggle(on);
            // I1 — the status line follows the verified EFFECTIVE state (the
            // Slint `@tr` ternary combines it with the toggle intent above).
            let effective = global_stealth_effective();
            sw.set_stealth_effective(effective);
            // The Diagnostics stealth row is otherwise seeded only when
            // Settings opens (populate_diagnostics); echo the same effective
            // state so a toggle made while Settings is already open is
            // reflected immediately, not after a close/reopen.
            sw.set_diag_stealth_on(effective);
            if let Ok(hwnd) = grab_hwnd(sw.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // F4 KB palette.
        if let Some(p) = self.palette.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(p.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // "✏ Написать" text-input window.
        if let Some(t) = self.text_ask.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // First-run wizard — also reflect the new state in its in-window Switch.
        if let Some(wz) = self.wizard.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(wz.window()) {
                apply_stealth_one(hwnd, on);
            }
            wz.set_stealth_on(on);
        }
        // 🆘 Help window (FIX #6 — previously dropped from some loops).
        if let Some(h) = self.help.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(h.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // Crash-recovery-offer window (FIX #6 — previously dropped from some loops).
        if let Some(ro) = self.recover_offer.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(ro.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // ТЗ1 transcript viewer — verbatim meeting transcript, the most sensitive
        // surface; must never stay captured after an OFF→ON toggle.
        if let Some(t) = self.transcript.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        // 🗄 Session archive — session titles + FTS snippets carry transcript
        // text. Setting the capture-exclusion flag is content-agnostic, so a
        // re-transcribe / Summary job in flight here is unaffected (only the
        // HWND affinity changes, not the window's data).
        if let Some(a) = self.archive.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(a.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
        if let Some(menu) = self.lock_menu.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(menu.window()) {
                apply_stealth_one(hwnd, on);
            }
        }
    }

    /// Re-skin EVERY open registry window to `scheme` (Theme is a per-window
    /// global, so each live window must be set individually). The caller drives
    /// the bar itself inline via `apply_scheme_bar`. Future windows still
    /// inherit the choice through `global_scheme()` at construction.
    pub(crate) fn apply_scheme(&self, scheme: i32) {
        for tile in self.tiles.borrow().iter() {
            apply_scheme_tile(tile, scheme);
        }
        if let Some(sw) = self.settings.borrow().as_ref() {
            apply_scheme_settings(sw, scheme);
        }
        if let Some(p) = self.palette.borrow().as_ref() {
            apply_scheme_palette(p, scheme);
        }
        if let Some(t) = self.text_ask.borrow().as_ref() {
            apply_scheme_text_ask(t, scheme);
        }
        if let Some(wz) = self.wizard.borrow().as_ref() {
            apply_scheme_wizard(wz, scheme);
        }
        if let Some(h) = self.help.borrow().as_ref() {
            apply_scheme_help(h, scheme);
        }
        if let Some(ro) = self.recover_offer.borrow().as_ref() {
            apply_scheme_recover_offer(ro, scheme);
        }
        if let Some(t) = self.transcript.borrow().as_ref() {
            apply_scheme_transcript(t, scheme);
        }
        if let Some(a) = self.archive.borrow().as_ref() {
            apply_scheme_archive(a, scheme);
        }
        if let Some(menu) = self.lock_menu.borrow().as_ref() {
            apply_scheme_lock_menu(menu, scheme);
        }
    }

    /// Push the live open-tile count to the bar's `open-tiles` property (the
    /// `+ tile (N)` label + the "close all" chip). Registry-scoped wrapper over
    /// `refresh_open_tiles` for callers that already hold a concrete bar handle.
    pub(crate) fn refresh_tiles_chip(&self, overlay: &OverlayBarWindow) {
        overlay.set_open_tiles(self.tiles.borrow().len() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::pos_on_visible_monitor;
    use slint_replay::win32::MonitorRect;

    /// The owner's real dual-monitor layout: landscape primary 1920×1080 at
    /// (0,0) + PORTRAIT secondary 1200×1920 at negative x.
    fn owner_monitors() -> Vec<MonitorRect> {
        vec![
            MonitorRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
                is_primary: true,
            },
            MonitorRect {
                left: -1200,
                top: 0,
                right: 0,
                bottom: 1920,
                is_primary: false,
            },
        ]
    }

    #[test]
    fn saved_pos_validation_covers_owner_layout() {
        let mons = owner_monitors();
        assert!(pos_on_visible_monitor((300, 200), &mons)); // on primary
        assert!(pos_on_visible_monitor((-800, 1500), &mons)); // portrait at negative x
        assert!(pos_on_visible_monitor((-4, 0), &mons)); // edge slack (8px)
        assert!(!pos_on_visible_monitor((2500, 200), &mons)); // right of everything
        assert!(!pos_on_visible_monitor((300, 1300), &mons)); // below primary, x not on portrait
                                                              // Monitor unplugged (stale saved pos) → nothing visible → fallback to center.
        assert!(!pos_on_visible_monitor(
            (-800, 1500),
            &owner_monitors()[..1]
        ));
        assert!(!pos_on_visible_monitor((100, 100), &[])); // no monitors at all
    }
}
