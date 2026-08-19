//! Pure macOS capture-liveness policy used by the canonical shared host.

#[cfg(target_os = "macos")]
pub(super) const TICK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
const STAGNANT_TICKS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    Idle,
    Stop,
}

#[derive(Debug, Default, Clone, Copy)]
struct StreamWatch {
    expected: bool,
    baseline: u64,
    stagnant_ticks: u32,
}

impl StreamWatch {
    fn observe(&mut self, emitted: u64) {
        if !self.expected {
            if emitted != 0 {
                self.expected = true;
                self.baseline = emitted;
            }
            return;
        }
        if emitted == self.baseline {
            self.stagnant_ticks = self.stagnant_ticks.saturating_add(1);
        } else {
            self.baseline = emitted;
            self.stagnant_ticks = 0;
        }
    }

    fn stagnant(self) -> bool {
        self.expected && self.stagnant_ticks >= STAGNANT_TICKS
    }
}

/// One-shot fail-safe state. A stream becomes expected only after it has
/// produced a chunk, so missing TCC permission cannot stop a never-started
/// capture. Once a live stream stalls, the owner must finalize the session and
/// wait for an explicit user Start.
#[derive(Debug, Default)]
pub(super) struct CaptureWatchdog {
    mic: StreamWatch,
    system: StreamWatch,
    stop_requested: bool,
}

impl CaptureWatchdog {
    pub(super) fn tick(
        &mut self,
        session_intended: bool,
        snapshot: Option<(u64, u64)>,
    ) -> Decision {
        if !session_intended {
            self.reset();
            return Decision::Idle;
        }
        if self.stop_requested {
            return Decision::Idle;
        }

        let Some((mic_emitted, system_emitted)) = snapshot else {
            return if self.mic.expected || self.system.expected {
                self.request_stop()
            } else {
                Decision::Idle
            };
        };
        self.mic.observe(mic_emitted);
        self.system.observe(system_emitted);
        if self.mic.stagnant() || self.system.stagnant() {
            self.request_stop()
        } else {
            Decision::Idle
        }
    }

    fn request_stop(&mut self) -> Decision {
        self.stop_requested = true;
        Decision::Stop
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;

    fn stall(watchdog: &mut CaptureWatchdog, snapshot: (u64, u64)) -> Decision {
        let mut decision = Decision::Idle;
        for _ in 0..=STAGNANT_TICKS {
            decision = watchdog.tick(true, Some(snapshot));
            if decision == Decision::Stop {
                break;
            }
        }
        decision
    }

    #[test]
    fn flowing_stream_then_stall_requests_one_stop() {
        let mut watchdog = CaptureWatchdog::default();
        assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
        assert_eq!(stall(&mut watchdog, (1, 0)), Decision::Stop);
        assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
    }

    #[test]
    fn disappeared_flowing_capture_requests_one_stop() {
        let mut watchdog = CaptureWatchdog::default();
        assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
        assert_eq!(watchdog.tick(true, None), Decision::Stop);
        assert_eq!(watchdog.tick(true, None), Decision::Idle);
    }

    #[test]
    fn never_flowed_streams_do_not_stop() {
        let mut watchdog = CaptureWatchdog::default();
        for _ in 0..STAGNANT_TICKS * 3 {
            assert_eq!(watchdog.tick(true, Some((0, 0))), Decision::Idle);
            assert_eq!(watchdog.tick(true, None), Decision::Idle);
        }
    }

    #[test]
    fn progress_clears_a_partial_stall() {
        let mut watchdog = CaptureWatchdog::default();
        assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
        for _ in 0..STAGNANT_TICKS - 1 {
            assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
        }
        assert_eq!(watchdog.tick(true, Some((2, 0))), Decision::Idle);
        assert_eq!(watchdog.tick(true, Some((2, 0))), Decision::Idle);
    }

    #[test]
    fn intentional_stop_rearms_the_next_session() {
        let mut watchdog = CaptureWatchdog::default();
        assert_eq!(watchdog.tick(true, Some((1, 0))), Decision::Idle);
        assert_eq!(stall(&mut watchdog, (1, 0)), Decision::Stop);
        assert_eq!(watchdog.tick(false, None), Decision::Idle);
        assert_eq!(watchdog.tick(true, Some((2, 0))), Decision::Idle);
        assert_eq!(stall(&mut watchdog, (2, 0)), Decision::Stop);
    }
}
