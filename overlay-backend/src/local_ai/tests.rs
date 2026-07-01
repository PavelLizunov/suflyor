//! Unit tests for `local_ai.rs`, split out to keep the module file lean.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

fn asset(name: &str) -> GhAsset {
    GhAsset {
        name: name.to_string(),
        browser_download_url: format!("https://example/{name}"),
        size: 123,
    }
}

/// v0.18.0 — the "smarter/faster" model picker. The 12B is chosen ONLY when
/// the user asked for quality AND a complete file is present; everything
/// else (quality-off, file absent, file truncated) falls back to the
/// always-installed E4B so the server can never fail to find a model.
#[test]
fn pick_llama_gguf_prefers_12b_only_when_present_and_wanted() {
    let dir = Path::new("C:/root/llama.cpp");
    let e4b = dir.join(GEMMA_FILE);
    let q = dir.join(GEMMA12_FILE);
    // 12B only when BOTH wanted AND present; every other combo → E4B.
    assert_eq!(pick_llama_gguf(dir, true, true), q);
    assert_eq!(pick_llama_gguf(dir, true, false), e4b); // wanted, absent
    assert_eq!(pick_llama_gguf(dir, false, true), e4b); // present, not wanted
    assert_eq!(pick_llama_gguf(dir, false, false), e4b);
}

/// A truncated/partial 12B (smaller than the pinned size) must read as
/// ABSENT so the user is re-offered the download and the launch path falls
/// back to E4B instead of handing llama-server a corrupt file.
#[test]
fn quality_model_present_rejects_truncated_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("llama.cpp")).unwrap();
    assert!(!quality_model_present(root), "absent file → not present");
    std::fs::write(quality_gguf_path(root), b"partial").unwrap();
    assert!(!quality_model_present(root), "truncated file → not present");
}

#[test]
fn quality_gguf_path_is_under_llama_dir() {
    let p = quality_gguf_path(Path::new("C:/root"));
    assert!(p.ends_with(GEMMA12_FILE));
    assert!(p.to_string_lossy().contains("llama.cpp"));
}

/// The bar's active-model label must follow the pick: quality-off (or 12B
/// absent) → the E4B basename. (Quality-on+present needs a 6 GB file, so the
/// 12B branch is covered by `pick_llama_gguf` above, not here.)
#[test]
fn active_local_model_name_reports_e4b_when_not_quality() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(active_local_model_name(tmp.path(), false), GEMMA_FILE);
    // quality wanted but 12B absent → still E4B (safe fallback).
    assert_eq!(active_local_model_name(tmp.path(), true), GEMMA_FILE);
}

/// The bar must show the fast vs smart model distinctly. Pin the friendly
/// label against the ACTUAL shipped GGUF constants (so a future filename
/// rename that breaks the mapping fails here) plus the 12B-before-4B order
/// and the non-Gemma fallback.
#[test]
fn local_model_label_distinguishes_fast_and_smart() {
    // Real shipped basenames map to the at-a-glance labels.
    assert_eq!(local_model_label(GEMMA_FILE), "Gemma 4B");
    assert_eq!(local_model_label(GEMMA12_FILE), "Gemma 12B");
    // Case-insensitive + 12B wins over the generic 4b branch.
    assert_eq!(local_model_label("GEMMA-4-12B-IT.gguf"), "Gemma 12B");
    // A Gemma file with no size token → bare "Gemma" (never empty).
    assert_eq!(local_model_label("gemma-it.gguf"), "Gemma");
    // Non-Gemma local model → first filename token, never empty.
    assert_eq!(local_model_label("qwen2.5-7b-instruct.gguf"), "qwen2");
    assert_eq!(local_model_label(""), "—");
}

/// The projector-attach rules: E4B always gets its F32 (once present); the
/// 12B gets its own projector ONLY when present AND the engine build is
/// gemma4uv-capable (`.llama-build` >= GEMMA4UV_MIN_BUILD) — else text-only,
/// because an old engine would crash-loop on the gemma4uv type (the user's
/// "сломалась"). Other models never get a Gemma projector.
#[test]
fn mmproj_attach_rules_e4b_and_gated_12b() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    // E4B projector absent → text-only; present → attached.
    assert!(mmproj_for_model(dir, &dir.join(GEMMA_FILE)).is_none());
    std::fs::write(dir.join(MMPROJ_FILE), b"x").unwrap();
    assert_eq!(
        mmproj_for_model(dir, &dir.join(GEMMA_FILE)),
        Some(dir.join(MMPROJ_FILE))
    );
    // 12B projector present but NO build stamp → engine assumed too old → none.
    std::fs::write(dir.join(GEMMA12_MMPROJ_FILE), b"x").unwrap();
    assert!(mmproj_for_model(dir, &dir.join(GEMMA12_FILE)).is_none());
    // An OLD build stamp (< floor) → still none (would crash-loop).
    std::fs::write(dir.join(".llama-build"), b"b9410").unwrap();
    assert!(mmproj_for_model(dir, &dir.join(GEMMA12_FILE)).is_none());
    // A gemma4uv-capable build → 12B finally gets its projector.
    std::fs::write(dir.join(".llama-build"), format!("b{GEMMA4UV_MIN_BUILD}")).unwrap();
    assert_eq!(
        mmproj_for_model(dir, &dir.join(GEMMA12_FILE)),
        Some(dir.join(GEMMA12_MMPROJ_FILE))
    );
    // Non-Gemma model never gets a Gemma projector.
    assert!(mmproj_for_model(dir, &dir.join("qwen2.5-7b.gguf")).is_none());
}

#[test]
fn build_tag_parses_with_or_without_b() {
    assert_eq!(parse_build_tag("b9626"), Some(9626));
    assert_eq!(parse_build_tag("  b9626\n"), Some(9626));
    assert_eq!(parse_build_tag("9626"), Some(9626));
    assert_eq!(parse_build_tag(""), None);
    assert_eq!(parse_build_tag("master"), None);
}

/// The engine-update throttle: no engine → never (install()'s job); engine
/// present + no stamp → check now; fresh stamp → wait; stale stamp → check.
#[test]
fn engine_update_throttle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let llama = root.join("llama.cpp");
    std::fs::create_dir_all(&llama).unwrap();
    // No llama-server.exe yet → updater stays out of the way.
    assert!(!should_check_engine_update(root));
    // Pretend an engine is installed.
    std::fs::write(llama.join("llama-server.exe"), b"x").unwrap();
    // No .update-check stamp → check now.
    assert!(should_check_engine_update(root));
    // A fresh stamp → within the throttle window → skip.
    std::fs::write(llama.join(".update-check"), now_unix().to_string()).unwrap();
    assert!(!should_check_engine_update(root));
    // A stamp older than the interval → check again.
    let stale = now_unix().saturating_sub(ENGINE_UPDATE_THROTTLE_SECS + 1);
    std::fs::write(llama.join(".update-check"), stale.to_string()).unwrap();
    assert!(should_check_engine_update(root));
}

/// The binary swap backs up every overwritten live file and copies new ones
/// in, and `.gguf` models (absent from staging) are never touched.
#[test]
fn swap_backs_up_and_overwrites_keeping_models() {
    let tmp = tempfile::tempdir().unwrap();
    let live = tmp.path().join("llama.cpp");
    let staging = tmp.path().join("staging");
    let backup = tmp.path().join("backup");
    std::fs::create_dir_all(&live).unwrap();
    std::fs::create_dir_all(&staging).unwrap();
    // Live: old engine + a precious model.
    std::fs::write(live.join("llama-server.exe"), b"OLD-EXE").unwrap();
    std::fs::write(live.join("ggml.dll"), b"OLD-DLL").unwrap();
    std::fs::write(live.join("gemma.gguf"), b"MODEL").unwrap();
    // Staging: new engine binaries only (no model).
    std::fs::write(staging.join("llama-server.exe"), b"NEW-EXE").unwrap();
    std::fs::write(staging.join("ggml.dll"), b"NEW-DLL").unwrap();

    swap_engine_binaries(&staging, &live, &backup).unwrap();

    // New binaries are in place.
    assert_eq!(
        std::fs::read(live.join("llama-server.exe")).unwrap(),
        b"NEW-EXE"
    );
    assert_eq!(std::fs::read(live.join("ggml.dll")).unwrap(), b"NEW-DLL");
    // The model is untouched.
    assert_eq!(std::fs::read(live.join("gemma.gguf")).unwrap(), b"MODEL");
    // The old binaries are backed up; the model was not (never overwritten).
    assert_eq!(
        std::fs::read(backup.join("llama-server.exe")).unwrap(),
        b"OLD-EXE"
    );
    assert_eq!(std::fs::read(backup.join("ggml.dll")).unwrap(), b"OLD-DLL");
    assert!(!backup.join("gemma.gguf").exists());
}

#[test]
fn prune_engine_backups_keeps_count_and_spares_manual_and_live() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for name in [
        "llama.cpp.backup-b9000",    // updater-made (ours)
        "llama.cpp.backup-b9100",    // updater-made (ours)
        "llama.cpp.backup-prev",     // updater-made, no-stamp variant (ours)
        "llama.cpp.backup-may",      // a MANUAL snapshot — must be spared
        "llama.cpp.backup-baseline", // manual, starts with -b but NOT digits
        "llama.cpp",                 // the live engine dir — must be spared
    ] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    // keep >= count of ours → no-op.
    assert_eq!(prune_engine_backups(root, 10), 0);
    // keep 0 → all THREE updater backups removed; manual + live untouched.
    assert_eq!(prune_engine_backups(root, 0), 3);
    assert!(!root.join("llama.cpp.backup-b9000").exists());
    assert!(!root.join("llama.cpp.backup-b9100").exists());
    assert!(!root.join("llama.cpp.backup-prev").exists());
    assert!(
        root.join("llama.cpp.backup-may").exists(),
        "manual backup spared"
    );
    assert!(
        root.join("llama.cpp.backup-baseline").exists(),
        "manual `-b…`-but-not-digits backup spared"
    );
    assert!(root.join("llama.cpp").exists(), "live engine dir spared");
}

/// v0.10.2 — the GigaAM vocab is BUNDLED (include_bytes) so the install never
/// depends on the flaky HF download (HF served an HTML error page for it,
/// aborting installs). Guard the embedded asset is present + the right shape
/// (gigaam-v3 = 257 tokens, starts with the `<unk>` entry — NOT an HTML body).
#[test]
fn bundled_gigaam_vocab_is_sane() {
    assert!(
        GIGAAM_VOCAB.len() > 1000,
        "bundled vocab too small ({} bytes) — asset missing/truncated?",
        GIGAAM_VOCAB.len()
    );
    assert!(
        GIGAAM_VOCAB.starts_with(b"<unk>"),
        "vocab must start with the <unk> token (rules out an HTML error page)"
    );
    let lines = GIGAAM_VOCAB.iter().filter(|&&b| b == b'\n').count();
    assert!(lines >= 250, "expected ~257 vocab lines, got {lines}");
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
    let pick = pick_llama(&assets, GpuKind::Nvidia).unwrap();
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
    let pick = pick_llama(&assets, GpuKind::None).unwrap();
    assert!(pick.version.is_none());
    assert!(pick.cudart_url.is_none());
    assert!(pick.build_url.ends_with("llama-b9410-bin-win-cpu-x64.zip"));
}

#[test]
fn pick_vulkan_for_non_nvidia_gpu() {
    // AMD/Intel (GpuKind::Other) → the Vulkan build, no cudart (Баг2).
    let assets = vec![
        asset("llama-b9410-bin-win-cpu-x64.zip"),
        asset("llama-b9410-bin-win-cuda-13.3-x64.zip"),
        asset("cudart-llama-bin-win-cuda-13.3-x64.zip"),
        asset("llama-b9410-bin-win-vulkan-x64.zip"),
    ];
    let pick = pick_llama(&assets, GpuKind::Other).unwrap();
    assert_eq!(pick.version.as_deref(), Some("Vulkan"));
    assert!(pick.cudart_url.is_none());
    assert!(pick
        .build_url
        .ends_with("llama-b9410-bin-win-vulkan-x64.zip"));
}

#[test]
fn pick_cpu_when_non_nvidia_but_no_vulkan_asset() {
    // AMD/Intel machine but the release has no Vulkan build → CPU fallthrough.
    let assets = vec![
        asset("llama-b9410-bin-win-cpu-x64.zip"),
        asset("llama-b9410-bin-win-cuda-13.3-x64.zip"),
        asset("cudart-llama-bin-win-cuda-13.3-x64.zip"),
    ];
    let pick = pick_llama(&assets, GpuKind::Other).unwrap();
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
    let pick = pick_llama(&assets, GpuKind::Nvidia).unwrap();
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

#[cfg(windows)]
#[test]
fn listener_pids_on_port_parses_only_listening_target_port() {
    let netstat = "\
  Proto  Local Address          Foreign Address        State           PID
  TCP    127.0.0.1:8080         0.0.0.0:0              LISTENING       111
  TCP    127.0.0.1:8081         0.0.0.0:0              LISTENING       222
  TCP    127.0.0.1:8080         127.0.0.1:50000        ESTABLISHED     333
  TCP    [::1]:8080             [::]:0                 LISTENING       111
";
    assert_eq!(listener_pids_on_port(netstat, "8080"), vec!["111"]);
    assert_eq!(listener_pids_on_port(netstat, "8081"), vec!["222"]);
}

#[cfg(windows)]
#[test]
fn path_is_under_root_rejects_sibling_prefix() {
    let root = "c:\\users\\me\\suflyor-local-ai";
    assert!(path_is_under_root(
        "C:\\Users\\Me\\suflyor-local-ai\\llama.cpp\\llama-server.exe",
        root
    ));
    assert!(path_is_under_root("C:\\Users\\Me\\suflyor-local-ai", root));
    assert!(!path_is_under_root(
        "C:\\Users\\Me\\suflyor-local-ai-old\\llama-server.exe",
        root
    ));
    assert!(!path_is_under_root("", root));
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

// P1-2: swap_engine_binaries must install engine files from a NESTED staging
// layout (verify-before-swap finds llama-server.exe recursively; the swap used to
// read only direct children → copied 0 files yet returned Ok → phantom "updated").
#[test]
fn swap_installs_nested_engine_layout() {
    let staging = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    let nested = staging.path().join("llama-build-x64");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("llama-server.exe"), b"EXE").unwrap();
    std::fs::write(nested.join("ggml.dll"), b"DLL").unwrap();
    swap_engine_binaries(staging.path(), live.path(), &backup_root.path().join("bk")).unwrap();
    assert!(
        live.path().join("llama-server.exe").is_file(),
        "nested exe installed"
    );
    assert!(
        live.path().join("ggml.dll").is_file(),
        "nested dll installed"
    );
}

#[test]
fn swap_installs_flat_engine_layout() {
    let staging = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    std::fs::write(staging.path().join("llama-server.exe"), b"EXE").unwrap();
    std::fs::write(staging.path().join("cudart.dll"), b"DLL").unwrap();
    swap_engine_binaries(staging.path(), live.path(), &backup_root.path().join("bk")).unwrap();
    assert!(live.path().join("llama-server.exe").is_file());
    assert!(live.path().join("cudart.dll").is_file());
}

#[test]
fn swap_fails_without_llama_server_so_no_phantom_update() {
    // No llama-server.exe staged → Err, so update_llama_engine never stamps
    // .llama-build on a copied-nothing "success" (P1-2).
    let staging = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    std::fs::write(staging.path().join("readme.txt"), b"x").unwrap();
    std::fs::write(staging.path().join("only.dll"), b"DLL").unwrap();
    let r = swap_engine_binaries(staging.path(), live.path(), &backup_root.path().join("bk"));
    assert!(r.is_err(), "must fail when no llama-server.exe is staged");
    assert!(!live.path().join("llama-server.exe").is_file());
}

#[test]
fn swap_rejects_ambiguous_duplicate_engine_file() {
    let staging = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let backup_root = tempfile::tempdir().unwrap();
    std::fs::write(staging.path().join("llama-server.exe"), b"EXE").unwrap();
    let sub = staging.path().join("dup");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("ggml.dll"), b"A").unwrap();
    std::fs::write(staging.path().join("ggml.dll"), b"B").unwrap();
    let r = swap_engine_binaries(staging.path(), live.path(), &backup_root.path().join("bk"));
    assert!(
        r.is_err(),
        "duplicate engine filename across dirs must be rejected"
    );
}

// P1-1: zip-slip guard for the engine extractor — only entries that stay inside
// the extraction dir are allowed.
#[test]
fn archive_entry_safety_rejects_zip_slip() {
    // safe relative entries
    assert!(archive_entry_is_safe("build/llama-server.exe"));
    assert!(archive_entry_is_safe("ggml.dll"));
    assert!(archive_entry_is_safe("a/b/c.dll"));
    assert!(archive_entry_is_safe("")); // tar -tf trailing blank line
                                        // escapes — all rejected
    assert!(!archive_entry_is_safe("../escape.txt"));
    assert!(!archive_entry_is_safe("a/../../escape"));
    assert!(!archive_entry_is_safe("..\\escape")); // backslash-normalised
    assert!(!archive_entry_is_safe("/etc/passwd")); // posix-absolute
    assert!(!archive_entry_is_safe("C:/escape.txt")); // drive
    assert!(!archive_entry_is_safe("C:\\escape.txt"));
    assert!(!archive_entry_is_safe("\\\\server\\share\\x")); // UNC
                                                             // Windows trailing-space coercion: ".. " / "..  " resolve to ".." → rejected.
    assert!(!archive_entry_is_safe(".. /x"));
    assert!(!archive_entry_is_safe("a/..  /b"));
    // A bare "." current-dir component is harmless and must stay allowed
    // (tar may emit "./"-prefixed entries).
    assert!(archive_entry_is_safe("./build/x.dll"));
}
