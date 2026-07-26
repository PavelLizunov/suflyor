//! In-app installer for the LOCAL AI stack (llama.cpp + Gemma, whisper.cpp +
//! Whisper-turbo, GigaAM-v3). This is the Rust port of `scripts/setup-local-ai.ps1`
//! so the user can install + launch everything from a button in Settings instead
//! of running a separate PowerShell script.
//!
//! Design: the whole pipeline is BLOCKING and runs on a caller-provided worker
//! thread (never the UI thread). It shells out to the same OS tools the script
//! relies on -- `curl.exe` (resilient resumable downloads; the HuggingFace Xet
//! CDN resets open-ended GETs, and `curl -C -` resumes to a known size) and
//! `tar.exe` (bsdtar, ships in Windows 10 1803+, extracts the release zips) --
//! plus `nvidia-smi` for GPU detection. The GitHub release JSON is fetched with
//! curl and parsed with serde_json, so there is no async runtime here at all.
//!
//! Progress is reported through a `&dyn Fn(Progress)` callback the UI turns into
//! `slint::invoke_from_event_loop` property updates.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

// ---- pinned model coordinates (HuggingFace) + exact sizes (integrity) -------
// RAM-safe fallback: Gemma 4 12B QAT. It is always installed so a machine that
// does not qualify for the 26B matrix, or whose 26B file disappears, still has
// a verified local model to launch.
const GEMMA_URL: &str = "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/main/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf";
const GEMMA_FILE: &str = "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf";
const GEMMA_SIZE: u64 = 6_716_355_328;
const GEMMA_SHA256: &str = "cc9ff072e0a8203429ed854e6662c17a6c2bc1e5dca5b475dd4736caaacbc165";

// The model installed by the previous release. Keep recognising it during an
// upgrade: replacing its persisted model id with the new 12B filename before
// the user has downloaded 12B would leave an otherwise working installation
// with no launchable model.
const LEGACY_GEMMA_FILE: &str = "gemma-4-E4B-it-Q4_K_M.gguf";
const LEGACY_GEMMA_SIZE: u64 = 4_977_169_568;

// Owner-approved primary model. The byte size and SHA-256 are independently
// pinned from the immutable Hugging Face revision c099eb4.
const GEMMA26_URL: &str = "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF/resolve/c099eb4/gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf";
const GEMMA26_FILE: &str = "gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf";
const GEMMA26_SIZE: u64 = 10_546_934_240;
const GEMMA26_SHA256: &str = "2a1d26dfe6ea00a467940a5728316af6edb366bbdba950d65b85d232392fb658";

// Vision projector for the 12B fallback. Uses the model's own gemma4uv
// projector and is only attached on a compatible llama.cpp build.
const MMPROJ_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/main/mmproj-F16.gguf";
const MMPROJ_FILE: &str = "mmproj-12b-F16.gguf";
const MMPROJ_SIZE: u64 = 175_115_840;
const MMPROJ_SHA256: &str = "ecc4e93128da8363b7dbf2193eab98cf1142353f52ceaa0c95c0872997aaadd3";
/// Minimum llama.cpp release build (the `bNNNN` tag) that can load Gemma 4
/// "gemma4uv" projector. Below this we keep the 12B text-only (no crash).
const GEMMA4UV_MIN_BUILD: u32 = 9626;

const WHISPER_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin";
const WHISPER_FILE: &str = "ggml-large-v3-turbo-q8_0.bin";
const WHISPER_SIZE: u64 = 874_188_075;
const WHISPER_SHA256: &str = "317eb69c11673c9de1e1f0d459b253999804ec71ac4c23c17ecf5fbe24e259a1";
const WHISPER_MODEL_ID: &str = "whisper-large-v3-turbo";

const GIGAAM_MODEL_URL: &str =
    "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc.int8.onnx";
const GIGAAM_MODEL_SIZE: u64 = 224_893_347;
const GIGAAM_SHA256: &str = "2e3fcb7a7b66030336fd10c2fcfb033bd1dc7e1bf238fe5cfd83b1d0cfc9d28e";
const GIGAAM_VOCAB_URL: &str =
    "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc_vocab.txt";
/// GigaAM-v3 vocab (2 KB, FIXED for this model) — BUNDLED via include_bytes so the
/// install never depends on the flaky HF download. HF has repeatedly served an
/// HTML error page for this tiny file, which (before v0.10.2) aborted the WHOLE
/// install at the vocab step → gemma never deployed + server never launched. The
/// download (`GIGAAM_VOCAB_URL` / `curl_small`) is kept only as a fallback.
const GIGAAM_VOCAB: &[u8] = include_bytes!("../assets/gigaam-v3-vocab.txt");

const LLAMA_REPO: &str = "ggml-org/llama.cpp";
const WHISPER_REPO: &str = "ggml-org/whisper.cpp";

/// Local server endpoints the installer configures + launches.
pub const LLAMA_BASE_URL: &str = "http://127.0.0.1:8080/v1";
pub const WHISPER_BASE_URL: &str = "http://127.0.0.1:8081/v1";
const LLAMA_PORT: &str = "8080";
const WHISPER_PORT: &str = "8081";

/// True only for Suflyor's bundled loopback llama.cpp endpoint. Port 8080 on a
/// LAN host or another local port is not ours to restart or relabel.
#[must_use]
pub fn is_managed_llama_endpoint(base_url: &str) -> bool {
    let Some(without_scheme) = base_url.trim().strip_prefix("http://") else {
        return false;
    };
    let (authority, path) = without_scheme
        .split_once('/')
        .map_or((without_scheme, ""), |(authority, path)| (authority, path));
    if format!("/{path}").trim_end_matches('/') != "/v1" {
        return false;
    }
    let host_port = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split_once("]:")
    } else {
        authority.rsplit_once(':')
    };
    host_port.is_some_and(|(host, port)| {
        port == LLAMA_PORT
            && matches!(
                host.to_ascii_lowercase().as_str(),
                "127.0.0.1" | "localhost" | "::1"
            )
    })
}

/// Select the local provider and repair a bundled managed endpoint before any
/// request can use its persisted model fields. Custom local servers retain
/// their configured model and prep-model values.
pub fn select_local_provider(cfg: &mut crate::config::Config, root: &Path) -> bool {
    let provider_changed = cfg.ai_provider != "local";
    cfg.ai_provider = "local".to_string();
    let model_state_changed = repair_managed_model_state(cfg, root);
    provider_changed || model_state_changed
}

const STRICT_LLAMA_READY_BUDGET: Duration = Duration::from_secs(120);

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
    pub const fn uses_primary_26b(self) -> bool {
        matches!(
            self,
            Self::Primary26Vram8 | Self::Primary26Vram12 | Self::Primary26Vram16
        )
    }
}

/// Whether a profile is in the owner's confirmed 26B-A4B matrix.
#[must_use]
pub const fn primary_26b_allowed(profile: HardwareModelProfile) -> bool {
    profile.uses_primary_26b()
}

/// Select only an owner-confirmed VRAM/RAM pair. Inputs are nominal binary GiB:
/// 8/16 -> 12B; 8/32, 12/24-32, and 16/32 -> the corresponding 26B profile.
#[must_use]
pub const fn select_hardware_model_profile(
    vram_gib: Option<u64>,
    ram_gib: Option<u64>,
) -> HardwareModelProfile {
    let (Some(vram), Some(ram)) = (vram_gib, ram_gib) else {
        return HardwareModelProfile::Unknown;
    };
    match (vram, ram) {
        (16, 32) => HardwareModelProfile::Primary26Vram16,
        (12, 24..=32) => HardwareModelProfile::Primary26Vram12,
        (8, 32) => HardwareModelProfile::Primary26Vram8,
        (8, 16) => HardwareModelProfile::Fallback12B,
        _ => HardwareModelProfile::Unknown,
    }
}

fn hardware_profile_status(profile: HardwareModelProfile) -> String {
    match profile {
        HardwareModelProfile::Unknown => {
            "Hardware profile unknown — installing the Gemma 12B fallback".to_string()
        }
        HardwareModelProfile::Fallback12B => {
            "Hardware profile 8 GB VRAM / 16 GB RAM — using Gemma 12B".to_string()
        }
        HardwareModelProfile::Primary26Vram8 => {
            "Hardware profile 8 GB VRAM / 32 GB RAM — using Gemma 26B-A4B".to_string()
        }
        HardwareModelProfile::Primary26Vram12 => {
            "Hardware profile 12 GB VRAM / 24-32 GB RAM — using Gemma 26B-A4B".to_string()
        }
        HardwareModelProfile::Primary26Vram16 => {
            "Hardware profile 16 GB VRAM / 32 GB RAM — using Gemma 26B-A4B".to_string()
        }
    }
}

/// CREATE_NO_WINDOW — keep the spawned console servers windowless.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `install` returns this exact error message when the user cancels mid-run, so
/// the UI can show "Отменено" instead of treating it as a failure.
pub const CANCEL_SENTINEL: &str = "__cancelled__";

// ---- public API ------------------------------------------------------------

/// Options for an install run.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Install root (binaries + models). Default: `default_root()`.
    pub root: PathBuf,
    /// Force the CPU llama.cpp build even if an NVIDIA GPU is present.
    pub force_cpu: bool,
    pub skip_llama: bool,
    pub skip_whisper: bool,
    pub skip_gigaam: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            root: default_root(),
            force_cpu: false,
            skip_llama: false,
            skip_whisper: false,
            skip_gigaam: false,
        }
    }
}

/// Live progress messages emitted during an install.
#[derive(Debug, Clone)]
pub enum Progress {
    /// A new phase started (human-readable, already localised by the caller is
    /// not expected — these are short English step labels).
    Step(String),
    /// Byte progress for the current download.
    Bytes {
        label: String,
        done: u64,
        total: u64,
    },
    /// GPU/CPU verdict once the LLM server is up.
    Gpu(String),
}

/// What the UI needs after a successful install: the values to write into
/// `Config`, the GPU verdict, and the live server child handles (so the app can
/// kill them on quit).
#[derive(Debug)]
pub struct LocalAiResult {
    pub ai_local_model: String,
    /// `true` when the owner-approved 26B primary actually started; `false`
    /// when the installed/serving model is the 12B fallback.
    pub ai_local_quality: bool,
    /// Whether the model that actually started also has its matching projector
    /// attached. The 26B primary is text-only in this candidate.
    pub ai_local_vision: bool,
    pub hardware_profile: HardwareModelProfile,
    pub stt_gigaam_dir: String,
    pub on_gpu: bool,
    pub cuda_version: Option<String>,
    pub servers: Vec<Child>,
}

/// Default install root: `%USERPROFILE%\suflyor-local-ai`.
#[must_use]
pub fn default_root() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("suflyor-local-ai")
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
enum GpuKind {
    Nvidia,
    Other,
    None,
}

/// Classify the GPU for engine selection. NVIDIA (CUDA) is checked first via the
/// cheap `nvidia-smi`; only a non-NVIDIA machine pays the WMI query for an AMD /
/// Intel adapter (Vulkan). No detectable GPU → CPU.
fn detect_gpu() -> GpuKind {
    if detect_nvidia() {
        GpuKind::Nvidia
    } else if detect_non_nvidia_gpu() && vulkan_loader_present() {
        // Vulkan needs BOTH a non-NVIDIA GPU AND the loader (vulkan-1.dll, shipped
        // with Win10+). Without the loader the Vulkan build can't load at all — and
        // the -ngl 0 fallback re-runs the SAME exe, so it couldn't rescue it — so
        // use CPU rather than risk a failed install on a loader-less box (Баг2).
        GpuKind::Other
    } else {
        GpuKind::None
    }
}

/// True if the Vulkan loader (`vulkan-1.dll`) is present in System32 — required for
/// the Vulkan llama build to load at all.
fn vulkan_loader_present() -> bool {
    std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("vulkan-1.dll"))
        .is_some_and(|p| p.is_file())
}

/// True if a non-NVIDIA display adapter (AMD / Intel) is present. Best-effort name
/// match over WMI; a false positive only means we try the Vulkan build and fall
/// back to CPU if it can't offload (Баг2), so this never makes things worse.
fn detect_non_nvidia_gpu() -> bool {
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

fn detect_nvidia_vram_gib() -> Option<u64> {
    let out = run_capture(
        "nvidia-smi",
        &["--query-gpu=memory.total", "--format=csv,noheader,nounits"],
    )
    .ok()?;
    if !out.status.success() {
        return None;
    }
    // One selected dedicated adapter; never add VRAM across devices.
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .max()
        .map(|mib| (mib + 512) / 1024)
}

#[cfg(any(windows, test))]
const AMD_VENDOR_ID: u32 = 0x1002;
#[cfg(any(windows, test))]
const INTEL_VENDOR_ID: u32 = 0x8086;
#[cfg(any(windows, test))]
const DXGI_ADAPTER_FLAG_REMOTE_BIT: u32 = 0x1;
#[cfg(any(windows, test))]
const DXGI_ADAPTER_FLAG_SOFTWARE_BIT: u32 = 0x2;

/// Best-effort VRAM discovery for Vulkan-capable AMD/Intel adapters. `nvidia-smi`
/// is NVIDIA-only, while the launcher also offers a Vulkan build on these GPUs.
/// DXGI reports the full 64-bit dedicated VRAM value; legacy WMI `AdapterRAM` is
/// only 32-bit and would truncate modern 8-16 GiB adapters.
#[cfg(windows)]
fn detect_non_nvidia_vram_gib() -> Option<u64> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

    // DXGI factory/adapter calls are the direct Windows API for enumerating
    // display adapters. Failure is an unknown profile, never an invented value.
    let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>().ok()? };
    let mut adapters = Vec::new();
    for index in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        adapters.push((desc.VendorId, desc.Flags, desc.DedicatedVideoMemory as u64));
    }
    select_non_nvidia_dedicated_vram_gib(&adapters)
}

#[cfg(not(windows))]
fn detect_non_nvidia_vram_gib() -> Option<u64> {
    None
}

/// Select a single usable AMD/Intel DXGI adapter. The tuples are
/// `(vendor_id, flags, dedicated_vram_bytes)` so the hardware rule remains
/// unit-testable on non-Windows CI.
#[cfg(any(windows, test))]
fn select_non_nvidia_dedicated_vram_gib(adapters: &[(u32, u32, u64)]) -> Option<u64> {
    adapters
        .iter()
        .filter(|(vendor, flags, bytes)| {
            (*vendor == AMD_VENDOR_ID || *vendor == INTEL_VENDOR_ID)
                && (*flags & (DXGI_ADAPTER_FLAG_REMOTE_BIT | DXGI_ADAPTER_FLAG_SOFTWARE_BIT)) == 0
                && *bytes > 0
        })
        .map(|(_, _, bytes)| bytes.div_ceil(GIB))
        .max()
}

fn detect_system_ram_gib() -> Option<u64> {
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

fn detected_hardware_model_profile(force_cpu: bool) -> HardwareModelProfile {
    if force_cpu {
        return HardwareModelProfile::Unknown;
    }
    let nvidia_vram = detect_nvidia_vram_gib();
    let non_nvidia_vram = if nvidia_vram.is_none() {
        detect_non_nvidia_vram_gib()
    } else {
        None
    };
    hardware_profile_from_discovery(
        force_cpu,
        nvidia_vram,
        non_nvidia_vram,
        detect_system_ram_gib(),
    )
}

/// Detect whether this machine is currently in the confirmed 26B-A4B matrix.
/// This is worker-only: discovery may query WMI/DXGI and must not block Slint.
#[must_use]
pub fn primary_26b_allowed_on_current_hardware() -> bool {
    primary_26b_allowed(detected_hardware_model_profile(false))
}

/// Keep the source of VRAM explicit so AMD/Intel Vulkan hardware follows the
/// same owner-approved matrix as NVIDIA, while tests need no Windows commands.
fn hardware_profile_from_discovery(
    force_cpu: bool,
    nvidia_vram_gib: Option<u64>,
    non_nvidia_vram_gib: Option<u64>,
    ram_gib: Option<u64>,
) -> HardwareModelProfile {
    if force_cpu {
        HardwareModelProfile::Unknown
    } else {
        select_hardware_model_profile(nvidia_vram_gib.or(non_nvidia_vram_gib), ram_gib)
    }
}

/// Write the installer's resulting endpoints/models into a `Config`, switching
/// it to the local stack. Secrets are untouched. The actual installed selection
/// wins over stale preferences, and the bundled single-model server never
/// inherits an external prep-model id.
pub fn apply_result(cfg: &mut crate::config::Config, res: &LocalAiResult) {
    cfg.ai_provider = "local".to_string();
    cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();
    cfg.ai_local_model = res.ai_local_model.clone();
    cfg.ai_local_prep_model.clear();
    cfg.ai_local_quality = res.ai_local_quality;
    // Default STT to Whisper (mixed RU+EN); the GigaAM dir is also filled so the
    // user can switch to GigaAM (best Russian) in Settings without re-installing.
    cfg.stt_provider = "whisper".to_string();
    cfg.stt_whisper_url = WHISPER_BASE_URL.to_string();
    cfg.stt_whisper_model = WHISPER_MODEL_ID.to_string();
    cfg.stt_gigaam_dir = res.stt_gigaam_dir.clone();
    // Only the 12B fallback has a pinned projector. Never route F8 to the
    // text-only 26B server. Preserve an explicitly configured cloud/separate
    // vision endpoint; only a route that resolves back to managed :8080 is
    // disabled.
    cfg.ai_local_vision = !res.ai_local_quality && res.ai_local_vision;
    if cfg.ai_local_vision {
        cfg.vision_provider = "same".to_string();
    } else if vision_routes_to_managed_llama(cfg) {
        cfg.vision_provider = "off".to_string();
    }
}

/// Run the full install pipeline. BLOCKING — call from a worker thread. Reports
/// progress via `on`. Returns the values to persist + the live server handles.
pub fn install(
    opts: &InstallOptions,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<LocalAiResult> {
    preflight().context("environment preflight failed")?;
    std::fs::create_dir_all(&opts.root)
        .with_context(|| format!("create install root {}", opts.root.display()))?;
    bail_if_cancelled(cancel)?;

    let llama_dir = opts.root.join("llama.cpp");
    let whisper_dir = opts.root.join("whisper.cpp");
    let gigaam_dir = opts.root.join("gigaam-v3");
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let hardware_profile = detected_hardware_model_profile(opts.force_cpu);
    let mut prefer_quality = hardware_profile.uses_primary_26b();
    on(Progress::Step(hardware_profile_status(hardware_profile)));

    // P1.5 — fail fast on insufficient disk BEFORE pulling gigabytes. Count only
    // what we'd actually fetch: a model already complete at its dest is skipped
    // (mirrors reuse_if_available's dest check), and the server binaries add a
    // flat allowance only when not already installed.
    {
        let mut need: u64 = 0;
        if !opts.skip_llama {
            if file_len(&llama_dir.join(GEMMA_FILE)) < GEMMA_SIZE {
                need += GEMMA_SIZE;
            }
            if file_len(&llama_dir.join(MMPROJ_FILE)) < MMPROJ_SIZE {
                need += MMPROJ_SIZE;
            }
            if prefer_quality && file_len(&llama_dir.join(GEMMA26_FILE)) < GEMMA26_SIZE {
                need += GEMMA26_SIZE;
            }
            if find_exe(&llama_dir, "llama-server.exe").is_none() {
                need += LLAMA_BINARIES_ALLOWANCE;
            }
        }
        if !opts.skip_whisper {
            if file_len(&whisper_dir.join(WHISPER_FILE)) < WHISPER_SIZE {
                need += WHISPER_SIZE;
            }
            if find_exe(&whisper_dir, "whisper-server.exe").is_none()
                && find_exe(&whisper_dir, "server.exe").is_none()
            {
                need += WHISPER_BINARIES_ALLOWANCE;
            }
        }
        if !opts.skip_gigaam && file_len(&gigaam_dir.join("model.int8.onnx")) < GIGAAM_MODEL_SIZE {
            need += GIGAAM_MODEL_SIZE;
        }
        ensure_disk_space(&opts.root, need, on)?;
    }

    let gpu = if opts.force_cpu {
        GpuKind::None
    } else {
        detect_gpu()
    };
    let mut cuda_version: Option<String> = None;

    // ---- llama.cpp + Gemma -------------------------------------------------
    if !opts.skip_llama {
        on(Progress::Step("Installing llama.cpp".to_string()));
        std::fs::create_dir_all(&llama_dir)?;
        if find_exe(&llama_dir, "llama-server.exe").is_none() {
            let rel = github_release(LLAMA_REPO)?;
            let pick = pick_llama(&rel.assets, gpu)?;
            cuda_version = pick.version.clone();
            let blabel = format!("llama.cpp {}", pick.version.as_deref().unwrap_or("CPU"));
            download_and_extract(
                &pick.build_url,
                pick.build_size,
                &blabel,
                &llama_dir,
                cancel,
                on,
            )?;
            if let Some(cu) = &pick.cudart_url {
                download_and_extract(cu, pick.cudart_size, "CUDA runtime", &llama_dir, cancel, on)?;
            }
            // Stamp the build tag (e.g. "b9626") so the engine-version gate knows
            // whether this binary can load the 12B's gemma4uv vision projector, and
            // so the updater can compare installed-vs-latest. Best-effort: a missing
            // stamp just means 12B vision stays gated off (safe) until next update.
            write_build_stamp(&llama_dir, &rel.tag_name);
        }
        // Reuse an existing Gemma (e.g. a prior manual ~\llama.cpp) instead of
        // re-downloading 5 GB.
        let gemma_dest = llama_dir.join(GEMMA_FILE);
        if reuse_if_available(
            &gemma_dest,
            GEMMA_SIZE,
            GEMMA_SHA256,
            &[home.join("llama.cpp").join(GEMMA_FILE)],
        ) {
            on(Progress::Step("Reusing existing Gemma model".to_string()));
        } else {
            curl_resumable(GEMMA_URL, &gemma_dest, GEMMA_SIZE, "Gemma", cancel, on)?;
        }
        verify_sha256(&gemma_dest, GEMMA_SHA256, "Gemma model")?;

        // Vision projector (mmproj) — enables image reading on the same model so
        // F8 screenshots can be analysed locally without any cloud egress.
        let mmproj_dest = llama_dir.join(MMPROJ_FILE);
        if reuse_if_available(
            &mmproj_dest,
            MMPROJ_SIZE,
            MMPROJ_SHA256,
            &[home.join("llama.cpp").join(MMPROJ_FILE)],
        ) {
            on(Progress::Step(
                "Reusing existing vision projector".to_string(),
            ));
        } else {
            curl_resumable(
                MMPROJ_URL,
                &mmproj_dest,
                MMPROJ_SIZE,
                "Vision projector (mmproj)",
                cancel,
                on,
            )?;
        }
        verify_sha256(&mmproj_dest, MMPROJ_SHA256, "Vision projector")?;

        if prefer_quality {
            let primary_dest = llama_dir.join(GEMMA26_FILE);
            if reuse_if_available(
                &primary_dest,
                GEMMA26_SIZE,
                GEMMA26_SHA256,
                &[home.join("llama.cpp").join(GEMMA26_FILE)],
            ) {
                on(Progress::Step(
                    "Reusing existing Gemma 26B-A4B model".to_string(),
                ));
            } else {
                curl_resumable(
                    GEMMA26_URL,
                    &primary_dest,
                    GEMMA26_SIZE,
                    "Gemma 26B-A4B",
                    cancel,
                    on,
                )?;
            }
            verify_sha256(&primary_dest, GEMMA26_SHA256, "Gemma 26B-A4B model")?;
            cache_quality_model_verification(&primary_dest, true);
        }
    }

    // ---- whisper.cpp + Whisper-turbo --------------------------------------
    if !opts.skip_whisper {
        bail_if_cancelled(cancel)?;
        on(Progress::Step("Installing whisper.cpp".to_string()));
        std::fs::create_dir_all(&whisper_dir)?;
        if find_exe(&whisper_dir, "whisper-server.exe").is_none()
            && find_exe(&whisper_dir, "server.exe").is_none()
        {
            let assets = github_assets(WHISPER_REPO)?;
            // whisper.cpp ships CPU + cuBLAS(NVIDIA) builds only — no Vulkan; so GPU
            // whisper stays NVIDIA-only, AMD/Intel use the CPU whisper build (Баг2).
            let (url, size) = pick_whisper(&assets, gpu != GpuKind::Nvidia)?;
            download_and_extract(&url, size, "whisper.cpp", &whisper_dir, cancel, on)?;
        }
        let whisper_dest = whisper_dir.join(WHISPER_FILE);
        if reuse_if_available(
            &whisper_dest,
            WHISPER_SIZE,
            WHISPER_SHA256,
            &[home.join("whisper.cpp").join(WHISPER_FILE)],
        ) {
            on(Progress::Step("Reusing existing Whisper model".to_string()));
        } else {
            curl_resumable(
                WHISPER_URL,
                &whisper_dest,
                WHISPER_SIZE,
                "Whisper",
                cancel,
                on,
            )?;
        }
        verify_sha256(&whisper_dest, WHISPER_SHA256, "Whisper model")?;
    }

    // ---- GigaAM-v3 (in-process; no server) — OPTIONAL local STT -----------
    // NON-FATAL (v0.10.2): GigaAM is the *optional* best-Russian STT; the default
    // STT is Whisper (see `apply_result`) and cloud Whisper also remains. So a
    // GigaAM hiccup must NOT abort the install before the llama-server (LLM)
    // launches. Before this, a tester's vocab.txt download (HF served an HTML
    // error page) aborted the whole install at the `?` → gemma never deployed +
    // server never started. Now we log + continue; `gigaam_ok` gates the dir we
    // hand back so STT cleanly stays on Whisper if GigaAM didn't complete.
    let mut gigaam_ok = false;
    if !opts.skip_gigaam {
        bail_if_cancelled(cancel)?;
        on(Progress::Step("Downloading GigaAM-v3".to_string()));
        let giga_res = (|| -> Result<()> {
            std::fs::create_dir_all(&gigaam_dir)?;
            // transcribe_rs loads exactly `model.int8.onnx` + `vocab.txt`.
            let giga_dest = gigaam_dir.join("model.int8.onnx");
            if !reuse_if_available(&giga_dest, GIGAAM_MODEL_SIZE, GIGAAM_SHA256, &[]) {
                curl_resumable(
                    GIGAAM_MODEL_URL,
                    &giga_dest,
                    GIGAAM_MODEL_SIZE,
                    "GigaAM",
                    cancel,
                    on,
                )?;
            }
            verify_sha256(&giga_dest, GIGAAM_SHA256, "GigaAM model")?;
            // vocab.txt — write the BUNDLED copy (no flaky HF download for this
            // 2 KB file). Fall back to the network only if the embedded write fails.
            let vocab_dest = gigaam_dir.join("vocab.txt");
            if std::fs::write(&vocab_dest, GIGAAM_VOCAB).is_err() {
                curl_small(GIGAAM_VOCAB_URL, &vocab_dest)?;
            }
            Ok(())
        })();
        match giga_res {
            Ok(()) => gigaam_ok = true,
            Err(e) => {
                eprintln!(
                    "[local-ai] GigaAM STT setup failed — continuing (STT stays on Whisper): {e:#}"
                );
                on(Progress::Step(
                    "GigaAM STT unavailable — continuing".to_string(),
                ));
            }
        }
    }

    // ---- launch servers ----------------------------------------------------
    let mut servers: Vec<Child> = Vec::new();
    if !opts.skip_llama {
        on(Progress::Step("Starting llama-server :8080".to_string()));
        let exe = find_exe(&llama_dir, "llama-server.exe")
            .context("llama-server.exe not found after install")?;
        // Free :8080 of OUR stale/projector-less server so the fresh --mmproj
        // server can bind. Owner-aware: if a DIFFERENT app holds :8080, fail with
        // a clear conflict instead of killing it (audit P0.1).
        if !stop_listener_on_port(LLAMA_PORT, &opts.root) {
            bail!(
                "port :8080 is in use by another application — close it (or stop that server) and retry the local-AI install"
            );
        }
        std::thread::sleep(Duration::from_millis(800));
        let gguf = selected_llama_gguf(&llama_dir, prefer_quality);
        let gguf_s = gguf.to_string_lossy().into_owned();
        let alias = gguf
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mmproj =
            mmproj_for_model(&llama_dir, &gguf).map(|path| path.to_string_lossy().into_owned());
        let args = llama_server_args(&gguf_s, &alias, mmproj.as_deref(), gpu == GpuKind::None);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let child = launch_hidden(&exe, &arg_refs)?;
        servers.push(child);
    }
    if !opts.skip_whisper {
        on(Progress::Step("Starting whisper-server :8081".to_string()));
        let exe = find_exe(&whisper_dir, "whisper-server.exe")
            .or_else(|| find_exe(&whisper_dir, "server.exe"))
            .context("whisper-server.exe not found after install")?;
        if !stop_listener_on_port(WHISPER_PORT, &opts.root) {
            bail!(
                "port :8081 is in use by another application - close it (or stop that server) and retry the local-AI install"
            );
        }
        std::thread::sleep(Duration::from_millis(300));
        let bin = whisper_dir.join(WHISPER_FILE);
        let child = launch_hidden(
            &exe,
            &[
                "-m",
                &bin.to_string_lossy(),
                "--host",
                "127.0.0.1",
                "--port",
                WHISPER_PORT,
                "--inference-path",
                "/v1/audio/transcriptions",
            ],
        )?;
        servers.push(child);
    }

    // ---- wait for llama readiness + verify GPU offload --------------------
    let mut on_gpu = false;
    let mut effective_gpu = gpu;
    if !opts.skip_llama {
        on(Progress::Step("Waiting for the model to load".to_string()));
        // The user-facing message (shown as the tile's {e}) must be actionable RU;
        // the inner detail only reaches the log via {e:#}.
        const NOT_READY_RU: &str =
            "Локальная модель установилась, но не смогла запуститься на этом \
             компьютере (не успела прогреться). Попробуйте переустановить, либо \
             включите облачный AI в Настройках → AI.";
        // P0.2: fail (or fall back) if the model never loads or can't generate —
        // don't report success on a wedged server.
        // A stale listener can answer generic readiness while the child we just
        // launched has already failed to bind. Require both that exact child to
        // remain alive and that `/models` advertises the alias it was launched
        // with before this install can persist its profile.
        let ready: Result<()> = servers
            .first_mut()
            .is_some_and(|llama| wait_for_expected_llama(&alias, STRICT_LLAMA_READY_BUDGET, llama))
            .then_some(())
            .ok_or_else(|| {
                anyhow!("the newly launched llama-server did not become ready with model {alias}")
            });
        if let Err(e) = ready {
            // If a GPU launch cannot become ready, restore the always-installed
            // 12B fallback. A failed 26B first retries 12B with llama.cpp's
            // automatic GPU fit; a failed 12B (or failed GPU-fit retry) gets one
            // final CPU launch.
            if gpu != GpuKind::None {
                log::warn!(
                    "local-ai: selected launch not ready ({e:#}); falling back to Gemma 12B"
                );
                on(Progress::Step(
                    "Primary model did not start — using Gemma 12B".to_string(),
                ));
                // Free :8080 — owner-aware, this kills the failed GPU llama WE
                // launched. Then drop only its now-dead handle: llama is servers[0]
                // here (this block runs only when !skip_llama, and llama is pushed
                // before whisper). Do NOT pop() — that would kill the whisper server
                // (pushed last) and wedge its readiness wait below (CRITICAL fix).
                if !stop_listener_on_port(LLAMA_PORT, &opts.root) {
                    return Err(anyhow!(
                        "could not reclaim :8080 after the failed llama launch"
                    ))
                    .context(NOT_READY_RU);
                }
                if !servers.is_empty() {
                    // Reap the failed llama's handle (its process was already killed
                    // by the port-free above) so the Child isn't dropped unreaped
                    // (clippy::zombie_processes). Do NOT touch whisper (servers[1..]).
                    let mut dead = servers.remove(0);
                    let _ = dead.wait();
                }
                std::thread::sleep(Duration::from_millis(800));
                let exe2 = find_exe(&llama_dir, "llama-server.exe")
                    .context("llama-server.exe missing for 12B fallback")?;
                let gguf2 = llama_dir.join(GEMMA_FILE);
                let gguf2_s = gguf2.to_string_lossy().into_owned();
                let mmproj2 = mmproj_for_model(&llama_dir, &gguf2)
                    .map(|path| path.to_string_lossy().into_owned());
                let mut force_cpu = !prefer_quality;
                let fallback_args =
                    llama_server_args(&gguf2_s, GEMMA_FILE, mmproj2.as_deref(), force_cpu);
                let arg_refs: Vec<&str> = fallback_args.iter().map(String::as_str).collect();
                servers.push(launch_hidden(&exe2, &arg_refs)?);
                let fallback_ready: Result<()> = servers
                    .last_mut()
                    .is_some_and(|llama| {
                        wait_for_expected_llama(GEMMA_FILE, STRICT_LLAMA_READY_BUDGET, llama)
                    })
                    .then_some(())
                    .ok_or_else(|| anyhow!("the newly launched 12B fallback did not become ready"));
                if let Err(fallback_error) = fallback_ready {
                    if force_cpu {
                        return Err(fallback_error).context(NOT_READY_RU);
                    }
                    log::warn!(
                        "local-ai: Gemma 12B GPU launch not ready ({fallback_error:#}); retrying on CPU"
                    );
                    on(Progress::Step(
                        "GPU fallback did not start — retrying Gemma 12B on CPU".to_string(),
                    ));
                    if !stop_listener_on_port(LLAMA_PORT, &opts.root) {
                        return Err(anyhow!(
                            "could not reclaim :8080 after the failed 12B GPU launch"
                        ))
                        .context(NOT_READY_RU);
                    }
                    if let Some(mut dead) = servers.pop() {
                        let _ = dead.wait();
                    }
                    std::thread::sleep(Duration::from_millis(800));
                    force_cpu = true;
                    let cpu_args =
                        llama_server_args(&gguf2_s, GEMMA_FILE, mmproj2.as_deref(), true);
                    let cpu_refs: Vec<&str> = cpu_args.iter().map(String::as_str).collect();
                    servers.push(launch_hidden(&exe2, &cpu_refs)?);
                    if !servers.last_mut().is_some_and(|llama| {
                        wait_for_expected_llama(GEMMA_FILE, STRICT_LLAMA_READY_BUDGET, llama)
                    }) {
                        return Err(anyhow!(
                            "the newly launched 12B CPU fallback did not become ready"
                        ))
                        .context(NOT_READY_RU);
                    }
                }
                prefer_quality = false;
                if force_cpu {
                    effective_gpu = GpuKind::None;
                }
            } else {
                return Err(e).context(NOT_READY_RU);
            }
        }
        match effective_gpu {
            GpuKind::Nvidia => {
                on_gpu = verify_gpu_offload(24);
                let verdict = if on_gpu {
                    format!("GPU (CUDA {})", cuda_version.as_deref().unwrap_or("?"))
                } else {
                    "CPU (GPU offload not detected — update the NVIDIA driver)".to_string()
                };
                on(Progress::Gpu(verdict));
            }
            GpuKind::Other => {
                // Vulkan offload isn't visible to nvidia-smi; the model loaded +
                // generated above, so report GPU. The tester confirms the speedup.
                on_gpu = true;
                on(Progress::Gpu("GPU (Vulkan)".to_string()));
            }
            GpuKind::None => on(Progress::Gpu("CPU".to_string())),
        }
    }
    if !opts.skip_whisper {
        // P0.2: whisper had no strict readiness check after launch.
        on(Progress::Step("Waiting for whisper-server".to_string()));
        wait_ready(&format!("{WHISPER_BASE_URL}/models"), 60)
            .context("whisper-server did not become ready")?;
    }

    let ai_local_vision = !opts.skip_llama
        && !prefer_quality
        && mmproj_for_model(&llama_dir, &llama_dir.join(GEMMA_FILE)).is_some();
    Ok(LocalAiResult {
        ai_local_model: if prefer_quality {
            GEMMA26_FILE.to_string()
        } else {
            GEMMA_FILE.to_string()
        },
        ai_local_quality: prefer_quality,
        ai_local_vision,
        hardware_profile,
        // Only advertise the GigaAM dir if it actually completed — otherwise STT
        // stays cleanly on Whisper (the default) instead of pointing at a partial
        // GigaAM that would bail at session start.
        stt_gigaam_dir: if gigaam_ok {
            gigaam_dir.to_string_lossy().to_string()
        } else {
            String::new()
        },
        on_gpu,
        cuda_version,
        servers,
    })
}

/// One-shot reachability probe: true if the URL answers anything (even a 404),
/// i.e. a server is listening. A connection failure returns false.
fn is_reachable(url: &str) -> bool {
    run_capture("curl.exe", &["-s", "-o", "NUL", "--max-time", "2", url])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn models_list_expected_model(http_success: bool, body: &str, expected: &str) -> bool {
    http_success
        && serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|json| json.get("data")?.as_array().cloned())
            .is_some_and(|models| {
                models.iter().any(|model| {
                    model.get("id").and_then(serde_json::Value::as_str) == Some(expected)
                })
            })
}

fn completion_has_choice(http_success: bool, body: &str) -> bool {
    http_success
        && serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|json| json.get("choices")?.as_array().cloned())
            .is_some_and(|choices| {
                choices.iter().any(|choice| {
                    choice
                        .get("message")
                        .is_some_and(serde_json::Value::is_object)
                })
            })
}

fn expected_model_is_ready(
    models_http_success: bool,
    models_body: &str,
    expected: &str,
    completion_http_success: bool,
    completion_body: &str,
) -> bool {
    models_list_expected_model(models_http_success, models_body, expected)
        && completion_has_choice(completion_http_success, completion_body)
}

fn curl_success_body(args: &[&str]) -> Option<String> {
    run_capture("curl.exe", args).ok().and_then(|out| {
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    })
}

/// Readiness belongs to the exact llama child just launched, never to a
/// different child (such as Whisper) returned alongside it.
fn launched_llama_alive(child: &mut Child) -> bool {
    matches!(child.try_wait(), Ok(None))
}

#[cfg(windows)]
fn launched_llama_owns_listener(child: &mut Child) -> bool {
    if !launched_llama_alive(child) {
        return false;
    }
    let Ok(out) = run_capture("netstat", &["-ano", "-p", "tcp"]) else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let child_pid = child.id().to_string();
    listener_pids_on_port(&text, LLAMA_PORT)
        .iter()
        .any(|pid| *pid == child_pid)
}

#[cfg(not(windows))]
fn launched_llama_owns_listener(child: &mut Child) -> bool {
    launched_llama_alive(child)
}

fn wait_for_expected_llama(expected: &str, budget: Duration, llama: &mut Child) -> bool {
    wait_for_expected_model_at(LLAMA_BASE_URL, expected, budget, llama)
}

/// Strict readiness for a newly launched llama child at an OpenAI-compatible
/// endpoint. Keeping the endpoint explicit makes the stale-listener regression
/// testable without touching the real managed :8080 port.
fn wait_for_expected_model_at(
    base_url: &str,
    expected: &str,
    budget: Duration,
    llama: &mut Child,
) -> bool {
    let base_url = base_url.trim_end_matches('/');
    let models_url = format!("{base_url}/models");
    let completion_url = format!("{base_url}/chat/completions");
    let completion_body = serde_json::json!({
        "model": expected,
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 1,
    })
    .to_string();
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if !launched_llama_alive(llama) {
            return false;
        }
        let models = curl_success_body(&["-f", "-sS", "--max-time", "3", &models_url]);
        let completion = if models
            .as_deref()
            .is_some_and(|body| models_list_expected_model(true, body, expected))
        {
            curl_success_body(&[
                "-f",
                "-sS",
                "--max-time",
                "8",
                "-X",
                "POST",
                &completion_url,
                "-H",
                "Content-Type: application/json",
                "-d",
                &completion_body,
            ])
        } else {
            None
        };
        if launched_llama_owns_listener(llama)
            && expected_model_is_ready(
                models.is_some(),
                models.as_deref().unwrap_or_default(),
                expected,
                completion.is_some(),
                completion.as_deref().unwrap_or_default(),
            )
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    false
}

/// Stop local-AI servers that this app owns.
///
/// Child handles cover the normal in-app install / auto-start path. The port
/// sweep is a backstop for older versions and race windows where a managed
/// server is alive but no handle made it into AppState yet. The sweep is
/// owner-aware: only listeners whose executable lives under `root` are killed.
pub fn stop_managed_servers<I>(root: &Path, servers: I)
where
    I: IntoIterator<Item = Child>,
{
    terminate_servers(servers);
    let _ = stop_listener_on_port(LLAMA_PORT, root);
    let _ = stop_listener_on_port(WHISPER_PORT, root);
}

/// Terminate the given managed-server child processes (kill the whole tree)
/// WITHOUT sweeping any port. Used to clean up the children of a relaunch that
/// failed to bind, so a dead/wedged llama is reaped immediately instead of
/// leaking until quit — and without the port sweep that `stop_managed_servers`
/// does, which could kill a HEALTHY server on the other port (whisper :8081).
pub fn terminate_servers<I>(servers: I)
where
    I: IntoIterator<Item = Child>,
{
    for child in servers {
        terminate_child_tree(child);
    }
}

fn terminate_child_tree(mut child: Child) {
    #[cfg(windows)]
    {
        let pid = child.id().to_string();
        if !kill_pid_tree(&pid) {
            let _ = child.kill();
        }
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// Resolve the full exe path of a PID via PowerShell (always present on PATH;
/// `wmic` is deprecated). None when the process is gone or we can't read it
/// (e.g. an elevated/other-user process — in which case we conservatively treat
/// it as NOT ours and never kill it).
#[cfg(windows)]
fn exe_path_for_pid(pid: &str) -> Option<String> {
    let out = run_capture(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {pid} -ErrorAction SilentlyContinue).Path"),
        ],
    )
    .ok()?;
    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if p.is_empty() {
        None
    } else {
        Some(p)
    }
}

/// Free `port` of OUR orphaned server so a fresh one can bind. OWNER-AWARE
/// (audit P0.1): only a LISTENING process whose exe lives under `root` (our
/// install dir, e.g. `…\suflyor-local-ai`) is killed — a stranger's process on
/// the port is left ALIVE and logged. Returns `true` when the port is free of
/// any non-ours listener (so the caller may bind), `false` when a stranger holds
/// it (so the caller surfaces a port-conflict instead of stealing the port).
///
/// Why this matters: a stale projector-less llama-server orphaned by a
/// force-killed previous run keeps :8080; the new `--mmproj` server can't bind,
/// `wait_ready` still sees the old one answer, and F8 vision returns HTTP 500.
/// We must replace OUR orphan but never an unrelated app's server. Parses
/// `netstat -ano`.
#[cfg(windows)]
fn kill_pid_tree(pid: &str) -> bool {
    run_capture("taskkill", &["/T", "/F", "/PID", pid])
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn listener_pids_on_port<'a>(netstat: &'a str, port: &str) -> Vec<&'a str> {
    let suffix = format!(":{port}");
    let mut pids = Vec::new();
    for line in netstat.lines() {
        // Columns: Proto  LocalAddr  ForeignAddr  State  PID
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5
            && cols[3].eq_ignore_ascii_case("LISTENING")
            && cols[1].ends_with(suffix.as_str())
        {
            let pid = cols[4];
            if pid != "0" && !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }
    pids
}

#[cfg(windows)]
fn path_is_under_root(path: &str, root_lc: &str) -> bool {
    let root = root_lc.trim_end_matches(['\\', '/']);
    if root.is_empty() {
        return false;
    }
    let path_lc = path.to_lowercase();
    path_lc == root
        || path_lc.starts_with(&format!("{root}\\"))
        || path_lc.starts_with(&format!("{root}/"))
}

#[cfg(windows)]
fn stop_listener_on_port(port: &str, root: &Path) -> bool {
    let Ok(out) = run_capture("netstat", &["-ano", "-p", "tcp"]) else {
        log::warn!("port {port}: netstat failed; cannot safely reclaim the listener");
        return false;
    };
    if !out.status.success() {
        log::warn!(
            "port {port}: netstat exited unsuccessfully; cannot safely reclaim the listener"
        );
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let suffix = format!(":{port}");
    let root_lc = root.to_string_lossy().to_lowercase();
    let mut killed: Vec<String> = Vec::new();
    let mut free_of_strangers = true;
    for line in text.lines() {
        // Columns: Proto  LocalAddr  ForeignAddr  State  PID
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5
            && cols[3].eq_ignore_ascii_case("LISTENING")
            && cols[1].ends_with(suffix.as_str())
        {
            let pid = cols[4];
            if pid == "0" || killed.iter().any(|k| k == pid) {
                continue;
            }
            match exe_path_for_pid(pid) {
                Some(p) if path_is_under_root(&p, &root_lc) => {
                    if kill_pid_tree(pid) {
                        killed.push(pid.to_string());
                    } else {
                        log::warn!("port {port}: could not stop managed listener PID {pid}");
                        free_of_strangers = false;
                    }
                }
                other => {
                    log::warn!(
                        "port {port}: PID {pid} (exe {}) is not under our install dir — leaving it alive",
                        other.as_deref().unwrap_or("<unknown>")
                    );
                    free_of_strangers = false;
                }
            }
        }
    }
    free_of_strangers
}

#[cfg(not(windows))]
fn stop_listener_on_port(_port: &str, _root: &Path) -> bool {
    true
}

/// Free the llama port (:8080) owner-aware so a model switch can relaunch the
/// server with the OTHER GGUF — covers a server we manage AND one an external
/// `setup-local-ai.ps1` started (same exe under `root`), which `ensure_servers`
/// would otherwise see still answering and skip. Whisper (:8081) is untouched,
/// so switching the LLM never disturbs local STT. Returns true if the port is
/// free of FOREIGN listeners afterwards (one we can't/won't kill → false).
pub fn free_llama_port(root: &Path) -> bool {
    stop_listener_on_port(LLAMA_PORT, root)
}

/// Honest outcome of a [`switch_local_model`] so the UI never claims success
/// when the new server didn't actually come up (review v0.18.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSwitch {
    /// :8080 now answers with the requested GGUF loaded.
    Switched,
    /// The requested profile failed strict readiness, but the previous profile
    /// was restored and its child handles are returned.
    RolledBack,
    /// The requested primary could not be started, but the RAM-safe fallback
    /// passed readiness and is now serving. Callers must persist that downgrade.
    FallbackStarted,
    /// A FOREIGN process holds :8080 (started outside our `root`) we won't
    /// force-kill — the OLD model keeps serving, so the switch did NOT happen.
    PortBusy,
    /// The requested 26B file is absent, has the wrong size, or fails its
    /// pinned SHA-256 review; the serving model is not stopped.
    TargetUnavailable,
    /// The requested primary is outside the owner's confirmed VRAM/RAM matrix.
    HardwareUnsupported,
    /// Freed + relaunched but the server never became reachable in time
    /// (missing binary/GGUF, failed bind, or still cold-loading past the wait).
    FailedToStart,
}

#[must_use]
pub const fn switch_commits_choice(outcome: ModelSwitch) -> bool {
    matches!(outcome, ModelSwitch::Switched)
}

/// Transactionally switch between the 12B fallback and 26B primary. A target is
/// accepted only after `/models` advertises its exact alias and a minimal chat
/// completion succeeds. On failure the previous model is relaunched.
#[must_use]
pub fn switch_local_model(
    root: &Path,
    previous_quality: bool,
    prefer_quality: bool,
    want_whisper: bool,
) -> (ModelSwitch, Vec<Child>) {
    // This is a worker-only launch boundary. A UI presence query is deliberately
    // stat-only, but loading the primary always performs (or reuses) an exact
    // SHA-256 review before we stop the currently serving fallback.
    if prefer_quality {
        if !primary_26b_allowed_on_current_hardware() {
            return (ModelSwitch::HardwareUnsupported, Vec::new());
        }
        if !quality_model_verified(root) {
            return (ModelSwitch::TargetUnavailable, Vec::new());
        }
    }
    // A foreign owner we can't kill means the old model stays up — don't lie.
    if !free_llama_port(root) {
        return (ModelSwitch::PortBusy, Vec::new());
    }
    // Let the OS release the port before the relaunch binds it.
    std::thread::sleep(Duration::from_millis(800));
    let expected = active_local_model_name(root, prefer_quality);
    let mut started = ensure_servers(root, true, want_whisper, prefer_quality);
    if started
        .first_mut()
        .is_some_and(|llama| wait_for_expected_llama(&expected, STRICT_LLAMA_READY_BUDGET, llama))
    {
        return (ModelSwitch::Switched, started);
    }
    terminate_servers(started);
    if !free_llama_port(root) {
        return (ModelSwitch::FailedToStart, Vec::new());
    }
    std::thread::sleep(Duration::from_millis(800));
    let rollback_quality = effective_verified_local_quality(root, previous_quality);
    let rollback_expected = active_local_model_name(root, rollback_quality);
    let mut rollback = ensure_servers(root, true, want_whisper, rollback_quality);
    if rollback.first_mut().is_some_and(|llama| {
        wait_for_expected_llama(&rollback_expected, STRICT_LLAMA_READY_BUDGET, llama)
    }) {
        (ModelSwitch::RolledBack, rollback)
    } else {
        terminate_servers(rollback);
        (ModelSwitch::FailedToStart, Vec::new())
    }
}

/// True if llama-server is answering on :8080 (even a 503 "loading" counts —
/// the process is alive and bound). A `false` means a truly dead port
/// (connection refused), which is the ONLY thing the boot/watchdog recovery
/// acts on. Public so the runtime watchdog can distinguish "server crashed"
/// from "server answered with an error" before deciding to relaunch.
#[must_use]
pub fn llama_reachable() -> bool {
    is_reachable(&format!("{LLAMA_BASE_URL}/models"))
}

/// Make :8080 actually serve — the robust primitive shared by boot and the
/// runtime watchdog. If llama already answers (even a mid-load 503) we leave
/// it ALONE: killing a healthy/warming server would defeat warm-up and drop
/// in-flight requests. Only a truly-dead port triggers a clean owner-aware
/// free + relaunch via [`switch_local_model`], which POLLS until the fresh
/// server answers and returns the honest [`ModelSwitch`]. Whisper (:8081) is
/// never touched here (boot launches STT separately). Call from a worker
/// thread (blocks up to ~21 s on the relaunch+poll path).
#[must_use]
pub fn ensure_llama_serving(root: &Path, prefer_quality: bool) -> (ModelSwitch, Vec<Child>) {
    if llama_reachable() {
        // Alive (serving or cold-loading) — do not disturb.
        return (ModelSwitch::Switched, Vec::new());
    }
    restart_llama_server(root, prefer_quality)
}

/// Cold-start or reinstall recovery: reclaim only a managed listener, launch
/// the effective persisted choice, and require exact-model readiness.
#[must_use]
pub fn restart_llama_server(root: &Path, prefer_quality: bool) -> (ModelSwitch, Vec<Child>) {
    if !free_llama_port(root) {
        return (ModelSwitch::PortBusy, Vec::new());
    }
    std::thread::sleep(Duration::from_millis(800));
    let effective_quality = prefer_quality
        && primary_26b_allowed_on_current_hardware()
        && effective_verified_local_quality(root, true);
    let expected = active_local_model_name(root, effective_quality);
    let mut started = ensure_servers(root, true, false, effective_quality);
    if started
        .first_mut()
        .is_some_and(|llama| wait_for_expected_llama(&expected, STRICT_LLAMA_READY_BUDGET, llama))
    {
        return (
            if prefer_quality && !effective_quality {
                ModelSwitch::FallbackStarted
            } else {
                ModelSwitch::Switched
            },
            started,
        );
    }
    terminate_servers(started);
    if !effective_quality || !free_llama_port(root) {
        return (ModelSwitch::FailedToStart, Vec::new());
    }
    std::thread::sleep(Duration::from_millis(800));
    let fallback_expected = active_local_model_name(root, false);
    let mut fallback = ensure_servers(root, true, false, false);
    if fallback.first_mut().is_some_and(|llama| {
        wait_for_expected_llama(&fallback_expected, STRICT_LLAMA_READY_BUDGET, llama)
    }) {
        (ModelSwitch::FallbackStarted, fallback)
    } else {
        terminate_servers(fallback);
        (ModelSwitch::FailedToStart, Vec::new())
    }
}

/// Friendly, compact label for a local model basename.
#[must_use]
pub fn local_model_label(basename: &str) -> String {
    let l = basename.to_ascii_lowercase();
    if l.contains("26b") {
        "Gemma 26B-A4B".to_string()
    } else if l.contains("12b") {
        "Gemma 12B".to_string()
    } else if l.contains("e4b") || l.contains("e2b") || l.contains("4b") {
        "Gemma 4B".to_string()
    } else if l.contains("gemma") {
        "Gemma".to_string()
    } else {
        basename
            .trim_end_matches(".gguf")
            .trim_end_matches(".bin")
            .split(['-', '.', '/', ' ', ':'])
            .find(|s| !s.is_empty())
            .unwrap_or("—")
            .to_string()
    }
}

/// Basename of the candidate profile Settings should display. This is stat-only
/// and safe on the UI thread; the worker-side launcher independently resolves
/// the exact verified GGUF through [`selected_llama_gguf`].
#[must_use]
pub fn active_local_model_name(root: &Path, prefer_quality: bool) -> String {
    if effective_local_quality(root, prefer_quality) {
        GEMMA26_FILE.to_string()
    } else {
        fallback_model_name(root)
    }
}

fn fallback_model_name(root: &Path) -> String {
    fallback_llama_gguf(&root.join("llama.cpp"))
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| GEMMA_FILE.to_string())
}

/// Absolute path the optional 26B primary GGUF lives at (whether or not it
/// has been downloaded yet) under an install `root`.
#[must_use]
pub fn quality_gguf_path(root: &Path) -> PathBuf {
    root.join("llama.cpp").join(GEMMA26_FILE)
}

/// Fast, stat-only 26B presence check for Settings and component rows. Exact
/// SHA-256 validation is intentionally deferred to [`quality_model_verified`],
/// which is called only from worker-side launch/switch paths. Opening Settings
/// must never stream the 10.5-GB model from disk.
#[must_use]
pub fn quality_model_present(root: &Path) -> bool {
    file_has_expected_size(&quality_gguf_path(root), GEMMA26_SIZE)
}

/// Resolve a persisted primary preference to the candidate profile displayed by
/// Settings. This is intentionally stat-only; worker launch paths use
/// [`effective_verified_local_quality`] before loading the primary.
#[must_use]
pub fn effective_local_quality(root: &Path, requested_quality: bool) -> bool {
    requested_quality && quality_model_present(root)
}

/// Worker-side counterpart to [`effective_local_quality`]. The 26B becomes a
/// launch target only after the exact pinned SHA-256 check succeeds.
fn effective_verified_local_quality(root: &Path, requested_quality: bool) -> bool {
    requested_quality && quality_model_verified(root)
}

/// Repair persisted bundled-model state without touching custom local servers.
/// Returns `true` when the caller must save the config. This is used at boot and
/// when switching back to Suflyor's endpoint so a vanished/partial primary
/// cannot leave a stale model or prep-model id in requests. Same-size integrity
/// failures are repaired by [`repair_managed_model_state_after_verification`].
pub fn repair_managed_model_state(cfg: &mut crate::config::Config, root: &Path) -> bool {
    if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {
        return false;
    }
    let quality = effective_local_quality(root, cfg.ai_local_quality);
    repair_managed_model_state_for_quality(cfg, root, quality)
}

/// Worker-only version of [`repair_managed_model_state`]. It is called after a
/// launch attempt, so persistence records the 12B fallback if the exact 26B
/// SHA-256 review rejected a same-size replacement.
pub fn repair_managed_model_state_after_verification(
    cfg: &mut crate::config::Config,
    root: &Path,
) -> bool {
    if !is_managed_llama_endpoint(&cfg.ai_local_base_url) {
        return false;
    }
    let quality = effective_verified_local_quality(root, cfg.ai_local_quality);
    repair_managed_model_state_for_quality(cfg, root, quality)
}

/// Whether the configured local text endpoint may safely receive an image
/// attachment. Managed profiles are checked from their actual selected model
/// and projector state instead of trusting the persisted UI flag: the 26B-A4B
/// primary is deliberately text-only in this candidate.
#[must_use]
pub fn local_vision_enabled(cfg: &crate::config::Config, root: &Path) -> bool {
    cfg.ai_local_vision
        && (!is_managed_llama_endpoint(&cfg.ai_local_base_url)
            || managed_model_vision_capable(
                root,
                effective_local_quality(root, cfg.ai_local_quality),
            ))
}

fn repair_managed_model_state_for_quality(
    cfg: &mut crate::config::Config,
    root: &Path,
    quality: bool,
) -> bool {
    let model = if quality {
        GEMMA26_FILE.to_string()
    } else {
        fallback_model_name(root)
    };
    let vision_capable = managed_model_vision_capable(root, quality);
    let local_vision = cfg.ai_local_vision && vision_capable;
    let vision_provider = if !vision_capable && vision_routes_to_managed_llama(cfg) {
        "off".to_string()
    } else {
        cfg.vision_provider.clone()
    };
    let changed = cfg.ai_local_base_url != LLAMA_BASE_URL
        || cfg.ai_local_quality != quality
        || cfg.ai_local_model != model
        || !cfg.ai_local_prep_model.is_empty()
        || cfg.ai_local_vision != local_vision
        || cfg.vision_provider != vision_provider;
    // The managed server is launched on 127.0.0.1. Canonicalise legacy
    // localhost/[::1] spellings so persisted requests use the same listener.
    cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();
    cfg.ai_local_quality = quality;
    cfg.ai_local_model = model;
    cfg.ai_local_prep_model.clear();
    cfg.ai_local_vision = local_vision;
    cfg.vision_provider = vision_provider;
    changed
}

/// True when either the current 12B fallback or the legacy 4B fallback is
/// complete. The latter remains launchable only to preserve an existing install
/// until the user runs the new installer.
#[must_use]
pub fn base_model_present(root: &Path) -> bool {
    let llama_dir = root.join("llama.cpp");
    file_has_expected_size(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE)
        || file_has_expected_size(&llama_dir.join(LEGACY_GEMMA_FILE), LEGACY_GEMMA_SIZE)
}

/// The conventional GigaAM model directory under the local-AI root
/// (`<root>/gigaam-v3`) — the SAME location the installer writes to. The
/// readiness API uses this when `config.stt_gigaam_dir` is unset, so it agrees
/// with where a fresh install lands (single source of truth for the path).
#[must_use]
pub fn gigaam_default_dir(root: &Path) -> PathBuf {
    root.join("gigaam-v3")
}

/// True when a complete GigaAM model lives in `dir` (`model.int8.onnx` present
/// at the pinned size). Mirrors the installer's own "needs download?" size check
/// so the readiness API can't disagree with it; a truncated file reads as absent.
#[must_use]
pub fn gigaam_model_present(dir: &Path) -> bool {
    file_len(&dir.join("model.int8.onnx")) >= GIGAAM_MODEL_SIZE
}

/// The 26B vision sidecar is deliberately not shipped in this candidate: its
/// runtime memory has not been confirmed. Keep the old UI contract false.
#[must_use]
pub fn quality_vision_present(_root: &Path) -> bool {
    false
}

/// See [`quality_vision_present`].
#[must_use]
pub fn quality_vision_supported(_root: &Path) -> bool {
    false
}

/// Resource text for the selected endpoint/model. The only numbers shown are
/// the owner-approved hardware matrix and exact disk sizes. Vision memory is
/// intentionally explicit as unknown for both bundled models.
#[must_use]
pub fn local_model_resource_warning(root: &Path, base_url: &str, model_id: &str) -> String {
    if !is_managed_llama_endpoint(base_url) {
        return "[!] Требования к памяти выбранной внешней модели неизвестны.".to_string();
    }
    let lower = model_id.to_ascii_lowercase();
    if lower.contains("26b-a4b") {
        let profile = detected_hardware_model_profile(false);
        let matrix = match profile {
            HardwareModelProfile::Primary26Vram8 => "профиль 8 ГБ VRAM / 32 ГБ RAM",
            HardwareModelProfile::Primary26Vram12 => "профиль 12 ГБ VRAM / 24-32 ГБ RAM",
            HardwareModelProfile::Primary26Vram16 => "профиль 16 ГБ VRAM / 32 ГБ RAM",
            HardwareModelProfile::Unknown | HardwareModelProfile::Fallback12B => {
                "профиль железа не подтверждён"
            }
        };
        format!(
            "[!] Gemma 26B-A4B: {:.1} GiB на диске; {matrix}. Память для vision: неизвестно.",
            GEMMA26_SIZE as f64 / GIB as f64
        )
    } else if lower.contains("12b") {
        format!(
            "[!] Gemma 12B QAT fallback: {:.1} GiB на диске. Матрица 8 ГБ VRAM / 16 ГБ RAM подтверждена владельцем. Память для vision: неизвестно.",
            GEMMA_SIZE as f64 / GIB as f64
        )
    } else if lower.contains("e4b") || lower.contains("4b") {
        format!(
            "[!] Legacy Gemma 4B: {:.1} GiB на диске. Память для vision: неизвестно.",
            LEGACY_GEMMA_SIZE as f64 / GIB as f64
        )
    } else if model_id.trim().is_empty() && base_model_present(root) {
        local_model_resource_warning(root, base_url, &fallback_model_name(root))
    } else {
        "[!] Требования к памяти выбранной локальной модели неизвестны.".to_string()
    }
}

/// Pick which llama GGUF to load: the 26B only when requested and complete;
/// otherwise the always-installed 12B fallback.
/// Centralised so `ensure_servers` and `install`'s launch agree. Does the disk
/// check then defers the choice to the pure [`pick_llama_gguf`] (unit-tested
/// without materialising a 6 GB file).
fn selected_llama_gguf(llama_dir: &Path, prefer_quality: bool) -> PathBuf {
    if !prefer_quality {
        // The RAM-safe fallback never needs the optional primary, so do not
        // stream 10.5 GB merely to start the 12B server.
        return fallback_llama_gguf(llama_dir);
    }
    // Selection is a worker-only launch boundary. The exact pinned hash is
    // rechecked here (or served from the matching metadata cache), never from
    // the Settings/UI path.
    let present =
        cached_pinned_file_matches(&llama_dir.join(GEMMA26_FILE), GEMMA26_SIZE, GEMMA26_SHA256);
    pick_llama_gguf(llama_dir, prefer_quality, present)
}

/// Prefer the current 12B fallback, but keep the previous 4B artifact
/// launchable during an in-place upgrade that has not downloaded 12B yet.
fn fallback_llama_gguf(llama_dir: &Path) -> PathBuf {
    complete_fallback_llama_gguf(llama_dir).unwrap_or_else(|| llama_dir.join(GEMMA_FILE))
}

/// Complete fallback GGUF available for a launch-time model-load check. The
/// current 12B is preferred, while a complete legacy 4B remains supported until
/// the user installs 12B. `None` means a binary-only verification is the best
/// safe check available.
fn complete_fallback_llama_gguf(llama_dir: &Path) -> Option<PathBuf> {
    let current = llama_dir.join(GEMMA_FILE);
    if file_has_expected_size(&current, GEMMA_SIZE) {
        Some(current)
    } else {
        let legacy = llama_dir.join(LEGACY_GEMMA_FILE);
        file_has_expected_size(&legacy, LEGACY_GEMMA_SIZE).then_some(legacy)
    }
}

/// Pure model-choice rule (no I/O): 26B only when wanted and present.
fn pick_llama_gguf(llama_dir: &Path, prefer_quality: bool, quality_present: bool) -> PathBuf {
    if prefer_quality && quality_present {
        llama_dir.join(GEMMA26_FILE)
    } else {
        llama_dir.join(GEMMA_FILE)
    }
}

/// Download (resumable) + SHA-verify the optional 26B model into `root`, on
/// demand from Settings. Mirrors the installer's download→verify discipline
/// (P1.5: a tampered byte-stream fails the pinned hash and the partial file is
/// left for a clean re-pull, never launched). Does NOT restart the server —
/// the caller flips `ai_local_quality` and restarts so the new GGUF loads.
///
/// # Errors
/// Hardware outside the confirmed matrix, network/disk failure, cancellation,
/// or a SHA-256 mismatch after download.
pub fn download_quality_model(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    if !primary_26b_allowed_on_current_hardware() {
        bail!("Gemma 26B-A4B requires a confirmed VRAM/RAM hardware profile");
    }
    let llama_dir = root.join("llama.cpp");
    std::fs::create_dir_all(&llama_dir)
        .with_context(|| format!("create llama dir {}", llama_dir.display()))?;
    let dest = llama_dir.join(GEMMA26_FILE);
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if reuse_if_available(
        &dest,
        GEMMA26_SIZE,
        GEMMA26_SHA256,
        &[home.join("llama.cpp").join(GEMMA26_FILE)],
    ) {
        on(Progress::Step("Основная модель уже загружена".to_string()));
    } else {
        curl_resumable(
            GEMMA26_URL,
            &dest,
            GEMMA26_SIZE,
            "Gemma 26B-A4B",
            cancel,
            on,
        )?;
    }
    verify_sha256(&dest, GEMMA26_SHA256, "Gemma 26B-A4B model")?;
    cache_quality_model_verification(&dest, true);
    Ok(())
}

/// No 26B vision download is exposed until its memory profile is confirmed.
pub fn download_quality_vision(
    _root: &Path,
    _cancel: &AtomicBool,
    _on: &dyn Fn(Progress),
) -> Result<()> {
    bail!("26B vision memory profile is unknown")
}

/// The installed llama.cpp release build number (the `bNNNN` tag), read from the
/// `.llama-build` stamp `install`/the engine-updater write next to the binaries.
/// `None` when the stamp is missing/unparseable (an old install) → treated as
/// too-old by the gemma4uv gate (so we stay safe, never crash).
fn installed_llama_build(llama_dir: &Path) -> Option<u32> {
    parse_build_tag(&std::fs::read_to_string(llama_dir.join(".llama-build")).ok()?)
}

/// Parse a llama.cpp build tag (`b9626`, or a bare `9626`) into its number.
/// `None` for anything unparseable (an old/garbage stamp) → callers treat that
/// as "too old", staying on the safe side of the gemma4uv gate.
fn parse_build_tag(tag: &str) -> Option<u32> {
    tag.trim().trim_start_matches('b').parse::<u32>().ok()
}

/// Record which llama.cpp build is installed (the `bNNNN` tag, e.g. `b9626`).
/// Best-effort: a write failure just leaves the gate conservative (12B vision
/// stays off until the next successful install/update). Trims to keep the stamp
/// a clean single token regardless of what the GitHub API returned.
fn write_build_stamp(llama_dir: &Path, tag: &str) {
    let tag = tag.trim();
    if !tag.is_empty() {
        let _ = std::fs::write(llama_dir.join(".llama-build"), tag);
    }
}

/// True if the installed llama.cpp is new enough to load the 12B's "gemma4uv"
/// projector (build >= [`GEMMA4UV_MIN_BUILD`]). A missing/old stamp → false.
fn llama_build_supports_gemma4uv(llama_dir: &Path) -> bool {
    installed_llama_build(llama_dir).is_some_and(|b| b >= GEMMA4UV_MIN_BUILD)
}

/// The vision projector to attach for `gguf`, if present and loadable. Only the
/// 12B fallback has a pinned projector in this candidate. The 26B stays
/// text-only until its vision memory profile is confirmed.
fn mmproj_for_model(llama_dir: &Path, gguf: &Path) -> Option<PathBuf> {
    let name = gguf.file_name().and_then(|n| n.to_str())?;
    if name == GEMMA_FILE && llama_build_supports_gemma4uv(llama_dir) {
        let proj = llama_dir.join(MMPROJ_FILE);
        (file_len(&proj) == MMPROJ_SIZE).then_some(proj)
    } else {
        None
    }
}

/// Whether the effective managed profile can accept screenshots on the server
/// Suflyor launches. The 26B primary has no pinned projector in this candidate.
fn managed_model_vision_capable(root: &Path, quality: bool) -> bool {
    if quality || !base_model_present(root) {
        return false;
    }
    let llama_dir = root.join("llama.cpp");
    mmproj_for_model(&llama_dir, &fallback_llama_gguf(&llama_dir)).is_some()
}

/// True when F8's configured route resolves back to Suflyor's managed text
/// server. Besides `same`, a `local` vision provider with an empty (or explicit
/// managed) URL inherits `ai_local_base_url` and is the same unsafe route for a
/// text-only profile.
fn vision_routes_to_managed_llama(cfg: &crate::config::Config) -> bool {
    match cfg.vision_provider.as_str() {
        "same" => true,
        "local" => {
            let base_url = if cfg.vision_local_base_url.trim().is_empty() {
                &cfg.ai_local_base_url
            } else {
                &cfg.vision_local_base_url
            };
            is_managed_llama_endpoint(base_url)
        }
        _ => false,
    }
}

// ---- engine auto-update (keep llama.cpp fresh) -----------------------------

/// How long between unattended boot-time "is there a newer llama.cpp?" checks.
/// llama.cpp tags builds almost daily; a weekly cadence keeps the engine current
/// (so the 12B's gemma4uv vision + perf fixes land) without re-pulling ~160 MB
/// every launch. The manual Settings button bypasses this throttle.
const ENGINE_UPDATE_THROTTLE_SECS: u64 = 7 * 24 * 60 * 60;

/// Scratch port the verify-before-swap launch binds (never the live :8080).
const ENGINE_VERIFY_PORT: &str = "8077";

/// Outcome of an engine-update run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineUpdate {
    /// Already on the latest build (or newer) — nothing downloaded.
    UpToDate { build: u32 },
    /// Swapped the engine from `from` (None if the old build was unstamped) to
    /// the new `to` build. The caller should (re)start the server to pick it up.
    Updated { from: Option<u32>, to: u32 },
    /// A newer build exists but we did NOT swap (verify failed, download failed,
    /// or no engine installed). The live engine is UNCHANGED. `reason` is a short
    /// English log string — never surface it verbatim to the user.
    Skipped { reason: String },
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether boot should run an engine-update check now: only when an engine is
/// already installed AND the weekly throttle has elapsed (a missing stamp =
/// never checked = yes). Keeps boot from hitting GitHub on every launch.
#[must_use]
pub fn should_check_engine_update(root: &Path) -> bool {
    let llama_dir = root.join("llama.cpp");
    if find_exe(&llama_dir, "llama-server.exe").is_none() {
        return false; // first install is install()'s job, not the updater's.
    }
    match std::fs::read_to_string(llama_dir.join(".update-check"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        Some(last) => now_unix().saturating_sub(last) >= ENGINE_UPDATE_THROTTLE_SECS,
        None => true,
    }
}

/// Record that an engine-update check ran now (regardless of outcome), so the
/// throttle holds until the next interval. Best-effort.
pub fn mark_engine_update_checked(root: &Path) {
    let stamp = root.join("llama.cpp").join(".update-check");
    let _ = std::fs::write(stamp, now_unix().to_string());
}

/// The installed llama.cpp build number, for display ("движок bNNNN"). `None`
/// when unstamped (an engine installed before stamping existed).
#[must_use]
pub fn installed_engine_build(root: &Path) -> Option<u32> {
    installed_llama_build(&root.join("llama.cpp"))
}

/// Update the installed llama.cpp engine to the latest ggml-org release **if
/// newer**, verifying the new binaries actually run on THIS machine BEFORE
/// swapping the live ones — a regressed build can never brick local AI.
///
/// Sequence: compare installed `.llama-build` vs the latest release tag → if not
/// newer, [`EngineUpdate::UpToDate`]. Otherwise download the GPU/CPU-matched
/// build (+cudart) into a staging dir, smoke-launch it on a scratch port with the
/// smallest installed model and wait for `/v1/models` (proves the exe + DLLs +
/// CUDA init + a model load all work). Only on success: stop the live server,
/// back up the binaries we overwrite, copy the new ones in (models untouched),
/// stamp the build. On ANY failure the live engine is left UNCHANGED.
///
/// The caller MUST serialize this against the watchdog / install / switch (hold
/// `local_ai_lock`) and (re)start the server afterwards on `Updated`.
pub fn update_llama_engine(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<EngineUpdate> {
    let llama_dir = root.join("llama.cpp");
    if find_exe(&llama_dir, "llama-server.exe").is_none() {
        return Ok(EngineUpdate::Skipped {
            reason: "no engine installed".into(),
        });
    }
    let installed = installed_llama_build(&llama_dir);

    on(Progress::Step("Checking for a newer llama.cpp".into()));
    let rel = github_release(LLAMA_REPO)?;
    let latest =
        parse_build_tag(&rel.tag_name).context("latest llama.cpp release has no build tag")?;
    if installed.is_some_and(|cur| cur >= latest) {
        return Ok(EngineUpdate::UpToDate { build: latest });
    }
    bail_if_cancelled(cancel)?;

    // Download the matching build into a CLEAN staging dir (never touch the live
    // binaries until the new ones are proven good). The staging dir is a SIBLING
    // of `llama.cpp` (NOT inside it) so `find_exe(&llama_dir, …)` — which recurses
    // — can never pick up the half-downloaded staged binary and launch it on :8080.
    let gpu = detect_gpu();
    let pick = pick_llama(&rel.assets, gpu)?;
    let staging = root.join(".llama-staging-update");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).context("create engine staging dir")?;
    let blabel = format!("llama.cpp {} (update)", rel.tag_name);
    download_and_extract(
        &pick.build_url,
        pick.build_size,
        &blabel,
        &staging,
        cancel,
        on,
    )?;
    if let Some(cu) = &pick.cudart_url {
        download_and_extract(cu, pick.cudart_size, "CUDA runtime", &staging, cancel, on)?;
    }
    bail_if_cancelled(cancel)?;

    // Verify-before-swap: prove the staged engine runs on THIS box.
    on(Progress::Step("Verifying the new engine".into()));
    let staged_exe = match find_exe(&staging, "llama-server.exe") {
        Some(e) => e,
        None => {
            let _ = std::fs::remove_dir_all(&staging);
            return Ok(EngineUpdate::Skipped {
                reason: "staged build missing llama-server.exe".into(),
            });
        }
    };
    if !verify_engine_runs(&staged_exe, &llama_dir, gpu != GpuKind::None, cancel) {
        let _ = std::fs::remove_dir_all(&staging);
        return Ok(EngineUpdate::Skipped {
            reason: "new build failed to launch — kept the current engine".into(),
        });
    }

    // Swap. Free the live :8080 first so its binaries aren't file-locked.
    on(Progress::Step("Installing the new engine".into()));
    let _ = stop_listener_on_port(LLAMA_PORT, root);
    let backup_dir = root.join(
        installed
            .map(|b| format!("llama.cpp.backup-b{b}"))
            .unwrap_or_else(|| "llama.cpp.backup-prev".into()),
    );
    // Windows frees a just-killed process's file handles ASYNCHRONOUSLY, so
    // llama-server.exe can stay locked (os error 32 — sharing violation) for a
    // beat after the stop. swap_engine_binaries copies the .exe FIRST and bails
    // before touching any DLL when it's locked, so a failed attempt leaves the
    // live engine intact and is safe to retry. A fixed 500 ms was too short on
    // some machines → the swap failed on the FIRST "Update engine" click and
    // worked only on the 2nd (by then the server was already down). Retry with
    // backoff until the handle frees.
    let mut result = swap_engine_binaries(&staging, &llama_dir, &backup_dir);
    for attempt in 1..=7u32 {
        if result.is_ok() {
            break;
        }
        bail_if_cancelled(cancel)?;
        on(Progress::Step(format!(
            "Waiting for the old engine to close… ({attempt}/7)"
        )));
        std::thread::sleep(Duration::from_millis(750));
        result = swap_engine_binaries(&staging, &llama_dir, &backup_dir);
    }
    result.context("swap engine binaries (engine still locked after retries)")?;
    // P1-2: post-swap sanity — the live engine MUST now exist before we stamp the
    // build as updated. swap_engine_binaries already bails when the staged build
    // has no llama-server.exe; this guards any residual "swap returned Ok but the
    // live exe isn't there" path so the UI never reports a phantom update.
    if !llama_dir.join("llama-server.exe").is_file() {
        let _ = std::fs::remove_dir_all(&staging);
        return Ok(EngineUpdate::Skipped {
            reason: "engine install produced no llama-server.exe — kept the current engine".into(),
        });
    }
    let _ = std::fs::remove_dir_all(&staging);
    write_build_stamp(&llama_dir, &rel.tag_name);
    // fs-audit #1 — verify-before-swap means only the immediately-previous
    // engine is ever a rollback candidate, so keep just the newest backup; the
    // rest accumulated unbounded (~150-300 MB each) before this.
    let _ = prune_engine_backups(root, 1);

    Ok(EngineUpdate::Updated {
        from: installed,
        to: latest,
    })
}

/// Launch a staged engine on a scratch port with the installed fallback model
/// and wait for `/v1/models`. Proves binary + DLL + CUDA + model-load all work
/// (the regression class that would brick local AI). A legacy 4B-only install
/// must exercise that model too; fall back to a `--version` integrity check only
/// when neither supported fallback is complete. Always reaps the test server.
fn verify_engine_runs(
    staged_exe: &Path,
    llama_dir: &Path,
    use_gpu: bool,
    cancel: &AtomicBool,
) -> bool {
    if cancel.load(Ordering::Relaxed) {
        return false;
    }
    let root = llama_dir.parent().unwrap_or(llama_dir);
    // Skip the projector: we're verifying the BINARY, not vision, and a
    // text-only load is lighter on VRAM/time. Prefer the current 12B fallback,
    // but retain the legacy 4B load check for an in-place upgrade where 12B has
    // not been installed yet.
    let model = complete_fallback_llama_gguf(llama_dir);
    let Some(model) = model else {
        // No weights yet — at least prove the image + its DLLs load.
        return run_capture(&staged_exe.to_string_lossy(), &["--version"])
            .map(|o| o.status.success())
            .unwrap_or(false);
    };
    let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);
    let model_s = model.to_string_lossy().into_owned();
    let mut args = vec![
        "-m".to_string(),
        model_s,
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        ENGINE_VERIFY_PORT.to_string(),
        "-c".to_string(),
        "2048".to_string(),
    ];
    if !use_gpu {
        args.push("-ngl".to_string());
        args.push("0".to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let child = match launch_hidden(staged_exe, &arg_refs) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let ok = wait_ready(
        &format!("http://127.0.0.1:{ENGINE_VERIFY_PORT}/v1/models"),
        90,
    )
    .is_ok();
    terminate_child_tree(child);
    let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);
    ok
}

/// Copy the staged engine files over the live install dir, backing up each live
/// file we overwrite. The `.gguf` models stay put (they're not in `staging`).
///
/// The `.exe` files are copied FIRST: on Windows you cannot overwrite a running
/// image, so if `llama-server.exe` is still locked the copy fails and we bail
/// having touched no DLL — never a half-swapped (new-DLL/old-exe) engine.
fn swap_engine_binaries(staging: &Path, live: &Path, backup: &Path) -> Result<()> {
    std::fs::create_dir_all(backup).context("create engine backup dir")?;
    // Only the engine binaries — copying a stray non-binary (e.g. a license txt)
    // over the live dir is pointless and risks shadowing a model/stamp. The
    // llama.cpp + cudart zips ship exactly these.
    let is_engine_file = |p: &Path| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("exe") || e.eq_ignore_ascii_case("dll"))
    };
    // RECURSIVELY collect engine files. The llama.cpp / cudart zips sometimes nest
    // the binaries in a subfolder (or the two zips extract into different subdirs);
    // a direct-children-only read then copies ZERO files while verify-before-swap
    // (which finds llama-server.exe recursively) still passes — stamping a phantom
    // "updated" with the live engine unchanged (P1-2). Reject a duplicate engine
    // filename across locations rather than guess which copy to install.
    let mut by_name: std::collections::BTreeMap<std::ffi::OsString, PathBuf> = Default::default();
    let mut stack = vec![staging.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_engine_file(&p) {
                if let Some(name) = p.file_name().map(|n| n.to_os_string()) {
                    if by_name.insert(name.clone(), p).is_some() {
                        bail!(
                            "ambiguous staged engine: duplicate {} in the downloaded archive",
                            name.to_string_lossy()
                        );
                    }
                }
            }
        }
    }
    // Must install at least the server binary — otherwise this is not a real engine
    // and we must NOT report success / write the build stamp (P1-2).
    let has_server = by_name.keys().any(|n| {
        n.to_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("llama-server.exe"))
    });
    if !has_server {
        bail!("staged build has no llama-server.exe to install");
    }
    let mut files: Vec<PathBuf> = by_name.into_values().collect();
    files.sort_by_key(|p| {
        // exes (false → sorts first) before DLLs, so the locked-live-exe copy is
        // attempted first and a sharing violation bails before any DLL is touched
        // (keeps the live engine intact + the attempt safely retryable).
        !p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    });
    for src in files {
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = live.join(name);
        if dst.exists() {
            let _ = std::fs::copy(&dst, backup.join(name));
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("install engine file {}", name.to_string_lossy()))?;
    }
    Ok(())
}

/// Keep the newest `keep` engine rollback backups, removing older ones. The
/// updater names each backup after the PREVIOUS build (`llama.cpp.backup-b<N>`,
/// or `llama.cpp.backup-prev` when the old build had no stamp), so they are
/// uniquely named and accumulated forever before this. Only the updater's OWN
/// naming is pruned — a hand-made `llama.cpp.backup-*` that doesn't match
/// `-b<N>` / `-prev` (e.g. a manual `-may` snapshot) is deliberately left alone.
/// Best-effort; returns the number of backup dirs removed.
fn prune_engine_backups(root: &Path, keep: usize) -> usize {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    let mut backups: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_dir() {
                return None;
            }
            let name = p.file_name()?.to_str()?;
            // Match ONLY the updater's own scheme: `llama.cpp.backup-b<digits>`
            // (the previous build number) or exactly `llama.cpp.backup-prev`. The
            // digit check (not a bare `starts_with("…-b")`) means a hand-made
            // `llama.cpp.backup-baseline` / `-may` snapshot is never pruned.
            let ours = name == "llama.cpp.backup-prev"
                || name.strip_prefix("llama.cpp.backup-b").is_some_and(|rest| {
                    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
                });
            if !ours {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, p))
        })
        .collect();
    if backups.len() <= keep {
        return 0;
    }
    backups.sort_by_key(|b| std::cmp::Reverse(b.0)); // newest first
    let mut removed = 0usize;
    for (_, p) in backups.into_iter().skip(keep) {
        match std::fs::remove_dir_all(&p) {
            Ok(()) => {
                removed += 1;
                log::info!("local AI: pruned old engine backup {}", p.display());
            }
            Err(e) => log::warn!("local AI: cannot prune engine backup {}: {e}", p.display()),
        }
    }
    removed
}

/// Idempotent boot-time GC of orphaned engine-update artifacts: a
/// `.llama-staging-update` dir left by a crashed/killed mid-update (the updater
/// otherwise only reclaims it on the NEXT attempt — which may never come if the
/// user stops updating or switches to cloud AI), plus stale rollback backups
/// beyond the newest one. Safe to call unconditionally at boot: a live update
/// holds `local_ai_lock`, so any staging dir present here is by definition
/// orphaned. Best-effort. (fs-audit #1.)
pub fn sweep_orphaned_engine_artifacts(root: &Path) {
    let staging = root.join(".llama-staging-update");
    if staging.exists() {
        match std::fs::remove_dir_all(&staging) {
            Ok(()) => log::info!("local AI: swept orphaned engine staging dir"),
            Err(e) => log::warn!("local AI: cannot sweep engine staging dir: {e}"),
        }
    }
    let pruned = prune_engine_backups(root, 1);
    if pruned > 0 {
        log::info!("local AI: swept {pruned} stale engine backup(s) at boot");
    }
}

/// On launch, start the local servers the config points at but that aren't
/// already running (the app kills its servers on quit, so after a restart
/// following an in-app install they'd be down). Uses the binaries + models
/// under `root` (the installer/script layout); skips a server whose port
/// already answers. `prefer_quality` picks the 12B GGUF when present (see
/// [`selected_llama_gguf`]). Best-effort — a missing binary just means that
/// server is not started. Returns the launched child handles for kill-on-quit.
#[must_use]
pub fn ensure_servers(
    root: &Path,
    want_llama: bool,
    want_whisper: bool,
    prefer_quality: bool,
) -> Vec<Child> {
    let mut started = Vec::new();
    // Any GPU (NVIDIA CUDA or AMD/Intel Vulkan build) → let current llama.cpp
    // auto-fit layers; CPU-only explicitly disables offload.
    let use_gpu = detect_gpu() != GpuKind::None;
    // NOTE: deliberately launch-only — do NOT kill+relaunch a server that is
    // already answering. Live smoke showed that relaunching the (warm) server on
    // startup defeats the model warm-up (the warm-up then hits a cold-loading
    // server → HTTP 503) — and an orphan launched WITH --mmproj already has the
    // projector, so the relaunch is usually needless. The rare projector-less
    // orphan (old install force-killed) is accepted; install()'s owner-aware
    // stop_listener_on_port still frees :8080 for a fresh install.
    if want_llama && !is_reachable(&format!("{LLAMA_BASE_URL}/models")) {
        let llama_dir = root.join("llama.cpp");
        let gguf = selected_llama_gguf(&llama_dir, prefer_quality);
        // The MATCHING vision projector for the selected model, if downloaded
        // (12B QAT ↔ mmproj-12b-F16). A mismatched projector
        // crashes llama-server on model load; a missing one → the model runs
        // text-only (F8 vision then prompts to download the right projector).
        let mmproj_s =
            mmproj_for_model(&llama_dir, &gguf).map(|p| p.to_string_lossy().into_owned());
        if let Some(exe) = find_exe(&llama_dir, "llama-server.exe") {
            if gguf.exists() {
                let gguf_s = gguf.to_string_lossy().into_owned();
                let alias = gguf
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let args = llama_server_args(&gguf_s, &alias, mmproj_s.as_deref(), !use_gpu);
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                if let Ok(child) = launch_hidden(&exe, &arg_refs) {
                    started.push(child);
                }
            }
        }
    }
    if want_whisper && !is_reachable(&format!("{WHISPER_BASE_URL}/models")) {
        let whisper_dir = root.join("whisper.cpp");
        let bin = whisper_dir.join(WHISPER_FILE);
        let exe = find_exe(&whisper_dir, "whisper-server.exe")
            .or_else(|| find_exe(&whisper_dir, "server.exe"));
        if let Some(exe) = exe {
            if bin.exists() {
                if let Ok(child) = launch_hidden(
                    &exe,
                    &[
                        "-m",
                        &bin.to_string_lossy(),
                        "--host",
                        "127.0.0.1",
                        "--port",
                        WHISPER_PORT,
                        "--inference-path",
                        "/v1/audio/transcriptions",
                    ],
                ) {
                    started.push(child);
                }
            }
        }
    }
    started
}

/// Native launcher arguments shared by install, boot, watchdog and switching.
/// GPU launches deliberately omit `-ngl`: current llama.cpp auto-fits unset
/// offload parameters to free VRAM, which is required for the 8/12/16 GB hybrid
/// 26B profiles. CPU recovery is the only path that forces `-ngl 0`.
fn llama_server_args(
    model: &str,
    alias: &str,
    mmproj: Option<&str>,
    force_cpu: bool,
) -> Vec<String> {
    let mut args = vec![
        "-m".to_string(),
        model.to_string(),
        "--alias".to_string(),
        alias.to_string(),
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        LLAMA_PORT.to_string(),
        "-c".to_string(),
        "8192".to_string(),
        "--jinja".to_string(),
    ];
    if force_cpu {
        args.push("-ngl".to_string());
        args.push("0".to_string());
    }
    if let Some(projector) = mmproj {
        args.push("--mmproj".to_string());
        args.push(projector.to_string());
    }
    args
}

// ---- GitHub release asset selection ---------------------------------------

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    /// The release tag, e.g. `b9626` for a llama.cpp build. Used as the
    /// `.llama-build` stamp so we can later compare installed vs latest and
    /// gate gemma4uv (12B vision) on a new-enough engine.
    #[serde(default)]
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Clone)]
struct LlamaPick {
    build_url: String,
    build_size: u64,
    cudart_url: Option<String>,
    cudart_size: u64,
    version: Option<String>,
}

fn github_release(repo: &str) -> Result<GhRelease> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = run_capture(
        "curl.exe",
        &[
            "-sL",
            "--retry",
            "6",
            "--retry-all-errors",
            "--max-time",
            "40",
            &url,
        ],
    )
    .with_context(|| format!("fetch latest release for {repo}"))?;
    if !out.status.success() {
        bail!("GitHub API request for {repo} failed");
    }
    serde_json::from_slice(&out.stdout).with_context(|| format!("parse release JSON for {repo}"))
}

/// Just the downloadable assets of the latest release (whisper path + tests
/// don't need the tag). The llama path uses [`github_release`] to also stamp
/// the build tag.
fn github_assets(repo: &str) -> Result<Vec<GhAsset>> {
    Ok(github_release(repo)?.assets)
}

/// Parse the CUDA version out of a llama.cpp build asset name, e.g.
/// `llama-b9410-bin-win-cuda-13.3-x64.zip` -> (13, 3).
fn cuda_version_of(name: &str) -> Option<(u32, u32)> {
    let after = name.split("-bin-win-cuda-").nth(1)?; // "13.3-x64.zip"
    let ver = after.strip_suffix("-x64.zip")?; // "13.3"
    let mut it = ver.split('.');
    let maj: u32 = it.next()?.parse().ok()?;
    let min: u32 = it.next()?.parse().ok()?;
    Some((maj, min))
}

/// Pick the llama.cpp Windows build for the detected GPU (Баг2): NVIDIA → newest
/// CUDA build + matching cudart (RTX 50-series/Blackwell needs CUDA ≥ 12.8, so we
/// take the HIGHEST CUDA version); AMD/Intel → the Vulkan build; none (or no
/// matching GPU asset in this release) → the CPU build.
fn pick_llama(assets: &[GhAsset], gpu: GpuKind) -> Result<LlamaPick> {
    if gpu == GpuKind::Nvidia {
        let best = assets
            .iter()
            .filter(|a| a.name.starts_with("llama-"))
            .filter_map(|a| cuda_version_of(&a.name).map(|v| (v, a)))
            .max_by_key(|(v, _)| *v);
        if let Some(((maj, min), build)) = best {
            let needle = format!("-cuda-{maj}.{min}-x64.zip");
            let cudart = assets
                .iter()
                .find(|a| a.name.starts_with("cudart-") && a.name.ends_with(&needle))
                .ok_or_else(|| anyhow!("no cudart asset for CUDA {maj}.{min}"))?;
            return Ok(LlamaPick {
                build_url: build.browser_download_url.clone(),
                build_size: build.size,
                cudart_url: Some(cudart.browser_download_url.clone()),
                cudart_size: cudart.size,
                version: Some(format!("{maj}.{min}")),
            });
        }
        // No CUDA asset in this release → fall through to the CPU build.
    }
    if gpu == GpuKind::Other {
        if let Some(vk) = assets
            .iter()
            .find(|a| a.name.starts_with("llama-") && a.name.ends_with("-bin-win-vulkan-x64.zip"))
        {
            return Ok(LlamaPick {
                build_url: vk.browser_download_url.clone(),
                build_size: vk.size,
                cudart_url: None,
                cudart_size: 0,
                version: Some("Vulkan".to_string()),
            });
        }
        // No Vulkan asset in this release → fall through to the CPU build.
    }
    let cpu = assets
        .iter()
        .find(|a| a.name.starts_with("llama-") && a.name.ends_with("-bin-win-cpu-x64.zip"))
        .ok_or_else(|| anyhow!("no llama CPU build asset"))?;
    Ok(LlamaPick {
        build_url: cpu.browser_download_url.clone(),
        build_size: cpu.size,
        cudart_url: None,
        cudart_size: 0,
        version: None,
    })
}

/// Parse the CUDA version from a whisper cuBLAS asset name, e.g.
/// `whisper-cublas-12.4.0-bin-x64.zip` -> `(12, 4, 0)`.
fn whisper_cublas_version_of(name: &str) -> Option<(u32, u32, u32)> {
    let after = name.strip_prefix("whisper-cublas-")?; // "12.4.0-bin-x64.zip"
    let ver = after.strip_suffix("-bin-x64.zip")?; // "12.4.0"
    let mut it = ver.split('.');
    let maj: u32 = it.next()?.parse().ok()?;
    let min: u32 = it.next()?.parse().ok()?;
    let patch: u32 = it.next()?.parse().ok()?;
    Some((maj, min, patch))
}

/// Pick the whisper.cpp Windows build: the highest-version cuBLAS (GPU) build when
/// a GPU is available, else the plain CPU build (`whisper-bin-x64.zip`). Unlike
/// llama.cpp the cuBLAS zip BUNDLES the CUDA runtime DLLs, so there is no separate
/// cudart download. Verified on an RTX 5060 Ti (Blackwell, sm_120): cublas-12.4
/// GPU-accelerates via PTX JIT (whisper_init: use gpu = 1, model loads into VRAM).
/// Returns (url, size).
fn pick_whisper(assets: &[GhAsset], force_cpu: bool) -> Result<(String, u64)> {
    if !force_cpu {
        let best = assets
            .iter()
            .filter_map(|a| whisper_cublas_version_of(&a.name).map(|v| (v, a)))
            .max_by_key(|(v, _)| *v);
        if let Some((_, build)) = best {
            return Ok((build.browser_download_url.clone(), build.size));
        }
    }
    assets
        .iter()
        .find(|a| a.name == "whisper-bin-x64.zip")
        .map(|a| (a.browser_download_url.clone(), a.size))
        .ok_or_else(|| anyhow!("no whisper-bin-x64.zip asset"))
}

// ---- downloads + extraction (curl.exe + tar.exe) ---------------------------

/// Allow-list for release-asset downloads (mirrors update::is_trusted_download).
/// GitHub serves release zips from github.com (302 → the *.githubusercontent
/// hosts). Defends against a tampered GitHub-API response pointing the download
/// elsewhere. ggml-org release zips are unsigned, so Authenticode isn't an
/// option — this host pin is the available mitigation (audit: only the updater
/// had it before).
fn is_trusted_release_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/")
        || url.starts_with("https://release-assets.githubusercontent.com/")
}

fn download_and_extract(
    url: &str,
    size: u64,
    label: &str,
    dest_dir: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    if !is_trusted_release_url(url) {
        bail!("refusing to download server binary from untrusted URL");
    }
    let name = url.rsplit('/').next().unwrap_or("download.zip");
    let zip = dest_dir.join(name);
    // Download the zip with LIVE byte progress + cancel support (was a silent
    // blocking curl before, so the bar sat empty during the binary downloads).
    curl_resumable(url, &zip, size, label, cancel, on)?;
    extract_zip(&zip, dest_dir)?;
    let _ = std::fs::remove_file(&zip);
    Ok(())
}

/// Full path to the libarchive `tar.exe` under System32 (Win10 1803+), so PATH
/// order can't substitute a different `tar` for archive extraction (P1-1). Falls
/// back to a bare `tar.exe` only if System32's copy is somehow absent. (Mirrors
/// the TTS/OCR installers' system_bsdtar.)
fn system_tar() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join("System32").join("tar.exe"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("tar.exe"))
}

/// SECURITY (P1-1): true iff an archive entry path stays inside the extraction
/// dir. Rejects absolute paths, a leading `/` or `\` (incl. UNC), a drive prefix
/// (`C:`), and any `..` component — the zip-slip vectors a poisoned release asset
/// could use. Empty lines (tar -tf trailing newline) are ignored (safe).
fn archive_entry_is_safe(entry: &str) -> bool {
    let e = entry.trim().replace('\\', "/");
    if e.is_empty() {
        return true;
    }
    if e.starts_with('/') {
        return false; // posix-absolute or (normalised) UNC / leading backslash
    }
    let b = e.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return false; // drive-qualified (C:\...)
    }
    // Reject any `..` traversal component. Windows also strips trailing spaces
    // from a name, so ".. " / "..  " resolve to ".." too — collapse trailing
    // spaces before the compare. (A bare "." is the harmless current-dir, and
    // tar may emit "./"-prefixed entries, so it must NOT be rejected.)
    !e.split('/').any(|c| c.trim_end_matches(' ') == "..")
}

fn extract_zip(zip: &Path, dest_dir: &Path) -> Result<()> {
    // System32-pinned bsdtar (P1-1) so a `tar.exe` earlier on PATH can't be run.
    let tar = system_tar();
    let tar_s = tar.to_string_lossy().to_string();
    // SECURITY (P1-1): list entries and reject any that would escape dest BEFORE
    // extracting — a poisoned release zip could otherwise zip-slip via `..` or an
    // absolute/drive path. (Windows symlink creation needs a privilege the app
    // lacks, so a symlink entry just fails to extract rather than escaping.)
    let listing = run_capture(&tar_s, &["-tf", &zip.to_string_lossy()])
        .with_context(|| format!("list archive {}", zip.display()))?;
    if !listing.status.success() {
        bail!("could not list archive: {}", zip.display());
    }
    for entry in String::from_utf8_lossy(&listing.stdout).lines() {
        if !archive_entry_is_safe(entry) {
            bail!("refusing unsafe archive entry: {entry}");
        }
    }
    // bsdtar (tar.exe) on Windows 10 1803+ extracts zip archives.
    let status = launch_hidden_wait(
        &tar_s,
        &[
            "-xf",
            &zip.to_string_lossy(),
            "-C",
            &dest_dir.to_string_lossy(),
        ],
    )?;
    if !status.success() {
        bail!("extract failed: {}", zip.display());
    }
    Ok(())
}

/// Resilient resumable download to a known size. Re-runs `curl -C -` (which
/// resumes from the current file length) until the file reaches `expected`,
/// polling the file size meanwhile for live progress. Mirrors the script's
/// `Save-Model` loop (the HuggingFace Xet CDN resets open-ended GETs).
fn curl_resumable(
    url: &str,
    out: &Path,
    expected: u64,
    label: &str,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    for _ in 0..60 {
        bail_if_cancelled(cancel)?;
        let cur = file_len(out);
        if cur >= expected {
            break;
        }
        let mut child = spawn_hidden(
            "curl.exe",
            &[
                "-L",
                "--retry",
                "10",
                "--retry-all-errors",
                "--retry-delay",
                "2",
                "-C",
                "-",
                "-o",
                &out.to_string_lossy(),
                url,
            ],
        )?;
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                bail!("{CANCEL_SENTINEL}");
            }
            match child.try_wait().context("poll curl")? {
                Some(_) => break,
                None => {
                    on(Progress::Bytes {
                        label: label.to_string(),
                        done: file_len(out),
                        total: expected,
                    });
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }
    let cur = file_len(out);
    if cur < expected {
        bail!(
            "download incomplete: {} ({cur}/{expected} bytes)",
            out.display()
        );
    }
    on(Progress::Bytes {
        label: label.to_string(),
        done: expected,
        total: expected,
    });
    Ok(())
}

/// Download a SMALL artifact (e.g. the GigaAM `vocab.txt`) with retries.
///
/// Two failure modes are handled, because they bit a tester (2026-06-05: HF
/// served an HTML error page for `vocab.txt`, leaving the install with no usable
/// vocab → STT dead, fixed only by hand-copying a good file):
///  - `-f` makes curl treat an HTTP 4xx/5xx as an ERROR, so its own
///    `--retry-all-errors` actually re-fetches a 404/503/rate-limit (without
///    `-f`, curl downloads the error BODY at exit 0 and never retries);
///  - a 200-with-HTML-body soft-error (not an HTTP error, so `-f` can't catch
///    it) is detected by the leading-`<` guard and re-attempted at the APP level
///    a few times with a short delay (transient HF hiccups usually clear).
///
/// A real model/vocab artifact never begins with `<`, so the content check is a
/// cheap, size-pin-free sanity guard. The bad/partial file is removed between
/// attempts and on final failure.
fn curl_small(url: &str, out: &Path) -> Result<()> {
    const ATTEMPTS: u32 = 3;
    let mut last_err = format!("download failed: {}", out.display());
    for attempt in 1..=ATTEMPTS {
        let status = launch_hidden_wait(
            "curl.exe",
            &[
                "-fsL",
                "--retry",
                "8",
                "--retry-all-errors",
                "-o",
                &out.to_string_lossy(),
                url,
            ],
        )?;
        if status.success() && file_len(out) > 0 {
            // Reject a CDN/HTTP error page that landed with a 200 body.
            let looks_html = std::fs::read(out).ok().and_then(|b| b.first().copied()) == Some(b'<');
            if !looks_html {
                return Ok(());
            }
            last_err = format!("download looks like an HTML error page: {}", out.display());
        }
        // Clean up the partial/bad file before the next attempt (or before bail).
        let _ = std::fs::remove_file(out);
        if attempt < ATTEMPTS {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    bail!("{last_err} (after {ATTEMPTS} attempts)");
}

// ---- GPU verification + readiness ------------------------------------------

/// True if `nvidia-smi`'s compute-apps list mentions `llama-server`.
fn parse_compute_apps(stdout: &str) -> bool {
    stdout
        .lines()
        .any(|l| l.to_ascii_lowercase().contains("llama-server"))
}

fn verify_gpu_offload(tries: u32) -> bool {
    for _ in 0..tries {
        std::thread::sleep(Duration::from_secs(5));
        if let Ok(out) = run_capture(
            "nvidia-smi",
            &[
                "--query-compute-apps=process_name,used_memory",
                "--format=csv,noheader",
            ],
        ) {
            if parse_compute_apps(&String::from_utf8_lossy(&out.stdout)) {
                return true;
            }
        }
    }
    false
}

/// Poll an OpenAI-style `/models` endpoint until it answers (server ready) or
/// the budget runs out. Errors when the server never became reachable within
/// the budget — audit P0.2: install used to report success even when the model
/// never loaded.
fn wait_ready(url: &str, max_secs: u64) -> Result<()> {
    let deadline = max_secs / 2;
    for _ in 0..deadline {
        if let Ok(out) = run_capture("curl.exe", &["-s", "-o", "NUL", "--max-time", "2", url]) {
            if out.status.success() {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    bail!("server at {url} did not become ready within {max_secs}s")
}

/// Beyond reachability: verify the llama server lists a model AND can actually
/// generate. A reachable `/models` alone isn't enough — a wedged or broken
/// model still answers `/models` but fails real requests. Audit P0.2.
///
/// On a weak or virtualised machine the weights are still warming up after the
/// port opens: llama.cpp binds :8080 and serves `/models` long before the model
/// finishes loading, returning HTTP 503 ("loading model") to BOTH `/models` and
/// a generation request until it's ready. We therefore POLL the WHOLE readiness
/// — `/models` must list a loaded model AND a 1-token generation must succeed —
/// until both pass OR a wall-clock budget is spent. The budget is wall-clock
/// (not an attempt count) so a server that *hangs* each request can't over-run.
/// A heartbeat keeps the install status ticking so the wait doesn't look frozen.
///
/// (v0.10.5 — extends v0.10.4, which only retried the GENERATION step and so
/// still false-failed at the `/models did not return a model list` check on a
/// box where the model hadn't finished loading when the check first ran. That
/// false failure aborted the install BEFORE `apply_result`, leaving both the
/// gemma model AND the GigaAM dir UNSET in config — a tester hit exactly this.)
fn verify_llama_ready(on: &dyn Fn(Progress)) -> Result<()> {
    let start = Instant::now();
    let budget = Duration::from_secs(240); // ~4 min warm-up on a slow/VM box
                                           // String::new() is read on the bail path if the very first iteration lists a
                                           // model but the generation curl errors before `last` is reassigned, so this
                                           // is NOT a dead store.
    let mut last = String::new();
    loop {
        // Step 1: /models must list a loaded model. While the weights load,
        // llama.cpp answers 503 "loading" here too (no "data") — so this is part
        // of the poll, not a one-shot check (the v0.10.4 gap).
        let models_ok = match run_capture(
            "curl.exe",
            &["-s", "--max-time", "5", &format!("{LLAMA_BASE_URL}/models")],
        ) {
            Ok(o) => {
                let body = String::from_utf8_lossy(&o.stdout);
                if body.contains("\"data\"") {
                    true
                } else {
                    last = body.trim().to_string();
                    false
                }
            }
            Err(_) => false,
        };
        // Step 2: only once a model is listed, prove it actually generates (the
        // server accepts /chat/completions without a model field — uses the
        // loaded one). A 1-token reply containing "choices" = genuinely ready.
        if models_ok {
            if let Ok(s) = run_capture(
                "curl.exe",
                &[
                    "-s",
                    "--max-time",
                    "20",
                    "-X",
                    "POST",
                    &format!("{LLAMA_BASE_URL}/chat/completions"),
                    "-H",
                    "Content-Type: application/json",
                    "-d",
                    r#"{"messages":[{"role":"user","content":"hi"}],"max_tokens":1}"#,
                ],
            ) {
                last = String::from_utf8_lossy(&s.stdout).trim().to_string();
                if last.contains("choices") {
                    return Ok(());
                }
            }
        }
        let elapsed = start.elapsed();
        if elapsed >= budget {
            break;
        }
        // Not ready yet: a warming model replies 503 "loading"; an empty/timed-out
        // body means the request was refused before a reply. Tick the status (so
        // the UI shows movement) + log it for the tester, then wait and retry.
        let secs = elapsed.as_secs();
        on(Progress::Step(format!(
            "Waiting for the model to load… ({secs}s)"
        )));
        eprintln!(
            "[local-ai] llama not ready after {secs}s (models_ok={models_ok}), retrying in 5s…"
        );
        std::thread::sleep(Duration::from_secs(5));
    }
    // Keep a short, secret-free snippet of the last reply in the error chain so
    // the LOG (printed with {e:#}) shows WHY. The tile shows the caller's
    // actionable RU context, not this technical detail. If the server never
    // produced ANY body (curl errored/timed out on every probe), `last` is still
    // empty — say so explicitly instead of logging a blank "(last reply: )".
    let snippet: String = if last.is_empty() {
        "no reply (curl error or timeout on every probe)".to_string()
    } else {
        last.chars().take(160).collect()
    };
    bail!("llama never became ready within the warm-up budget (last reply: {snippet})");
}

// ---- process + fs helpers --------------------------------------------------

fn preflight() -> Result<()> {
    if run_capture("curl.exe", &["--version"]).is_err() {
        bail!("curl.exe not found (needs Windows 10 1803+)");
    }
    if run_capture(&system_tar().to_string_lossy(), &["--version"]).is_err() {
        bail!("tar.exe not found (needs Windows 10 1803+)");
    }
    Ok(())
}

fn file_len(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

fn file_has_expected_size(path: &Path, expected_size: u64) -> bool {
    file_len(path) == expected_size
}

/// Exact pinned-file verification. Size is only a fast rejection; a same-sized
/// corrupted or replaced model must never be launched.
fn pinned_file_matches(path: &Path, expected_size: u64, expected_sha256: &str) -> bool {
    file_has_expected_size(path, expected_size)
        && sha256_hex_of(path).is_some_and(|hash| hash.eq_ignore_ascii_case(expected_sha256))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
struct PinnedFileVerification {
    stamp: FileStamp,
    matches: bool,
}

/// Hashing a 10.5-GB primary is valid only outside the UI path. Cache its
/// exact result by file metadata so a switch's preflight and its subsequent
/// launch select do not stream the model twice. A changed size or mtime forces
/// a new SHA-256 review before the file can be loaded.
static PINNED_FILE_VERIFICATIONS: OnceLock<Mutex<HashMap<PathBuf, PinnedFileVerification>>> =
    OnceLock::new();

fn file_stamp(path: &Path) -> Option<FileStamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

fn cached_pinned_file_matches(path: &Path, expected_size: u64, expected_sha256: &str) -> bool {
    let Some(stamp) = file_stamp(path) else {
        return false;
    };
    if stamp.len != expected_size {
        return false;
    }
    let cache = PINNED_FILE_VERIFICATIONS.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let cached = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(result) = cached.get(path) {
            if result.stamp == stamp {
                return result.matches;
            }
        }
    }
    let matches = pinned_file_matches(path, expected_size, expected_sha256);
    // Do not cache a result for bytes that changed while they were being read.
    if file_stamp(path).as_ref() != Some(&stamp) {
        return false;
    }
    let mut cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cached.insert(
        path.to_path_buf(),
        PinnedFileVerification { stamp, matches },
    );
    matches
}

/// Record a completed download/install SHA check for later worker-side launch
/// selection. This is never consulted by the UI presence query.
fn cache_quality_model_verification(path: &Path, matches: bool) {
    let Some(stamp) = file_stamp(path) else {
        return;
    };
    let cache = PINNED_FILE_VERIFICATIONS.get_or_init(|| Mutex::new(HashMap::new()));
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(
            path.to_path_buf(),
            PinnedFileVerification { stamp, matches },
        );
}

/// Remove an exact-size file that has already failed a worker-side SHA review.
///
/// This deliberately does no hashing itself: callers must first establish that
/// the bytes are invalid. Keeping a same-size rejected primary makes the
/// Settings presence check hide the download action and strands the user with
/// no recovery path. Forget the cache too, so a fresh download is always
/// reviewed as new bytes even on a coarse-mtime filesystem.
fn discard_rejected_pinned_file(path: &Path, expected_size: u64) {
    if !file_has_expected_size(path, expected_size) {
        return;
    }
    if std::fs::remove_file(path).is_ok() {
        if let Some(cache) = PINNED_FILE_VERIFICATIONS.get() {
            cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(path);
        }
        log::warn!("local AI: removed primary model after failed pinned SHA-256 review");
    }
}

fn quality_model_verified(root: &Path) -> bool {
    let path = quality_gguf_path(root);
    let verified = cached_pinned_file_matches(&path, GEMMA26_SIZE, GEMMA26_SHA256);
    if !verified {
        // Settings deliberately uses a cheap size-only presence check. Once a
        // worker-side SHA review rejects a same-size file, remove it so that
        // presence check exposes the normal re-download control. Keeping this
        // here covers both an explicit profile switch and cold-start fallback.
        discard_rejected_pinned_file(&path, GEMMA26_SIZE);
    }
    verified
}

/// Bail with the cancel sentinel if the user requested cancellation.
fn bail_if_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        bail!("{CANCEL_SENTINEL}");
    }
    Ok(())
}

/// If `dest` already holds the full file, keep it. Otherwise look through
/// `candidates` for a complete copy and hard-link it into `dest` (instant on
/// the same volume; falls back to a byte copy). Returns true if `dest` now has
/// the full file, so the caller can skip the download — lets the installer
/// reuse a model the user already has elsewhere instead of re-fetching it.
///
/// A candidate is adopted ONLY when its SHA-256 matches `expected_sha256` (P1.5
/// regression fix): matching by size alone would hard-link a wrong-but-right-
/// sized file into `dest`, which then fails the post-download verify, gets
/// deleted, and is re-adopted from the same candidate on the NEXT run — a
/// permanent, retry-proof install dead-end. Hashing the candidate first means a
/// bad one is skipped and the installer falls through to a fresh download. A
/// bad `dest` (adopted on size at the top) is still caught + deleted by the
/// caller's `verify_sha256`, so a re-run re-downloads it.
fn reuse_if_available(
    dest: &Path,
    expected: u64,
    expected_sha256: &str,
    candidates: &[PathBuf],
) -> bool {
    if file_len(dest) >= expected {
        return true;
    }
    for cand in candidates {
        if cand.as_path() != dest
            && file_len(cand) >= expected
            && sha256_hex_of(cand)
                .map(|h| h.eq_ignore_ascii_case(expected_sha256))
                .unwrap_or(false)
        {
            let _ = std::fs::remove_file(dest);
            if std::fs::hard_link(cand, dest).is_ok() || std::fs::copy(cand, dest).is_ok() {
                return file_len(dest) >= expected;
            }
        }
    }
    false
}

const GIB: u64 = 1_073_741_824;
/// Flat disk allowance for the llama.cpp build zip + cudart + their extraction
/// (exact size isn't known until the GitHub API call). Whisper's cuBLAS zip
/// bundles its runtime so it needs a little less.
const LLAMA_BINARIES_ALLOWANCE: u64 = 1_500_000_000;
const WHISPER_BINARIES_ALLOWANCE: u64 = 1_000_000_000;

/// Stream a file through SHA-256, returning lowercase hex. None on an open/read
/// error (the caller decides whether that's fatal). Shared by `verify_sha256`
/// (the post-download gate) and `reuse_if_available` (the candidate check).
fn sha256_hex_of(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

/// P1.5 — verify a downloaded OR size-reused model against its pinned SHA-256
/// (the HuggingFace LFS object id). On mismatch the file at `path` is DELETED and
/// the install fails. `reuse_if_available` independently hash-verifies a reuse
/// CANDIDATE before adopting it, so a wrong candidate is never linked in here —
/// together they guarantee a re-run either re-downloads or fails cleanly, and
/// never silently accepts bad bytes.
fn verify_sha256(path: &Path, expected_hex: &str, label: &str) -> Result<()> {
    let got = sha256_hex_of(path).with_context(|| format!("open {} to verify", path.display()))?;
    if !got.eq_ignore_ascii_case(expected_hex) {
        let _ = std::fs::remove_file(path);
        bail!(
            "{label} failed its SHA-256 integrity check — the file was corrupt or tampered and has been removed; retry the local-AI install"
        );
    }
    log::info!("{label} sha256 verified");
    Ok(())
}

/// Best-effort free bytes on the volume backing `path`. Shells out (consistent
/// with this module's nvidia-smi / netstat / curl calls) to PowerShell for a
/// culture-invariant integer — fsutil / dir print localized grouped numbers that
/// break parsing on a non-English Windows. None when the query fails, so the
/// caller skips the pre-check rather than blocking a possibly-valid install.
fn free_bytes_on_volume(path: &Path) -> Option<u64> {
    let root = path.ancestors().last().unwrap_or(path);
    let script = format!(
        "[System.IO.DriveInfo]::new([string]'{}').AvailableFreeSpace",
        root.to_string_lossy()
    );
    let out = run_capture(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
    )
    .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u64>()
        .ok()
}

/// P1.5 — bail before downloading if the volume can't hold `need` bytes (+10%
/// headroom for extraction temp + slack). Reports the expected vs available
/// figures via `on`. A failed free-space query is non-fatal (the per-download
/// completion check still guards a truly full disk).
fn ensure_disk_space(root: &Path, need: u64, on: &dyn Fn(Progress)) -> Result<()> {
    if need == 0 {
        return Ok(());
    }
    let want = need.saturating_add(need / 10);
    let Some(free) = free_bytes_on_volume(root) else {
        return Ok(());
    };
    on(Progress::Step(format!(
        "Disk check: ~{} GB required, {} GB free",
        want.div_ceil(GIB),
        free / GIB
    )));
    if free < want {
        bail!(
            "not enough free disk space — the local AI needs about {} GB on the drive holding {}, but only {} GB is free; free up space and retry",
            want.div_ceil(GIB),
            root.display(),
            free / GIB
        );
    }
    Ok(())
}

fn find_exe(dir: &Path, name: &str) -> Option<PathBuf> {
    let want = name.to_ascii_lowercase();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_ascii_lowercase() == want)
                .unwrap_or(false)
            {
                return Some(p);
            }
        }
    }
    None
}

/// Build a windowless `Command` (no console flash for the spawned servers/tools).
fn hidden_command(exe: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(exe);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Spawn a long-lived hidden child (server / streaming curl) and return it.
fn spawn_hidden(exe: &str, args: &[&str]) -> Result<Child> {
    hidden_command(exe, args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("spawn {exe}"))
}

/// Launch a hidden server process (kept alive; returned to the caller).
fn launch_hidden(exe: &Path, args: &[&str]) -> Result<Child> {
    let exe_s = exe.to_string_lossy().to_string();
    let child = spawn_hidden(&exe_s, args)?;
    // Tie the server's lifetime to ours so a hard exit of THIS process can't
    // orphan it on :8080 (see `assign_to_lifetime_job`).
    #[cfg(windows)]
    assign_to_lifetime_job(&child);
    Ok(child)
}

/// Assign a spawned child to a process-wide Windows Job Object configured with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. The job handle is created once and held
/// for the whole process lifetime, so when this process exits — INCLUDING a hard
/// `TerminateProcess` (Task Manager, or an in-place upgrade that replaces the
/// exe) — the OS closes our last handle to the job and terminates every server
/// we put in it.
///
/// Without this, a force-killed/upgraded app orphans `llama-server` still
/// squatting :8080; the next launch's `ensure_servers` then sees the port
/// "reachable" and never relaunches, so local AI looks dead until the user hits
/// Settings → "Install local AI" (the only path that force-frees the port).
/// Best-effort: any Win32 failure is logged and the child behaves as before.
#[cfg(windows)]
fn assign_to_lifetime_job(child: &Child) {
    use std::os::windows::io::AsRawHandle;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    // Raw handle stored as usize so the OnceLock is Send+Sync. 0 = unavailable.
    static JOB: OnceLock<usize> = OnceLock::new();
    let raw = *JOB.get_or_init(|| unsafe {
        let handle = match CreateJobObjectW(None, windows::core::PCWSTR::null()) {
            Ok(h) => h,
            Err(e) => {
                log::warn!("CreateJobObject failed: {e}; local-AI servers may orphan on kill");
                return 0;
            }
        };
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(e) = SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            std::ptr::addr_of!(info).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) {
            log::warn!("SetInformationJobObject failed: {e}; local-AI servers may orphan on kill");
            return 0;
        }
        handle.0 as usize
    });
    if raw == 0 {
        return;
    }
    let job = HANDLE(raw as *mut std::ffi::c_void);
    let proc = HANDLE(child.as_raw_handle());
    unsafe {
        if let Err(e) = AssignProcessToJobObject(job, proc) {
            log::warn!("AssignProcessToJobObject failed: {e}");
        }
    }
}

/// Run a hidden command to completion, returning its exit status.
fn launch_hidden_wait(exe: &str, args: &[&str]) -> Result<std::process::ExitStatus> {
    spawn_hidden(exe, args)?
        .wait()
        .with_context(|| format!("wait {exe}"))
}

/// Run a command and capture its output (used for short queries: nvidia-smi,
/// curl version/JSON, readiness probes).
fn run_capture(exe: &str, args: &[&str]) -> Result<std::process::Output> {
    hidden_command(exe, args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("run {exe}"))
}

#[cfg(test)]
mod tests;
