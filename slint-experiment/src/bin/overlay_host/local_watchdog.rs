use std::time::{Duration, Instant};

/// Local-AI watchdog: how often to confirm llama-server is still answering on
/// :8080. llama binds the HTTP port early (returns 503 while the model loads),
/// so a still-unreachable port at the next tick means the process is genuinely
/// dead, not slow-loading.
pub(super) const WATCHDOG_SECS: u64 = 15;
/// Minimum gap between two auto-(re)start attempts, so a server that can't
/// start (missing model/binary) isn't hammered. A healthy server resets this.
pub(super) const WATCHDOG_COOLDOWN_SECS: u64 = 30;
/// Stop auto-retrying after this many consecutive failed (re)starts so a
/// genuinely broken install doesn't spawn forever; a reachable server (e.g.
/// after a manual Install) re-arms the counter to 0.
pub(super) const WATCHDOG_MAX_FAILS: u32 = 6;

/// The local-AI watchdog's pure decision state (cooldown timestamp + the
/// consecutive-failure cap), extracted from the live loop so the safety-critical
/// retry policy is unit-testable — the loop itself interleaves this with network
/// probes, the lifecycle lock, and process spawns, none of which a test can
/// reach (audit B1). Time is passed in as an `Instant` so tests can offset it.
#[derive(Default)]
pub(super) struct WatchdogState {
    last_attempt: Option<Instant>,
    pub(super) consecutive_fails: u32,
}

impl WatchdogState {
    /// Whether to attempt a (re)start NOW. The caller has already confirmed the
    /// server is unreachable and that local AI is wanted. True iff the cooldown
    /// has elapsed since the last attempt (or there was none) AND we're still
    /// under the consecutive-failure cap.
    pub(super) fn should_restart(
        &self,
        now: Instant,
        cooldown: Duration,
        max_fails: u32,
    ) -> bool {
        let cooled = self
            .last_attempt
            .is_none_or(|t| now.duration_since(t) >= cooldown);
        cooled && self.consecutive_fails < max_fails
    }

    /// The server answered — re-arm the cap so any future crash retries fresh.
    pub(super) fn note_reachable(&mut self) {
        self.consecutive_fails = 0;
    }

    /// Record a (re)start attempt at `now`: a confirmed `Switched` resets the
    /// fail count, anything else (PortBusy / FailedToStart) increments it.
    pub(super) fn note_attempt(&mut self, now: Instant, switched: bool) {
        self.last_attempt = Some(now);
        if switched {
            self.consecutive_fails = 0;
        } else {
            self.consecutive_fails += 1;
        }
    }
}

#[cfg(test)]
mod watchdog_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts
    use super::WatchdogState;
    use std::time::{Duration, Instant};

    const COOLDOWN: Duration = Duration::from_secs(30);
    const MAX: u32 = 6;

    #[test]
    fn first_attempt_allowed_immediately() {
        // No prior attempt → cooled, under cap → attempt.
        assert!(WatchdogState::default().should_restart(Instant::now(), COOLDOWN, MAX));
    }

    #[test]
    fn within_cooldown_skips_then_attempts_after() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        wd.note_attempt(t0, false);
        assert!(
            !wd.should_restart(t0 + Duration::from_secs(10), COOLDOWN, MAX),
            "10s < 30s cooldown → skip"
        );
        assert!(
            wd.should_restart(t0 + Duration::from_secs(31), COOLDOWN, MAX),
            "31s ≥ 30s cooldown → attempt"
        );
    }

    #[test]
    fn fail_cap_stops_then_reachable_rearms() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        // MAX cooled failures in a row.
        for i in 0..MAX {
            let now = t0 + Duration::from_secs(31 * u64::from(i + 1));
            assert!(
                wd.should_restart(now, COOLDOWN, MAX),
                "attempt {i} under cap"
            );
            wd.note_attempt(now, false);
        }
        assert!(
            !wd.should_restart(t0 + Duration::from_secs(10_000), COOLDOWN, MAX),
            "hit the fail cap → stop attempting"
        );
        wd.note_reachable(); // server came back on its own
        assert!(
            wd.should_restart(t0 + Duration::from_secs(10_000), COOLDOWN, MAX),
            "a reachable server re-arms the cap"
        );
    }

    #[test]
    fn switched_resets_fail_count() {
        let t0 = Instant::now();
        let mut wd = WatchdogState::default();
        wd.note_attempt(t0, false);
        wd.note_attempt(t0, false);
        wd.note_attempt(t0, true); // a confirmed restart
        assert_eq!(wd.consecutive_fails, 0, "Switched resets the counter");
        assert!(wd.should_restart(t0 + Duration::from_secs(31), COOLDOWN, MAX));
    }
}
