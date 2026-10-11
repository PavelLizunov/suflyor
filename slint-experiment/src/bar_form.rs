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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

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
    const FULL: f32 = BADGE + 3.0 * PAD + FULL_STATE + FULL_VOICE + FULL_SESSION
        + FULL_TILES + FULL_WINDOW + 5.0 * GAP + SPRING;
    const COMPACT: f32 = BADGE + 3.0 * PAD + COMPACT_STATE + COMPACT_VOICE
        + COMPACT_SESSION + COMPACT_TILES + COMPACT_WINDOW + 5.0 * GAP + SPRING;
    const MINI: f32 = BADGE + 3.0 * PAD + MINI_VOICE + MINI_SESSION + MINI_TILES
        + MINI_WINDOW + 4.0 * GAP + SPRING;
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
            assert_eq!(choose(true, width), compact, "compact preference at {width}");
        }
        assert_eq!(choose(false, 1920.0 / 1.75), BarForm::Full);
    }

    #[test]
    fn exact_fit_and_one_pixel_below_each_boundary() {
        for preference in [false, true] {
            for (width, at, below) in [
                (FULL, if preference { BarForm::Compact } else { BarForm::Full }, BarForm::Compact),
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
        expression.split('+').map(|term| {
            term.split('*').map(|factor| {
                let factor = factor.trim().trim_end_matches("px");
                factor.parse::<f32>().unwrap_or_else(|_| spec_value(source, factor))
            }).product::<f32>()
        }).sum()
    }

    #[test]
    fn named_constants_match_every_spec_region_and_computed_total() {
        let spec = include_str!("../ui/bar_spec.slint");
        for (name, value) in [
            ("pad", PAD), ("badge", BADGE), ("gap", GAP), ("spring", SPRING),
            ("full-state", FULL_STATE), ("full-voice", FULL_VOICE),
            ("full-session", FULL_SESSION), ("full-tiles", FULL_TILES),
            ("full-window", FULL_WINDOW), ("compact-state", COMPACT_STATE),
            ("compact-voice", COMPACT_VOICE), ("compact-session", COMPACT_SESSION),
            ("compact-tiles", COMPACT_TILES), ("compact-window", COMPACT_WINDOW),
            ("mini-voice", MINI_VOICE), ("mini-session", MINI_SESSION),
            ("mini-tiles", MINI_TILES), ("mini-window", MINI_WINDOW),
            ("full-width", FULL), ("compact-width", COMPACT), ("mini-width", MINI),
        ] {
            assert_eq!(spec_value(spec, name), value, "BarSpec drift: {name}");
        }
    }
}
