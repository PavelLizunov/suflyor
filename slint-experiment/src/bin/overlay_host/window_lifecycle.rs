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
    enum_monitors, get_window_rect, grab_hwnd, move_window_pos_only, pick_monitor, set_stealth, ui,
    ComponentHandle, Duration, HelpWindow, OverlayBarWindow, PaletteWindow, Rc, RecoverOfferWindow,
    RefCell, SettingsWindow, TextAskWindow, TileWindow, TileWindows, Timer, WizardWindow,
    HWND_GRAB_DELAY_MS, HWND_REVEAL_FAST_MS,
};

/// Phase E6 v36 — process-global tile body opacity (raw f32 bits in an
/// AtomicU32 so it stays lock-free). EVERY tile-spawn path reads this via
/// `apply_tile_hwnd_with_monitor` so a tile spawned before Settings is ever
/// opened still honours the saved transparency. Seeded from config at startup,
/// updated live by the Settings slider.
static TILE_BODY_OPACITY_BITS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0x3F80_0000); // 1.0_f32

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
    STEALTH_ON.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Read the current global stealth state (defaults to off).
pub(crate) fn global_stealth() -> bool {
    STEALTH_ON.load(std::sync::atomic::Ordering::Relaxed)
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

/// Apply the current global stealth flag to a freshly-shown window once
/// winit realizes its native HWND (same 200 ms delay as tile placement).
/// No-op when stealth is off. Used by windows that don't otherwise grab
/// their HWND post-show (the F4 palette). (#111)
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
    F: Fn(windows::Win32::Foundation::HWND) + 'static,
{
    // Park off-screen BEFORE the first frame (always — see fn doc). The reveal
    // tick decorates + (under stealth) WDAs, then moves it on-screen, so the
    // first visible frame is complete. Unconditional so a stealth toggle
    // mid-realize can't strand the window off the desktop either.
    win.window()
        .set_position(slint::PhysicalPosition::new(-32000, -32000));
    let _ = win.show();
    // V0.8.4 — reveal as soon as the HWND realizes (~1-2 frames) instead of a
    // fixed 200ms blind wait, so on-demand windows (Settings/help/palette/wizard/
    // tiles) pop nearly instantly. A fast attempt covers the common case; if the
    // HWND isn't grabbable yet, ONE conservative fallback at the old delay keeps a
    // slow first-realize safe (no window stranded off-screen). Stealth-safe: in
    // BOTH paths WDA is applied BEFORE a parked window is moved on-screen.
    let do_reveal: Rc<dyn Fn(&W) -> bool> = Rc::new(move |w: &W| -> bool {
        let Ok(hwnd) = grab_hwnd(w.window()) else {
            return false;
        };
        decorate(hwnd);
        if global_stealth() {
            let _ = set_stealth(hwnd, true);
        }
        // The off-screen frame is now painted + decorated (+ WDA under stealth):
        // reveal it centered on the picked monitor using the real HiDPI-aware
        // size, so the first ON-SCREEN frame is already complete (no flash).
        let (_x, _y, w_px, h_px) = get_window_rect(hwnd).unwrap_or((0, 0, 460, 360));
        let monitors = enum_monitors();
        if let Some(mon) = pick_monitor(&monitors) {
            let cx = (mon.left + (mon.width() - w_px) / 2).max(mon.left + 8);
            let cy = (mon.top + (mon.height() - h_px) / 2).max(mon.top + 8);
            let _ = move_window_pos_only(hwnd, cx, cy);
        } else {
            let _ = move_window_pos_only(hwnd, 100, 100);
        }
        true
    });
    let weak = win.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_REVEAL_FAST_MS), move || {
        let Some(w) = weak.upgrade() else { return };
        let revealed = {
            let f = &*do_reveal;
            f(&w)
        };
        if revealed {
            return;
        }
        // HWND not realized within the fast window — one conservative retry.
        let weak2 = w.as_weak();
        let do_reveal2 = do_reveal.clone();
        Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
            if let Some(w) = weak2.upgrade() {
                let f = &*do_reveal2;
                let _ = f(&w);
            }
        });
    });
}

/// #B1 — push the LIVE open-tile count to the bar's `open-tiles` property so
/// the "+ tile (N)" label and the "close all" chip reflect reality. Call this
/// after EVERY `tiles.push(...)` and EVERY close-handler `tiles.retain(...)`
/// (and in the close-all handler). Distinct from `tiles_spawned`, which is a
/// monotonic display counter for the per-tile #N badge and must not change.
pub(crate) fn refresh_open_tiles(weak: &slint::Weak<OverlayBarWindow>, tiles: &TileWindows) {
    if let Some(o) = weak.upgrade() {
        o.set_open_tiles(tiles.borrow().len() as i32);
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

/// Single owner of the on-demand overlay windows whose stealth + theme must
/// stay in lock-step (§5.1). Every field is an `Rc<RefCell<…>>` clone of the
/// slot created in `main`, so the whole struct is cheap to `clone()` into each
/// handler closure. The bar (`OverlayBarWindow`) is intentionally NOT a field:
/// it is the composition root, has bespoke stealth side effects (the taskbar
/// button + the `stealth-active` chip), and the three handlers drive it inline
/// with subtly different rules (only some toggle `set_skip_taskbar`). The
/// persistent, pre-stealthed capture overlay is likewise excluded (§5.1).
#[derive(Clone)]
pub(crate) struct WindowRegistry {
    pub tiles: TileWindows,
    pub settings: Rc<RefCell<Option<SettingsWindow>>>,
    pub palette: Rc<RefCell<Option<PaletteWindow>>>,
    pub text_ask: Rc<RefCell<Option<TextAskWindow>>>,
    pub wizard: Rc<RefCell<Option<WizardWindow>>>,
    pub help: Rc<RefCell<Option<HelpWindow>>>,
    pub recover_offer: Rc<RefCell<Option<RecoverOfferWindow>>>,
}

impl WindowRegistry {
    /// Apply the WDA_EXCLUDEFROMCAPTURE flag to EVERY open registry window
    /// (tiles + Settings + palette + text-ask + wizard + Help + recover-offer)
    /// in one call. This replaces the three near-identical hand-written loops
    /// in the bar / wizard / Settings stealth handlers; the per-window blocks
    /// below mirror those loops exactly (same `grab_hwnd` + `set_stealth`
    /// pattern, same UI-property echoes), so the only behavioural change is
    /// that Help + the recover-offer can never again be forgotten in one loop.
    /// The caller still drives the bar itself inline (the taskbar button is
    /// toggled in some handlers but not all). The capture overlay is excluded
    /// (it is pre-stealthed for its whole lifetime).
    pub(crate) fn apply_stealth(&self, on: bool) {
        // All tiles.
        for t in self.tiles.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // Settings — also reflect the new state in its in-window Switch.
        if let Some(sw) = self.settings.borrow().as_ref() {
            sw.set_stealth_toggle(on);
            if let Ok(hwnd) = grab_hwnd(sw.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // F4 KB palette.
        if let Some(p) = self.palette.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(p.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // "✏ Написать" text-input window.
        if let Some(t) = self.text_ask.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // First-run wizard — also reflect the new state in its in-window Switch.
        if let Some(wz) = self.wizard.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(wz.window()) {
                let _ = set_stealth(hwnd, on);
            }
            wz.set_stealth_on(on);
        }
        // 🆘 Help window (FIX #6 — previously dropped from some loops).
        if let Some(h) = self.help.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(h.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // Crash-recovery-offer window (FIX #6 — previously dropped from some loops).
        if let Some(ro) = self.recover_offer.borrow().as_ref() {
            if let Ok(hwnd) = grab_hwnd(ro.window()) {
                let _ = set_stealth(hwnd, on);
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
    }

    /// Push the live open-tile count to the bar's `open-tiles` property (the
    /// `+ tile (N)` label + the "close all" chip). Registry-scoped wrapper over
    /// `refresh_open_tiles` for callers that already hold a concrete bar handle.
    pub(crate) fn refresh_tiles_chip(&self, overlay: &OverlayBarWindow) {
        overlay.set_open_tiles(self.tiles.borrow().len() as i32);
    }
}
