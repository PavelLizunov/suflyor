//! Native Windows endpoint notifications and the pure recovery policy.

use anyhow::{Context, Result};
use std::sync::mpsc;
use wasapi::Direction;
use windows::core::{implement, PCWSTR};
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient,
    IMMNotificationClient_Impl, MMDeviceEnumerator, PKEY_AudioEngine_DeviceFormat,
    DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeviceSelection {
    FollowDefault,
    Pinned(String),
}

impl DeviceSelection {
    pub(crate) fn from_configured(name: Option<String>) -> Self {
        match name {
            Some(name) if !name.trim().is_empty() => Self::Pinned(name),
            _ => Self::FollowDefault,
        }
    }

    pub(crate) fn configured_name(&self) -> Option<&str> {
        match self {
            Self::FollowDefault => None,
            Self::Pinned(name) => Some(name),
        }
    }

    pub(crate) fn mode_label(&self) -> &'static str {
        match self {
            Self::FollowDefault => "follow_default",
            Self::Pinned(_) => "pinned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioFlow {
    Render,
    Capture,
    Other,
}

impl AudioFlow {
    pub(crate) fn from_direction(direction: Direction) -> Self {
        match direction {
            Direction::Render => Self::Render,
            Direction::Capture => Self::Capture,
        }
    }

    fn from_windows(flow: EDataFlow) -> Self {
        if flow == eRender {
            Self::Render
        } else if flow == eCapture {
            Self::Capture
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteRole {
    Console,
    Other,
}

impl RouteRole {
    fn from_windows(role: ERole) -> Self {
        if role == eConsole {
            Self::Console
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RouteNotification {
    DeviceStateChanged {
        endpoint_id: String,
        is_active: bool,
    },
    DeviceRemoved {
        endpoint_id: String,
    },
    DefaultDeviceChanged {
        flow: AudioFlow,
        role: RouteRole,
        endpoint_id: Option<String>,
    },
    DeviceFormatChanged {
        endpoint_id: String,
    },
}

impl RouteNotification {
    pub(crate) fn kind_label(&self) -> &'static str {
        match self {
            Self::DeviceStateChanged { .. } => "device_state",
            Self::DeviceRemoved { .. } => "device_removed",
            Self::DefaultDeviceChanged { .. } => "default_device",
            Self::DeviceFormatChanged { .. } => "device_format",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryReason {
    DeviceStateChanged,
    DeviceRemoved,
    DefaultDeviceChanged,
    DeviceFormatChanged,
}

impl RecoveryReason {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::DeviceStateChanged => "device_state_changed",
            Self::DeviceRemoved => "device_removed",
            Self::DefaultDeviceChanged => "default_device_changed",
            Self::DeviceFormatChanged => "device_format_changed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryDecision {
    Recover(RecoveryReason),
    IgnorePinnedDefaultChange,
    Unrelated,
}

pub(crate) struct RecoveryPolicy<'a> {
    selection: &'a DeviceSelection,
    flow: AudioFlow,
    endpoint_id: &'a str,
}

impl<'a> RecoveryPolicy<'a> {
    pub(crate) fn new(
        selection: &'a DeviceSelection,
        direction: Direction,
        endpoint_id: &'a str,
    ) -> Self {
        Self {
            selection,
            flow: AudioFlow::from_direction(direction),
            endpoint_id,
        }
    }

    pub(crate) fn decide(&self, notification: &RouteNotification) -> RecoveryDecision {
        let current = |id: &str| id.eq_ignore_ascii_case(self.endpoint_id);
        match notification {
            RouteNotification::DeviceStateChanged {
                endpoint_id,
                is_active: false,
            } if current(endpoint_id) => {
                RecoveryDecision::Recover(RecoveryReason::DeviceStateChanged)
            }
            RouteNotification::DeviceRemoved { endpoint_id } if current(endpoint_id) => {
                RecoveryDecision::Recover(RecoveryReason::DeviceRemoved)
            }
            RouteNotification::DeviceFormatChanged { endpoint_id } if current(endpoint_id) => {
                RecoveryDecision::Recover(RecoveryReason::DeviceFormatChanged)
            }
            RouteNotification::DefaultDeviceChanged {
                flow,
                role,
                endpoint_id: Some(endpoint_id),
            } if *flow == self.flow && *role == RouteRole::Console && !current(endpoint_id) => {
                match self.selection {
                    DeviceSelection::FollowDefault => {
                        RecoveryDecision::Recover(RecoveryReason::DefaultDeviceChanged)
                    }
                    DeviceSelection::Pinned(_) => RecoveryDecision::IgnorePinnedDefaultChange,
                }
            }
            _ => RecoveryDecision::Unrelated,
        }
    }
}

#[implement(IMMNotificationClient)]
struct EndpointNotificationClient {
    tx: mpsc::Sender<RouteNotification>,
}

#[allow(non_snake_case)]
impl IMMNotificationClient_Impl for EndpointNotificationClient_Impl {
    fn OnDeviceStateChanged(
        &self,
        endpoint_id: &PCWSTR,
        new_state: windows::Win32::Media::Audio::DEVICE_STATE,
    ) -> windows::core::Result<()> {
        if let Some(endpoint_id) = endpoint_id_string(endpoint_id) {
            let _ = self.tx.send(RouteNotification::DeviceStateChanged {
                endpoint_id,
                is_active: new_state == DEVICE_STATE_ACTIVE,
            });
        }
        Ok(())
    }

    fn OnDeviceAdded(&self, _endpoint_id: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, endpoint_id: &PCWSTR) -> windows::core::Result<()> {
        if let Some(endpoint_id) = endpoint_id_string(endpoint_id) {
            let _ = self
                .tx
                .send(RouteNotification::DeviceRemoved { endpoint_id });
        }
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        role: ERole,
        endpoint_id: &PCWSTR,
    ) -> windows::core::Result<()> {
        let _ = self.tx.send(RouteNotification::DefaultDeviceChanged {
            flow: AudioFlow::from_windows(flow),
            role: RouteRole::from_windows(role),
            endpoint_id: endpoint_id_string(endpoint_id),
        });
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        endpoint_id: &PCWSTR,
        key: &windows::Win32::Foundation::PROPERTYKEY,
    ) -> windows::core::Result<()> {
        if *key == PKEY_AudioEngine_DeviceFormat {
            if let Some(endpoint_id) = endpoint_id_string(endpoint_id) {
                let _ = self
                    .tx
                    .send(RouteNotification::DeviceFormatChanged { endpoint_id });
            }
        }
        Ok(())
    }
}

fn endpoint_id_string(endpoint_id: &PCWSTR) -> Option<String> {
    if endpoint_id.is_null() {
        return None;
    }
    // SAFETY: Windows owns this null-terminated string for the duration of the
    // callback. Copy it before returning from the COM call.
    unsafe { endpoint_id.to_string().ok() }
}

pub(crate) struct RouteWatcher {
    enumerator: IMMDeviceEnumerator,
    callback: IMMNotificationClient,
}

impl RouteWatcher {
    pub(crate) fn register() -> Result<(Self, mpsc::Receiver<RouteNotification>)> {
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
                .context("create MMDeviceEnumerator")?;
        let (tx, rx) = mpsc::channel();
        let callback: IMMNotificationClient = EndpointNotificationClient { tx }.into();
        unsafe { enumerator.RegisterEndpointNotificationCallback(&callback) }
            .context("register endpoint notification callback")?;
        Ok((
            Self {
                enumerator,
                callback,
            },
            rx,
        ))
    }
}

impl Drop for RouteWatcher {
    fn drop(&mut self) {
        let _ = unsafe {
            self.enumerator
                .UnregisterEndpointNotificationCallback(&self.callback)
        };
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn policy<'a>(selection: &'a DeviceSelection) -> RecoveryPolicy<'a> {
        RecoveryPolicy::new(selection, Direction::Render, "CURRENT")
    }

    #[test]
    fn configured_device_mode_is_explicit() {
        assert_eq!(
            DeviceSelection::from_configured(None),
            DeviceSelection::FollowDefault
        );
        assert_eq!(
            DeviceSelection::from_configured(Some("   ".into())),
            DeviceSelection::FollowDefault
        );
        assert_eq!(
            DeviceSelection::from_configured(Some("A50 Voice".into())),
            DeviceSelection::Pinned("A50 Voice".into())
        );
    }

    #[test]
    fn default_follower_reopens_only_for_matching_console_default() {
        let selection = DeviceSelection::FollowDefault;
        let policy = policy(&selection);
        assert_eq!(
            policy.decide(&RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Render,
                role: RouteRole::Console,
                endpoint_id: Some("NEW".into()),
            }),
            RecoveryDecision::Recover(RecoveryReason::DefaultDeviceChanged)
        );
        for notification in [
            RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Capture,
                role: RouteRole::Console,
                endpoint_id: Some("NEW".into()),
            },
            RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Render,
                role: RouteRole::Other,
                endpoint_id: Some("NEW".into()),
            },
            RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Render,
                role: RouteRole::Console,
                endpoint_id: None,
            },
            RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Render,
                role: RouteRole::Console,
                endpoint_id: Some("current".into()),
            },
        ] {
            assert_eq!(policy.decide(&notification), RecoveryDecision::Unrelated);
        }
    }

    #[test]
    fn pinned_device_never_follows_default_change() {
        let selection = DeviceSelection::Pinned("A50 Voice".into());
        assert_eq!(
            policy(&selection).decide(&RouteNotification::DefaultDeviceChanged {
                flow: AudioFlow::Render,
                role: RouteRole::Console,
                endpoint_id: Some("NEW".into()),
            }),
            RecoveryDecision::IgnorePinnedDefaultChange
        );
    }

    #[test]
    fn current_endpoint_topology_and_format_changes_reopen() {
        for selection in [
            DeviceSelection::FollowDefault,
            DeviceSelection::Pinned("A50 Voice".into()),
        ] {
            let policy = policy(&selection);
            for (notification, reason) in [
                (
                    RouteNotification::DeviceStateChanged {
                        endpoint_id: "current".into(),
                        is_active: false,
                    },
                    RecoveryReason::DeviceStateChanged,
                ),
                (
                    RouteNotification::DeviceRemoved {
                        endpoint_id: "CURRENT".into(),
                    },
                    RecoveryReason::DeviceRemoved,
                ),
                (
                    RouteNotification::DeviceFormatChanged {
                        endpoint_id: "Current".into(),
                    },
                    RecoveryReason::DeviceFormatChanged,
                ),
            ] {
                assert_eq!(
                    policy.decide(&notification),
                    RecoveryDecision::Recover(reason)
                );
            }
        }
    }

    #[test]
    fn unrelated_endpoint_never_reopens() {
        let selection = DeviceSelection::FollowDefault;
        let policy = policy(&selection);
        for notification in [
            RouteNotification::DeviceStateChanged {
                endpoint_id: "OTHER".into(),
                is_active: false,
            },
            RouteNotification::DeviceStateChanged {
                endpoint_id: "CURRENT".into(),
                is_active: true,
            },
            RouteNotification::DeviceRemoved {
                endpoint_id: "OTHER".into(),
            },
            RouteNotification::DeviceFormatChanged {
                endpoint_id: "OTHER".into(),
            },
        ] {
            assert_eq!(policy.decide(&notification), RecoveryDecision::Unrelated);
        }
    }
}
