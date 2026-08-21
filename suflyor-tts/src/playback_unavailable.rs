//! Honest compile seam for platforms whose playback transport is not ported.

use anyhow::Result;

const UNSUPPORTED: &str = "neural speech playback is not supported on this platform";

/// Matches the sidecar's internal playback API without pretending audio ran.
pub struct Playback {
    _private: (),
}

impl Playback {
    pub fn start(_sample_rate: u32, _on_exit: Option<Box<dyn FnOnce() + Send>>) -> Result<Self> {
        anyhow::bail!(UNSUPPORTED)
    }

    pub fn feed(&self, _samples: Vec<f32>) {}

    pub fn end_of_stream(&self) {}

    pub fn pause(&self) {}

    pub fn resume(&self) {}

    pub fn seek_seconds(&self, _seconds: i32) {}

    pub fn set_speed(&self, _speed: f32) {}

    pub fn stop(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn start_fails_without_invoking_exit_callback() {
        let called = Arc::new(AtomicBool::new(false));
        let callback_called = Arc::clone(&called);
        let result = Playback::start(
            16_000,
            Some(Box::new(move || {
                callback_called.store(true, Ordering::Relaxed);
            })),
        );

        assert!(result.is_err());
        assert!(!called.load(Ordering::Relaxed));
    }
}
