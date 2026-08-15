//! Platform-neutral capability states.
//!
//! Variants are stable machine codes. Native backends map OS states onto them;
//! diagnostics and UI attach sanitised details or translated copy separately.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityState {
    Available,
    NeedsPermission(PermissionKind),
    Denied(PermissionKind),
    RestartRequired(PermissionKind),
    Unsupported(CapabilityReason),
    Degraded(CapabilityReason),
    Unavailable(CapabilityReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    Microphone,
    SystemAudioCapture,
    ScreenRecording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityReason {
    NotSupportedOnPlatform,
    DeviceUnavailable,
    DeviceBusy,
    ComponentUnavailable,
    RequirementsNotMet,
    RuntimeFailure,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn state_category(state: CapabilityState) -> &'static str {
        match state {
            CapabilityState::Available => "available",
            CapabilityState::NeedsPermission(_) => "needs_permission",
            CapabilityState::Denied(_) => "denied",
            CapabilityState::RestartRequired(_) => "restart_required",
            CapabilityState::Unsupported(_) => "unsupported",
            CapabilityState::Degraded(_) => "degraded",
            CapabilityState::Unavailable(_) => "unavailable",
        }
    }

    #[test]
    fn every_state_has_a_distinct_category() {
        let states = [
            (CapabilityState::Available, "available"),
            (
                CapabilityState::NeedsPermission(PermissionKind::Microphone),
                "needs_permission",
            ),
            (
                CapabilityState::Denied(PermissionKind::SystemAudioCapture),
                "denied",
            ),
            (
                CapabilityState::RestartRequired(PermissionKind::ScreenRecording),
                "restart_required",
            ),
            (
                CapabilityState::Unsupported(CapabilityReason::NotSupportedOnPlatform),
                "unsupported",
            ),
            (
                CapabilityState::Degraded(CapabilityReason::DeviceBusy),
                "degraded",
            ),
            (
                CapabilityState::Unavailable(CapabilityReason::RuntimeFailure),
                "unavailable",
            ),
        ];

        for (state, expected) in states {
            assert_eq!(state_category(state), expected);
        }
    }

    #[test]
    fn payloads_remain_part_of_state_equality() {
        assert_ne!(
            CapabilityState::Denied(PermissionKind::Microphone),
            CapabilityState::Denied(PermissionKind::ScreenRecording)
        );
        assert_ne!(
            CapabilityState::Unavailable(CapabilityReason::DeviceUnavailable),
            CapabilityState::Unavailable(CapabilityReason::ComponentUnavailable)
        );
        assert_ne!(
            CapabilityState::Degraded(CapabilityReason::DeviceBusy),
            CapabilityState::Degraded(CapabilityReason::RequirementsNotMet)
        );
    }
}
