#[cfg(windows)]
use std::path::PathBuf;

use super::{run_capture, GIB};

/// Confirmed hardware matrix supplied by the owner. Values outside the matrix
/// remain unknown; they are never rounded into a stronger profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareModelProfile {
    Unknown,
    Fallback12B,
    Primary26Vram8,
    Primary26Vram12,
    Primary26Vram16,
}

impl HardwareModelProfile {
    #[must_use]
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Fallback12B,
            2 => Self::Primary26Vram8,
            3 => Self::Primary26Vram12,
            4 => Self::Primary26Vram16,
            _ => Self::Unknown,
        }
    }

    #[must_use]
    pub const fn index(self) -> i32 {
        match self {
            Self::Unknown => 0,
            Self::Fallback12B => 1,
            Self::Primary26Vram8 => 2,
            Self::Primary26Vram12 => 3,
            Self::Primary26Vram16 => 4,
        }
    }

    #[must_use]
    pub const fn uses_primary_26b(self) -> bool {
        matches!(
            self,
            Self::Primary26Vram8 | Self::Primary26Vram12 | Self::Primary26Vram16
        )
    }

    #[must_use]
    pub const fn context_tokens(self, prep: bool) -> u32 {
        match (self, prep) {
            (Self::Primary26Vram8, false) | (Self::Fallback12B, _) => 32_768,
            (Self::Primary26Vram8, true) | (Self::Primary26Vram12, false) => 65_536,
            (Self::Primary26Vram12, true) | (Self::Primary26Vram16, _) => 98_304,
            (Self::Unknown, _) => 8_192,
        }
    }

    #[must_use]
    pub const fn requires_prep_switch(self) -> bool {
        matches!(self, Self::Primary26Vram8 | Self::Primary26Vram12)
    }
}

/// Whether a profile is in the owner's confirmed 26B-A4B matrix.
#[must_use]
pub const fn primary_26b_allowed(profile: HardwareModelProfile) -> bool {
    profile.uses_primary_26b()
}

/// Select an owner-confirmed NVIDIA VRAM tier with its minimum RAM threshold.
/// Inputs are nominal binary GiB: each VRAM tier keeps its minimum RAM but
/// accepts any greater value:
/// 16/32+, 12/24+, 8/32+ -> the corresponding 26B profile; 8/16..31 -> 12B.
#[must_use]
pub const fn select_hardware_model_profile(
    vram_gib: Option<u64>,
    ram_gib: Option<u64>,
) -> HardwareModelProfile {
    let (Some(vram), Some(ram)) = (vram_gib, ram_gib) else {
        return HardwareModelProfile::Unknown;
    };
    match (vram, ram) {
        (16, 32..) => HardwareModelProfile::Primary26Vram16,
        (12, 24..) => HardwareModelProfile::Primary26Vram12,
        (8, 32..) => HardwareModelProfile::Primary26Vram8,
        (8, 16..=31) => HardwareModelProfile::Fallback12B,
        _ => HardwareModelProfile::Unknown,
    }
}

/// Snap a near-nominal VRAM reading to the closest approved matrix tier.
///
/// Tolerance is ±1 GiB — enough to absorb firmware/driver underreport
/// (e.g. a 16 GiB card reporting 15 GiB). Readings 2+ GiB away from any
/// tier pass through unchanged, so clearly smaller hardware (6 GiB) never
/// enters the confirmed matrix.
#[must_use]
pub(super) fn normalize_vram_gib(raw: u64) -> u64 {
    match raw {
        7..=9 => 8,
        11..=13 => 12,
        15..=17 => 16,
        _ => raw,
    }
}

/// Snap a near-nominal system-RAM reading to the closest approved tier.
///
/// With an iGPU enabled in firmware, `TotalPhysicalMemory` reports usable
/// RAM minus the iGPU reservation, so a 32 GiB machine can show 31 GiB.
/// ±1 GiB tolerance covers this without promoting clearly smaller hardware
/// (e.g. 22 GiB stays 22 → `Unknown`).
#[must_use]
pub(super) fn normalize_ram_gib(raw: u64) -> u64 {
    match raw {
        15..=17 => 16,
        23..=25 => 24,
        31..=33 => 32,
        _ => raw,
    }
}

pub(super) fn hardware_profile_status(profile: HardwareModelProfile) -> String {
    match profile {
        HardwareModelProfile::Unknown => {
            "Hardware profile unknown — installing the Gemma 12B fallback".to_string()
        }
        HardwareModelProfile::Fallback12B => {
            "Hardware profile 8 GB VRAM / 16-31 GB RAM — using Gemma 12B".to_string()
        }
        HardwareModelProfile::Primary26Vram8 => {
            "Hardware profile 8 GB VRAM / 32+ GB RAM — using Gemma 26B-A4B".to_string()
        }
        HardwareModelProfile::Primary26Vram12 => {
            "Hardware profile 12 GB VRAM / 24+ GB RAM — using Gemma 26B-A4B".to_string()
        }
        HardwareModelProfile::Primary26Vram16 => {
            "Hardware profile 16 GB VRAM / 32+ GB RAM — using Gemma 26B-A4B".to_string()
        }
    }
}

/// True if an NVIDIA GPU is present (`nvidia-smi -L` succeeds with output).
#[must_use]
pub fn detect_nvidia() -> bool {
    match run_capture("nvidia-smi", &["-L"]) {
        Ok(out) => out.status.success() && !out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Which GPU acceleration to target for the local engine (Баг2): NVIDIA → CUDA
/// build; any other GPU (AMD/Intel) → Vulkan build; none → CPU.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum GpuKind {
    Nvidia,
    Other,
    None,
}

/// Classify the GPU for engine selection. NVIDIA (CUDA) is checked first via the
/// cheap `nvidia-smi`; only a non-NVIDIA machine pays the WMI query for an AMD /
/// Intel adapter (Vulkan). No detectable GPU → CPU.
pub(super) fn detect_gpu() -> GpuKind {
    #[cfg(target_os = "macos")]
    {
        GpuKind::Other // Metal GPU acceleration on macOS / Apple Silicon
    }
    #[cfg(windows)]
    {
        if detect_nvidia() {
            GpuKind::Nvidia
        } else if detect_non_nvidia_gpu() && vulkan_loader_present() {
            GpuKind::Other
        } else {
            GpuKind::None
        }
    }
}

/// True if the Vulkan loader (`vulkan-1.dll`) is present in System32 — required for
/// the Vulkan llama build to load at all.
#[cfg(windows)]
pub(super) fn vulkan_loader_present() -> bool {
    std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("vulkan-1.dll"))
        .is_some_and(|p| p.is_file())
}

/// True if a non-NVIDIA display adapter (AMD / Intel) is present. Best-effort name
/// match over WMI; a false positive only means we try the Vulkan build and fall
/// back to CPU if it can't offload (Баг2), so this never makes things worse.
#[cfg(windows)]
pub(super) fn detect_non_nvidia_gpu() -> bool {
    let out = match run_capture(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController).Name -join ';'",
        ],
    ) {
        Ok(o) if o.status.success() => o.stdout,
        _ => return false,
    };
    let names = String::from_utf8_lossy(&out).to_lowercase();
    ["radeon", "amd", "intel", "arc"]
        .iter()
        .any(|k| names.contains(k))
}

#[allow(dead_code)]
pub(super) fn detect_nvidia_vram_gib() -> Option<u64> {
    detect_nvidia_memory_mib().map(|(_, total)| (total + 512) / 1024)
}

pub(super) fn detect_nvidia_memory_mib() -> Option<(u64, u64)> {
    let out = run_capture(
        "nvidia-smi",
        &[
            "--query-gpu=memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ],
    )
    .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_memory_mib(&String::from_utf8_lossy(&out.stdout))
}

pub(super) fn parse_nvidia_memory_mib(text: &str) -> Option<(u64, u64)> {
    // One selected dedicated adapter; never add VRAM across devices.
    text.lines()
        .filter_map(|line| {
            let (used, total) = line.split_once(',')?;
            Some((used.trim().parse().ok()?, total.trim().parse().ok()?))
        })
        .max_by_key(|(_, total)| *total)
}

pub(super) fn detect_system_ram_gib() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let out = run_capture("sysctl", &["-n", "hw.memsize"]).ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<u64>()
            .ok()
            .map(|bytes| bytes / GIB)
    }
    #[cfg(windows)]
    {
        let out = run_capture(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ],
        )
        .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<u64>()
            .ok()
            .map(|bytes| bytes.div_ceil(GIB))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    None
}

pub(super) fn detected_hardware_model_profile(force_cpu: bool) -> HardwareModelProfile {
    if force_cpu {
        return HardwareModelProfile::Unknown;
    }
    #[cfg(target_os = "macos")]
    {
        let ram = detect_system_ram_gib().unwrap_or(16);
        hardware_profile_from_discovery(false, Some(ram), Some(ram))
    }
    #[cfg(windows)]
    {
        let raw_vram = detect_nvidia_vram_gib();
        let raw_ram = detect_system_ram_gib();
        log::info!(
            "local-ai hardware discovery: raw_vram_gib={raw_vram:?} raw_ram_gib={raw_ram:?}"
        );
        hardware_profile_from_discovery(force_cpu, raw_vram, raw_ram)
    }
}

/// Detect whether this machine is currently in the confirmed 26B-A4B matrix.
/// This is worker-only: discovery may query WMI/DXGI and must not block Slint.
#[must_use]
pub fn primary_26b_allowed_on_current_hardware() -> bool {
    primary_26b_allowed(detected_hardware_model_profile(false))
}

#[must_use]
pub fn current_hardware_model_profile() -> HardwareModelProfile {
    detected_hardware_model_profile(false)
}

#[must_use]
pub fn current_server_profile(prefer_quality: bool) -> HardwareModelProfile {
    profile_for_model(current_hardware_model_profile(), prefer_quality)
}

pub(super) fn profile_for_model(
    detected: HardwareModelProfile,
    prefer_quality: bool,
) -> HardwareModelProfile {
    if prefer_quality || detected == HardwareModelProfile::Unknown {
        detected
    } else {
        HardwareModelProfile::Fallback12B
    }
}

/// The confirmed matrix is NVIDIA-only. AMD/Intel remain on the safe fallback
/// until they have their own measured profiles.
///
/// Raw readings pass through ±1 GiB normalization before the minimum-RAM
/// matrix lookup, absorbing iGPU memory reservations and firmware underreport.
pub(super) fn hardware_profile_from_discovery(
    force_cpu: bool,
    nvidia_vram_gib: Option<u64>,
    ram_gib: Option<u64>,
) -> HardwareModelProfile {
    if force_cpu {
        return HardwareModelProfile::Unknown;
    }
    let vram = nvidia_vram_gib.map(normalize_vram_gib);
    let ram = ram_gib.map(normalize_ram_gib);
    if vram != nvidia_vram_gib || ram != ram_gib {
        log::info!(
            "local-ai hardware normalization: vram {nvidia_vram_gib:?} -> {vram:?}, ram {ram_gib:?} -> {ram:?}"
        );
    }
    let profile = select_hardware_model_profile(vram, ram);
    log::info!("local-ai hardware profile: {profile:?}");
    profile
}
