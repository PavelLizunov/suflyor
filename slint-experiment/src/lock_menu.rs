//! Per-open focus state for the native lock-mode drop-down.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Phase {
    #[default]
    Closed,
    Opening,
    Arming,
    Open,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusTransition {
    None,
    Acquired,
    IgnoreInitialLoss,
    DismissOutside,
}

#[derive(Debug, Default)]
pub struct FocusState {
    generation: u64,
    phase: Phase,
    suppress_next_open: bool,
}

impl FocusState {
    pub fn begin_open(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.phase = Phase::Opening;
        self.generation
    }

    pub fn close(&mut self) {
        self.phase = Phase::Closed;
    }

    pub fn is_current_open(&self, generation: u64) -> bool {
        self.generation == generation && self.phase != Phase::Closed
    }

    pub fn mark_revealed(&mut self, generation: u64) -> bool {
        if self.generation == generation && self.phase == Phase::Opening {
            self.phase = Phase::Arming;
            true
        } else {
            false
        }
    }

    /// Arm outside-focus dismissal after the reveal callback returns and its
    /// native hide/restyle/show focus noise has drained from the event queue.
    pub fn arm(&mut self, generation: u64) -> bool {
        if self.generation == generation && self.phase == Phase::Arming {
            self.phase = Phase::Open;
            true
        } else {
            false
        }
    }

    pub fn focus_changed(&mut self, focused: bool) -> FocusTransition {
        match (self.phase, focused) {
            (Phase::Opening | Phase::Arming | Phase::Open, true) => FocusTransition::Acquired,
            (Phase::Opening | Phase::Arming, false) => FocusTransition::IgnoreInitialLoss,
            (Phase::Open, false) => {
                self.phase = Phase::Closed;
                FocusTransition::DismissOutside
            }
            _ => FocusTransition::None,
        }
    }

    /// A chip press first transfers native focus to the owner bar, then emits
    /// the Slint click. Swallow that same click so a focus-dismiss cannot reopen
    /// the menu. A short timer clears this for ordinary bar clicks.
    pub fn suppress_next_open(&mut self) {
        self.suppress_next_open = true;
    }

    pub fn consume_suppressed_open(&mut self) -> bool {
        std::mem::take(&mut self.suppress_next_open)
    }

    pub fn clear_suppressed_open(&mut self) {
        self.suppress_next_open = false;
    }

    pub fn diagnostic_snapshot(&self) -> (&'static str, u64) {
        let phase = match self.phase {
            Phase::Closed => "closed",
            Phase::Opening => "opening",
            Phase::Arming => "arming",
            Phase::Open => "open",
        };
        (phase, self.generation)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{FocusState, FocusTransition};

    fn open_and_arm(state: &mut FocusState) -> u64 {
        let generation = state.begin_open();
        assert!(state.mark_revealed(generation));
        assert!(state.arm(generation));
        generation
    }

    #[test]
    fn initial_focus_noise_cannot_dismiss_but_outside_focus_loss_does() {
        let mut state = FocusState::default();
        let generation = state.begin_open();

        assert_eq!(state.focus_changed(true), FocusTransition::Acquired);
        assert_eq!(
            state.focus_changed(false),
            FocusTransition::IgnoreInitialLoss
        );
        assert!(state.mark_revealed(generation));
        assert_eq!(
            state.focus_changed(false),
            FocusTransition::IgnoreInitialLoss
        );
        assert!(state.arm(generation));
        assert_eq!(state.focus_changed(false), FocusTransition::DismissOutside);
        assert!(!state.is_current_open(generation));
    }

    #[test]
    fn second_open_arms_even_without_a_second_focus_gain() {
        let mut state = FocusState::default();
        open_and_arm(&mut state);
        assert_eq!(state.focus_changed(false), FocusTransition::DismissOutside);

        let second = state.begin_open();
        assert_eq!(state.focus_changed(true), FocusTransition::Acquired);
        assert!(state.mark_revealed(second));
        assert!(state.arm(second));
        assert_eq!(state.focus_changed(false), FocusTransition::DismissOutside);
    }

    #[test]
    fn owner_focus_loss_swallow_prevents_chip_reopen_race() {
        let mut state = FocusState::default();
        open_and_arm(&mut state);
        assert_eq!(state.focus_changed(false), FocusTransition::DismissOutside);
        state.suppress_next_open();
        assert!(state.consume_suppressed_open());
        assert!(!state.consume_suppressed_open());
    }

    #[test]
    fn stale_reveal_cannot_arm_a_new_open() {
        let mut state = FocusState::default();
        let first = state.begin_open();
        let second = state.begin_open();
        assert!(!state.mark_revealed(first));
        assert!(state.mark_revealed(second));
        assert!(!state.arm(first));
        assert!(state.arm(second));
    }
}
