//! Dynamic floating "tile" windows: each carries a Q+A pair generated
//! from the live transcript. Tiles auto-place in a 2-column grid on the
//! preferred monitor (non-primary if available), auto-expire after a TTL,
//! and are evicted FIFO when capacity is reached.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{
    AppHandle, LogicalPosition, Manager, Monitor, WebviewUrl, WebviewWindowBuilder,
};

const TILE_W: f64 = 380.0;
/// Initial height when the tile window is created. TileWindow.tsx's
/// useLayoutEffect can resize up to TILE_H_MAX after markdown paints.
const TILE_H: f64 = 280.0;
/// Worst-case height after markdown auto-grow. MUST match the upper
/// bound in TileWindow.tsx. Grid uses this for row pitch so a tall
/// row-0 tile doesn't overlap row-1.
///
/// Math for fit-on-1080p-portable monitor (ARZOPA = 1920×1080):
///   3 rows × 400 + 4×PAD = 1248 < 1080 — STILL overflows by 168
///   2 rows × 400 + 3×PAD = 836 — fits comfortably
/// MAX_TILES = 6 (2 cols × 3 rows) but only first 4 visible always;
/// 5-6 may overflow until older ones get TTL-evicted.
const TILE_H_MAX: f64 = 400.0;
const PAD: f64 = 12.0;
const COLS: usize = 2;
/// 6 tiles = 2 cols × 3 rows. Top 4 always visible on 1080p, bottom 2
/// may overflow until TTL-evict (120s) frees space.
const MAX_TILES: usize = 6;
const TTL_SECS: u64 = 120;

#[derive(Clone)]
struct ActiveTile {
    #[allow(dead_code)]
    id: String,
    label: String,
    created: Instant,
    pinned: bool,
    /// Screen slot index used by `grid_position`. Tracked PER TILE (not
    /// derived from Vec.len()) because the Vec gets holes when tiles are
    /// closed manually, evicted by FIFO, or destroyed externally. Without
    /// this, the next spawn picks `slot = Vec.len()` which collides with
    /// a still-on-screen survivor → live regression 2026-05-25: "оношко
    /// заспавнилось на окошке".
    slot: usize,
}

#[derive(Default)]
pub struct TileManager {
    active: Vec<ActiveTile>,
}

pub type SharedTiles = Arc<Mutex<TileManager>>;

pub fn shared() -> SharedTiles {
    Arc::new(Mutex::new(TileManager::default()))
}

/// Pick the preferred display: explicit override, otherwise first non-primary,
/// otherwise primary. Returns the monitor's logical position+size.
fn pick_monitor(app: &AppHandle, preferred_name: Option<&str>) -> Option<MonitorRect> {
    let monitors = app.available_monitors().ok()?;
    if monitors.is_empty() {
        return None;
    }
    let primary = app.primary_monitor().ok().flatten();
    let primary_name = primary.as_ref().and_then(|m| m.name().cloned());

    // 1. Explicit name override
    if let Some(name) = preferred_name {
        if let Some(m) = monitors
            .iter()
            .find(|m| m.name().map(|n| n.as_str()) == Some(name))
        {
            return Some(MonitorRect::from(m));
        }
    }

    // 2. First non-primary monitor (the user explicitly wants tiles on the
    //    second screen — Zoom on monitor #1, tiles on monitor #2)
    if monitors.len() > 1 {
        if let Some(m) = monitors
            .iter()
            .find(|m| m.name().cloned() != primary_name)
        {
            return Some(MonitorRect::from(m));
        }
    }

    // 3. Fallback to primary
    primary.map(|m| MonitorRect::from(&m))
}

struct MonitorRect {
    x: f64,
    y: f64,
    w: f64,
    /// Currently unused — kept for future "wrap to next column at bottom"
    /// behaviour. Cheap to populate, makes the struct geometrically complete.
    #[allow(dead_code)]
    h: f64,
}

impl From<&Monitor> for MonitorRect {
    fn from(m: &Monitor) -> Self {
        let pos = m.position();
        let size = m.size();
        let scale = m.scale_factor();
        Self {
            x: pos.x as f64 / scale,
            y: pos.y as f64 / scale,
            w: size.width as f64 / scale,
            h: size.height as f64 / scale,
        }
    }
}

/// Compute the absolute position for the Nth tile in the chosen monitor.
/// Layout: top-right anchor, 2-column grid filling downward; when a column
/// pair is full (monitor.h exhausted) wraps LEFTward to the next pair of
/// columns. Prevents tiles drifting off-screen on portrait/short monitors.
///
/// Row pitch uses TILE_H_MAX (not TILE_H) so a row-0 tile that grows after
/// markdown render can't overlap the row-1 tile below it. Cost: small visual
/// gap under short tiles. Worth it — overlap was reported in live test.
fn grid_position(monitor: &MonitorRect, index: usize) -> LogicalPosition<f64> {
    let total_w = (TILE_W * COLS as f64) + (PAD * (COLS - 1) as f64);
    let row_h = TILE_H_MAX + PAD;
    // How many rows of tiles fit in this monitor without falling off the
    // bottom. Always at least 1, otherwise division by zero / no slot.
    let max_rows = (((monitor.h - PAD * 2.0) / row_h).floor() as usize).max(1);
    let per_pair = COLS * max_rows;
    let pair = index / per_pair;
    let local = index % per_pair;
    let col = local % COLS;
    let row = local / COLS;
    let start_x = monitor.x + monitor.w - total_w - PAD - pair as f64 * (total_w + PAD);
    let start_y = monitor.y + PAD;
    LogicalPosition::new(
        start_x + col as f64 * (TILE_W + PAD),
        start_y + row as f64 * row_h,
    )
}

/// Tile-color-coding categories. Carried via `?kind=` query into the
/// tile WebView so each tile gets a distinct CSS class — user sees at
/// a glance whether a suggestion came from auto-detector, their own
/// mic, the system audio (interviewer), or a manual hotkey.
#[derive(Debug, Clone, Copy)]
pub enum TileKind {
    /// Detector auto-triggered (question/keyword in transcript)
    Auto,
    /// Push-to-talk or click on 🔊 (interviewer side)
    System,
    /// Push-to-talk or click on 🎤 (user side)
    Mic,
    /// F6 manual spawn from last transcript line
    Manual,
}

impl TileKind {
    fn as_str(self) -> &'static str {
        match self {
            TileKind::Auto => "auto",
            TileKind::System => "system",
            TileKind::Mic => "mic",
            TileKind::Manual => "manual",
        }
    }
}

/// Spawn a new tile window. Returns the chosen label.
pub fn spawn_tile(
    app: &AppHandle,
    tiles: &SharedTiles,
    question: String,
    answer: String,
    preferred_monitor: Option<String>,
) -> Result<String> {
    spawn_tile_with_stealth(app, tiles, question, answer, preferred_monitor, false, TileKind::Auto)
}

pub fn spawn_tile_with_stealth(
    app: &AppHandle,
    tiles: &SharedTiles,
    question: String,
    answer: String,
    preferred_monitor: Option<String>,
    stealth: bool,
    kind: TileKind,
) -> Result<String> {
    let id = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let label = format!("tile-{id}");

    let monitor = pick_monitor(app, preferred_monitor.as_deref())
        .context("no monitor available")?;

    // Reserve a slot atomically: find FIRST FREE slot (not Vec.len()) so
    // we reuse gaps left by manually-closed / TTL-evicted tiles. Evicts
    // oldest if all MAX_TILES slots are occupied. Bug-hunt 2026-05-25:
    // Vec.len() was wrong because manual close shrinks the Vec but doesn't
    // shift survivors' on-screen positions — next spawn would collide.
    let (slot, oldest_to_close) = {
        let mut mgr = tiles.lock();
        let mut to_close = None;
        // Collect occupied slots so we can find the first free one.
        let occupied: std::collections::HashSet<usize> =
            mgr.active.iter().map(|t| t.slot).collect();
        let free_slot = (0..MAX_TILES).find(|i| !occupied.contains(i));
        let slot = match free_slot {
            Some(s) => s,
            None => {
                // All slots occupied — evict oldest, reuse its slot so we
                // immediately fill the hole instead of leaving a gap.
                if let Some(oldest) = mgr.active.first().cloned() {
                    let evicted_slot = oldest.slot;
                    mgr.active.remove(0);
                    to_close = Some(oldest.label);
                    evicted_slot
                } else {
                    0 // Shouldn't happen — empty Vec yet len >= MAX is impossible.
                }
            }
        };
        // Reserve the slot with a temporary placeholder. The slot field is
        // what grid_position uses, so concurrent spawns can't collide.
        mgr.active.push(ActiveTile {
            id: id.clone(),
            label: label.clone(),
            created: Instant::now(),
            pinned: false,
            slot,
        });
        (slot, to_close)
    };
    if let Some(old_label) = oldest_to_close {
        if let Some(w) = app.get_webview_window(&old_label) {
            let _ = w.close();
        }
    }
    let position = grid_position(&monitor, slot);

    // URL-encode question/answer into the route. WebView is sandboxed —
    // params are read in TileWindow.tsx via URLSearchParams.
    let q_enc = urlencoding_min(&question);
    let a_enc = urlencoding_min(&answer);
    let route = format!(
        "index.html?tile=1&id={}&kind={}&q={}&a={}",
        id, kind.as_str(), q_enc, a_enc
    );

    let window = match WebviewWindowBuilder::new(app, &label, WebviewUrl::App(route.into()))
        .title("Tile")
        .inner_size(TILE_W, TILE_H)
        .position(position.x, position.y)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .shadow(false)
        .focused(false)
        .build()
    {
        Ok(w) => w,
        Err(e) => {
            // Build failed — undo the slot reservation so the grid doesn't leak.
            let mut mgr = tiles.lock();
            mgr.active.retain(|t| t.label != label);
            return Err(anyhow::anyhow!("WebviewWindowBuilder failed: {e}"));
        }
    };

    // (Dropped a redundant `window.set_size(...)` here that duplicated the
    // builder's `.inner_size()` — was flagged as a frame-flicker source.)
    // STEALTH: only enable when caller asks for it (config.stealth_enabled).
    if stealth {
        if let Err(e) = window.set_content_protected(true) {
            log::warn!("tile content protection failed for {label}: {e}");
        }
    }

    // RECONCILE ON EXTERNAL CLOSE: if the user Alt+F4's a tile (or it
    // crashes), Tauri tears down the webview but our `active` Vec still
    // holds the entry. Without this handler, the next spawn picks a slot
    // index based on stale length and either overlaps a real tile or
    // leaves a gap, plus pin/close operations fail silently. Reconcile
    // by removing the entry on Destroyed event.
    let tiles_for_close = tiles.clone();
    let label_for_close = label.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            let mut mgr = tiles_for_close.lock();
            let before = mgr.active.len();
            mgr.active.retain(|t| t.label != label_for_close);
            if mgr.active.len() < before {
                log::debug!("tile destroyed externally, state reconciled: {label_for_close}");
            }
        }
    });

    let app_clone = app.clone();
    let label_for_ttl = label.clone();
    let tiles_for_ttl = tiles.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(TTL_SECS)).await;
        // Atomic check+remove: prevents a race where the user pins the tile
        // *between* a non-atomic pin check and the close call. With the
        // check inside the same lock as removal, the worst case is that
        // `set_tile_pinned()` races with the TTL — and that ordering is
        // deterministic by mutex, not by wall clock. If take_if_unpinned
        // wins, set_tile_pinned simply returns false (tile gone).
        if take_if_unpinned(&tiles_for_ttl, &label_for_ttl) {
            if let Some(w) = app_clone.get_webview_window(&label_for_ttl) {
                let _ = w.close();
            }
        }
    });

    log::info!("tile spawned: label={label} q='{}'", question.chars().take(60).collect::<String>());
    Ok(label)
}

pub fn set_tile_pinned(tiles: &SharedTiles, label: &str, pinned: bool) -> bool {
    let mut mgr = tiles.lock();
    if let Some(t) = mgr.active.iter_mut().find(|t| t.label == label) {
        t.pinned = pinned;
        true
    } else {
        false
    }
}

/// Atomic: if tile `label` exists AND isn't pinned, remove it from the
/// active list and return true. Caller is then responsible for closing
/// the webview window itself. This is the only safe primitive for the
/// TTL closer — see comment in spawn_tile_with_stealth().
fn take_if_unpinned(tiles: &SharedTiles, label: &str) -> bool {
    let mut mgr = tiles.lock();
    if let Some(pos) = mgr.active.iter().position(|t| t.label == label) {
        if !mgr.active[pos].pinned {
            mgr.active.remove(pos);
            return true;
        }
    }
    false
}

pub fn close_tile_by_label(app: &AppHandle, tiles: &SharedTiles, label: &str) {
    let mut mgr = tiles.lock();
    if let Some(pos) = mgr.active.iter().position(|t| t.label == label) {
        mgr.active.remove(pos);
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}

/// Periodically reap expired tiles (defensive — TTL task above usually
/// handles it, but if a tile's task ever panics or is dropped, this
/// sweeps any zombies). Like the TTL task, this respects pin state.
pub fn reaper_tick(app: &AppHandle, tiles: &SharedTiles) {
    let to_close = reap_expired(tiles, Instant::now(), Duration::from_secs(TTL_SECS + 5));
    // Close webviews outside the mutex to avoid deadlock if Tauri's close
    // path takes any internal lock.
    for label in to_close {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.close();
        }
    }
}

/// Atomic core of reaper_tick — testable without AppHandle. Removes
/// expired-and-unpinned tiles from `active` and returns their labels for
/// the caller to close. `grace` is added to TTL so we only reap stuff
/// the TTL task should have already handled.
fn reap_expired(tiles: &SharedTiles, now: Instant, grace: Duration) -> Vec<String> {
    let mut mgr = tiles.lock();
    let expired: Vec<String> = mgr
        .active
        .iter()
        .filter(|t| !t.pinned && now.duration_since(t.created) > grace)
        .map(|t| t.label.clone())
        .collect();
    // Remove from active under the same lock so a concurrent pin can't
    // race in between (mirrors take_if_unpinned semantics).
    mgr.active.retain(|t| !expired.contains(&t.label));
    expired
}

/// Minimal URL-encoder for our use (text params, no binary). Keeps things
/// dependency-free — full `urlencoding` crate isn't justified for this.
fn urlencoding_min(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_monitor() -> MonitorRect {
        // 1920x1080 at origin (0, 0) — typical primary.
        MonitorRect { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0 }
    }

    /// Each tile in the grid must occupy a distinct rectangle even if it
    /// auto-grows to TILE_H_MAX after markdown renders. Uses TILE_H_MAX
    /// (worst case) for overlap calc, mirroring the bug in live test where
    /// row spacing of TILE_H let tall tiles eat into the row below.
    #[test]
    fn grid_positions_do_not_overlap_at_worst_case_height() {
        let m = mock_monitor();
        let positions: Vec<_> = (0..MAX_TILES).map(|i| grid_position(&m, i)).collect();
        for (i, p1) in positions.iter().enumerate() {
            for (j, p2) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dx = (p1.x - p2.x).abs();
                let dy = (p1.y - p2.y).abs();
                let overlap = dx < TILE_W && dy < TILE_H_MAX;
                assert!(
                    !overlap,
                    "tiles {i} and {j} overlap at worst-case height: \
                     ({}, {}) vs ({}, {}) on {}x{} monitor — TILE_H_MAX={}",
                    p1.x, p1.y, p2.x, p2.y, m.w, m.h, TILE_H_MAX
                );
            }
        }
    }

    /// Tiles must stay within monitor bounds (no off-screen rendering).
    /// The grid now wraps to a NEW column-pair (leftward) when the current
    /// pair fills vertically — see grid_position. So even on a small/portrait
    /// monitor every tile up to MAX_TILES should fit on-screen vertically.
    #[test]
    fn grid_positions_stay_within_monitor_bounds() {
        let m = mock_monitor();
        for i in 0..MAX_TILES {
            let p = grid_position(&m, i);
            assert!(
                p.x >= m.x && p.x + TILE_W <= m.x + m.w,
                "tile {i} x={} off horizontally", p.x
            );
            assert!(p.y >= m.y, "tile {i} y={} above monitor top", p.y);
            // After the wrap-to-next-pair fix: vertical must NOT exceed bottom.
            assert!(
                p.y + TILE_H_MAX <= m.y + m.h,
                "tile {i} y={} + height={} = {} exceeds monitor bottom y={} h={} -> bottom {}",
                p.y, TILE_H_MAX, p.y + TILE_H_MAX, m.y, m.h, m.y + m.h
            );
        }
    }

    /// REGRESSION (S1): on a SHORT monitor (e.g. 1080p in landscape only fits
    /// 2 rows of 412-px-tall tiles), tiles beyond row 2 must wrap to the
    /// NEXT column-pair on the left rather than render below the bottom edge.
    #[test]
    fn grid_wraps_to_next_pair_on_short_monitor() {
        // Tall enough for 2 rows: 2 * (TILE_H_MAX + PAD) + 2*PAD ≈ 850
        let m = MonitorRect { x: 0.0, y: 0.0, w: 1920.0, h: 900.0 };
        let row_h = TILE_H_MAX + PAD;
        let max_rows = (((m.h - PAD * 2.0) / row_h).floor() as usize).max(1);
        assert!(max_rows >= 2, "test fixture should allow ≥2 rows");
        // Tile at index `per_pair` (first slot of pair 2) must move LEFT
        // by one column-pair width.
        let p0 = grid_position(&m, 0);
        let per_pair = COLS * max_rows;
        let p_next = grid_position(&m, per_pair);
        assert!(
            p_next.x < p0.x,
            "first tile of pair 2 should be left of pair 1 — got pair1.x={} pair2.x={}",
            p0.x, p_next.x
        );
        // All MAX_TILES must still be on-screen vertically.
        for i in 0..MAX_TILES {
            let p = grid_position(&m, i);
            assert!(
                p.y + TILE_H_MAX <= m.y + m.h,
                "tile {i} off-screen vertically on short monitor"
            );
        }
    }

    /// Top-right anchor: first tile's right edge should hug monitor's right edge.
    #[test]
    fn first_tile_is_anchored_top_right() {
        let m = mock_monitor();
        let p0 = grid_position(&m, 0);
        // Position 0 is left column. Right column position 1 should have
        // its right edge near monitor's right edge (within PAD).
        let p1 = grid_position(&m, 1);
        let right_edge = p1.x + TILE_W;
        assert!(
            (m.x + m.w - right_edge - PAD).abs() < 1.0,
            "tile 1 right edge {right_edge} should be near monitor right {}", m.x + m.w
        );
        assert!(p0.y < m.y + TILE_H, "first row should be at the top");
    }

    #[test]
    fn urlencoding_handles_cyrillic_and_special() {
        let s = urlencoding_min("Что такое etcd?");
        // Cyrillic bytes percent-encoded, '?' encoded, ASCII letters preserved.
        assert!(!s.contains('?'), "? must be encoded");
        assert!(!s.contains(' '), "space must be encoded");
        assert!(s.contains("etcd"), "ASCII preserved");
        assert!(s.contains("%"), "non-ASCII bytes encoded");
    }

    #[test]
    fn urlencoding_roundtrip_safe_chars_unchanged() {
        let s = urlencoding_min("AbC.123-_~");
        assert_eq!(s, "AbC.123-_~");
    }

    #[test]
    fn urlencoding_empty_string_stays_empty() {
        assert_eq!(urlencoding_min(""), "");
    }

    #[test]
    fn urlencoding_only_specials_all_encoded() {
        // Every byte must be percent-encoded
        let s = urlencoding_min("?&=#");
        assert_eq!(s, "%3F%26%3D%23");
    }

    /// `set_tile_pinned` finds the right tile by label and only mutates it.
    #[test]
    fn set_tile_pinned_finds_by_label_and_isolated() {
        let mgr = shared();
        {
            let mut m = mgr.lock();
            m.active.push(ActiveTile { id: "a".into(), label: "tile-1".into(), created: Instant::now(), pinned: false, slot: 0 });
            m.active.push(ActiveTile { id: "b".into(), label: "tile-2".into(), created: Instant::now(), pinned: false, slot: 1 });
            m.active.push(ActiveTile { id: "c".into(), label: "tile-3".into(), created: Instant::now(), pinned: false, slot: 2 });
        }
        assert!(set_tile_pinned(&mgr, "tile-2", true), "should find tile-2");
        {
            let m = mgr.lock();
            assert!(!m.active[0].pinned, "tile-1 must not be touched");
            assert!(m.active[1].pinned, "tile-2 must be pinned");
            assert!(!m.active[2].pinned, "tile-3 must not be touched");
        }
    }

    #[test]
    fn set_tile_pinned_unknown_label_returns_false() {
        let mgr = shared();
        assert!(!set_tile_pinned(&mgr, "nonexistent", true));
    }

    // ── take_if_unpinned — atomic TTL pin-race primitive ──

    #[test]
    fn take_if_unpinned_removes_unpinned() {
        let mgr = shared();
        {
            let mut m = mgr.lock();
            m.active.push(ActiveTile {
                id: "a".into(),
                label: "tile-1".into(),
                created: Instant::now(),
                pinned: false,
                slot: 0,
            });
        }
        assert!(take_if_unpinned(&mgr, "tile-1"));
        assert_eq!(mgr.lock().active.len(), 0, "tile must be removed from active");
    }

    #[test]
    fn take_if_unpinned_leaves_pinned() {
        let mgr = shared();
        {
            let mut m = mgr.lock();
            m.active.push(ActiveTile {
                id: "a".into(),
                label: "tile-pinned".into(),
                created: Instant::now(),
                pinned: true,
                slot: 0,
            });
        }
        assert!(!take_if_unpinned(&mgr, "tile-pinned"));
        assert_eq!(mgr.lock().active.len(), 1, "pinned tile must stay");
        assert!(mgr.lock().active[0].pinned, "pin state unchanged");
    }

    #[test]
    fn take_if_unpinned_unknown_label_returns_false() {
        let mgr = shared();
        assert!(!take_if_unpinned(&mgr, "ghost"));
    }

    /// Regression: the TTL pin race. Before the atomic fix, the sequence
    /// {check pin status} → release lock → {set_pin true} → {close tile} could
    /// destroy a freshly-pinned tile. With take_if_unpinned, set_tile_pinned
    /// and take_if_unpinned are serialised by the mutex — whoever wins, the
    /// outcome is consistent.
    #[test]
    fn ttl_race_atomic_when_take_wins() {
        let mgr = shared();
        {
            let mut m = mgr.lock();
            m.active.push(ActiveTile {
                id: "a".into(),
                label: "tile-race".into(),
                created: Instant::now(),
                pinned: false,
                slot: 0,
            });
        }
        // TTL fires first → tile removed; subsequent set_pin must fail.
        let took = take_if_unpinned(&mgr, "tile-race");
        let pinned_after = set_tile_pinned(&mgr, "tile-race", true);
        assert!(took, "take should succeed when called first on unpinned tile");
        assert!(!pinned_after, "pin must not silently succeed on already-removed tile");
    }

    // ── reap_expired (atomic core of reaper_tick) ──

    #[test]
    fn reap_removes_only_expired_unpinned() {
        let mgr = shared();
        let now = Instant::now();
        let old = now - Duration::from_secs(200); // older than TTL + grace
        let new = now - Duration::from_secs(5);   // fresh
        {
            let mut m = mgr.lock();
            // Old, unpinned — must be reaped
            m.active.push(ActiveTile { id: "a".into(), label: "old-unpinned".into(), created: old, pinned: false, slot: 0 });
            // Old, pinned — must survive
            m.active.push(ActiveTile { id: "b".into(), label: "old-pinned".into(), created: old, pinned: true, slot: 1 });
            // New, unpinned — must survive (not yet expired)
            m.active.push(ActiveTile { id: "c".into(), label: "new-unpinned".into(), created: new, pinned: false, slot: 2 });
        }
        let reaped = reap_expired(&mgr, now, Duration::from_secs(125)); // TTL + 5
        assert_eq!(reaped, vec!["old-unpinned".to_string()]);
        let labels: Vec<_> = mgr.lock().active.iter().map(|t| t.label.clone()).collect();
        assert_eq!(labels, vec!["old-pinned".to_string(), "new-unpinned".into()]);
    }

    #[test]
    fn reap_empty_active_returns_empty() {
        let mgr = shared();
        let reaped = reap_expired(&mgr, Instant::now(), Duration::from_secs(125));
        assert!(reaped.is_empty());
    }

    #[test]
    fn reap_zero_grace_treats_any_age_as_expired() {
        let mgr = shared();
        let now = Instant::now();
        {
            let mut m = mgr.lock();
            // Created exactly now — duration_since == 0
            m.active.push(ActiveTile { id: "x".into(), label: "fresh".into(), created: now, pinned: false, slot: 0 });
        }
        // Sleep a microsecond so now elapses
        std::thread::sleep(Duration::from_millis(2));
        let later = Instant::now();
        let reaped = reap_expired(&mgr, later, Duration::from_millis(1));
        assert_eq!(reaped, vec!["fresh".to_string()], "any duration > grace should reap");
    }

    /// Pure-fn helper that mirrors the slot-picking logic inside spawn_tile.
    /// Extracted so we can unit-test gap-reuse without spinning up Tauri.
    fn pick_free_slot(occupied: &[usize], max: usize) -> Option<usize> {
        let set: std::collections::HashSet<usize> = occupied.iter().copied().collect();
        (0..max).find(|i| !set.contains(i))
    }

    /// Regression: when a non-last tile is closed (×, TTL, external),
    /// the Vec gets a hole. Next spawn used to pick `slot = Vec.len()`,
    /// which COLLIDED with a still-on-screen survivor at that index.
    /// Live bug 2026-05-25: "оношко заспавнилось на окошке". Fix: track
    /// `slot` per ActiveTile and pick first FREE index, not Vec length.
    #[test]
    fn slot_picker_reuses_gap_after_middle_close() {
        // Initial: 3 tiles at slots 0, 1, 2.
        // User closes the middle one → surviving slots = {0, 2}.
        // Old code: new tile got slot = Vec.len() = 2 → COLLISION with
        //           the surviving slot-2 tile.
        // New code: finds first free index = 1 → no collision.
        let after_middle_close = vec![0_usize, 2];
        assert_eq!(pick_free_slot(&after_middle_close, 6), Some(1));
    }

    #[test]
    fn slot_picker_reuses_oldest_after_full_eviction() {
        // All 6 slots occupied. Spawn-time eviction removes oldest
        // (whichever sits at index 0 in Vec — that's the FIRST-INSERTED).
        // Whatever slot that oldest occupied is the one the new tile
        // should reuse, otherwise we leave a permanent gap.
        let all_full = vec![0_usize, 1, 2, 3, 4, 5];
        assert_eq!(
            pick_free_slot(&all_full, 6),
            None,
            "no free slot when all 6 occupied — caller must evict first"
        );
        // After evicting whatever was at slot 0 → free slot is 0 again.
        let after_evict = vec![1_usize, 2, 3, 4, 5];
        assert_eq!(pick_free_slot(&after_evict, 6), Some(0));
    }

    #[test]
    fn slot_picker_starts_at_zero_when_empty() {
        assert_eq!(pick_free_slot(&[], 6), Some(0));
    }

    #[test]
    fn slot_picker_handles_unordered_occupied() {
        // Vec might be in any order; HashSet check is order-agnostic.
        let occupied = vec![3_usize, 0, 5];
        assert_eq!(pick_free_slot(&occupied, 6), Some(1));
    }

    #[test]
    fn ttl_race_atomic_when_pin_wins() {
        let mgr = shared();
        {
            let mut m = mgr.lock();
            m.active.push(ActiveTile {
                id: "b".into(),
                label: "tile-race2".into(),
                created: Instant::now(),
                pinned: false,
                slot: 0,
            });
        }
        // User pins first → TTL must respect it and leave the tile alone.
        let pinned_first = set_tile_pinned(&mgr, "tile-race2", true);
        let took_after = take_if_unpinned(&mgr, "tile-race2");
        assert!(pinned_first, "pin succeeds before TTL fires");
        assert!(!took_after, "TTL must respect pin set just before it ran");
        assert_eq!(mgr.lock().active.len(), 1, "pinned tile survives");
    }
}
