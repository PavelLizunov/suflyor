---
schema_version: "1.0"
artifact_kind: file-map
file_id: "F_28199aa3a8ff"
source_path: "overlay-backend/src/local_ai/hardware_profile.rs"
batch_id: "B06"
total_lines: 351
symbols_count: 24
review_state: validated
---

# File Map: `overlay-backend/src/local_ai/hardware_profile.rs`

- **Batch:** B06
- **Physical Lines:** 351
- **Coverage:** 351/351 lines (100%)

## Types & Structures (2)

| Kind | Name | Line | Visibility |
|---|---|---|---|
| enum | `HardwareModelProfile` | L9 | pub |
| enum | `GpuKind` | L155 | pub(super) |

## Symbols & Routines (24)

| Kind | Name | Line | Signature |
|---|---|---|---|
| function | `from_index` | L19 | `fn from_index(index: i32) -> Self` |
| function | `index` | L30 | `fn index(self) -> i32` |
| function | `uses_primary_26b` | L41 | `fn uses_primary_26b(self) -> bool` |
| function | `context_tokens` | L49 | `fn context_tokens(self, prep: bool) -> u32` |
| function | `requires_prep_switch` | L59 | `fn requires_prep_switch(self) -> bool` |
| function | `primary_26b_allowed` | L66 | `fn primary_26b_allowed(profile: HardwareModelProfile) -> bool` |
| function | `select_hardware_model_profile` | L75 | `fn select_hardware_model_profile(vram_gib: Option<u64>, ram_gib: Option<u64>,) -> HardwareModelProfile` |
| function | `normalize_vram_gib` | L98 | `fn normalize_vram_gib(raw: u64) -> u64` |
| function | `normalize_ram_gib` | L114 | `fn normalize_ram_gib(raw: u64) -> u64` |
| function | `hardware_profile_status` | L123 | `fn hardware_profile_status(profile: HardwareModelProfile) -> String` |
| function | `detect_nvidia` | L145 | `fn detect_nvidia() -> bool` |
| function | `detect_gpu` | L164 | `fn detect_gpu() -> GpuKind` |
| function | `vulkan_loader_present` | L184 | `fn vulkan_loader_present() -> bool` |
| function | `detect_non_nvidia_gpu` | L194 | `fn detect_non_nvidia_gpu() -> bool` |
| function | `detect_nvidia_vram_gib` | L213 | `fn detect_nvidia_vram_gib() -> Option<u64>` |
| function | `detect_nvidia_memory_mib` | L217 | `fn detect_nvidia_memory_mib() -> Option<(u64, u64)>` |
| function | `parse_nvidia_memory_mib` | L232 | `fn parse_nvidia_memory_mib(text: &str) -> Option<(u64, u64)>` |
| function | `detect_system_ram_gib` | L242 | `fn detect_system_ram_gib() -> Option<u64>` |
| function | `detected_hardware_model_profile` | L280 | `fn detected_hardware_model_profile(force_cpu: bool) -> HardwareModelProfile` |
| function | `primary_26b_allowed_on_current_hardware` | L303 | `fn primary_26b_allowed_on_current_hardware() -> bool` |
| function | `current_hardware_model_profile` | L308 | `fn current_hardware_model_profile() -> HardwareModelProfile` |
| function | `current_server_profile` | L313 | `fn current_server_profile(prefer_quality: bool) -> HardwareModelProfile` |
| function | `profile_for_model` | L317 | `fn profile_for_model(detected: HardwareModelProfile, prefer_quality: bool,) -> HardwareModelProfile` |
| function | `hardware_profile_from_discovery` | L333 | `fn hardware_profile_from_discovery(force_cpu: bool, nvidia_vram_gib: Option<u64>, ram_gib: Option<u64>,) -> HardwareModelProfile` |
