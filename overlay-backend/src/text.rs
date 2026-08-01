//! Small shared text helpers used by both the backend prompt builders
//! and the UI crates.

/// Flatten every run of whitespace (including `\n` / `\t`) in `s` to a
/// single space and trim both ends. Whitespace-only input yields `""`.
#[must_use]
pub fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn collapse_ws_flattens_runs_and_trims() {
        assert_eq!(collapse_ws("  a \t b\n\nc  "), "a b c");
        assert_eq!(collapse_ws("one-line"), "one-line");
        assert_eq!(collapse_ws("   "), "");
        assert_eq!(collapse_ws(""), "");
    }
}
