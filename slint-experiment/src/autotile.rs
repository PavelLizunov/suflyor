//! Three-position UI mapping onto the existing automatic-tile config flags.

#[must_use]
pub const fn mode_from_flags(suppress: bool, every_line: bool) -> i32 {
    if suppress {
        0
    } else if every_line {
        2
    } else {
        1
    }
}

#[must_use]
pub const fn flags_from_mode(mode: i32) -> Option<(bool, bool)> {
    match mode {
        0 => Some((true, false)),
        1 => Some((false, false)),
        2 => Some((false, true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn modes_round_trip_and_suppression_wins_over_legacy_every_line() {
        for (mode, flags) in [(0, (true, false)), (1, (false, false)), (2, (false, true))] {
            assert_eq!(flags_from_mode(mode), Some(flags));
            assert_eq!(mode_from_flags(flags.0, flags.1), mode);
        }
        assert_eq!(mode_from_flags(true, true), 0);
        assert_eq!(flags_from_mode(mode_from_flags(true, true)), Some((true, false)));
        assert_eq!(flags_from_mode(-1), None);
        assert_eq!(flags_from_mode(3), None);
    }
}
