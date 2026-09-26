---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_03105b92ee6c"
source_path: "overlay-backend/src/audio_route.rs"
batch_id: "B07"
total_lines: 434
symbols_count: 24
review_state: validated
---

# File Map: `overlay-backend/src/audio_route.rs`

- **Batch:** B07
- **Physical Lines:** 434
- **Coverage:** 434/434 lines (100%)

## Types & Structures (9)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `DeviceSelection` | L15 | pub(crate) |
| enum | `AudioFlow` | L44 | pub(crate) |
| enum | `RouteRole` | L70 | pub(crate) |
| enum | `RouteNotification` | L86 | pub(crate) |
| enum | `RecoveryReason` | L116 | pub(crate) |
| enum | `RecoveryDecision` | L135 | pub(crate) |
| struct | `RecoveryPolicy` | L141 | pub(crate) |
| struct | `EndpointNotificationClient` | L193 | private |
| struct | `RouteWatcher` | L265 | pub(crate) |

## Symbols & Routines (24)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `from_configured` | L21 | `fn from_configured(name: Option<String>) -> Self` |
| function | `configured_name` | L28 | `fn configured_name(&self) -> Option<&str>` |
| function | `mode_label` | L35 | `fn mode_label(&self) -> &'static str` |
| function | `from_direction` | L51 | `fn from_direction(direction: Direction) -> Self` |
| function | `from_windows` | L58 | `fn from_windows(flow: EDataFlow) -> Self` |
| function | `from_windows` | L76 | `fn from_windows(role: ERole) -> Self` |
| function | `kind_label` | L105 | `fn kind_label(&self) -> &'static str` |
| function | `label` | L124 | `fn label(self) -> &'static str` |
| function | `new` | L148 | `fn new(selection: &'a DeviceSelection, direction: Direction, endpoint_id: &'a str,) -> Self` |
| function | `decide` | L160 | `fn decide(&self, notification: &RouteNotification) -> RecoveryDecision` |
| function | `OnDeviceStateChanged` | L199 | `fn OnDeviceStateChanged(&self, endpoint_id: &PCWSTR, new_state: windows::Win32::Media::Audio::DEVICE_STATE,) -> windows::core::Result<()>` |
| function | `OnDeviceAdded` | L213 | `fn OnDeviceAdded(&self, _endpoint_id: &PCWSTR) -> windows::core::Result<()>` |
| function | `OnDeviceRemoved` | L217 | `fn OnDeviceRemoved(&self, endpoint_id: &PCWSTR) -> windows::core::Result<()>` |
| function | `OnDefaultDeviceChanged` | L226 | `fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, endpoint_id: &PCWSTR,) -> windows::core::Result<()>` |
| function | `OnPropertyValueChanged` | L240 | `fn OnPropertyValueChanged(&self, endpoint_id: &PCWSTR, key: &windows::Win32::Foundation::PROPERTYKEY,) -> windows::core::Result<()>` |
| function | `endpoint_id_string` | L256 | `fn endpoint_id_string(endpoint_id: &PCWSTR) -> Option<String>` |
| function | `register` | L271 | `fn register() -> Result<(Self, mpsc::Receiver<RouteNotification>)>` |
| function | `drop` | L290 | `fn drop(&mut self) -> ()` |
| function | `policy` | L304 | `fn policy(selection: &'a DeviceSelection) -> RecoveryPolicy<'a>` |
| function | `configured_device_mode_is_explicit` | L309 | `fn configured_device_mode_is_explicit() -> ()` |
| function | `default_follower_reopens_only_for_matching_console_default` | L325 | `fn default_follower_reopens_only_for_matching_console_default() -> ()` |
| function | `pinned_device_never_follows_default_change` | L363 | `fn pinned_device_never_follows_default_change() -> ()` |
| function | `current_endpoint_topology_and_format_changes_reopen` | L376 | `fn current_endpoint_topology_and_format_changes_reopen() -> ()` |
| function | `unrelated_endpoint_never_reopens` | L412 | `fn unrelated_endpoint_never_reopens() -> ()` |
