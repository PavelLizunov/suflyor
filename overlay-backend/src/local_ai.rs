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
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

// ---- pinned model coordinates (HuggingFace) + exact sizes (integrity) -------
const GEMMA_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf";
const GEMMA_FILE: &str = "gemma-4-E4B-it-Q4_K_M.gguf";
const GEMMA_SIZE: u64 = 4_977_169_568;
// Pinned SHA-256 = the HuggingFace LFS object id of the exact file above
// (cross-checked at pin time: the API-reported LFS size equals GEMMA_SIZE).
// Hardcoded so a tampered API/CDN response can't supply a matching hash for
// swapped bytes (P1.5).
const GEMMA_SHA256: &str = "519b9793ed6ce0ff530f1b7c96e848e08e49e7af4d57bb97f76215963a54146d";

// ---- OPTIONAL "smarter" model: Gemma 4 12B QAT (downloaded on demand) -------
// Same family/prompt as E4B (so the vision projector + chat template still fit),
// QAT 4-bit ≈ bf16 quality. ~2× slower than E4B + ~9.5 GB VRAM (bench 2026-06-13),
// so it is NOT bundled in the installer — the user pulls it from Settings when
// they want "smarter", and the app loads it instead of E4B when ai_local_quality
// is on AND this file is present. SHA-256 pinned (verify-before-launch, P1.5).
const GEMMA12_URL: &str = "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/main/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf";
const GEMMA12_FILE: &str = "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf";
const GEMMA12_SIZE: u64 = 6_716_355_328;
const GEMMA12_SHA256: &str = "cc9ff072e0a8203429ed854e6662c17a6c2bc1e5dca5b475dd4736caaacbc165";

// Vision projector for Gemma 4 (multimodal). Loaded via llama-server `--mmproj`
// so the SAME local model reads images — F8 screenshots stay fully local with no
// cloud egress. Same HuggingFace repo as the model. We ship F32 (full precision)
// per user preference; F16/BF16 work too — precision isn't the bottleneck, local
// F8 reads the screen reliably for the descriptive capture task (1024 tokens).
const MMPROJ_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/mmproj-F32.gguf";
const MMPROJ_FILE: &str = "mmproj-F32.gguf";
const MMPROJ_SIZE: u64 = 1_912_464_192;
const MMPROJ_SHA256: &str = "343cdea7775835ebdd1caa6c42ec3ec3e711d082835c72253d4e87c4b7e303d0";

// Vision projector for the 12B — its OWN, from the 12B repo. Uses a newer
// "gemma4uv" projector type, so it ONLY loads on a recent enough llama.cpp
// (a May build dies with "unknown projector type: gemma4uv" → crash-loop). We
// gate attaching it on the installed build >= GEMMA4UV_MIN_BUILD (verified live
// 2026-06-14: b9626 loads it). Saved under a DISTINCT local name so it never
// clobbers the E4B projectors. SHA-256 pinned (verify-before-launch, P1.5).
const GEMMA12_MMPROJ_URL: &str =
    "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/main/mmproj-F16.gguf";
const GEMMA12_MMPROJ_FILE: &str = "mmproj-12b-F16.gguf";
const GEMMA12_MMPROJ_SIZE: u64 = 175_115_840;
const GEMMA12_MMPROJ_SHA256: &str =
    "ecc4e93128da8363b7dbf2193eab98cf1142353f52ceaa0c95c0872997aaadd3";
/// Minimum llama.cpp release build (the `bNNNN` tag) that can load the 12B's
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

    let use_gpu = detect_nvidia() && !opts.force_cpu;
    let mut cuda_version: Option<String> = None;

    // ---- llama.cpp + Gemma -------------------------------------------------
    if !opts.skip_llama {
        on(Progress::Step("Installing llama.cpp".to_string()));
        std::fs::create_dir_all(&llama_dir)?;
        if find_exe(&llama_dir, "llama-server.exe").is_none() {
            let rel = github_release(LLAMA_REPO)?;
            let pick = pick_llama(&rel.assets, !use_gpu)?;
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
    if !opts.skip_llama {
        on(Progress::Step("Waiting for the model to load".to_string()));
        // P0.2: fail the install if the model never loads or can't generate —
        // don't report success on a wedged server.
        wait_ready(&format!("{LLAMA_BASE_URL}/models"), 120)
            .context("llama-server did not become ready")?;
        // The tile shows {e} (this top-level context), so it must be actionable
        // RU; the inner bail (with a reply snippet) only reaches the log via {e:#}.
        verify_llama_ready(on).context(
            "Локальная модель установилась, но не смогла запуститься на этом \
             компьютере (не успела прогреться). Попробуйте переустановить, либо \
             включите облачный AI в Настройках → AI.",
        )?;
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
    if !opts.skip_whisper {
        // P0.2: whisper had no strict readiness check after launch.
        on(Progress::Step("Waiting for whisper-server".to_string()));
        wait_ready(&format!("{WHISPER_BASE_URL}/models"), 60)
            .context("whisper-server did not become ready")?;
    }

    Ok(LocalAiResult {
        ai_local_model: GEMMA_FILE.to_string(),
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

#[cfg(all(windows, test))]
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
        return true; // can't enumerate — best-effort; let the bind attempt decide
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
                    let _ = kill_pid_tree(pid);
                    killed.push(pid.to_string());
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
    /// A FOREIGN process holds :8080 (started outside our `root`) we won't
    /// force-kill — the OLD model keeps serving, so the switch did NOT happen.
    PortBusy,
    /// Freed + relaunched but the server never became reachable in time
    /// (missing binary/GGUF, failed bind, or still cold-loading past the wait).
    FailedToStart,
}

/// Restart llama-server with the GGUF `prefer_quality` selects: free :8080
/// owner-aware, relaunch via [`ensure_servers`], then POLL `/models` until the
/// fresh server answers (model load is a few seconds; 12B cold ≈ 5 s). Returns
/// the honest [`ModelSwitch`] + any launched child handles. Whisper (:8081) is
/// left alone. Call from a worker thread (it blocks up to ~20 s).
#[must_use]
pub fn switch_local_model(
    root: &Path,
    prefer_quality: bool,
    want_whisper: bool,
) -> (ModelSwitch, Vec<Child>) {
    // A foreign owner we can't kill means the old model stays up — don't lie.
    if !free_llama_port(root) {
        return (ModelSwitch::PortBusy, Vec::new());
    }
    // Let the OS release the port before the relaunch binds it.
    std::thread::sleep(Duration::from_millis(800));
    let started = ensure_servers(root, true, want_whisper, prefer_quality);
    let url = format!("{LLAMA_BASE_URL}/models");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if is_reachable(&url) {
            return (ModelSwitch::Switched, started);
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    (ModelSwitch::FailedToStart, started)
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
    // Dead port: free any stale under-root listener and relaunch the selected
    // GGUF, confirming readiness before reporting success.
    switch_local_model(root, prefer_quality, false)
}

/// Friendly, compact label for a LOCAL model GGUF basename — so the bar's
/// active-stack readout says "Gemma 4B" / "Gemma 12B" (the user must be able
/// to tell the fast vs smart model apart at a glance) instead of a bare
/// "gemma". Pure (no I/O); falls back to the first filename token for any
/// non-Gemma local model. Checks 12B before 4B so the "12b" filename never
/// matches the generic "4b" branch.
#[must_use]
pub fn local_model_label(basename: &str) -> String {
    let l = basename.to_ascii_lowercase();
    if l.contains("12b") {
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

/// Basename of the GGUF [`selected_llama_gguf`] would load — so callers keep
/// `config.ai_local_model` (the bar's active-stack readout) in sync with the
/// model actually serving. Pure string pick mirroring [`pick_llama_gguf`].
#[must_use]
pub fn active_local_model_name(root: &Path, prefer_quality: bool) -> String {
    pick_llama_gguf(
        &root.join("llama.cpp"),
        prefer_quality,
        quality_model_present(root),
    )
    .file_name()
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_default()
}

/// Absolute path the optional 12B "smarter" GGUF lives at (whether or not it
/// has been downloaded yet) under an install `root`.
#[must_use]
pub fn quality_gguf_path(root: &Path) -> PathBuf {
    root.join("llama.cpp").join(GEMMA12_FILE)
}

/// True when the 12B model is downloaded AND complete (size matches the pin) —
/// the cheap presence check the UI uses to show "download" vs "switch". A
/// truncated/partial file reads as absent so the user is offered the download
/// again (the launch path also falls back to E4B on a bad file).
#[must_use]
pub fn quality_model_present(root: &Path) -> bool {
    file_len(&quality_gguf_path(root)) >= GEMMA12_SIZE
}

/// True when the base E4B model is downloaded + complete (size matches the pin).
/// This is the MINIMUM for local AI to answer; `quality_model_present` is the
/// optional 12B on top. Mirrors `quality_model_present`; used by the components
/// readiness API (a truncated file reads as absent, so the user is re-offered
/// the download).
#[must_use]
pub fn base_model_present(root: &Path) -> bool {
    file_len(&root.join("llama.cpp").join(GEMMA_FILE)) >= GEMMA_SIZE
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

/// True when the 12B's OWN vision projector is downloaded AND complete. The UI
/// uses it to show "download 12B vision" vs "vision ready". A truncated file
/// reads as absent (the launch only attaches a present projector).
#[must_use]
pub fn quality_vision_present(root: &Path) -> bool {
    file_len(&root.join("llama.cpp").join(GEMMA12_MMPROJ_FILE)) >= GEMMA12_MMPROJ_SIZE
}

/// True when the installed llama.cpp is new enough to LOAD the 12B projector
/// (gemma4uv). The UI uses it to only offer the 12B-vision download on a capable
/// engine — on an old engine it points the user at "update the engine" instead.
#[must_use]
pub fn quality_vision_supported(root: &Path) -> bool {
    llama_build_supports_gemma4uv(&root.join("llama.cpp"))
}

/// Pick which llama GGUF to load: the 12B ONLY when the user asked for it AND
/// the file is actually present+complete; otherwise the always-installed E4B.
/// Centralised so `ensure_servers` and `install`'s launch agree. Does the disk
/// check then defers the choice to the pure [`pick_llama_gguf`] (unit-tested
/// without materialising a 6 GB file).
fn selected_llama_gguf(llama_dir: &Path, prefer_quality: bool) -> PathBuf {
    let present = file_len(&llama_dir.join(GEMMA12_FILE)) >= GEMMA12_SIZE;
    pick_llama_gguf(llama_dir, prefer_quality, present)
}

/// Pure model-choice rule (no I/O): 12B only when wanted AND present.
fn pick_llama_gguf(llama_dir: &Path, prefer_quality: bool, quality_present: bool) -> PathBuf {
    if prefer_quality && quality_present {
        llama_dir.join(GEMMA12_FILE)
    } else {
        llama_dir.join(GEMMA_FILE)
    }
}

/// Download (resumable) + SHA-verify the optional 12B model into `root`, on
/// demand from Settings. Mirrors the installer's download→verify discipline
/// (P1.5: a tampered byte-stream fails the pinned hash and the partial file is
/// left for a clean re-pull, never launched). Does NOT restart the server —
/// the caller flips `ai_local_quality` and restarts so the new GGUF loads.
///
/// # Errors
/// Network/disk failure, cancellation, or a SHA-256 mismatch after download.
pub fn download_quality_model(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    let llama_dir = root.join("llama.cpp");
    std::fs::create_dir_all(&llama_dir)
        .with_context(|| format!("create llama dir {}", llama_dir.display()))?;
    let dest = llama_dir.join(GEMMA12_FILE);
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if reuse_if_available(
        &dest,
        GEMMA12_SIZE,
        GEMMA12_SHA256,
        &[home.join("llama.cpp").join(GEMMA12_FILE)],
    ) {
        on(Progress::Step("Умная модель уже загружена".to_string()));
    } else {
        curl_resumable(GEMMA12_URL, &dest, GEMMA12_SIZE, "Gemma 12B", cancel, on)?;
    }
    verify_sha256(&dest, GEMMA12_SHA256, "Gemma 12B model")?;
    Ok(())
}

/// Download (resumable) + SHA-verify the 12B's OWN vision projector so F8 vision
/// works on the 12B. Same verify-before-launch discipline as the model download
/// (P1.5). Saved under a DISTINCT name so it never clobbers the E4B projectors.
/// Does NOT restart the server — the caller restarts so the projector loads. The
/// launch path additionally gates attaching it on a gemma4uv-capable engine.
///
/// # Errors
/// Network/disk failure, cancellation, or a SHA-256 mismatch after download.
pub fn download_quality_vision(
    root: &Path,
    cancel: &AtomicBool,
    on: &dyn Fn(Progress),
) -> Result<()> {
    let llama_dir = root.join("llama.cpp");
    std::fs::create_dir_all(&llama_dir)
        .with_context(|| format!("create llama dir {}", llama_dir.display()))?;
    let dest = llama_dir.join(GEMMA12_MMPROJ_FILE);
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    if reuse_if_available(
        &dest,
        GEMMA12_MMPROJ_SIZE,
        GEMMA12_MMPROJ_SHA256,
        &[home.join("llama.cpp").join(GEMMA12_MMPROJ_FILE)],
    ) {
        on(Progress::Step(
            "Vision-проектор 12B уже загружен".to_string(),
        ));
    } else {
        curl_resumable(
            GEMMA12_MMPROJ_URL,
            &dest,
            GEMMA12_MMPROJ_SIZE,
            "Vision 12B",
            cancel,
            on,
        )?;
    }
    verify_sha256(&dest, GEMMA12_MMPROJ_SHA256, "Gemma 12B vision projector")?;
    Ok(())
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

/// The vision projector to attach for `gguf`, if present AND loadable. The E4B
/// (n_embd 2560 ↔ `mmproj-F32.gguf`) always gets its projector. The 12B gets its
/// OWN `mmproj-12b-F16.gguf` ONLY when (a) it's downloaded AND (b) the installed
/// llama.cpp is new enough to load its "gemma4uv" type ([`llama_build_supports_gemma4uv`]) —
/// otherwise the server would die with "unknown projector type" and crash-loop,
/// so the 12B falls back to TEXT-ONLY. `None` for any other model.
fn mmproj_for_model(llama_dir: &Path, gguf: &Path) -> Option<PathBuf> {
    let name = gguf.file_name().and_then(|n| n.to_str())?;
    if name == GEMMA_FILE {
        let proj = llama_dir.join(MMPROJ_FILE);
        proj.exists().then_some(proj)
    } else if name == GEMMA12_FILE && llama_build_supports_gemma4uv(llama_dir) {
        let proj = llama_dir.join(GEMMA12_MMPROJ_FILE);
        proj.exists().then_some(proj)
    } else {
        None
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
    let use_gpu = detect_nvidia();
    let pick = pick_llama(&rel.assets, !use_gpu)?;
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
    if !verify_engine_runs(&staged_exe, &llama_dir, use_gpu, cancel) {
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

/// Launch a staged engine on a scratch port with the smallest installed model
/// and wait for `/v1/models`. Proves binary + DLL + CUDA + model-load all work
/// (the regression class that would brick local AI). Falls back to a `--version`
/// integrity check when no model is on disk yet. Always reaps the test server.
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
    // Smallest always-present model (E4B, text-only — fast warm-up). Skip the
    // projector: we're verifying the BINARY, not vision, and a text-only load is
    // lighter on VRAM/time.
    let model = {
        let e4b = llama_dir.join(GEMMA_FILE);
        let q = llama_dir.join(GEMMA12_FILE);
        if e4b.exists() {
            Some(e4b)
        } else if q.exists() {
            Some(q)
        } else {
            None
        }
    };
    let Some(model) = model else {
        // No weights yet — at least prove the image + its DLLs load.
        return run_capture(&staged_exe.to_string_lossy(), &["--version"])
            .map(|o| o.status.success())
            .unwrap_or(false);
    };
    let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);
    let model_s = model.to_string_lossy().into_owned();
    let ngl = if use_gpu { "99" } else { "0" };
    let child = match launch_hidden(
        staged_exe,
        &[
            "-m",
            &model_s,
            "--host",
            "127.0.0.1",
            "--port",
            ENGINE_VERIFY_PORT,
            "-ngl",
            ngl,
            "-c",
            "2048",
        ],
    ) {
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
    let mut files: Vec<PathBuf> = std::fs::read_dir(staging)
        .context("read staging dir")?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_engine_file(p))
        .collect();
    files.sort_by_key(|p| {
        // exes (false → sorts first), then everything else.
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
    let use_gpu = detect_nvidia();
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
        // (E4B ↔ mmproj-F32, 12B ↔ mmproj-12b-F16). A mismatched projector
        // crashes llama-server on model load; a missing one → the model runs
        // text-only (F8 vision then prompts to download the right projector).
        let mmproj_s =
            mmproj_for_model(&llama_dir, &gguf).map(|p| p.to_string_lossy().into_owned());
        if let Some(exe) = find_exe(&llama_dir, "llama-server.exe") {
            if gguf.exists() {
                let gguf_s = gguf.to_string_lossy().into_owned();
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
                if let Some(p) = &mmproj_s {
                    args.push("--mmproj");
                    args.push(p.as_str());
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
