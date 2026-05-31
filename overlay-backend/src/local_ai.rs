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

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

// ---- pinned model coordinates (HuggingFace) + exact sizes (integrity) -------
const GEMMA_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf";
const GEMMA_FILE: &str = "gemma-4-E4B-it-Q4_K_M.gguf";
const GEMMA_SIZE: u64 = 4_977_169_568;

// Vision projector for Gemma 4 (multimodal). Loaded via llama-server `--mmproj`
// so the SAME local model reads images — F8 screenshots stay fully local with no
// cloud egress. Same HuggingFace repo as the model. We ship F32 (full precision)
// per user preference; F16/BF16 work too — precision isn't the bottleneck, local
// F8 reads the screen reliably for the descriptive capture task (1024 tokens).
const MMPROJ_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/mmproj-F32.gguf";
const MMPROJ_FILE: &str = "mmproj-F32.gguf";
const MMPROJ_SIZE: u64 = 1_912_464_192;

const WHISPER_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin";
const WHISPER_FILE: &str = "ggml-large-v3-turbo-q8_0.bin";
const WHISPER_SIZE: u64 = 874_188_075;
const WHISPER_MODEL_ID: &str = "whisper-large-v3-turbo";

const GIGAAM_MODEL_URL: &str =
    "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc.int8.onnx";
const GIGAAM_MODEL_SIZE: u64 = 224_893_347;
const GIGAAM_VOCAB_URL: &str =
    "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc_vocab.txt";

const LLAMA_REPO: &str = "ggml-org/llama.cpp";
const WHISPER_REPO: &str = "ggml-org/whisper.cpp";

/// Local server endpoints the installer configures + launches.
pub const LLAMA_BASE_URL: &str = "http://127.0.0.1:8080/v1";
pub const WHISPER_BASE_URL: &str = "http://127.0.0.1:8081/v1";
const LLAMA_PORT: &str = "8080";
const WHISPER_PORT: &str = "8081";

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

/// Write the installer's resulting endpoints/models into a `Config`, switching
/// it to the local stack. Secrets (groq key / ai bearer) are untouched because
/// only these fields are mutated and the caller saves the whole struct.
pub fn apply_result(cfg: &mut crate::config::Config, res: &LocalAiResult) {
    cfg.ai_provider = "local".to_string();
    cfg.ai_local_base_url = LLAMA_BASE_URL.to_string();
    cfg.ai_local_model = res.ai_local_model.clone();
    // Default STT to Whisper (mixed RU+EN); the GigaAM dir is also filled so the
    // user can switch to GigaAM (best Russian) in Settings without re-installing.
    cfg.stt_provider = "whisper".to_string();
    cfg.stt_whisper_url = WHISPER_BASE_URL.to_string();
    cfg.stt_whisper_model = WHISPER_MODEL_ID.to_string();
    cfg.stt_gigaam_dir = res.stt_gigaam_dir.clone();
    // Gemma 4 is multimodal; the installer fetches the vision projector and
    // launches llama-server with --mmproj, so F8 screenshots run fully locally on
    // the SAME server as text — verified working for the real F8 task
    // (descriptive prompt, 1024 tokens): the model reads the screen and answers
    // well. So switch F8 vision to local too — fully local, no cloud egress.
    cfg.ai_local_vision = true;
    cfg.vision_provider = "same".to_string();
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

    let use_gpu = detect_nvidia() && !opts.force_cpu;
    let mut cuda_version: Option<String> = None;

    // ---- llama.cpp + Gemma -------------------------------------------------
    if !opts.skip_llama {
        on(Progress::Step("Installing llama.cpp".to_string()));
        std::fs::create_dir_all(&llama_dir)?;
        if find_exe(&llama_dir, "llama-server.exe").is_none() {
            let assets = github_assets(LLAMA_REPO)?;
            let pick = pick_llama(&assets, !use_gpu)?;
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
        }
        // Reuse an existing Gemma (e.g. a prior manual ~\llama.cpp) instead of
        // re-downloading 5 GB.
        let gemma_dest = llama_dir.join(GEMMA_FILE);
        if reuse_if_available(
            &gemma_dest,
            GEMMA_SIZE,
            &[home.join("llama.cpp").join(GEMMA_FILE)],
        ) {
            on(Progress::Step("Reusing existing Gemma model".to_string()));
        } else {
            curl_resumable(GEMMA_URL, &gemma_dest, GEMMA_SIZE, "Gemma", cancel, on)?;
        }

        // Vision projector (mmproj) — enables image reading on the same model so
        // F8 screenshots can be analysed locally without any cloud egress.
        let mmproj_dest = llama_dir.join(MMPROJ_FILE);
        if reuse_if_available(
            &mmproj_dest,
            MMPROJ_SIZE,
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
            let (url, size) = pick_whisper(&assets, !use_gpu)?;
            download_and_extract(&url, size, "whisper.cpp", &whisper_dir, cancel, on)?;
        }
        let whisper_dest = whisper_dir.join(WHISPER_FILE);
        if reuse_if_available(
            &whisper_dest,
            WHISPER_SIZE,
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
    }

    // ---- GigaAM-v3 (in-process; no server) --------------------------------
    if !opts.skip_gigaam {
        bail_if_cancelled(cancel)?;
        on(Progress::Step("Downloading GigaAM-v3".to_string()));
        std::fs::create_dir_all(&gigaam_dir)?;
        // transcribe_rs loads exactly `model.int8.onnx` + `vocab.txt`.
        let giga_dest = gigaam_dir.join("model.int8.onnx");
        if !reuse_if_available(&giga_dest, GIGAAM_MODEL_SIZE, &[]) {
            curl_resumable(
                GIGAAM_MODEL_URL,
                &giga_dest,
                GIGAAM_MODEL_SIZE,
                "GigaAM",
                cancel,
                on,
            )?;
        }
        curl_small(GIGAAM_VOCAB_URL, &gigaam_dir.join("vocab.txt"))?;
    }

    // ---- launch servers ----------------------------------------------------
    let mut servers: Vec<Child> = Vec::new();
    if !opts.skip_llama {
        on(Progress::Step("Starting llama-server :8080".to_string()));
        let exe = find_exe(&llama_dir, "llama-server.exe")
            .context("llama-server.exe not found after install")?;
        // Free :8080 from any stale/projector-less server (incl. one orphaned by
        // a previous app run) so the fresh --mmproj server can actually bind.
        stop_listener_on_port(LLAMA_PORT);
        std::thread::sleep(Duration::from_millis(800));
        let gguf = llama_dir.join(GEMMA_FILE);
        let gguf_s = gguf.to_string_lossy().into_owned();
        let mmproj = llama_dir.join(MMPROJ_FILE);
        let mmproj_s = mmproj.to_string_lossy().into_owned();
        let ngl = if use_gpu { "99" } else { "0" };
        let mut args: Vec<&str> = vec![
            "-m",
            &gguf_s,
            "--host",
            "127.0.0.1",
            "--port",
            LLAMA_PORT,
            "-ngl",
            ngl,
            "-c",
            "8192",
            "--jinja",
        ];
        // Gemma 4 is multimodal — load the projector so the same server reads
        // images (F8 vision). Guarded so a projector-less install still starts.
        if mmproj.exists() {
            args.push("--mmproj");
            args.push(&mmproj_s);
        }
        let child = launch_hidden(&exe, &args)?;
        servers.push(child);
    }
    if !opts.skip_whisper {
        on(Progress::Step("Starting whisper-server :8081".to_string()));
        let exe = find_exe(&whisper_dir, "whisper-server.exe")
            .or_else(|| find_exe(&whisper_dir, "server.exe"))
            .context("whisper-server.exe not found after install")?;
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
    if !opts.skip_llama {
        on(Progress::Step("Waiting for the model to load".to_string()));
        wait_ready(&format!("{LLAMA_BASE_URL}/models"), 120);
        if use_gpu {
            on_gpu = verify_gpu_offload(24);
            let verdict = if on_gpu {
                format!("GPU (CUDA {})", cuda_version.as_deref().unwrap_or("?"))
            } else {
                "CPU (GPU offload not detected — update the NVIDIA driver)".to_string()
            };
            on(Progress::Gpu(verdict));
        } else {
            on(Progress::Gpu("CPU".to_string()));
        }
    }

    Ok(LocalAiResult {
        ai_local_model: GEMMA_FILE.to_string(),
        stt_gigaam_dir: gigaam_dir.to_string_lossy().to_string(),
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

/// Best-effort: kill whatever process is LISTENING on `port` so a fresh server
/// can bind it. Critical before (re)launching llama-server with `--mmproj`: a
/// stale projector-less server (e.g. orphaned by a previous app run that was
/// force-killed, so it was never reaped) keeps the port, the new server fails to
/// bind, and `wait_ready` still sees the old one answer — so the install reports
/// success while F8 vision returns HTTP 500. Parses `netstat -ano`.
#[cfg(windows)]
fn stop_listener_on_port(port: &str) {
    let Ok(out) = run_capture("netstat", &["-ano", "-p", "tcp"]) else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let suffix = format!(":{port}");
    let mut killed: Vec<String> = Vec::new();
    for line in text.lines() {
        // Columns: Proto  LocalAddr  ForeignAddr  State  PID
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5
            && cols[3].eq_ignore_ascii_case("LISTENING")
            && cols[1].ends_with(suffix.as_str())
        {
            let pid = cols[4];
            if pid != "0" && !killed.iter().any(|k| k == pid) {
                let _ = run_capture("taskkill", &["/F", "/PID", pid]);
                killed.push(pid.to_string());
            }
        }
    }
}

#[cfg(not(windows))]
fn stop_listener_on_port(_port: &str) {}

/// On launch, start the local servers the config points at but that aren't
/// already running (the app kills its servers on quit, so after a restart
/// following an in-app install they'd be down). Uses the binaries + models
/// under `root` (the installer/script layout); skips a server whose port
/// already answers. Best-effort — a missing binary just means that server is
/// not started. Returns the launched child handles for kill-on-quit tracking.
#[must_use]
pub fn ensure_servers(root: &Path, want_llama: bool, want_whisper: bool) -> Vec<Child> {
    let mut started = Vec::new();
    let use_gpu = detect_nvidia();
    if want_llama && !is_reachable(&format!("{LLAMA_BASE_URL}/models")) {
        let llama_dir = root.join("llama.cpp");
        let gguf = llama_dir.join(GEMMA_FILE);
        let mmproj = llama_dir.join(MMPROJ_FILE);
        if let Some(exe) = find_exe(&llama_dir, "llama-server.exe") {
            if gguf.exists() {
                let gguf_s = gguf.to_string_lossy().into_owned();
                let mmproj_s = mmproj.to_string_lossy().into_owned();
                let ngl = if use_gpu { "99" } else { "0" };
                let mut args: Vec<&str> = vec![
                    "-m",
                    &gguf_s,
                    "--host",
                    "127.0.0.1",
                    "--port",
                    LLAMA_PORT,
                    "-ngl",
                    ngl,
                    "-c",
                    "8192",
                    "--jinja",
                ];
                // Load the vision projector if it's present so a restart keeps
                // F8 local vision working (downloaded by the installer).
                if mmproj.exists() {
                    args.push("--mmproj");
                    args.push(&mmproj_s);
                }
                if let Ok(child) = launch_hidden(&exe, &args) {
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

fn github_assets(repo: &str) -> Result<Vec<GhAsset>> {
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
    let rel: GhRelease = serde_json::from_slice(&out.stdout)
        .with_context(|| format!("parse release JSON for {repo}"))?;
    Ok(rel.assets)
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

/// Pick the llama.cpp Windows build: newest CUDA build + matching cudart, or the
/// CPU build when `force_cpu` (or no CUDA build exists). RTX 50-series (Blackwell)
/// needs CUDA >= 12.8, so we always take the HIGHEST available CUDA version.
fn pick_llama(assets: &[GhAsset], force_cpu: bool) -> Result<LlamaPick> {
    if !force_cpu {
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

fn download_and_extract(
    url: &str,
    size: u64,
    label: &str,
    dest_dir: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    let name = url.rsplit('/').next().unwrap_or("download.zip");
    let zip = dest_dir.join(name);
    // Download the zip with LIVE byte progress + cancel support (was a silent
    // blocking curl before, so the bar sat empty during the binary downloads).
    curl_resumable(url, &zip, size, label, cancel, on)?;
    extract_zip(&zip, dest_dir)?;
    let _ = std::fs::remove_file(&zip);
    Ok(())
}

fn extract_zip(zip: &Path, dest_dir: &Path) -> Result<()> {
    // bsdtar (tar.exe) on Windows 10 1803+ extracts zip archives.
    let status = launch_hidden_wait(
        "tar.exe",
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

fn curl_small(url: &str, out: &Path) -> Result<()> {
    let status = launch_hidden_wait(
        "curl.exe",
        &[
            "-sL",
            "--retry",
            "8",
            "--retry-all-errors",
            "-o",
            &out.to_string_lossy(),
            url,
        ],
    )?;
    if !status.success() || file_len(out) == 0 {
        bail!("download failed: {}", out.display());
    }
    Ok(())
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
/// the budget runs out. Best-effort — never errors.
fn wait_ready(url: &str, max_secs: u64) {
    let deadline = max_secs / 2;
    for _ in 0..deadline {
        if let Ok(out) = run_capture("curl.exe", &["-s", "-o", "NUL", "--max-time", "2", url]) {
            if out.status.success() {
                return;
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

// ---- process + fs helpers --------------------------------------------------

fn preflight() -> Result<()> {
    if run_capture("curl.exe", &["--version"]).is_err() {
        bail!("curl.exe not found (needs Windows 10 1803+)");
    }
    if run_capture("tar.exe", &["--version"]).is_err() {
        bail!("tar.exe not found (needs Windows 10 1803+)");
    }
    Ok(())
}

fn file_len(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
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
fn reuse_if_available(dest: &Path, expected: u64, candidates: &[PathBuf]) -> bool {
    if file_len(dest) >= expected {
        return true;
    }
    for cand in candidates {
        if cand.as_path() != dest && file_len(cand) >= expected {
            let _ = std::fs::remove_file(dest);
            if std::fs::hard_link(cand, dest).is_ok() || std::fs::copy(cand, dest).is_ok() {
                return file_len(dest) >= expected;
            }
        }
    }
    false
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
    spawn_hidden(&exe_s, args)
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example/{name}"),
            size: 123,
        }
    }

    #[test]
    fn cuda_version_parse() {
        assert_eq!(
            cuda_version_of("llama-b9410-bin-win-cuda-13.3-x64.zip"),
            Some((13, 3))
        );
        assert_eq!(
            cuda_version_of("llama-b1-bin-win-cuda-12.4-x64.zip"),
            Some((12, 4))
        );
        assert_eq!(cuda_version_of("llama-b1-bin-win-cpu-x64.zip"), None);
        // cudart name also contains the substring but we never feed it here
        assert_eq!(
            cuda_version_of("cudart-llama-bin-win-cuda-13.3-x64.zip"),
            Some((13, 3))
        );
    }

    #[test]
    fn pick_newest_cuda_and_matching_cudart() {
        let assets = vec![
            asset("llama-b9410-bin-win-cpu-x64.zip"),
            asset("llama-b9410-bin-win-cpu-arm64.zip"),
            asset("llama-b9410-bin-win-cuda-12.4-x64.zip"),
            asset("llama-b9410-bin-win-cuda-13.3-x64.zip"),
            asset("cudart-llama-bin-win-cuda-12.4-x64.zip"),
            asset("cudart-llama-bin-win-cuda-13.3-x64.zip"),
            asset("llama-b9410-bin-win-vulkan-x64.zip"),
        ];
        let pick = pick_llama(&assets, false).unwrap();
        assert_eq!(pick.version.as_deref(), Some("13.3"));
        assert!(pick
            .build_url
            .ends_with("llama-b9410-bin-win-cuda-13.3-x64.zip"));
        assert!(pick
            .cudart_url
            .unwrap()
            .ends_with("cudart-llama-bin-win-cuda-13.3-x64.zip"));
    }

    #[test]
    fn pick_cpu_when_forced() {
        let assets = vec![
            asset("llama-b9410-bin-win-cuda-13.3-x64.zip"),
            asset("cudart-llama-bin-win-cuda-13.3-x64.zip"),
            asset("llama-b9410-bin-win-cpu-x64.zip"),
        ];
        let pick = pick_llama(&assets, true).unwrap();
        assert!(pick.version.is_none());
        assert!(pick.cudart_url.is_none());
        assert!(pick.build_url.ends_with("llama-b9410-bin-win-cpu-x64.zip"));
    }

    #[test]
    fn pick_whisper_cpu_takes_plain_build() {
        let assets = vec![
            asset("whisper-bin-Win32.zip"),
            asset("whisper-blas-bin-x64.zip"),
            asset("whisper-cublas-12.4.0-bin-x64.zip"),
            asset("whisper-bin-x64.zip"),
        ];
        // force_cpu = true -> plain CPU build even though a cuBLAS build exists.
        assert!(pick_whisper(&assets, true)
            .unwrap()
            .0
            .ends_with("whisper-bin-x64.zip"));
    }

    #[test]
    fn pick_whisper_gpu_takes_highest_cublas() {
        let assets = vec![
            asset("whisper-bin-x64.zip"),
            asset("whisper-cublas-11.8.0-bin-x64.zip"),
            asset("whisper-cublas-12.4.0-bin-x64.zip"),
            asset("whisper-blas-bin-x64.zip"),
        ];
        // force_cpu = false -> highest-version cuBLAS (GPU) build.
        assert!(pick_whisper(&assets, false)
            .unwrap()
            .0
            .ends_with("whisper-cublas-12.4.0-bin-x64.zip"));
    }

    #[test]
    fn pick_whisper_gpu_falls_back_to_cpu_when_no_cublas() {
        let assets = vec![
            asset("whisper-bin-Win32.zip"),
            asset("whisper-blas-bin-x64.zip"),
            asset("whisper-bin-x64.zip"),
        ];
        // GPU requested but no cuBLAS asset in the release -> plain CPU build.
        assert!(pick_whisper(&assets, false)
            .unwrap()
            .0
            .ends_with("whisper-bin-x64.zip"));
    }

    #[test]
    #[ignore = "hits the live GitHub API (run with --ignored)"]
    fn live_pick_llama_is_blackwell_capable() {
        let assets = github_assets(LLAMA_REPO).unwrap();
        let pick = pick_llama(&assets, false).unwrap();
        let v = pick.version.expect("a CUDA build should exist");
        let mut it = v.split('.');
        let maj: u32 = it.next().unwrap().parse().unwrap();
        let min: u32 = it.next().unwrap().parse().unwrap();
        // Blackwell (RTX 50xx) needs CUDA >= 12.8; the newest pick must satisfy it.
        assert!(
            maj > 12 || (maj == 12 && min >= 8),
            "picked CUDA {v} is too old for Blackwell"
        );
        assert!(
            pick.cudart_url.is_some(),
            "a matching cudart must be picked"
        );
        // whisper picker against the live release too: GPU path must land on a
        // cuBLAS build (Blackwell-capable via PTX JIT), CPU path on the plain build.
        let wassets = github_assets(WHISPER_REPO).unwrap();
        assert!(pick_whisper(&wassets, false)
            .unwrap()
            .0
            .contains("whisper-cublas-"));
        assert!(pick_whisper(&wassets, true)
            .unwrap()
            .0
            .ends_with("whisper-bin-x64.zip"));
    }

    #[test]
    fn compute_apps_detects_llama() {
        assert!(parse_compute_apps("C:\\x\\llama-server.exe, 4096 MiB"));
        assert!(!parse_compute_apps(
            "C:\\x\\dwm.exe, [N/A]\nexplorer.exe, [N/A]"
        ));
    }

    #[test]
    fn apply_result_sets_local_and_keeps_secrets() {
        let mut cfg = crate::config::Config {
            groq_api_key: "gsk_secret".to_string(),
            ai_bearer: "bridge_secret".to_string(),
            // a prior cloud setting — apply_result switches F8 to local on a
            // local install (vision rides the same local server).
            vision_provider: "cloud".to_string(),
            ..Default::default()
        };
        let res = LocalAiResult {
            ai_local_model: GEMMA_FILE.to_string(),
            stt_gigaam_dir: "C:\\root\\gigaam-v3".to_string(),
            on_gpu: true,
            cuda_version: Some("13.3".to_string()),
            servers: Vec::new(),
        };
        apply_result(&mut cfg, &res);
        assert_eq!(cfg.ai_provider, "local");
        assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);
        assert_eq!(cfg.ai_local_model, GEMMA_FILE);
        assert_eq!(cfg.stt_provider, "whisper");
        assert_eq!(cfg.stt_whisper_url, WHISPER_BASE_URL);
        assert_eq!(cfg.stt_gigaam_dir, "C:\\root\\gigaam-v3");
        // secrets preserved
        assert_eq!(cfg.groq_api_key, "gsk_secret");
        assert_eq!(cfg.ai_bearer, "bridge_secret");
        // installer enables fully-local F8 vision (Gemma 4 + mmproj on the same
        // local server).
        assert!(cfg.ai_local_vision);
        assert_eq!(cfg.vision_provider, "same");
    }
}
