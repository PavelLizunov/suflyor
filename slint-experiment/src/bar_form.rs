//! Select the largest declared bar form allowed by the preference and work area.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarForm {
    Full,
    Compact,
    Mini,
    TooNarrow,
}

#[must_use]
pub fn effective_form(
    user_wants_compact: bool,
    monitor_width_logical: f32,
    full: f32,
    compact: f32,
    mini: f32,
    margin: f32,
) -> BarForm {
    let fits = |width| monitor_width_logical >= width + 2.0 * margin;
    if !user_wants_compact && fits(full) {
        BarForm::Full
    } else if fits(compact) {
        BarForm::Compact
    } else if fits(mini) {
        BarForm::Mini
    } else {
        BarForm::TooNarrow
    }
}

/// Clamp a saved physical-pixel offset against the current work area and size.
/// If either dimension cannot fit, keep the top-left controls reachable.
#[must_use]
pub fn saved_bar_position(
    offset: (i32, i32),
    work: crate::win32::MonitorRect,
    width: i32,
    height: i32,
) -> (i32, i32) {
    if width > work.width() || height > work.height() {
        return (work.left, work.top);
    }
    let max_x = work.right.saturating_sub(width.max(0)).max(work.left);
    let max_y = work.bottom.saturating_sub(height.max(0)).max(work.top);
    (
        work.left.saturating_add(offset.0).clamp(work.left, max_x),
        work.top.saturating_add(offset.1).clamp(work.top, max_y),
    )
}

/// A straddling rectangle is not contained, even if its centre is on-screen.
#[must_use]
pub fn bar_rect_inside_any(
    (x, y, width, height): (i32, i32, i32, i32),
    works: &[crate::win32::MonitorRect],
) -> bool {
    width > 0
        && height > 0
        && works.iter().any(|work| {
            x >= work.left
                && y >= work.top
                && i64::from(x) + i64::from(width) <= i64::from(work.right)
                && i64::from(y) + i64::from(height) <= i64::from(work.bottom)
        })
}

/// On a work-area change, prefer that monitor's saved offset; otherwise keep
/// the current origin. Both paths use the same clamp for the current bar size.
#[must_use]
pub fn bar_position_after_monitor_change(
    saved_offset: Option<(i32, i32)>,
    work: crate::win32::MonitorRect,
    (x, y, width, height): (i32, i32, i32, i32),
) -> (i32, i32) {
    let offset = saved_offset.unwrap_or((x.saturating_sub(work.left), y.saturating_sub(work.top)));
    saved_bar_position(offset, work, width, height)
}

#[must_use]
pub fn should_save_bar_position(
    dragged_since_save: bool,
    stable: bool,
    dragging_now: bool,
    hidden: bool,
) -> bool {
    dragged_since_save && stable && !dragging_now && !hidden
}

/// Only drag callbacks make a position eligible. Two equal samples establish
/// stability; programmatic moves reset sampling, not drag intent. A failed save
/// keeps that intent for retry, and a successful save consumes it.
#[derive(Default)]
pub struct BarPositionTracker {
    previous: Option<(i32, i32, i32, i32)>,
    dragged_since_save: bool,
}

impl BarPositionTracker {
    pub fn programmatic_move(&mut self) {
        self.previous = None;
    }

    pub fn user_drag(&mut self) {
        self.previous = None;
        self.dragged_since_save = true;
    }

    pub fn observe(
        &mut self,
        rect: (i32, i32, i32, i32),
        dragging_now: bool,
        hidden: bool,
    ) -> bool {
        let stable = self.previous.replace(rect) == Some(rect);
        if dragging_now || hidden {
            self.previous = None;
        }
        should_save_bar_position(self.dragged_since_save, stable, dragging_now, hidden)
    }

    pub fn saved(&mut self, rect: (i32, i32, i32, i32)) {
        self.previous = Some(rect);
        self.dragged_since_save = false;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn work(left: i32, top: i32) -> crate::win32::MonitorRect {
        crate::win32::MonitorRect {
            left,
            top,
            right: left + 1200,
            bottom: top + 800,
            is_primary: false,
        }
    }

    #[test]
    fn saved_offsets_fit_or_clamp_each_edge_in_physical_pixels() {
        for area in [work(0, 0), work(-1200, -100), work(48, 40)] {
            for (offset, expected) in [
                ((100, 200), (100, 200)),
                ((-1, 200), (0, 200)),
                ((1101, 200), (1100, 200)),
                ((100, -1), (100, 0)),
                ((100, 751), (100, 750)),
                ((1100, 750), (1100, 750)),
                ((i32::MAX, i32::MIN), (1100, 0)),
            ] {
                let actual = saved_bar_position(offset, area, 100, 50);
                assert_eq!(actual, (area.left + expected.0, area.top + expected.1));
                assert!(bar_rect_inside_any((actual.0, actual.1, 100, 50), &[area]));
            }
            for (width, height) in [(1201, 50), (100, 801), (1201, 801)] {
                assert_eq!(
                    saved_bar_position((100, 200), area, width, height),
                    (area.left, area.top)
                );
            }
        }
    }

    #[test]
    fn containment_requires_the_whole_rect_in_one_work_area() {
        for area in [work(0, 0), work(-1200, -100), work(48, 40)] {
            assert!(bar_rect_inside_any(
                (area.left, area.top, 1200, 800),
                &[area]
            ));
            for (dx, dy, width, height) in [
                (-1, 200, 100, 50),
                (1101, 200, 100, 50),
                (100, -1, 100, 50),
                (100, 751, 100, 50),
                (0, 0, 1201, 50),
                (0, 0, 100, 801),
                (0, 0, 0, 50),
                (0, 0, 100, -1),
            ] {
                assert!(!bar_rect_inside_any(
                    (area.left + dx, area.top + dy, width, height),
                    &[area]
                ));
            }
        }
        let works = [work(-1200, 0), work(0, 0)];
        assert!(bar_rect_inside_any((-1100, 24, 100, 50), &works));
        assert!(!bar_rect_inside_any((-50, 24, 100, 50), &works));
        assert!(!bar_rect_inside_any((0, 24, 100, 50), &[]));
        assert!(!bar_rect_inside_any((i32::MAX, 24, 100, 50), &works));
    }

    #[test]
    fn save_decision_requires_drag_intent_stability_release_and_visibility() {
        for dragged in [false, true] {
            for stable in [false, true] {
                for dragging in [false, true] {
                    for hidden in [false, true] {
                        assert_eq!(
                            should_save_bar_position(dragged, stable, dragging, hidden),
                            (dragged, stable, dragging, hidden) == (true, true, false, false),
                            "dragged={dragged}, stable={stable}, dragging={dragging}, hidden={hidden}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn changed_monitor_prefers_saved_offset_and_clamps_only_when_needed() {
        for area in [work(0, 0), work(-1200, -100), work(48, 40)] {
            let rect = (area.left + 214, area.top + 268, 874, 64);
            for (saved, expected) in [
                (Some((270, 268)), (270, 268)),
                (None, (214, 268)),
                (Some((1100, 780)), (326, 736)),
            ] {
                assert_eq!(
                    bar_position_after_monitor_change(saved, area, rect),
                    (area.left + expected.0, area.top + expected.1)
                );
            }
            assert_eq!(
                bar_position_after_monitor_change(
                    None,
                    area,
                    (area.left + 1100, area.top + 780, 874, 64),
                ),
                (area.left + 326, area.top + 736)
            );
            assert_eq!(
                bar_position_after_monitor_change(Some((270, 268)), area, (0, 0, 1201, 64)),
                (area.left, area.top)
            );
        }
    }

    #[test]
    fn only_settled_user_drags_are_saved_and_failed_saves_can_retry() {
        let mut tracker = BarPositionTracker::default();
        let initial = (100, 24, 100, 50);
        let moved = (200, 124, 100, 50);
        tracker.programmatic_move();
        assert!(!tracker.observe(initial, false, false));
        assert!(!tracker.observe(initial, false, false));
        assert!(!tracker.observe(initial, false, false));
        // A held mouse or an OS move cannot manufacture drag intent.
        assert!(!tracker.observe(moved, true, false));
        assert!(!tracker.observe(moved, false, false));
        assert!(!tracker.observe(moved, false, false));
        tracker.user_drag();
        assert!(!tracker.observe(moved, true, false));
        assert!(!tracker.observe(moved, true, false));
        assert!(!tracker.observe(moved, false, false));
        assert!(tracker.observe(moved, false, false));
        assert!(
            tracker.observe(moved, false, false),
            "failed save can retry"
        );
        tracker.saved(moved);
        assert!(!tracker.observe(moved, false, false));
        tracker.programmatic_move();
        assert!(!tracker.observe(initial, false, false));
        assert!(!tracker.observe(initial, false, false));
        assert!(!tracker.observe(initial, false, false));
        assert!(!tracker.observe(moved, false, false));
        assert!(!tracker.observe(moved, false, false));
        tracker.user_drag();
        assert!(!tracker.observe(moved, false, true));
        assert!(!tracker.observe(moved, false, true));
        assert!(!tracker.observe(moved, false, false));
        assert!(tracker.observe(moved, false, false));
    }

    #[test]
    fn drag_before_first_tick_survives_programmatic_sample_reset() {
        let mut tracker = BarPositionTracker::default();
        let moved = (-1100, 24, 100, 50);
        tracker.programmatic_move();
        tracker.user_drag();
        tracker.programmatic_move();
        assert!(!tracker.observe(moved, false, false));
        assert!(tracker.observe(moved, false, false));
        tracker.saved(moved);
        assert!(!tracker.observe(moved, false, false));
    }

    #[test]
    fn temporary_resolution_clamp_and_restore_never_become_a_user_position() {
        let large = crate::win32::MonitorRect {
            right: 1920,
            bottom: 1080,
            ..work(0, 0)
        };
        let small = crate::win32::MonitorRect {
            right: 800,
            bottom: 600,
            ..work(0, 0)
        };
        let saved = (270, 268);
        let user_rect = (saved.0, saved.1, 874, 64);
        let mut tracker = BarPositionTracker::default();
        tracker.user_drag();
        assert!(!tracker.observe(user_rect, false, false));
        assert!(tracker.observe(user_rect, false, false));
        tracker.saved(user_rect);
        let clamped = bar_position_after_monitor_change(None, small, (270, 268, 586, 64));
        assert_eq!(clamped, (214, 268));
        tracker.programmatic_move();
        let small_rect = (clamped.0, clamped.1, 586, 64);
        assert!(!tracker.observe(small_rect, false, false));
        assert!(!tracker.observe(small_rect, false, false));
        let restored =
            bar_position_after_monitor_change(Some(saved), large, (clamped.0, clamped.1, 874, 64));
        assert_eq!(restored, saved);
        tracker.programmatic_move();
        assert!(!tracker.observe(user_rect, false, false));
        assert!(!tracker.observe(user_rect, false, false));
        // Even an unreported OS move after restore must not overwrite the entry.
        let os_rect = (214, 268, 874, 64);
        assert!(!tracker.observe(os_rect, false, false));
        assert!(!tracker.observe(os_rect, false, false));
    }

    const PAD: f32 = 8.0;
    const BADGE: f32 = 48.0;
    const GAP: f32 = 6.0;
    const SPRING: f32 = 8.0;
    const FULL_STATE: f32 = 70.0 + 26.0 + 26.0 + 2.0 * 2.0 + 2.0 * 2.0;
    const FULL_VOICE: f32 = 102.0 + 108.0 + 2.0 + 2.0 * 2.0;
    const FULL_SESSION: f32 = 82.0 + 26.0 + 2.0 + 2.0 * 2.0;
    const FULL_TILES: f32 = 96.0 + 92.0 + 22.0 + 26.0 + 3.0 * 2.0 + 2.0 * 2.0;
    const FULL_WINDOW: f32 = 26.0 + 26.0 + 2.0 + 2.0 * 2.0;
    const COMPACT_STATE: f32 = 26.0 + 2.0 * 2.0;
    const COMPACT_VOICE: f32 = FULL_VOICE;
    const COMPACT_SESSION: f32 = FULL_SESSION;
    const COMPACT_TILES: f32 = 26.0 + 2.0 * 2.0;
    const COMPACT_WINDOW: f32 = 26.0 + 26.0 + 26.0 + 2.0 * 2.0 + 2.0 * 2.0;
    const MINI_VOICE: f32 = 34.0 + 34.0 + 2.0 + 2.0 * 2.0;
    const MINI_SESSION: f32 = FULL_SESSION;
    const MINI_TILES: f32 = 26.0 + 26.0 + 2.0 + 2.0 * 2.0;
    const MINI_WINDOW: f32 = FULL_WINDOW;
    const FULL: f32 = BADGE
        + 3.0 * PAD
        + FULL_STATE
        + FULL_VOICE
        + FULL_SESSION
        + FULL_TILES
        + FULL_WINDOW
        + 5.0 * GAP
        + SPRING;
    const COMPACT: f32 = BADGE
        + 3.0 * PAD
        + COMPACT_STATE
        + COMPACT_VOICE
        + COMPACT_SESSION
        + COMPACT_TILES
        + COMPACT_WINDOW
        + 5.0 * GAP
        + SPRING;
    const MINI: f32 = BADGE
        + 3.0 * PAD
        + MINI_VOICE
        + MINI_SESSION
        + MINI_TILES
        + MINI_WINDOW
        + 4.0 * GAP
        + SPRING;
    const MARGIN: f32 = 20.0;

    fn choose(compact: bool, width: f32) -> BarForm {
        effective_form(compact, width, FULL, COMPACT, MINI, MARGIN)
    }

    #[test]
    fn monitor_width_matrix_for_both_preferences() {
        for (width, expanded, compact) in [
            (1920.0, BarForm::Full, BarForm::Compact),
            (1366.0, BarForm::Full, BarForm::Compact),
            (1280.0, BarForm::Full, BarForm::Compact),
            (1097.0, BarForm::Full, BarForm::Compact),
            (1024.0, BarForm::Full, BarForm::Compact),
            (960.0, BarForm::Full, BarForm::Compact),
            (800.0, BarForm::Compact, BarForm::Compact),
            (683.0, BarForm::Compact, BarForm::Compact),
            (480.0, BarForm::Mini, BarForm::Mini),
            (400.0, BarForm::TooNarrow, BarForm::TooNarrow),
        ] {
            assert_eq!(choose(false, width), expanded, "full preference at {width}");
            assert_eq!(
                choose(true, width),
                compact,
                "compact preference at {width}"
            );
        }
        assert_eq!(choose(false, 1920.0 / 1.75), BarForm::Full);
    }

    #[test]
    fn exact_fit_and_one_pixel_below_each_boundary() {
        for preference in [false, true] {
            for (width, at, below) in [
                (
                    FULL,
                    if preference {
                        BarForm::Compact
                    } else {
                        BarForm::Full
                    },
                    BarForm::Compact,
                ),
                (COMPACT, BarForm::Compact, BarForm::Mini),
                (MINI, BarForm::Mini, BarForm::TooNarrow),
            ] {
                let boundary = width + 2.0 * MARGIN;
                assert_eq!(choose(preference, boundary), at);
                assert_eq!(choose(preference, boundary - 1.0), below);
            }
        }
    }

    // BarSpec uses only sums, products and earlier property names. Evaluate that
    // small grammar rather than embedding a second literal total in the guard.
    fn spec_value(source: &str, name: &str) -> f32 {
        let expression = source
            .split_once(&format!("out property <length> {name}:"))
            .unwrap_or_else(|| panic!("missing BarSpec property {name}"))
            .1
            .split_once(';')
            .expect("terminated BarSpec expression")
            .0;
        expression
            .split('+')
            .map(|term| {
                term.split('*')
                    .map(|factor| {
                        let factor = factor.trim().trim_end_matches("px");
                        factor
                            .parse::<f32>()
                            .unwrap_or_else(|_| spec_value(source, factor))
                    })
                    .product::<f32>()
            })
            .sum()
    }

    #[test]
    fn named_constants_match_every_spec_region_and_computed_total() {
        let spec = include_str!("../ui/bar_spec.slint");
        for (name, value) in [
            ("pad", PAD),
            ("badge", BADGE),
            ("gap", GAP),
            ("spring", SPRING),
            ("full-state", FULL_STATE),
            ("full-voice", FULL_VOICE),
            ("full-session", FULL_SESSION),
            ("full-tiles", FULL_TILES),
            ("full-window", FULL_WINDOW),
            ("compact-state", COMPACT_STATE),
            ("compact-voice", COMPACT_VOICE),
            ("compact-session", COMPACT_SESSION),
            ("compact-tiles", COMPACT_TILES),
            ("compact-window", COMPACT_WINDOW),
            ("mini-voice", MINI_VOICE),
            ("mini-session", MINI_SESSION),
            ("mini-tiles", MINI_TILES),
            ("mini-window", MINI_WINDOW),
            ("full-width", FULL),
            ("compact-width", COMPACT),
            ("mini-width", MINI),
        ] {
            assert_eq!(spec_value(spec, name), value, "BarSpec drift: {name}");
        }
    }
}
