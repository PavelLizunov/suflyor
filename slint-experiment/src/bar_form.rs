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

/// Two equal samples after a change; programmatic moves establish a baseline
/// without becoming user preferences. Failed saves remain eligible for retry.
#[derive(Default)]
pub struct BarPositionTracker {
    previous: Option<(i32, i32, i32, i32)>,
    settled: Option<(i32, i32, i32, i32)>,
    suppress: bool,
}

impl BarPositionTracker {
    pub fn programmatic_move(&mut self) {
        self.previous = None;
        self.suppress = true;
    }

    pub fn user_drag(&mut self) {
        self.previous = None;
        self.suppress = false;
    }

    pub fn observe(&mut self, rect: (i32, i32, i32, i32), dragging: bool) -> bool {
        if dragging {
            self.user_drag();
            return false;
        }
        if self.previous.replace(rect) != Some(rect) {
            return false;
        }
        if self.suppress {
            self.settled = Some(rect);
            self.suppress = false;
            return false;
        }
        self.settled != Some(rect)
    }

    pub fn saved(&mut self, rect: (i32, i32, i32, i32)) {
        self.settled = Some(rect);
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
    fn only_changed_settled_user_rects_are_saved() {
        let mut tracker = BarPositionTracker::default();
        let initial = (100, 24, 100, 50);
        let moved = (200, 124, 100, 50);
        tracker.programmatic_move();
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(moved, true));
        assert!(!tracker.observe(moved, true));
        assert!(!tracker.observe(moved, false));
        assert!(tracker.observe(moved, false));
        assert!(tracker.observe(moved, false), "failed save can retry");
        tracker.saved(moved);
        assert!(!tracker.observe(moved, false));
        tracker.programmatic_move();
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(initial, false));
        assert!(!tracker.observe(moved, false));
        assert!(tracker.observe(moved, false));
    }

    #[test]
    fn drag_before_first_tick_is_not_lost_to_startup_suppression() {
        let mut tracker = BarPositionTracker::default();
        let moved = (-1100, 24, 100, 50);
        tracker.programmatic_move();
        tracker.user_drag();
        assert!(!tracker.observe(moved, false));
        assert!(tracker.observe(moved, false));
        tracker.saved(moved);
        assert!(!tracker.observe(moved, false));
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
