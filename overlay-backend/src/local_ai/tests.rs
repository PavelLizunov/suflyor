//! Unit tests for `local_ai.rs`, split out to keep the module file lean.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use std::process::{Child, Command};
#[cfg(windows)]
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

fn asset(name: &str) -> GhAsset {
    GhAsset {
        name: name.to_string(),
        browser_download_url: format!("https://example/{name}"),
        size: 123,
    }
}

fn long_running_child() -> Child {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "ping -n 30 127.0.0.1 > NUL"])
            .spawn()
            .unwrap()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh").args(["-c", "sleep 30"]).spawn().unwrap()
    }
}

fn process_is_running(pid: u32) -> bool {
    let pid = pid.to_string();
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .is_ok_and(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .split_whitespace()
                    .any(|field| field == pid.as_str())
            })
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid])
            .status()
            .is_ok_and(|status| status.success())
    }
}

#[test]
fn failed_install_cleanup_terminates_server_children() {
    let child = long_running_child();
    let pid = child.id();
    let mut cleanup = InstallServerCleanup::default();
    cleanup.children.push(child);

    drop(cleanup);

    assert!(
        !process_is_running(pid),
        "server child {pid} survived cleanup"
    );
}

#[test]
fn llama_readiness_requires_nonempty_text_message_content() {
    assert!(llama_reply_has_text_content(
        r#"{"choices":[{"message":{"content":"ok"}}]}"#
    ));
    for reply in [
        r#"{"choices":[{"message":{"content":""}}]}"#,
        r#"{"choices":[{"message":{"content":" \n\t "}}]}"#,
        r#"{"choices":[{"message":{}}]}"#,
        r#"{"choices":[{"message":{"content":[]}}]}"#,
        r#"{"choices":[]}"#,
        r#"{"choices":"present but not an array"}"#,
        "not JSON",
    ] {
        assert!(
            !llama_reply_has_text_content(reply),
            "must reject a non-textual or empty completion: {reply}"
        );
    }
}

fn make_complete(path: &Path, size: u64) {
    let file = std::fs::File::create(path).unwrap();
    file.set_len(size).unwrap();
}

fn local_result(quality: bool, vision: bool) -> LocalAiResult {
    LocalAiResult {
        ai_local_model: if quality {
            GEMMA26_FILE.to_string()
        } else {
            GEMMA_FILE.to_string()
        },
        ai_local_quality: quality,
        ai_local_vision: vision,
        hardware_profile: if quality {
            HardwareModelProfile::Primary26Vram8
        } else {
            HardwareModelProfile::Fallback12B
        },
        stt_gigaam_dir: "C:\\root\\gigaam-v3".to_string(),
        on_gpu: true,
        cuda_version: Some("13.3".to_string()),
        servers: Vec::new(),
    }
}

#[test]
fn owner_primary_coordinates_and_sha_are_exact() {
    assert_eq!(
        GEMMA_URL,
        "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/980b060c40a8539ac159e0501a3e0f66a6365af3/gemma-4-12B-it-qat-UD-Q4_K_XL.gguf"
    );
    assert_eq!(
        GEMMA_SHA256,
        "90fd44e29e0d7cffeb0fd00dc73cfdab9ed0b0e95306ecf7821ea634c940c370"
    );
    assert_eq!(GEMMA_SIZE, 6_716_356_800);
    assert_eq!(
        GEMMA26_URL,
        "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF/resolve/c099eb4/gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf"
    );
    assert_eq!(GEMMA26_FILE, "gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf");
    assert_eq!(
        GEMMA26_SHA256,
        "2a1d26dfe6ea00a467940a5728316af6edb366bbdba950d65b85d232392fb658"
    );
    assert_eq!(
        MMPROJ26_URL,
        "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF/resolve/c099eb4/mmproj-F16.gguf"
    );
    assert_eq!(MMPROJ26_SIZE, 1_193_058_784);
    assert_eq!(
        MMPROJ26_SHA256,
        "418a6d8723067cd712235facbbc5cba6c8fbbd413fc1292d2aace5a027d5a42f"
    );
}

/// Any explicit managed model is selected only when complete.
#[test]
fn pick_llama_gguf_uses_explicit_model_only_when_present() {
    let dir = Path::new("C:/root/llama.cpp");
    let fallback = dir.join(GEMMA_FILE);
    let legacy = dir.join(LEGACY_GEMMA_FILE);
    let primary = dir.join(GEMMA26_FILE);
    assert_eq!(
        pick_llama_gguf(dir, ManagedModel::Primary26B, true),
        primary
    );
    assert_eq!(
        pick_llama_gguf(dir, ManagedModel::Primary26B, false),
        fallback
    );
    assert_eq!(pick_llama_gguf(dir, ManagedModel::Legacy4B, true), legacy);
    assert_eq!(
        pick_llama_gguf(dir, ManagedModel::Fallback12B, true),
        fallback
    );
}

/// UI presence is an O(1) size check: Settings must never hash the 10.5-GB
/// model. Exact SHA-256 validation is covered separately at the launch boundary.
#[test]
fn quality_model_present_rejects_wrong_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("llama.cpp")).unwrap();
    assert!(!quality_model_present(root), "absent file → not present");
    std::fs::write(quality_gguf_path(root), b"partial").unwrap();
    assert!(!quality_model_present(root), "truncated file → not present");
    make_complete(&quality_gguf_path(root), GEMMA26_SIZE + 1);
    assert!(!quality_model_present(root), "oversized file → not present");
}

#[test]
fn stat_only_presence_accepts_an_exact_size_fixture_without_hashing() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("llama.cpp")).unwrap();
    make_complete(&quality_gguf_path(root), GEMMA26_SIZE);
    assert!(quality_model_present(root));
}

#[test]
fn fallback_presence_rejects_the_old_1472_byte_short_size() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    let path = llama_dir.join(GEMMA_FILE);
    make_complete(&path, GEMMA_SIZE - 1_472);
    assert!(!fallback_model_present(tmp.path()));
    make_complete(&path, GEMMA_SIZE);
    assert!(fallback_model_present(tmp.path()));
}

#[test]
fn pinned_presence_rejects_same_size_corruption() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("model.gguf");
    const PRIMARY_SHA: &str = "986a1b7135f4986150aa5fa0028feeaa66cdaf3ed6a00a355dd86e042f7fb494";
    std::fs::write(&path, b"primary").unwrap();
    assert!(pinned_file_matches(&path, 7, PRIMARY_SHA));
    std::fs::write(&path, b"corrupt").unwrap();
    assert_eq!(file_len(&path), 7, "fixture must keep the same byte length");
    assert!(!pinned_file_matches(&path, 7, PRIMARY_SHA));
}

#[test]
fn rejected_same_size_primary_is_removed_for_redownload() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("primary.gguf");
    std::fs::write(&path, b"corrupt").unwrap();
    assert_eq!(file_len(&path), 7, "fixture must be exact-size");

    discard_rejected_pinned_file(&path, 7);

    assert!(
        !path.exists(),
        "a SHA-rejected exact-size primary must not hide re-download"
    );
}

#[test]
fn quality_gguf_path_is_under_llama_dir() {
    let p = quality_gguf_path(Path::new("C:/root"));
    assert!(p.ends_with(GEMMA26_FILE));
    assert!(p.to_string_lossy().contains("llama.cpp"));
}

/// A persisted 26B choice whose file vanished resolves to and persists 12B,
/// while a stale prep-model id is removed from the managed single-model server.
#[test]
fn persisted_primary_falls_back_to_12b_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        active_local_model_name(tmp.path(), ManagedModel::Fallback12B),
        GEMMA_FILE
    );
    assert_eq!(
        active_local_model_name(tmp.path(), ManagedModel::Primary26B),
        GEMMA_FILE
    );
    assert!(!effective_local_quality(tmp.path(), true));

    let mut cfg = crate::config::Config {
        ai_local_base_url: "http://[::1]:8080/v1/".to_string(),
        ai_local_model: GEMMA26_FILE.to_string(),
        ai_local_prep_model: "stale-prep".to_string(),
        ai_local_quality: true,
        ai_local_vision: true,
        vision_provider: "same".to_string(),
        ..Default::default()
    };
    assert!(repair_managed_model_state(&mut cfg, tmp.path()));
    assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);
    assert!(!cfg.ai_local_quality);
    assert_eq!(cfg.ai_local_model, GEMMA_FILE);
    assert!(cfg.ai_local_prep_model.is_empty());
    assert!(!cfg.ai_local_vision);
    assert_eq!(cfg.vision_provider, "off");
    assert!(!repair_managed_model_state(&mut cfg, tmp.path()));
}

#[test]
fn managed_primary_uses_only_its_matching_projector() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    make_complete(&llama_dir.join(GEMMA26_FILE), GEMMA26_SIZE);
    make_complete(&llama_dir.join(MMPROJ26_FILE), MMPROJ26_SIZE);
    std::fs::write(
        llama_dir.join(".llama-build"),
        format!("b{GEMMA26_MIN_BUILD}"),
    )
    .unwrap();
    let mut cfg = crate::config::Config {
        ai_local_base_url: LLAMA_BASE_URL.to_string(),
        ai_local_model: GEMMA26_FILE.to_string(),
        ai_local_quality: true,
        ai_local_vision: true,
        vision_provider: "same".to_string(),
        ..Default::default()
    };

    assert!(local_vision_enabled(&cfg, tmp.path()));
    assert!(!repair_managed_model_state(&mut cfg, tmp.path()));
    assert!(cfg.ai_local_vision);
    assert_eq!(cfg.vision_provider, "same");
}

/// Without the matching projector, selecting "Same as text model above" must
/// still be repaired so F8 is not routed to a text-only server.
#[test]
fn managed_primary_repairs_same_vision_provider_after_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    make_complete(&llama_dir.join(GEMMA26_FILE), GEMMA26_SIZE);
    let mut cfg = crate::config::Config {
        ai_local_base_url: LLAMA_BASE_URL.to_string(),
        ai_local_model: GEMMA26_FILE.to_string(),
        ai_local_quality: true,
        vision_provider: "same".to_string(),
        ..Default::default()
    };

    assert!(repair_managed_model_state(&mut cfg, tmp.path()));
    assert_eq!(cfg.vision_provider, "off");
    assert!(cfg.vision_endpoint().is_none());
}

#[test]
fn local_vision_toggle_enables_f8_route_for_ready_12b() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    make_complete(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE);
    make_complete(&llama_dir.join(MMPROJ_FILE), MMPROJ_SIZE);
    std::fs::write(
        llama_dir.join(".llama-build"),
        format!("b{GEMMA4UV_MIN_BUILD}"),
    )
    .unwrap();
    let mut cfg = crate::config::Config {
        ai_local_base_url: LLAMA_BASE_URL.to_string(),
        ai_local_model: GEMMA_FILE.to_string(),
        vision_provider: "off".to_string(),
        ..Default::default()
    };

    assert!(local_vision_available(&cfg, tmp.path()));
    set_local_vision(&mut cfg, tmp.path(), true);
    assert!(cfg.ai_local_vision);
    assert_eq!(cfg.vision_provider, "same");
    assert!(cfg.vision_endpoint().is_some());

    set_local_vision(&mut cfg, tmp.path(), false);
    assert!(!cfg.ai_local_vision);
    assert_eq!(cfg.vision_provider, "off");
}

#[test]
fn custom_gguf_validation_and_repair_are_machine_local() {
    let tmp = tempfile::tempdir().unwrap();
    let good = tmp.path().join("my-model.GGUF");
    let bad = tmp.path().join("bad.gguf");
    std::fs::write(&good, b"GGUFfixture").unwrap();
    std::fs::write(&bad, b"nope").unwrap();

    assert_eq!(
        valid_custom_gguf_path(&good.to_string_lossy()),
        Some(good.clone())
    );
    assert!(valid_custom_gguf_path(&bad.to_string_lossy()).is_none());
    assert!(valid_custom_gguf_path("relative.gguf").is_none());

    let mut cfg = crate::config::Config {
        ai_local_base_url: LLAMA_BASE_URL.to_string(),
        ai_local_model: "stale".to_string(),
        ai_local_custom_gguf: good.to_string_lossy().into_owned(),
        ai_local_quality: true,
        ai_local_vision: true,
        vision_provider: "same".to_string(),
        ..Default::default()
    };
    assert!(repair_managed_model_state(&mut cfg, tmp.path()));
    assert_eq!(cfg.ai_local_custom_gguf, good.to_string_lossy());
    assert_eq!(cfg.ai_local_model, "my-model.GGUF");
    assert!(!cfg.ai_local_quality);
    assert!(!cfg.ai_local_vision);
    assert_eq!(cfg.vision_provider, "off");
    assert!(!local_vision_available(&cfg, tmp.path()));

    std::fs::remove_file(good).unwrap();
    assert!(repair_managed_model_state(&mut cfg, tmp.path()));
    assert!(cfg.ai_local_custom_gguf.is_empty());
    assert_eq!(cfg.ai_local_model, GEMMA_FILE);
}

#[test]
fn legacy_4b_install_survives_upgrade_until_12b_is_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    make_complete(&llama_dir.join(LEGACY_GEMMA_FILE), LEGACY_GEMMA_SIZE);

    assert!(base_model_present(tmp.path()));
    assert_eq!(
        active_local_model_name(tmp.path(), ManagedModel::Fallback12B),
        LEGACY_GEMMA_FILE
    );
    assert_eq!(
        active_local_model_name(tmp.path(), ManagedModel::Primary26B),
        LEGACY_GEMMA_FILE
    );

    let mut cfg = crate::config::Config {
        ai_local_base_url: "http://localhost:8080/v1".to_string(),
        ai_local_model: GEMMA26_FILE.to_string(),
        ai_local_prep_model: "stale-prep".to_string(),
        ai_local_quality: true,
        ..Default::default()
    };
    assert!(repair_managed_model_state(&mut cfg, tmp.path()));
    assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);
    assert!(!cfg.ai_local_quality);
    assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);
    assert!(cfg.ai_local_prep_model.is_empty());

    make_complete(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE);
    assert!(
        !repair_managed_model_state(&mut cfg, tmp.path()),
        "an explicit 4B selection must survive after 12B is installed"
    );
    assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);
}

#[test]
fn engine_verification_uses_complete_legacy_4b_until_12b_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();

    assert!(complete_fallback_llama_gguf(&llama_dir).is_none());

    let legacy = llama_dir.join(LEGACY_GEMMA_FILE);
    make_complete(&legacy, LEGACY_GEMMA_SIZE);
    assert_eq!(complete_fallback_llama_gguf(&llama_dir), Some(legacy));

    let current = llama_dir.join(GEMMA_FILE);
    make_complete(&current, GEMMA_SIZE);
    assert_eq!(complete_fallback_llama_gguf(&llama_dir), Some(current));
}

#[test]
fn selecting_local_provider_repairs_managed_state_before_prep_requests() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = crate::config::Config {
        ai_provider: "cloud".to_string(),
        ai_local_base_url: "http://[::1]:8080/v1".to_string(),
        ai_local_model: GEMMA26_FILE.to_string(),
        ai_local_prep_model: "stale-prep".to_string(),
        ai_local_quality: true,
        ..Default::default()
    };

    assert!(select_local_provider(&mut cfg, tmp.path()));
    assert_eq!(cfg.ai_provider, "local");
    assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);
    assert_eq!(cfg.ai_local_model, GEMMA_FILE);
    assert!(cfg.ai_local_prep_model.is_empty());
    assert_eq!(cfg.ai_endpoint(true).model, GEMMA_FILE);
}

/// The bar must show the fast vs smart model distinctly. Pin the friendly
/// label against the actual pinned GGUF constants.
#[test]
fn local_model_label_distinguishes_fallback_and_primary() {
    assert_eq!(local_model_label(GEMMA_FILE), "Gemma 12B");
    assert_eq!(local_model_label(GEMMA26_FILE), "Gemma 26B-A4B");
    assert_eq!(local_model_label("GEMMA-4-12B-IT.gguf"), "Gemma 12B");
    // A Gemma file with no size token → bare "Gemma" (never empty).
    assert_eq!(local_model_label("gemma-it.gguf"), "Gemma");
    // Non-Gemma local model → first filename token, never empty.
    assert_eq!(local_model_label("qwen2.5-7b-instruct.gguf"), "qwen2");
    assert_eq!(local_model_label(""), "—");
}

/// Each managed model gets only its matching complete projector on a compatible
/// engine build.
#[test]
fn mmproj_attach_rules_are_model_specific() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    assert!(mmproj_for_model(dir, &dir.join(GEMMA_FILE)).is_none());
    make_complete(&dir.join(MMPROJ_FILE), MMPROJ_SIZE);
    assert!(mmproj_for_model(dir, &dir.join(GEMMA_FILE)).is_none());
    std::fs::write(dir.join(".llama-build"), format!("b{GEMMA4UV_MIN_BUILD}")).unwrap();
    assert_eq!(
        mmproj_for_model(dir, &dir.join(GEMMA_FILE)),
        Some(dir.join(MMPROJ_FILE))
    );
    make_complete(&dir.join(MMPROJ26_FILE), MMPROJ26_SIZE);
    std::fs::write(dir.join(".llama-build"), format!("b{GEMMA26_MIN_BUILD}")).unwrap();
    assert_eq!(
        mmproj_for_model(dir, &dir.join(GEMMA26_FILE)),
        Some(dir.join(MMPROJ26_FILE))
    );
    // Non-Gemma model never gets a Gemma projector.
    assert!(mmproj_for_model(dir, &dir.join("qwen2.5-7b.gguf")).is_none());
}

#[test]
fn primary_model_requires_the_tested_llama_build() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(!llama_build_supports_26b(tmp.path()));
    std::fs::write(
        tmp.path().join(".llama-build"),
        format!("b{}", GEMMA26_MIN_BUILD - 1),
    )
    .unwrap();
    assert!(!llama_build_supports_26b(tmp.path()));
    std::fs::write(
        tmp.path().join(".llama-build"),
        format!("b{GEMMA26_MIN_BUILD}"),
    )
    .unwrap();
    assert!(llama_build_supports_26b(tmp.path()));
}

/// The owner matrix is minimum-RAM based: each VRAM tier keeps its profile at
/// the documented minimum and at any larger RAM value, while insufficient or
/// missing inputs stay `Unknown` (never promoted into a stronger profile).
#[test]
fn owner_hardware_matrix_is_minimum_ram() {
    use HardwareModelProfile::*;
    let cases: &[(Option<u64>, Option<u64>, HardwareModelProfile)] = &[
        // Exact minimums.
        (Some(8), Some(16), Fallback12B),
        (Some(8), Some(32), Primary26Vram8),
        (Some(12), Some(24), Primary26Vram12),
        (Some(16), Some(32), Primary26Vram16),
        // Extra RAM beyond the minimum keeps the tier (monotonic).
        (Some(8), Some(64), Primary26Vram8),
        (Some(12), Some(64), Primary26Vram12),
        (Some(16), Some(64), Primary26Vram16),
        // 8 VRAM with RAM inside the 16..31 fallback band.
        (Some(8), Some(24), Fallback12B),
        // Insufficient RAM for the VRAM tier.
        (Some(16), Some(24), Unknown),
        (Some(12), Some(16), Unknown),
        (Some(8), Some(8), Unknown),
        // Missing inputs.
        (None, Some(32), Unknown),
        (Some(16), None, Unknown),
        (None, None, Unknown),
    ];
    for (vram, ram, want) in cases {
        assert_eq!(
            select_hardware_model_profile(*vram, *ram),
            *want,
            "vram={vram:?} ram={ram:?}"
        );
    }
}

/// Tester regression: 16 GiB VRAM + 64 GiB RAM must flow through discovery to
/// Primary26Vram16 (previously rejected as Unknown), keeping Auto compact at
/// 16K while K96 and the profile ceiling both reach 96K.
#[test]
fn tester_16vram_64ram_gets_primary26_and_full_context() {
    let profile = hardware_profile_from_discovery(false, Some(16), Some(64));
    assert_eq!(profile, HardwareModelProfile::Primary26Vram16);
    assert!(primary_26b_allowed(profile));
    assert_eq!(profile.context_tokens(false), 98_304);
    assert_eq!(
        LocalContextPreset::Auto.context_tokens(profile, false),
        16_384
    );
    assert_eq!(
        LocalContextPreset::K96.context_tokens(profile, false),
        98_304
    );
}

#[test]
fn vram_release_uses_the_largest_dedicated_adapter_and_requires_a_drop() {
    assert_eq!(
        parse_nvidia_memory_mib("512, 8192\n1537, 16384\n"),
        Some((1537, 16384))
    );
    assert_eq!(parse_nvidia_memory_mib("not available\n"), None);
    assert!(vram_has_released(8192, Some(2048)));
    assert!(vram_has_released(8192, Some(8128)));
    assert!(!vram_has_released(8192, Some(8129)));
    assert!(!vram_has_released(8192, Some(8192)));
    assert!(!vram_has_released(8192, None));
    assert!(vram_is_at_baseline(2048, Some(2112)));
    assert!(!vram_is_at_baseline(2048, Some(2113)));
    assert!(!vram_is_at_baseline(2048, None));
}

#[test]
fn only_confirmed_profiles_allow_manual_primary_selection() {
    assert!(!primary_26b_allowed(HardwareModelProfile::Unknown));
    assert!(!primary_26b_allowed(HardwareModelProfile::Fallback12B));
    assert!(primary_26b_allowed(HardwareModelProfile::Primary26Vram8));
    assert!(primary_26b_allowed(HardwareModelProfile::Primary26Vram12));
    assert!(primary_26b_allowed(HardwareModelProfile::Primary26Vram16));
}

#[test]
fn only_nvidia_discovery_enters_the_confirmed_matrix() {
    assert_eq!(
        hardware_profile_from_discovery(false, Some(8), Some(32)),
        HardwareModelProfile::Primary26Vram8
    );
    assert_eq!(
        hardware_profile_from_discovery(false, Some(12), Some(24)),
        HardwareModelProfile::Primary26Vram12
    );
    assert_eq!(
        hardware_profile_from_discovery(false, Some(16), Some(32)),
        HardwareModelProfile::Primary26Vram16
    );
    assert_eq!(
        hardware_profile_from_discovery(false, None, Some(32)),
        HardwareModelProfile::Unknown,
        "AMD/Intel or unreported VRAM must not reuse the NVIDIA matrix"
    );
}

#[test]
fn normalization_snaps_near_nominal_readings_to_approved_tiers() {
    // VRAM: ±1 GiB around each approved tier (8, 12, 16).
    assert_eq!(normalize_vram_gib(7), 8);
    assert_eq!(normalize_vram_gib(8), 8);
    assert_eq!(normalize_vram_gib(9), 8);
    assert_eq!(normalize_vram_gib(11), 12);
    assert_eq!(normalize_vram_gib(12), 12);
    assert_eq!(normalize_vram_gib(13), 12);
    assert_eq!(normalize_vram_gib(15), 16);
    assert_eq!(normalize_vram_gib(16), 16);
    assert_eq!(normalize_vram_gib(17), 16);

    // RAM: ±1 GiB around each approved tier (16, 24, 32).
    assert_eq!(normalize_ram_gib(15), 16);
    assert_eq!(normalize_ram_gib(16), 16);
    assert_eq!(normalize_ram_gib(17), 16);
    assert_eq!(normalize_ram_gib(23), 24);
    assert_eq!(normalize_ram_gib(24), 24);
    assert_eq!(normalize_ram_gib(25), 24);
    assert_eq!(normalize_ram_gib(31), 32);
    assert_eq!(normalize_ram_gib(32), 32);
    assert_eq!(normalize_ram_gib(33), 32);
}

#[test]
fn normalization_never_promotes_clearly_smaller_hardware() {
    // 6 GiB VRAM is 2 away from the 8 GiB tier — must stay 6.
    assert_eq!(normalize_vram_gib(6), 6);
    assert_eq!(normalize_vram_gib(4), 4);
    assert_eq!(normalize_vram_gib(10), 10);
    assert_eq!(normalize_vram_gib(14), 14);
    assert_eq!(normalize_vram_gib(18), 18);

    // RAM clearly below a tier stays put.
    assert_eq!(normalize_ram_gib(14), 14);
    assert_eq!(normalize_ram_gib(22), 22);
    assert_eq!(normalize_ram_gib(26), 26);
    assert_eq!(normalize_ram_gib(30), 30);
    assert_eq!(normalize_ram_gib(34), 34);

    // End-to-end: 6 GiB VRAM + 32 GiB RAM must remain Unknown.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(6), Some(32)),
        HardwareModelProfile::Unknown,
        "6 GiB VRAM must never enter the confirmed 26B matrix"
    );
}

#[test]
fn igpu_ram_reservation_snaps_to_existing_profile() {
    // Owner's machine: 16 GiB NVIDIA + 32 GiB installed, iGPU reserves ~1 GiB
    // → TotalPhysicalMemory reports 31 GiB usable.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(16), Some(31)),
        HardwareModelProfile::Primary26Vram16,
        "16/31 must snap to the 16/32 profile (iGPU reservation)"
    );
    // Same for the 8 GiB VRAM tier.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(8), Some(31)),
        HardwareModelProfile::Primary26Vram8,
        "8/31 must snap to the 8/32 profile"
    );
    // 12 GiB VRAM tier already accepts 24..=32, so 31 matches without
    // normalization — verify it still works through the pipeline.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(12), Some(31)),
        HardwareModelProfile::Primary26Vram12
    );
    // Fallback tier: 8/15 → 8/16.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(8), Some(15)),
        HardwareModelProfile::Fallback12B,
        "8/15 must snap to the 8/16 fallback profile"
    );
}

#[test]
fn near_nominal_vram_snaps_to_existing_profile() {
    // 15 GiB reported for a 16 GiB card (firmware underreport).
    assert_eq!(
        hardware_profile_from_discovery(false, Some(15), Some(32)),
        HardwareModelProfile::Primary26Vram16,
        "15/32 must snap to the 16/32 profile"
    );
    // 11 GiB reported for a 12 GiB card.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(11), Some(32)),
        HardwareModelProfile::Primary26Vram12,
        "11/32 must snap to the 12/32 profile"
    );
    // 7 GiB reported for an 8 GiB card.
    assert_eq!(
        hardware_profile_from_discovery(false, Some(7), Some(16)),
        HardwareModelProfile::Fallback12B,
        "7/16 must snap to the 8/16 fallback profile"
    );
}

#[test]
fn strongest_adapter_vram_normalizes_correctly() {
    // Multi-GPU nvidia-smi output: iGPU (512 MiB) + dGPU (16384 MiB).
    // parse_nvidia_memory_mib picks the strongest (max total).
    let (used, total) = parse_nvidia_memory_mib("128, 512\n1024, 16384\n").unwrap();
    assert_eq!((used, total), (1024, 16384));
    // Same GiB conversion as detect_nvidia_vram_gib: round-to-nearest.
    let vram_gib = (total + 512) / 1024;
    assert_eq!(vram_gib, 16);
    assert_eq!(normalize_vram_gib(vram_gib), 16);

    // Strongest adapter reports 15360 MiB (some 16 GiB cards underreport).
    let (_, total_under) = parse_nvidia_memory_mib("256, 512\n900, 15360\n").unwrap();
    let vram_under = (total_under + 512) / 1024;
    assert_eq!(vram_under, 15, "15360 MiB rounds to 15 GiB");
    assert_eq!(
        normalize_vram_gib(vram_under),
        16,
        "15 GiB snaps back to the 16 GiB tier"
    );
}

#[test]
fn launcher_uses_fixed_context_with_the_confirmed_hardware_matrix() {
    let cases = [
        (
            HardwareModelProfile::Fallback12B,
            false,
            LocalContextPreset::K32,
            "32768",
            "34",
            None,
            false,
        ),
        (
            HardwareModelProfile::Primary26Vram8,
            false,
            LocalContextPreset::K32,
            "32768",
            "99",
            Some("20"),
            false,
        ),
        (
            HardwareModelProfile::Primary26Vram8,
            true,
            LocalContextPreset::K64,
            "32768",
            "99",
            Some("20"),
            true,
        ),
        (
            HardwareModelProfile::Primary26Vram12,
            false,
            LocalContextPreset::K64,
            "65536",
            "99",
            Some("8"),
            false,
        ),
        (
            HardwareModelProfile::Primary26Vram12,
            true,
            LocalContextPreset::K96,
            "65536",
            "99",
            Some("8"),
            true,
        ),
        (
            HardwareModelProfile::Primary26Vram16,
            true,
            LocalContextPreset::K96,
            "98304",
            "99",
            None,
            false,
        ),
    ];
    for (profile, prep, preset, context, ngl, ncmoe, q8) in cases {
        let args = llama_server_args("model.gguf", "alias", None, false, profile, preset, prep);
        assert!(args.windows(2).any(|pair| pair == ["-c", context]));
        assert!(args.windows(2).any(|pair| pair == ["-ngl", ngl]));
        assert!(args.windows(2).any(|pair| pair == ["-np", "1"]));
        assert!(args.iter().any(|arg| arg == "--no-mmap"));
        assert_eq!(
            args.windows(2)
                .find(|pair| pair.first().is_some_and(|arg| arg == "-ncmoe"))
                .and_then(|pair| pair.get(1))
                .map(String::as_str),
            ncmoe
        );
        assert_eq!(args.iter().any(|arg| arg == "-ctk"), q8);
        assert_eq!(args.iter().any(|arg| arg == "-ctv"), q8);
    }

    let unknown = llama_server_args(
        "model.gguf",
        "alias",
        Some("vision.gguf"),
        false,
        HardwareModelProfile::Unknown,
        LocalContextPreset::Auto,
        false,
    );
    assert!(!unknown.iter().any(|arg| arg == "-ngl"));
    assert!(unknown.windows(2).any(|pair| pair == ["--alias", "alias"]));
    let cpu = llama_server_args(
        "model.gguf",
        "alias",
        None,
        true,
        HardwareModelProfile::Unknown,
        LocalContextPreset::Auto,
        false,
    );
    assert!(cpu.windows(2).any(|pair| pair == ["-ngl", "0"]));
}

#[test]
fn context_presets_are_compact_clamped_and_stable() {
    let known = HardwareModelProfile::Primary26Vram16;
    let weak = HardwareModelProfile::Unknown;
    assert_eq!(
        LocalContextPreset::Auto.context_tokens(known, false),
        16_384
    );
    assert_eq!(LocalContextPreset::Auto.context_tokens(weak, false), 8_192);
    assert_eq!(LocalContextPreset::K96.context_tokens(known, true), 98_304);
    assert_eq!(LocalContextPreset::K96.context_tokens(weak, true), 8_192);

    for (index, value) in ["auto", "8k", "16k", "32k", "64k", "96k"]
        .into_iter()
        .enumerate()
    {
        let preset = LocalContextPreset::from_config(value);
        assert_eq!(preset.index(), index as i32);
        assert_eq!(LocalContextPreset::from_index(index as i32), preset);
        assert_eq!(preset.as_config(), value);
    }
    assert_eq!(
        LocalContextPreset::from_config("invalid"),
        LocalContextPreset::Auto
    );
    assert_eq!(LocalContextPreset::K8.estimated_vram_delta_mib(known), -168);
    assert_eq!(
        LocalContextPreset::K96.estimated_vram_delta_mib(known),
        1_687
    );
    assert_eq!(
        ManagedModel::from_config(LEGACY_GEMMA_FILE, false),
        ManagedModel::Legacy4B
    );
    assert_eq!(
        ManagedModel::from_config(GEMMA_FILE, true),
        ManagedModel::Primary26B
    );
    assert_eq!(ManagedModel::from_index(0).index(), 0);
    assert_eq!(ManagedModel::from_index(1).index(), 1);
    assert_eq!(ManagedModel::from_index(2).index(), 2);
    assert!(
        estimated_total_vram_mib(ManagedModel::Legacy4B, LocalContextPreset::Auto, known)
            < estimated_total_vram_mib(ManagedModel::Fallback12B, LocalContextPreset::Auto, known)
    );
    assert!(
        estimated_total_vram_mib(ManagedModel::Primary26B, LocalContextPreset::K8, known)
            < estimated_total_vram_mib(ManagedModel::Primary26B, LocalContextPreset::K96, known)
    );
}

#[test]
fn vision_memory_warning_is_explicitly_unknown() {
    let root = Path::new("C:/root");
    let primary = local_model_resource_warning(root, LLAMA_BASE_URL, GEMMA26_FILE);
    let fallback = local_model_resource_warning(root, LLAMA_BASE_URL, GEMMA_FILE);
    let legacy = local_model_resource_warning(root, LLAMA_BASE_URL, LEGACY_GEMMA_FILE);
    assert!(primary.contains("Память для vision: неизвестно"));
    assert!(fallback.contains("Память для vision: неизвестно"));
    assert!(legacy.contains("Память для vision: неизвестно"));
}

#[test]
fn managed_endpoint_accepts_ipv4_hostname_and_ipv6_loopback_only() {
    for endpoint in [
        "http://127.0.0.1:8080/v1",
        "http://localhost:8080/v1/",
        "http://[::1]:8080/v1",
    ] {
        assert!(is_managed_llama_endpoint(endpoint), "{endpoint}");
    }
    for endpoint in [
        "https://localhost:8080/v1",
        "http://127.0.0.1:11434/v1",
        "http://10.0.0.8:8080/v1",
        "http://[::2]:8080/v1",
    ] {
        assert!(!is_managed_llama_endpoint(endpoint), "{endpoint}");
    }
}

#[test]
fn strict_readiness_rejects_http_errors_wrong_model_and_malformed_choices() {
    let models = r#"{"data":[{"id":"gemma-26"}]}"#;
    let completion = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
    assert!(expected_model_is_ready(
        true, models, "gemma-26", true, completion
    ));
    assert!(!expected_model_is_ready(
        false, models, "gemma-26", true, completion
    ));
    assert!(!expected_model_is_ready(
        true, models, "gemma-12", true, completion
    ));
    assert!(!expected_model_is_ready(
        true,
        models,
        "gemma-26",
        true,
        r#"{"choices":[{}]}"#
    ));
}

#[cfg(windows)]
fn spawn_stale_llama_server(
    expected: &'static str,
) -> (String, mpsc::Sender<()>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    let (stop_tx, stop_rx) = mpsc::channel();
    let server = thread::spawn(move || loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).unwrap_or(0);
                let body = if request[..read].starts_with(b"GET /v1/models") {
                    format!(r#"{{"data":[{{"id":"{expected}"}}]}}"#)
                } else {
                    r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#.to_string()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if stop_rx.recv_timeout(Duration::from_millis(10)).is_ok() {
                    return;
                }
            }
            Err(_) => return,
        }
    });
    (base_url, stop_tx, server)
}

/// A stale listener may still advertise the requested model and answer a
/// completion after the freshly launched server loses the bind race. Install
/// readiness must reject that reply because its own child has already exited.
#[cfg(windows)]
#[test]
fn strict_readiness_rejects_stale_server_after_new_child_exits() {
    const EXPECTED: &str = "newly-launched-model";
    let (base_url, stop, server) = spawn_stale_llama_server(EXPECTED);
    let models_url = format!("{base_url}/models");
    let completion_url = format!("{base_url}/chat/completions");
    let probe_deadline = Instant::now() + Duration::from_secs(2);
    let (models, completion) = loop {
        let models = curl_success_body(&["-f", "-sS", "--max-time", "1", &models_url]);
        let completion = curl_success_body(&[
            "-f",
            "-sS",
            "--max-time",
            "1",
            "-X",
            "POST",
            &completion_url,
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"model":"newly-launched-model","messages":[{"role":"user","content":"hi"}],"max_tokens":1}"#,
        ]);
        if expected_model_is_ready(
            models.is_some(),
            models.as_deref().unwrap_or_default(),
            EXPECTED,
            completion.is_some(),
            completion.as_deref().unwrap_or_default(),
        ) || Instant::now() >= probe_deadline
        {
            break (models, completion);
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        expected_model_is_ready(
            models.is_some(),
            models.as_deref().unwrap_or_default(),
            EXPECTED,
            completion.is_some(),
            completion.as_deref().unwrap_or_default(),
        ),
        "models={models:?} completion={completion:?}"
    );

    let mut child = spawn_exiting_child();
    let _ = child.wait();
    let mut children = [child];
    assert!(
        !wait_for_expected_model_at(
            &base_url,
            EXPECTED,
            Duration::from_secs(1),
            &mut children[0]
        ),
        "a stale exact-model response must not verify an exited new child"
    );
    let _ = stop.send(());
    server.join().unwrap();
}

fn spawn_long_lived_child() -> Child {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping -n 4 127.0.0.1 > NUL"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 3"]);
        command
    };
    command.spawn().unwrap()
}

fn spawn_exiting_child() -> Child {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "exit 0"]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        command
    };
    command.spawn().unwrap()
}

#[test]
fn llama_readiness_ignores_an_exited_whisper_child() {
    let llama = spawn_long_lived_child();
    let mut whisper = spawn_exiting_child();
    let _ = whisper.wait().unwrap();
    let mut children = vec![llama, whisper];

    assert!(
        launched_llama_alive(&mut children[0]),
        "an exited optional Whisper child must not fail llama readiness"
    );
    terminate_servers(children);
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
        ai_local_prep_model: "stale-prep-model".to_string(),
        ai_local_quality: true,
        // a prior cloud setting — apply_result switches F8 to local on a
        // local install (vision rides the same local server).
        vision_provider: "cloud".to_string(),
        ..Default::default()
    };
    let res = local_result(false, true);
    apply_result(&mut cfg, &res);
    assert_eq!(cfg.ai_provider, "local");
    assert_eq!(cfg.ai_local_base_url, LLAMA_BASE_URL);
    assert_eq!(cfg.ai_local_model, GEMMA_FILE);
    assert!(!cfg.ai_local_quality);
    assert!(cfg.ai_local_prep_model.is_empty());
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

#[test]
fn apply_primary_routes_vision_only_when_the_installer_verified_it() {
    let mut cloud_cfg = crate::config::Config {
        ai_local_prep_model: "stale-prep-model".to_string(),
        vision_provider: "cloud".to_string(),
        ..Default::default()
    };
    apply_result(&mut cloud_cfg, &local_result(true, false));
    assert_eq!(cloud_cfg.ai_local_model, GEMMA26_FILE);
    assert!(cloud_cfg.ai_local_quality);
    assert!(cloud_cfg.ai_local_prep_model.is_empty());
    assert!(!cloud_cfg.ai_local_vision);
    assert_eq!(
        cloud_cfg.vision_provider, "cloud",
        "an explicit separate/cloud vision route is preserved"
    );

    let mut stale_same_cfg = crate::config::Config {
        vision_provider: "same".to_string(),
        ..Default::default()
    };
    apply_result(&mut stale_same_cfg, &local_result(true, true));
    assert!(stale_same_cfg.ai_local_vision);
    assert_eq!(stale_same_cfg.vision_provider, "same");

    let mut inherited_local_cfg = crate::config::Config {
        vision_provider: "local".to_string(),
        vision_local_base_url: String::new(),
        ..Default::default()
    };
    apply_result(&mut inherited_local_cfg, &local_result(true, false));
    assert_eq!(inherited_local_cfg.vision_provider, "off");

    let mut separate_local_cfg = crate::config::Config {
        vision_provider: "local".to_string(),
        vision_local_base_url: "http://127.0.0.1:8082/v1".to_string(),
        ..Default::default()
    };
    apply_result(&mut separate_local_cfg, &local_result(true, false));
    assert_eq!(
        separate_local_cfg.vision_provider, "local",
        "an explicit separate local vision server is preserved"
    );
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

// ---- v0.35.2: all built-in profiles remain selectable on ANY hardware -------

#[test]
fn all_managed_models_accessible_regardless_of_hardware() {
    // Every built-in profile index resolves to a distinct model with a valid
    // file name, independent of the detected hardware profile.
    let models = [
        ManagedModel::from_index(0),
        ManagedModel::from_index(1),
        ManagedModel::from_index(2),
    ];
    assert_eq!(models[0], ManagedModel::Legacy4B);
    assert_eq!(models[1], ManagedModel::Fallback12B);
    assert_eq!(models[2], ManagedModel::Primary26B);
    for m in &models {
        assert!(!m.file_name().is_empty());
    }
    // Round-trip through index.
    for m in models {
        assert_eq!(ManagedModel::from_index(m.index()), m);
    }
}

#[test]
fn low_hardware_profile_is_recommendation_only() {
    // 6 GiB NVIDIA + 16 GiB RAM → Unknown (not in the confirmed matrix).
    let profile = hardware_profile_from_discovery(false, Some(6), Some(16));
    assert_eq!(profile, HardwareModelProfile::Unknown);
    // The recommendation flag is false…
    assert!(!primary_26b_allowed(profile));
    // …but every ManagedModel variant is still constructible and the hardware
    // profile does NOT gate model access.
    assert_eq!(ManagedModel::from_index(2), ManagedModel::Primary26B);
    assert_eq!(
        ManagedModel::Primary26B.file_name(),
        "gemma-4-26B-A4B-it-UD-Q2_K_XL.gguf"
    );
}

#[test]
fn sixteen_vram_31_ram_normalizes_and_exposes_all_profiles() {
    // Owner's machine with iGPU: 16 GiB VRAM, 31 GiB usable RAM.
    let profile = hardware_profile_from_discovery(false, Some(16), Some(31));
    assert_eq!(profile, HardwareModelProfile::Primary26Vram16);
    assert!(primary_26b_allowed(profile));
    // All three models remain accessible.
    for idx in 0..3 {
        let m = ManagedModel::from_index(idx);
        assert!(!m.file_name().is_empty());
    }
}

#[test]
fn unknown_hardware_does_not_block_26b_download_path() {
    // download_quality_model no longer bails on hardware. Verify the function
    // proceeds past the hardware check by calling it with a temp root and an
    // immediate cancel — it must NOT return the old hardware-rejection error.
    let root = tempfile::tempdir().unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let result = download_quality_model(root.path(), &cancel, &|_| {});
    // The cancel flag is set, so the download aborts with a cancellation error
    // (or a mkdir/network error), but NEVER with the old hardware bail message.
    if let Err(e) = result {
        let msg = format!("{e:#}");
        assert!(
            !msg.contains("confirmed VRAM/RAM hardware profile"),
            "hardware must not reject the download: {msg}"
        );
    }
}

/// Every bundled model button must pull EXACTLY the clicked model from an
/// immutable Hugging Face revision — never /main, never hardware-redirected —
/// and verify it against a full LFS SHA-256. Pure metadata assertions (no
/// download), so they run anywhere.
#[test]
fn model_specs_pin_immutable_revisions_with_full_sha() {
    for model in [
        ManagedModel::Legacy4B,
        ManagedModel::Fallback12B,
        ManagedModel::Primary26B,
    ] {
        let spec = model.spec();
        assert_eq!(spec.file, model.file_name(), "spec file matches the model");
        assert!(spec.size > 0, "{}: size pinned", spec.label);
        // Immutable revision, never the moving /main branch.
        assert!(
            spec.url.starts_with("https://huggingface.co/"),
            "{}: HF origin",
            spec.label
        );
        assert!(
            spec.url.contains("/resolve/"),
            "{}: pinned resolve",
            spec.label
        );
        assert!(
            !spec.url.contains("/resolve/main/"),
            "{}: must never resolve /main",
            spec.label
        );
        let resolve_ref = spec
            .url
            .split("/resolve/")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or_default();
        assert!(
            resolve_ref.len() >= 7 && resolve_ref.chars().all(|c| c.is_ascii_hexdigit()),
            "{}: resolve ref {resolve_ref:?} is a commit hash, not a branch",
            spec.label
        );
        assert!(
            spec.url.ends_with(&format!("/{}", spec.file)),
            "{}: url downloads the exact model file",
            spec.label
        );
        // Full 64-char LFS object hash.
        assert_eq!(spec.sha256.len(), 64, "{}: sha256 length", spec.label);
        assert!(
            spec.sha256.chars().all(|c| c.is_ascii_hexdigit()),
            "{}: sha256 is hex",
            spec.label
        );
    }
}

/// The 4B profile is pinned to the exact LFS size + SHA read from the immutable
/// Hugging Face revision bfc15c3. Lock the values so an accidental edit (or a
/// silent /main re-upload) is caught here, not on a user's machine.
#[test]
fn legacy_4b_spec_matches_the_pinned_hf_revision() {
    let spec = ManagedModel::Legacy4B.spec();
    assert_eq!(spec.file, "gemma-4-E4B-it-Q4_K_M.gguf");
    assert_eq!(spec.size, 4_977_171_584);
    assert_eq!(
        spec.sha256,
        "85a896a047553e842f25297ee5b031d64ff30147d9c4af17b1e4b394cd1fab87"
    );
    assert!(spec
        .url
        .contains("/resolve/bfc15c382204943c3a8fff0c750b94ae2364d7a3/"));
    // The presence check must agree with the download spec's exact size + hash.
    assert_eq!(spec.size, LEGACY_GEMMA_SIZE);
    assert_eq!(spec.sha256, LEGACY_GEMMA_SHA256);
}

/// UI presence for the 4B model is an O(1) size check against the pinned size.
/// Both the current pinned size and the previous release's size are accepted.
#[test]
fn legacy_presence_uses_the_pinned_4b_size() {
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    let path = llama_dir.join(LEGACY_GEMMA_FILE);
    assert!(
        !legacy_model_present(tmp.path()),
        "absent file → not present"
    );
    make_complete(&path, LEGACY_GEMMA_SIZE - 1);
    assert!(
        !legacy_model_present(tmp.path()),
        "short file → not present"
    );
    make_complete(&path, LEGACY_GEMMA_SIZE);
    assert!(legacy_model_present(tmp.path()), "exact size → present");
    make_complete(&path, LEGACY_GEMMA_SIZE_PREV);
    assert!(
        legacy_model_present(tmp.path()),
        "previous-release size → present"
    );
}

/// Hardware must never block or redirect ANY model download (4B/12B/26B). Each
/// call proceeds straight to the (here cancelled) download path — no hardware
/// rejection. Cancel is set up-front so no network I/O happens.
#[test]
fn hardware_never_blocks_any_managed_model_download() {
    for model in [
        ManagedModel::Legacy4B,
        ManagedModel::Fallback12B,
        ManagedModel::Primary26B,
    ] {
        let root = tempfile::tempdir().unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let result = download_managed_model(root.path(), model, &cancel, &|_| {});
        if let Err(e) = result {
            let msg = format!("{e:#}");
            assert!(
                !msg.contains("confirmed VRAM/RAM hardware profile"),
                "hardware must not reject the {} download: {msg}",
                model.spec().label
            );
        }
    }
}

/// Regression: a previous-release 4B file (4 977 169 568 bytes) must remain
/// Installed + launchable after upgrade, and the explicit downloader must not
/// corrupt-resume it. A fresh/missing download still targets the new pinned
/// spec (exact size + SHA-256 from the immutable HF revision).
#[test]
fn old_size_4b_upgrade_compatibility_and_new_spec_integrity() {
    // -- new-spec integrity pins ------------------------------------------------
    let spec = ManagedModel::Legacy4B.spec();
    assert_eq!(spec.size, LEGACY_GEMMA_SIZE);
    assert_eq!(spec.size, 4_977_171_584);
    assert_eq!(spec.sha256, LEGACY_GEMMA_SHA256);
    assert_ne!(
        LEGACY_GEMMA_SIZE, LEGACY_GEMMA_SIZE_PREV,
        "old and new sizes must differ"
    );
    assert_eq!(LEGACY_GEMMA_SIZE_PREV, 4_977_169_568);

    // -- old-size file: presence + selection ------------------------------------
    let tmp = tempfile::tempdir().unwrap();
    let llama_dir = tmp.path().join("llama.cpp");
    std::fs::create_dir_all(&llama_dir).unwrap();
    let path = llama_dir.join(LEGACY_GEMMA_FILE);
    make_complete(&path, LEGACY_GEMMA_SIZE_PREV);

    assert!(
        legacy_model_present(tmp.path()),
        "old-size file must be recognised as Installed"
    );
    assert!(
        base_model_present(tmp.path()),
        "old-size file must satisfy base_model_present"
    );
    assert_eq!(
        effective_managed_model(tmp.path(), ManagedModel::Legacy4B),
        ManagedModel::Legacy4B,
        "old-size file must keep the Legacy4B selection"
    );
    assert_eq!(
        selected_llama_gguf(&llama_dir, ManagedModel::Legacy4B),
        path,
        "old-size file must be launchable"
    );
    assert_eq!(
        complete_fallback_llama_gguf(&llama_dir),
        Some(path.clone()),
        "old-size file must be the fallback when 12B is absent"
    );

    // -- explicit downloader must not corrupt-resume ----------------------------
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = download_managed_model(tmp.path(), ManagedModel::Legacy4B, &cancel, &|_| {});
    assert!(
        result.is_ok(),
        "downloader must accept the old-size file: {result:?}"
    );
    assert_eq!(
        file_len(&path),
        LEGACY_GEMMA_SIZE_PREV,
        "old-size file must not be modified by the downloader"
    );

    // -- repair keeps the old-size selection ------------------------------------
    let mut cfg = crate::config::Config {
        ai_local_base_url: "http://127.0.0.1:8080/v1".to_string(),
        ai_local_model: LEGACY_GEMMA_FILE.to_string(),
        ..Default::default()
    };
    assert!(
        !repair_managed_model_state(&mut cfg, tmp.path()),
        "old-size 4B selection must survive repair"
    );
    assert_eq!(cfg.ai_local_model, LEGACY_GEMMA_FILE);
}
