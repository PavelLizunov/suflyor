//! Tile presentation / HiDPI placement / maximize / drag — the win32-facing
//! LEAF helpers carved out of `tile_controller.rs` (a later wave of the
//! `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.10 and
//! `docs/overlay-host-current-review.md` §"tile_controller.rs стал новым
//! мини-монолитом").
//!
//! This module owns the per-tile window glue, none of the AI/conversation
//! machinery:
//!
//! - `present_tile_window` — show a freshly-built tile WITHOUT a stealth
//!   capture-flash (parks it off the virtual desktop first when stealth is on);
//! - `apply_tile_hwnd_with_monitor` — apply transparency + body opacity +
//!   always-on-top + stealth inheritance, then place the tile on a 2-column grid
//!   on the chosen monitor (HiDPI-aware, hard-clamped on-screen);
//! - `toggle_tile_maximize` — maximize/restore toggle that keeps the resized
//!   tile fully inside its monitor's work area;
//! - `wire_tile_drag` — seed the tile's `Theme.scheme` + wire the chrome-row
//!   cursor-delta drag callbacks;
//! - the per-spawn slot counters `TILE_SLOT_COUNTER` (grid index) and
//!   `TILE_DISPLAY_SEQ` (the tile-title #N badge).
//!
//! These reach the win32 helpers (`grab_hwnd`, `make_transparent_tile`,
//! `set_always_on_top`, `set_stealth`, `get_window_rect`, `enum_monitors`,
//! `pick_monitor`, `move_window_pos_only`, `work_area_for_window`,
//! `drag_begin`/`drag_update`), `window_lifecycle`'s process-global stealth /
//! scheme / tile-opacity (`global_stealth`, `apply_scheme_tile`, `global_scheme`,
//! `global_tile_opacity`), the Slint `TileWindow`, and the shared tuning
//! constants (`HWND_GRAB_DELAY_MS`, `TILE_DEFAULT_W`/`TILE_DEFAULT_H`) through the
//! crate-root glob below.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    apply_scheme_tile, drag_begin, drag_update, enum_monitors, get_window_rect, global_scheme,
    global_stealth, global_tile_opacity, grab_hwnd, make_transparent_tile, move_window_pos_only,
    pick_monitor, set_always_on_top, set_stealth, work_area_for_window, ComponentHandle, Duration,
    TileWindow, Timer, HWND_GRAB_DELAY_MS, TILE_DEFAULT_H, TILE_DEFAULT_W,
};

/// Atomic counter for tile-slot index — increments per spawn so
/// successive tiles distribute across a 2-column grid on the right
/// half of the chosen monitor.
static TILE_SLOT_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Monotonic counter for the tile-title #N badge. Increments per
/// spawn (never wraps) so the user can tell tiles apart in a busy
/// session. Reset only at process restart.
pub(crate) static TILE_DISPLAY_SEQ: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Phase E6 v17 — maximize toggle helper. User: "нет функционала
/// развернуть, нужно отдельной кнопкой или даб-кликом". Maximized
/// tile is 800×600 (~1.7× default); restored back to 460×360. Uses
/// Win32 SetWindowPos with current position so the tile expands in
/// place from its top-left corner. Flips tile.maximized so the
/// button glyph updates.
pub(crate) fn toggle_tile_maximize(hwnd: windows::Win32::Foundation::HWND, tile: &TileWindow) {
    // Phase E6 v18 fix — use Slint's window().set_size() not raw
    // Win32 SetWindowPos. SetWindowPos resized the OS window but
    // left Slint's layout pass thinking the size was still 460×360
    // → chrome buttons (pin/max/X) stayed at old logical positions
    // → user clicks hit dead space. set_size goes through the Slint
    // engine which both updates the OS window AND re-runs layout.
    // Fixes: "когда я развернул окно, другой его функционал завис".
    let new = !tile.get_maximized();
    let (w, h): (f32, f32) = if new { (800.0, 600.0) } else { (460.0, 360.0) };
    tile.window().set_size(slint::LogicalSize::new(w, h));
    tile.set_maximized(new);

    // Phase E6 v45 — keep the resized tile fully on-screen. Growing in
    // place from the top-left pushed tiles near a screen edge/corner off
    // the monitor (user: "тайл у угла раскрывается за экран"). Work in
    // PHYSICAL pixels (logical × DPI scale) since Win32 rects/positions
    // are physical, then nudge the origin back inside the tile's monitor.
    let scale = tile.window().scale_factor();
    let pw = (w * scale) as i32;
    let ph = (h * scale) as i32;
    // Clamp against the WORK AREA (monitor minus taskbar) of the tile's
    // own monitor so a maximized tile near an edge/corner stays fully
    // visible AND its bottom row (the follow-up input) clears the taskbar.
    if let (Ok((x, y, _r, _b)), Some(m)) = (get_window_rect(hwnd), work_area_for_window(hwnd)) {
        let mut nx = x;
        let mut ny = y;
        // Pull the right/bottom edges inside first, then guarantee the
        // top-left stays visible (matters if the tile is wider/taller
        // than the work area — keep the top-left corner reachable).
        if nx + pw > m.right {
            nx = m.right - pw;
        }
        if ny + ph > m.bottom {
            ny = m.bottom - ph;
        }
        if nx < m.left {
            nx = m.left;
        }
        if ny < m.top {
            ny = m.top;
        }
        if nx != x || ny != y {
            let _ = move_window_pos_only(hwnd, nx, ny);
        }
    }
    diag!("tile maximized -> {new} (logical {w}x{h}, phys {pw}x{ph})");
}

/// Wire the chrome-row drag callbacks on a tile so the user can move
/// it by pressing+dragging the title area. Phase E6 v22 — manual
/// cursor-delta drag (drag_begin on down, drag_update on move-while-
/// pressed). REPLACES the old WM_NCLBUTTONDOWN modal system-drag
/// which consumed the mouse-up before Slint saw it, leaving the
/// TouchArea stuck "pressed" → tile became undraggable/unclickable.
/// User: "вызванный тайл завис, двигается но ничего не прожимается".
pub(crate) fn wire_tile_drag(tile: &TileWindow) {
    // Seed this tile's Theme global from the process-global scheme. Called on
    // every tile-creation path, so newly-spawned tiles inherit the live choice
    // without threading the value through 5 call sites.
    apply_scheme_tile(tile, global_scheme());
    let weak = tile.as_weak();
    tile.on_drag_start_requested(move || {
        if let Some(t) = weak.upgrade() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                drag_begin(hwnd);
            }
        }
    });
    let weak_move = tile.as_weak();
    tile.on_drag_moved(move || {
        if let Some(t) = weak_move.upgrade() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                drag_update(hwnd);
            }
        }
    });
}

/// Apply transparency + position tile on the appropriate monitor.
///
/// Show a freshly-built tile WITHOUT a stealth capture-flash.
///
/// Bug: under stealth, every tile used to be `show()`n on-screen at winit's
/// default position and only stealthed ~200 ms later (WDA_EXCLUDEFROMCAPTURE
/// needs a realized HWND — see `apply_tile_hwnd_with_monitor`). For that gap the
/// tile was fully capturable, so a screen-share saw a ~0.1 s flash of the tile.
///
/// Fix: when stealth is on, park the window OFF the virtual desktop BEFORE its
/// first frame, so winit realizes the HWND off-screen and the tile is never
/// composited onto a real monitor. `apply_tile_hwnd_with_monitor` then applies
/// WDA *before* it moves the tile on-screen, so the first on-screen frame is
/// already excluded from capture. Same pattern the persistent capture overlay
/// uses. When stealth is off there's nothing to hide, so show normally.
pub(crate) fn present_tile_window(tile: &TileWindow) {
    if global_stealth() {
        tile.window()
            .set_position(slint::PhysicalPosition::new(-32000, -32000));
    }
    let _ = tile.show();
}

/// Phase E6 fix v2 (2026-05-27): previous "right-edge stack" math
/// overflowed monitor.bottom after ~slot 2 (tile_h+12 × N > screen
/// height) → user complaint "тайлы уходят за экран". Now uses a
/// 2-column × dynamic-rows grid with hard clamps to monitor bounds.
/// Pre-port React/Tauri used src-tauri's tile.rs::grid_position
/// (~80 LOC of layered math); this is a simpler 2-col wrap that
/// fits on any landscape monitor without overflow.
pub(crate) fn apply_tile_hwnd_with_monitor(tile: &TileWindow) {
    // Phase E6 v36 — every spawn path funnels through here, so this is
    // the one place to apply the saved tile body opacity. Without this,
    // only tiles that existed when the Settings slider moved went
    // transparent; freshly spawned tiles reset to opaque (user bug
    // report). Set synchronously on the passed handle so it takes
    // effect on the first painted frame.
    tile.set_body_opacity(global_tile_opacity());

    let weak = tile.as_weak();
    Timer::single_shot(Duration::from_millis(HWND_GRAB_DELAY_MS), move || {
        let Some(t) = weak.upgrade() else { return };
        let Ok(hwnd) = grab_hwnd(t.window()) else {
            return;
        };

        // Phase E6 fix v4 — use make_transparent_tile (no WS_EX_
        // TRANSPARENT) so tiles accept clicks for buttons + drag.
        // Previous make_transparent_overlay set WS_EX_TRANSPARENT
        // which made every click pass through to underlying windows
        // (Explorer/desktop), silently swallowing every chrome-row
        // press → drag-to-move never fired. Same root cause as user
        // complaint "тайлы нельзя двигать".
        let _ = make_transparent_tile(hwnd);

        // Phase E6 v5 — Slint's `always-on-top: true` declaration is
        // applied at window creation but doesn't reliably translate
        // to HWND_TOPMOST on Windows + winit + skia. Explicitly set
        // HWND_TOPMOST so tile windows sit above Explorer / desktop
        // / browser windows and the user can interact with them.
        // Without this, clicks land on whatever non-topmost window
        // is at the pixel under the tile.
        let _ = set_always_on_top(hwnd, true);

        // #111 — inherit stealth: a tile spawned while stealth is on must
        // also be excluded from screen capture (the toggle only covered tiles
        // that already existed). No-op when stealth is off.
        if global_stealth() {
            let _ = set_stealth(hwnd, true);
        }

        // Phase E6 fix v3 — read the ACTUAL physical window size that
        // Slint produced (HiDPI-aware), then place using that real
        // width so the right-edge alignment is accurate. Previous
        // version forced TILE_DEFAULT_W (460 raw pixels) which
        // overrode Slint's logical-to-physical scaling and made
        // tile content overflow the dark fill area on 125% scaling.
        let (_cur_x, _cur_y, real_w, real_h) =
            get_window_rect(hwnd).unwrap_or((0, 0, TILE_DEFAULT_W, TILE_DEFAULT_H));

        let monitors = enum_monitors();
        if let Some(mon) = pick_monitor(&monitors) {
            let gap_x: i32 = 12;
            let gap_y: i32 = 12;
            let top_margin: i32 = 80;
            let right_margin: i32 = 20;

            let usable_h = mon.height().saturating_sub(top_margin + 20);
            let rows = ((usable_h + gap_y) / (real_h + gap_y)).max(1) as usize;
            let cols: usize = 2;
            let total_slots = (rows * cols).max(1);

            // Phase E6 v9 — cascade-offset on wrap. Previously
            // `slot = COUNTER % total_slots` made the 5th+ tile land
            // ON TOP of the 1st tile, etc. User complaint: "потом
            // они начали друг на друга прыгать". Now: track which
            // cycle (wraparound generation) we're on, and offset
            // every wrapped tile by (cascade_dx, cascade_dy) per
            // cycle — visually a stagger like macOS cascade-windows.
            // Hard clamps still prevent off-screen.
            let raw_seq = TILE_SLOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let slot = raw_seq % total_slots;
            let cycle = raw_seq / total_slots; // 0 for first batch, 1 for second, etc.
            let cascade_dx: i32 = 32;
            let cascade_dy: i32 = 24;
            let row = slot / cols;
            let col = slot % cols;

            let x_outer = mon.left + mon.width() - real_w - right_margin;
            let x_inner = x_outer - real_w - gap_x;
            let x_base = if col == 0 { x_inner } else { x_outer };
            let y_base = mon.top + top_margin + (row as i32) * (real_h + gap_y);

            // Cascade offset grows leftward + downward so wrapped tiles
            // peek out from under their first-cycle siblings. Negative
            // dx on x because the right-cluster is already at right edge.
            let x = x_base - (cycle as i32) * cascade_dx;
            let y = y_base + (cycle as i32) * cascade_dy;

            // Hard clamp so a tile can never land off-screen even if
            // monitor enum returned weird coordinates (portrait
            // secondary at negative x). The max bound is `.max()`'d with the
            // min so a tile WIDER/TALLER than the monitor (possible on the
            // 1200px portrait secondary, or under heavy DPI) can't make
            // max < min and panic `i32::clamp` — it just pins to the top-left
            // margin instead of crashing.
            let x_min = mon.left + 8;
            let x_max = (mon.right - real_w - 8).max(x_min);
            let y_min = mon.top + 8;
            let y_max = (mon.bottom - real_h - 8).max(y_min);
            let x_clamped = x.clamp(x_min, x_max);
            let y_clamped = y.clamp(y_min, y_max);

            eprintln!(
                "[overlay-host] tile placement: monitor=({},{},{},{}) real_size=({},{}) slot={} cycle={} row={} col={} pos=({},{})",
                mon.left, mon.top, mon.right, mon.bottom,
                real_w, real_h, slot, cycle, row, col, x_clamped, y_clamped,
            );
            // Move-only — preserve Slint's natural size so HiDPI
            // rendering stays correct (text fills the dark fill area
            // instead of overflowing).
            let _ = move_window_pos_only(hwnd, x_clamped, y_clamped);
        } else {
            // No monitor from pick_monitor (degenerate — no primary display).
            // A stealth-parked tile would otherwise stay off the virtual desktop
            // (permanently invisible), so bring it back to a safe on-screen spot.
            let _ = move_window_pos_only(hwnd, 100, 100);
            eprintln!("[overlay-host] tile placement: no monitor from pick_monitor — fallback to (100, 100)");
        }
    });
}
