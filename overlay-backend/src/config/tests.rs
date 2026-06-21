//! Unit tests for `config.rs`, split out to keep the module file lean.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

/// #125 — server-only import copies the AI/STT server fields and KEEPS
/// every local field (profiles, devices, UI, snippets, hotkeys). Exercises
/// the pure `merge_server_settings` (NOT `import_server_settings_from`,
/// which would `save()` to the real %APPDATA% config — a test must never
/// touch the user's live config).
#[test]
fn merge_server_settings_takes_servers_keeps_locals() {
    // `current` = this PC: distinctive LOCAL values + placeholder servers.
    let mut current = Config::defaults();
    current.meeting_context = "LOCAL meeting ctx".into();
    current.context_profiles = vec![ContextProfile {
        name: "local-prof".into(),
        context: "local ctx".into(),
    }];
    current.active_profile = Some("local-prof".into());
    current.mic_device = Some("Local Mic".into());
    current.system_audio_device = Some("Local Loopback".into());
    current.tile_monitor_name = Some("Local Monitor".into());
    current.trigger_keywords = "localkw".into();
    current.ui_language = "en".into();
    current.color_scheme = 3;
    current.tile_font_size = 19;
    current.snippets = vec![Snippet {
        key: "loc".into(),
        title: "Local snippet".into(),
        body: "stays".into(),
    }];
    // Local PLACEHOLDER server values (must be OVERWRITTEN by the import).
    current.ai_provider = "cloud".into();
    current.ai_base_url = "http://OLD-cloud/v1".into();
    current.ai_bearer = "OLD-bearer".into();
    current.groq_api_key = "OLD-groq".into();

    // `imported` = backup carried from the other PC: different servers AND
    // different locals (the locals must be IGNORED).
    let mut imported = Config::defaults();
    imported.ai_provider = "local".into();
    imported.ai_base_url = "http://NEW-cloud:18902/v1".into();
    imported.ai_bearer = "NEW-bearer".into();
    imported.ai_model = "claude-haiku-4-5".into();
    imported.prep_model = "claude-sonnet-4-6".into();
    imported.ai_prompt_cache = true;
    imported.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    imported.ai_local_bearer = "NEW-local-bearer".into();
    imported.ai_local_model = "gemma-4-E4B".into();
    imported.ai_local_prep_model = "gemma-prep".into();
    imported.ai_local_vision = true;
    imported.ai_local_thinking = true;
    imported.vision_provider = "local".into();
    imported.vision_base_url = "http://NEW-vision-cloud/v1".into();
    imported.vision_bearer = "NEW-vision-bearer".into();
    imported.vision_model = "claude-opus-4-7".into();
    imported.vision_local_base_url = "http://127.0.0.1:8082/v1".into();
    imported.vision_local_bearer = "NEW-vision-local-bearer".into();
    imported.vision_local_model = "qwen2-vl".into();
    imported.stt_provider = "gigaam".into();
    imported.groq_api_key = "NEW-groq".into();
    imported.stt_language = Some("en".into());
    imported.stt_model = "whisper-large-v3-turbo".into();
    imported.stt_gigaam_dir = r"C:\NEW\gigaam".into();
    imported.stt_gigaam_gpu = true;
    imported.stt_whisper_url = "http://127.0.0.1:9999/v1".into();
    imported.stt_whisper_bearer = "NEW-whisper-bearer".into();
    imported.stt_whisper_model = "whisper-NEW".into();
    // Imported LOCALS that must NOT leak in.
    imported.meeting_context = "IMPORTED ctx (ignore)".into();
    imported.mic_device = Some("Imported Mic (ignore)".into());
    imported.ui_language = "ru".into();
    imported.color_scheme = 0;
    imported.snippets = vec![Snippet {
        key: "imp".into(),
        title: "Imported snippet (ignore)".into(),
        body: "nope".into(),
    }];

    let merged = merge_server_settings(&current, imported);

    // --- server fields come from `imported` ---
    assert_eq!(merged.ai_provider, "local");
    assert_eq!(merged.ai_base_url, "http://NEW-cloud:18902/v1");
    assert_eq!(merged.ai_bearer, "NEW-bearer");
    assert_eq!(merged.ai_model, "claude-haiku-4-5");
    assert_eq!(merged.prep_model, "claude-sonnet-4-6");
    assert!(merged.ai_prompt_cache);
    assert_eq!(merged.ai_local_base_url, "http://127.0.0.1:8080/v1");
    assert_eq!(merged.ai_local_bearer, "NEW-local-bearer");
    assert_eq!(merged.ai_local_model, "gemma-4-E4B");
    assert_eq!(merged.ai_local_prep_model, "gemma-prep");
    assert!(merged.ai_local_vision);
    assert!(merged.ai_local_thinking);
    assert_eq!(merged.vision_provider, "local");
    assert_eq!(merged.vision_base_url, "http://NEW-vision-cloud/v1");
    assert_eq!(merged.vision_bearer, "NEW-vision-bearer");
    assert_eq!(merged.vision_model, "claude-opus-4-7");
    assert_eq!(merged.vision_local_base_url, "http://127.0.0.1:8082/v1");
    assert_eq!(merged.vision_local_bearer, "NEW-vision-local-bearer");
    assert_eq!(merged.vision_local_model, "qwen2-vl");
    assert_eq!(merged.stt_provider, "gigaam");
    assert_eq!(merged.groq_api_key, "NEW-groq");
    assert_eq!(merged.stt_language.as_deref(), Some("en"));
    assert_eq!(merged.stt_model, "whisper-large-v3-turbo");
    assert_eq!(merged.stt_gigaam_dir, r"C:\NEW\gigaam");
    assert!(merged.stt_gigaam_gpu);
    assert_eq!(merged.stt_whisper_url, "http://127.0.0.1:9999/v1");
    assert_eq!(merged.stt_whisper_bearer, "NEW-whisper-bearer");
    assert_eq!(merged.stt_whisper_model, "whisper-NEW");

    // --- local fields stay from `current` (NOT from `imported`) ---
    assert_eq!(merged.meeting_context, "LOCAL meeting ctx");
    assert_eq!(merged.context_profiles.len(), 1);
    assert_eq!(merged.context_profiles[0].name, "local-prof");
    assert_eq!(merged.active_profile.as_deref(), Some("local-prof"));
    assert_eq!(merged.mic_device.as_deref(), Some("Local Mic"));
    assert_eq!(
        merged.system_audio_device.as_deref(),
        Some("Local Loopback")
    );
    assert_eq!(merged.tile_monitor_name.as_deref(), Some("Local Monitor"));
    assert_eq!(merged.trigger_keywords, "localkw");
    assert_eq!(merged.ui_language, "en");
    assert_eq!(merged.color_scheme, 3);
    assert_eq!(merged.tile_font_size, 19);
    assert_eq!(merged.snippets.len(), 1);
    assert_eq!(merged.snippets[0].key, "loc");
}

/// P1.7 helper — a config whose EVERY secret-bearing field holds a unique,
/// recognisable token, so the redaction guard can assert none of them ever
/// reach a preview string. Also distinctive non-secret server fields.
fn imported_with_secret_tokens() -> Config {
    let mut c = Config::defaults();
    c.ai_provider = "local".into();
    c.ai_base_url = "http://192.168.7.7:18902/v1".into();
    c.ai_bearer = "SECRET-AI-BEARER-zzz".into();
    c.ai_model = "claude-haiku-4-5".into();
    c.prep_model = "claude-sonnet-4-6".into();
    c.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    c.ai_local_bearer = "SECRET-LOCAL-BEARER-zzz".into();
    c.ai_local_model = "gemma-4-E4B".into();
    c.vision_provider = "cloud".into();
    c.vision_base_url = "http://192.168.7.8:18903/v1".into();
    c.vision_bearer = "SECRET-VISION-BEARER-zzz".into();
    c.vision_model = "claude-opus-4-7".into();
    c.vision_local_bearer = "SECRET-VISION-LOCAL-BEARER-zzz".into();
    c.stt_provider = "gigaam".into();
    c.groq_api_key = "gsk_SECRET_GROQ_zzz".into();
    c.stt_model = "whisper-large-v3-turbo".into();
    c.stt_whisper_url = "http://127.0.0.1:8081/v1".into();
    c.stt_whisper_bearer = "SECRET-WHISPER-BEARER-zzz".into();
    c.stt_gigaam_dir = r"D:\OTHER-PC\gigaam".into();
    c
}

/// P1.7 — `export_server_settings_to` writes ONLY the AI/STT/vision server
/// fields (incl. the creds — intentional for a PC->PC transfer) and NONE of
/// the machine-local fields (meeting_context, profiles, snippets, devices,
/// monitor pin, trigger keywords, UI/theme). Round-trips through serde.
#[test]
fn export_server_settings_writes_servers_only_no_locals() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("server-settings.json");

    // A config with BOTH distinctive servers AND distinctive locals.
    let mut cfg = imported_with_secret_tokens();
    cfg.meeting_context = "PRIVATE meeting ctx".into();
    cfg.context_profiles = vec![ContextProfile {
        name: "prof".into(),
        context: "ctx".into(),
    }];
    cfg.active_profile = Some("prof".into());
    cfg.mic_device = Some("My Mic".into());
    cfg.system_audio_device = Some("My Loopback".into());
    cfg.tile_monitor_name = Some("My Monitor".into());
    cfg.trigger_keywords = "mykw".into();
    cfg.ui_language = "en".into();
    cfg.color_scheme = 3;
    cfg.snippets = vec![Snippet {
        key: "s".into(),
        title: "secret snippet".into(),
        body: "body".into(),
    }];

    export_server_settings_to(&path, &cfg).unwrap();
    let raw = std::fs::read(&path).unwrap();
    let out: Config = serde_json::from_slice(&raw).unwrap();

    // Server fields present (incl. creds — intentional).
    assert_eq!(out.ai_provider, "local");
    assert_eq!(out.ai_base_url, "http://192.168.7.7:18902/v1");
    assert_eq!(out.ai_bearer, "SECRET-AI-BEARER-zzz");
    assert_eq!(out.groq_api_key, "gsk_SECRET_GROQ_zzz");
    assert_eq!(out.vision_bearer, "SECRET-VISION-BEARER-zzz");
    assert_eq!(out.stt_whisper_url, "http://127.0.0.1:8081/v1");
    // Machine-local fields are BLANK (came from defaults, not from cfg).
    assert_eq!(out.meeting_context, "");
    assert!(out.context_profiles.is_empty());
    assert!(out.active_profile.is_none());
    assert!(out.mic_device.is_none());
    assert!(out.system_audio_device.is_none());
    assert!(out.tile_monitor_name.is_none());
    // trigger_keywords / snippets default to the canned set, NOT the user's
    // custom values — the point is they don't carry the user's locals.
    assert_ne!(out.trigger_keywords, "mykw");
    assert!(out.snippets.iter().all(|s| s.key != "s"));
    assert_eq!(out.ui_language, default_ui_language());
    assert_eq!(out.color_scheme, 0);

    // Belt-and-braces: the user's private locals must not appear anywhere in
    // the raw bytes (the snippet/profile/context strings, mic name).
    let text = String::from_utf8_lossy(&raw);
    for needle in [
        "PRIVATE meeting ctx",
        "secret snippet",
        "My Mic",
        "My Loopback",
        "My Monitor",
        "mykw",
    ] {
        assert!(
            !text.contains(needle),
            "server-settings export leaked a local field: {needle}"
        );
    }
}

/// P1.7 — the preview reports provider / url / model / key-PRESENCE old->new
/// for every group, flags the machine-local GigaAM path, and (the headline
/// security invariant) NEVER includes a secret VALUE in ANY produced string.
#[test]
fn preview_server_settings_is_redacted_and_diffs() {
    // `current` = this PC: cloud, has a bearer + groq key, a LOCAL gigaam dir.
    let mut current = Config::defaults();
    current.ai_provider = "cloud".into();
    current.ai_base_url = "http://OLD-bridge/v1".into();
    current.ai_bearer = "OLD-SECRET-BEARER".into();
    current.ai_model = "claude-haiku-OLD".into();
    current.groq_api_key = "OLD-SECRET-GROQ".into();
    current.stt_provider = "cloud".into();
    current.stt_gigaam_dir = r"C:\THIS-PC\gigaam".into();

    let imported = imported_with_secret_tokens();
    let p = preview_server_settings(&current, &imported);

    // --- group diffs (neutral values flow through; presence is a bool) ---
    assert_eq!(p.cloud_ai.provider_old, "cloud");
    assert_eq!(p.cloud_ai.provider_new, "local");
    assert_eq!(p.cloud_ai.base_url_old, "http://OLD-bridge/v1");
    assert_eq!(p.cloud_ai.base_url_new, "http://192.168.7.7:18902/v1");
    assert_eq!(p.cloud_ai.model_old, "claude-haiku-OLD");
    assert_eq!(p.cloud_ai.model_new, "claude-haiku-4-5");
    assert!(p.cloud_ai.key_present_old && p.cloud_ai.key_present_new);
    assert!(p.cloud_ai.changed());

    // Local AI: current has no local bearer (defaults), imported does.
    assert!(!p.local_ai.key_present_old);
    assert!(p.local_ai.key_present_new);
    assert_eq!(p.local_ai.model_new, "gemma-4-E4B");

    // Vision: imported has a cloud vision bearer → present_new true.
    assert!(p.vision.key_present_new);
    assert_eq!(p.vision.provider_new, "cloud");

    // STT: groq key present both sides; provider cloud -> gigaam.
    assert!(p.stt.key_present_old && p.stt.key_present_new);
    assert_eq!(p.stt.provider_old, "cloud");
    assert_eq!(p.stt.provider_new, "gigaam");

    // Machine-local GigaAM dir: current kept, incoming surfaced (informational).
    assert_eq!(p.gigaam_dir_current, r"C:\THIS-PC\gigaam");
    assert_eq!(p.gigaam_dir_incoming, r"D:\OTHER-PC\gigaam");

    // --- THE redaction guard: no secret VALUE in ANY preview string ---
    // Collect every string field the preview produces and assert none of the
    // known secret tokens (from both `current` and `imported`) appears.
    let groups = [&p.cloud_ai, &p.local_ai, &p.vision, &p.stt];
    let mut all = String::new();
    for g in groups {
        all.push_str(&g.label);
        all.push_str(&g.provider_old);
        all.push_str(&g.provider_new);
        all.push_str(&g.base_url_old);
        all.push_str(&g.base_url_new);
        all.push_str(&g.model_old);
        all.push_str(&g.model_new);
    }
    all.push_str(&p.gigaam_dir_current);
    all.push_str(&p.gigaam_dir_incoming);
    for secret in [
        "OLD-SECRET-BEARER",
        "OLD-SECRET-GROQ",
        "SECRET-AI-BEARER-zzz",
        "SECRET-LOCAL-BEARER-zzz",
        "SECRET-VISION-BEARER-zzz",
        "SECRET-VISION-LOCAL-BEARER-zzz",
        "gsk_SECRET_GROQ_zzz",
        "SECRET-WHISPER-BEARER-zzz",
    ] {
        assert!(
            !all.contains(secret),
            "preview leaked a secret value: {secret}"
        );
    }
}

/// P1.7 — `mask_host` blanks the host (private LAN IP) for copyable/loggable
/// text but keeps scheme + port + path, and never echoes the host.
#[test]
fn mask_host_blanks_host_keeps_scheme_port_path() {
    assert_eq!(
        mask_host("http://192.168.0.142:18902/v1"),
        "http://***:18902/v1"
    );
    assert!(!mask_host("http://192.168.0.142:18902/v1").contains("192.168"));
    assert_eq!(mask_host("https://bridge.internal/api"), "https://***/api");
    assert_eq!(mask_host("http://127.0.0.1:8080/v1"), "http://***:8080/v1");
    assert_eq!(mask_host(""), "");
    // No scheme, host only.
    assert_eq!(mask_host("10.0.0.5:9000"), "***:9000");
}

/// P1.7 — the default Apply imports every server field EXCEPT the
/// machine-local `stt_gigaam_dir`, which is kept from THIS PC.
#[test]
fn apply_server_settings_keeps_local_gigaam_dir() {
    let mut current = Config::defaults();
    current.stt_gigaam_dir = r"C:\THIS-PC\gigaam".into();
    current.ai_bearer = "OLD".into();

    let imported = imported_with_secret_tokens(); // has D:\OTHER-PC\gigaam

    let next = apply_server_settings(&current, imported);
    // Server fields imported…
    assert_eq!(next.ai_provider, "local");
    assert_eq!(next.ai_bearer, "SECRET-AI-BEARER-zzz");
    assert_eq!(next.groq_api_key, "gsk_SECRET_GROQ_zzz");
    // …but the machine-local GigaAM path is KEPT from this PC.
    assert_eq!(next.stt_gigaam_dir, r"C:\THIS-PC\gigaam");
}

/// #131 — config-only readiness reflects the ACTIVE providers and never
/// puts a secret value (bearer / API key) into a detail string.
#[test]
fn readiness_reflects_active_providers() {
    // Cloud AI + Groq STT, fully configured.
    let mut c = Config::defaults();
    c.ai_provider = "cloud".into();
    c.ai_base_url = "http://bridge/v1".into();
    c.ai_bearer = "SECRET-bearer".into();
    c.ai_model = "claude-haiku".into();
    c.stt_provider = "cloud".into();
    c.groq_api_key = "gsk_SECRET".into();
    let r = c.readiness();
    assert!(r.ai.configured);
    assert!(r.ai.detail.contains("cloud") && r.ai.detail.contains("http://bridge/v1"));
    assert!(
        !r.ai.detail.contains("SECRET"),
        "no bearer/key in AI detail"
    );
    assert!(r.stt.configured);
    assert!(!r.stt.detail.contains("SECRET"), "no key in STT detail");
    assert!(r.mic.configured && r.sys.configured);

    // Cloud AI with empty bearer → not configured (cloud needs a bearer).
    let mut c_nb = c.clone();
    c_nb.ai_bearer = String::new();
    assert!(!c_nb.readiness().ai.configured);

    // Local AI: needs URL + model, NO bearer.
    let mut c2 = Config::defaults();
    c2.ai_provider = "local".into();
    c2.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    c2.ai_local_bearer = String::new();
    c2.ai_local_model = String::new();
    assert!(
        !c2.readiness().ai.configured,
        "local AI with empty model must be unconfigured"
    );
    c2.ai_local_model = "gemma".into();
    let r2 = c2.readiness();
    assert!(r2.ai.configured, "local AI needs no bearer");
    assert!(r2.ai.detail.contains("local"));

    // GigaAM STT: needs a model dir.
    let mut c3 = Config::defaults();
    c3.stt_provider = "gigaam".into();
    c3.stt_gigaam_dir = String::new();
    assert!(!c3.readiness().stt.configured);
    c3.stt_gigaam_dir = r"C:\m\gigaam".into();
    assert!(c3.readiness().stt.detail.contains("gigaam"));

    // Vision (F8): "off" → unconfigured; "same" → reuses the text endpoint;
    // detail carries provider + url + model but never a secret.
    let mut cv = Config::defaults();
    cv.vision_provider = "off".into();
    assert!(
        !cv.readiness().vision.configured,
        "vision=off must be unconfigured"
    );
    cv.vision_provider = "same".into();
    cv.ai_provider = "cloud".into();
    cv.ai_base_url = "http://bridge/v1".into();
    cv.ai_bearer = "SECRET-bearer".into();
    cv.ai_model = "claude-haiku".into();
    let rv = cv.readiness();
    assert!(rv.vision.configured, "vision=same reuses the text endpoint");
    assert!(
        rv.vision.detail.contains("same") && rv.vision.detail.contains("http://bridge/v1"),
        "vision detail shows provider + url"
    );
    assert!(
        !rv.vision.detail.contains("SECRET"),
        "no bearer in vision detail"
    );

    // Stealth bool passes through.
    let mut c4 = Config::defaults();
    c4.stealth_enabled = true;
    assert!(c4.readiness().stealth_on);
}

/// Write a config to a tmp file, read it back via raw serde_json,
/// verify all fields match. Doesn't use the global config_path() —
/// uses an explicit tmpfile to keep tests hermetic.
#[test]
fn config_save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut original = Config::defaults();
    original.meeting_context = "Test SRE interview context".into();
    original.ai_model = "claude-opus-4-7".into();
    original.stealth_enabled = true;
    original.context_profiles = vec![
        ContextProfile {
            name: "k8s".into(),
            context: "kubernetes intro".into(),
        },
        ContextProfile {
            name: "aws".into(),
            context: "aws basics".into(),
        },
    ];
    original.active_profile = Some("k8s".into());

    let bytes = serde_json::to_vec_pretty(&original).unwrap();
    std::fs::write(&path, &bytes).unwrap();

    let raw = std::fs::read(&path).unwrap();
    let loaded: Config = serde_json::from_slice(&raw).unwrap();

    assert_eq!(loaded.meeting_context, original.meeting_context);
    assert_eq!(loaded.ai_model, original.ai_model);
    assert_eq!(loaded.stealth_enabled, original.stealth_enabled);
    assert_eq!(loaded.context_profiles.len(), 2);
    assert_eq!(loaded.context_profiles[1].name, "aws");
    assert_eq!(loaded.active_profile.as_deref(), Some("k8s"));
}

/// Old config files won't have new fields (stealth_enabled, prep_model,
/// trigger_keywords, etc). #[serde(default)] on struct should silently
/// fill them with defaults instead of failing.
#[test]
fn config_partial_json_uses_serde_defaults() {
    // Minimal file — just ai_model. Everything else must come from defaults.
    let minimal = r#"{"ai_model":"claude-old"}"#;
    let cfg: Config = serde_json::from_str(minimal).expect("must parse with defaults");
    assert_eq!(cfg.ai_model, "claude-old");
    // Fields not in JSON default to their Default impl (empty strings, false, None).
    assert_eq!(cfg.ai_bearer, "");
    assert!(!cfg.stealth_enabled);
    assert!(cfg.context_profiles.is_empty());
    assert!(cfg.active_profile.is_none());
}

#[test]
fn config_defaults_stamp_current_schema_version() {
    // A fresh install (the Err/NotFound arms of load() use Config::defaults)
    // must carry the current schema version, so it never looks "older than
    // itself" to the load()-time migration stamp.
    assert_eq!(Config::defaults().config_version, CURRENT_CONFIG_VERSION);
}

#[test]
fn config_pre_versioning_json_reads_as_zero() {
    // A file written before versioning has no config_version key. serde must
    // fill the u32 Default (0) — that's the sentinel load() keys on to stamp
    // the file up to CURRENT (and, in future, run number-keyed migrations).
    let cfg: Config = serde_json::from_str(r#"{"ai_model":"x"}"#).unwrap();
    assert_eq!(cfg.config_version, 0);
    assert!(cfg.config_version < CURRENT_CONFIG_VERSION);
}

#[test]
fn preserve_corrupt_config_renames_aside_keeping_bytes() {
    // P1.4: an unparseable config.json must be moved to a recoverable
    // `*.broken-<ts>` sibling — never silently dropped — so a corruption
    // event can't destroy the user's live keys / profiles.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let garbage = b"{ this is not valid json @@@";
    std::fs::write(&path, garbage).unwrap();

    preserve_corrupt_config(&path);

    // Original gone (moved off the load path).
    assert!(!path.exists(), "corrupt config.json should be renamed away");
    // Exactly one recoverable sibling, holding the original bytes verbatim.
    let mut broken: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("config.json.broken-"))
        .collect();
    assert_eq!(
        broken.len(),
        1,
        "expected one .broken- backup, got {broken:?}"
    );
    let backup = dir.path().join(broken.remove(0));
    assert_eq!(std::fs::read(&backup).unwrap(), garbage);
}

#[test]
fn parse_error_never_echoes_the_offending_token() {
    // SECURITY regression guard (review BLOCKER): config.json holds live
    // secrets, and serde_json's Display echoes the offending TOKEN verbatim on
    // an `invalid type` error. parse_config_bytes must surface ONLY a
    // secret-free location, never the value at the failing position.
    let secret = "sk-LIVE-SECRET-DO-NOT-LOG-9f3a";
    // String where a u32 is expected → serde's raw Display would quote it.
    let json = format!("{{\"config_version\":\"{secret}\"}}");
    let err = parse_config_bytes(json.as_bytes()).unwrap_err();
    let msg = format!("{err:#}"); // anyhow alternate = full chain
    assert!(
        !msg.contains(secret),
        "parse error leaked the secret token: {msg}"
    );
    assert!(
        !msg.contains("invalid type"),
        "serde Display leaked through the wrapper: {msg}"
    );
    assert!(
        msg.contains("line") && msg.contains("column"),
        "sanitised error should still carry a location: {msg}"
    );
}

#[test]
fn config_with_utf8_bom_is_stripped_not_reset() {
    // Notepad "UTF-8 with BOM" / a PowerShell JSON round-trip prepend
    // EF BB BF. load() must strip it and parse, NOT fall back to defaults
    // (which would silently wipe the user's hand-edited config).
    let json = br#"{"response_language":"en"}"#;
    let mut with_bom = vec![0xEF_u8, 0xBB, 0xBF];
    with_bom.extend_from_slice(json);
    // Raw BOM bytes WOULD fail to parse (this is the bug being guarded).
    assert!(serde_json::from_slice::<Config>(&with_bom).is_err());
    // The load() path strips the BOM first, so it parses to the real value.
    let bytes = with_bom
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(&with_bom);
    let cfg: Config = serde_json::from_slice(bytes).expect("BOM-stripped parses");
    assert_eq!(cfg.response_language, "en");
}

#[test]
fn config_empty_object_yields_all_defaults() {
    // Even "{}" must parse — every field has a default.
    let cfg: Config = serde_json::from_str("{}").expect("empty object should parse");
    assert_eq!(cfg.ai_bearer, "");
    assert_eq!(cfg.ai_model, "");
    assert_eq!(cfg.response_language, "");
    assert!(!cfg.stealth_enabled);
}

#[test]
fn secret_redacted_blanks_every_secret_keeps_the_rest() {
    // fs-audit — the config.json.bak undo snapshot must carry NO plaintext
    // credentials, but must still preserve the non-secret fields so an undo
    // restores profiles / hotkeys / settings.
    let c = Config {
        config_version: 7, // a non-secret field that must survive
        ai_bearer: "sk-live-secret".into(),
        ai_local_bearer: "local-bearer".into(),
        vision_bearer: "vision-bearer".into(),
        vision_local_bearer: "vision-local".into(),
        groq_api_key: "gsk_secret".into(),
        stt_whisper_bearer: "whisper-bearer".into(),
        ..Default::default()
    };

    let r = secret_redacted(&c);
    assert!(r.ai_bearer.is_empty(), "ai_bearer redacted");
    assert!(r.ai_local_bearer.is_empty(), "ai_local_bearer redacted");
    assert!(r.vision_bearer.is_empty(), "vision_bearer redacted");
    assert!(
        r.vision_local_bearer.is_empty(),
        "vision_local_bearer redacted"
    );
    assert!(r.groq_api_key.is_empty(), "groq_api_key redacted");
    assert!(
        r.stt_whisper_bearer.is_empty(),
        "stt_whisper_bearer redacted"
    );
    assert_eq!(r.config_version, 7, "non-secret fields survive for undo");
    // And the serialized .bak bytes contain none of the secret values.
    let bytes = serde_json::to_vec(&r).expect("serialize redacted");
    let s = String::from_utf8(bytes).expect("utf8");
    for secret in [
        "sk-live-secret",
        "gsk_secret",
        "whisper-bearer",
        "vision-bearer",
    ] {
        assert!(
            !s.contains(secret),
            "secret {secret} leaked into .bak bytes"
        );
    }
}

#[test]
fn v015_retention_fields_default_to_pre_v015_behaviour() {
    // A pre-v0.15 config (no retention keys) must carry the pre-v0.15
    // CONSTANTS: audio keep 10 / no age limit, journals keep 100 / 500 MB.
    // (The prune behaviour for these values is pinned by the recorder's
    // prune tests + journal's size-cap tests; this test pins the VALUES.)
    let cfg: Config = serde_json::from_str("{}").expect("parses");
    assert_eq!(cfg.record_retention_sessions, 10);
    assert_eq!(cfg.record_retention_days, 0);
    assert_eq!(cfg.journal_retention_sessions, 100);
    assert_eq!(cfg.journal_max_total_mb, 500);
    assert_eq!(cfg.record_max_total_mb, 20_000);
    // And the explicit "unlimited" spelling round-trips.
    let cfg: Config = serde_json::from_str(
        r#"{"record_retention_sessions":0,"record_retention_days":30,
                "journal_retention_sessions":0,"journal_max_total_mb":0}"#,
    )
    .expect("parses");
    assert_eq!(cfg.record_retention_sessions, 0);
    assert_eq!(cfg.record_retention_days, 30);
    assert_eq!(cfg.journal_retention_sessions, 0);
    assert_eq!(cfg.journal_max_total_mb, 0);
}

#[test]
fn ai_endpoint_defaults_to_cloud() {
    let mut d = Config::defaults();
    d.ai_base_url = "http://bridge/v1".into();
    d.ai_bearer = "secret".into();
    d.ai_model = "claude-haiku-4-5".into();
    d.prep_model = "claude-sonnet-4-6".into();
    let live = d.ai_endpoint(false);
    assert!(!live.is_local);
    assert_eq!(live.base_url, "http://bridge/v1");
    assert_eq!(live.bearer, "secret");
    assert_eq!(live.model, "claude-haiku-4-5");
    assert_eq!(d.ai_endpoint(true).model, "claude-sonnet-4-6");
}

#[test]
fn vision_endpoint_off_is_none() {
    let mut d = Config::defaults();
    d.vision_provider = "off".into();
    assert!(d.vision_endpoint().is_none());
}

#[test]
fn vision_endpoint_same_reuses_text_endpoint() {
    let mut d = Config::defaults();
    d.vision_provider = "same".into();
    d.ai_provider = "local".into();
    d.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    d.ai_local_model = "gemma".into();
    let v = d.vision_endpoint();
    assert_eq!(v.as_ref().map(|e| e.is_local), Some(true));
    assert_eq!(
        v.as_ref().map(|e| e.base_url.clone()),
        Some("http://127.0.0.1:8080/v1".to_string())
    );
    assert_eq!(v.map(|e| e.model), Some("gemma".to_string()));
}

#[test]
fn vision_endpoint_cloud_falls_back_to_text_bridge_and_sonnet() {
    let mut d = Config::defaults();
    d.vision_provider = "cloud".into();
    d.ai_base_url = "http://bridge/v1".into();
    d.ai_bearer = "secret".into();
    // vision_* left empty → fall back to the text bridge + Sonnet default.
    let v = d.vision_endpoint();
    assert_eq!(v.as_ref().map(|e| e.is_local), Some(false));
    assert_eq!(
        v.as_ref().map(|e| e.base_url.clone()),
        Some("http://bridge/v1".to_string())
    );
    assert_eq!(
        v.as_ref().map(|e| e.bearer.clone()),
        Some("secret".to_string())
    );
    assert_eq!(v.map(|e| e.model), Some(DEFAULT_VISION_MODEL.to_string()));
}

#[test]
fn vision_endpoint_cloud_uses_explicit_fields_when_set() {
    let mut d = Config::defaults();
    d.vision_provider = "cloud".into();
    d.ai_base_url = "http://text-bridge/v1".into();
    d.vision_base_url = "http://vision-bridge/v1".into();
    d.vision_bearer = "vsecret".into();
    d.vision_model = "claude-opus-4-7".into();
    let v = d.vision_endpoint();
    assert_eq!(
        v.as_ref().map(|e| e.base_url.clone()),
        Some("http://vision-bridge/v1".to_string())
    );
    assert_eq!(
        v.as_ref().map(|e| e.bearer.clone()),
        Some("vsecret".to_string())
    );
    assert_eq!(v.map(|e| e.model), Some("claude-opus-4-7".to_string()));
}

#[test]
fn vision_endpoint_local_falls_back_to_text_local() {
    let mut d = Config::defaults();
    d.vision_provider = "local".into();
    d.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    d.ai_local_model = "gemma".into();
    // vision_local_* empty → fall back to ai_local_*.
    let v = d.vision_endpoint();
    assert_eq!(v.as_ref().map(|e| e.is_local), Some(true));
    assert_eq!(v.map(|e| e.model), Some("gemma".to_string()));
}

#[test]
fn vision_endpoint_default_provider_is_cloud() {
    // Fresh defaults → vision enabled (cloud) so F8 works out of the box.
    assert_eq!(Config::defaults().vision_provider, "cloud");
    assert!(Config::defaults().vision_endpoint().is_some());
}

#[test]
fn ai_endpoint_local_uses_local_fields_and_prep_fallback() {
    let mut d = Config::defaults();
    d.ai_provider = "local".into();
    d.ai_local_base_url = "http://127.0.0.1:11434/v1".into();
    d.ai_local_model = "qwen2.5:7b".into();
    d.ai_local_prep_model = String::new(); // empty → falls back to live
    let live = d.ai_endpoint(false);
    assert!(live.is_local);
    assert_eq!(live.base_url, "http://127.0.0.1:11434/v1");
    assert_eq!(live.model, "qwen2.5:7b");
    assert_eq!(
        d.ai_endpoint(true).model,
        "qwen2.5:7b",
        "empty local prep model falls back to live local model"
    );
    d.ai_local_prep_model = "qwen2.5:14b".into();
    assert_eq!(d.ai_endpoint(true).model, "qwen2.5:14b");
}

// V0.8.0 (Поток D) — ai_endpoint_cloud() always resolves to the cloud
// bridge + the smart prep_model, even when the active provider is local.
#[test]
fn ai_endpoint_cloud_always_uses_cloud_bridge_and_prep_model() {
    let mut d = Config::defaults();
    // Active provider is LOCAL (default-local user, the escalation scenario).
    d.ai_provider = "local".into();
    d.ai_local_base_url = "http://127.0.0.1:8080/v1".into();
    d.ai_local_model = "gemma-4-E4B".into();
    // Cloud bridge fields are still set (they always are in config).
    d.ai_base_url = "http://bridge/v1".into();
    d.ai_bearer = "secret".into();
    d.prep_model = "claude-sonnet-4-6".into();

    // Normal resolve honours the local provider...
    assert!(d.ai_endpoint(false).is_local);
    assert_eq!(d.ai_endpoint(false).base_url, "http://127.0.0.1:8080/v1");

    // ...but the cloud-escalation resolver IGNORES it: cloud bridge + smart.
    let cloud = d.ai_endpoint_cloud();
    assert!(!cloud.is_local, "escalation must bill + allow screenshots");
    assert_eq!(cloud.base_url, "http://bridge/v1");
    assert_eq!(cloud.bearer, "secret");
    assert_eq!(
        cloud.model, "claude-sonnet-4-6",
        "escalation uses the smart prep_model, not the fast ai_model"
    );
}

#[test]
fn stt_backend_defaults_to_cloud() {
    let d = Config::defaults();
    assert_eq!(d.stt_provider, "cloud");
    assert!(!d.stt_is_local());
    match d.stt_backend() {
        SttBackendCfg::Cloud { model, .. } => assert_eq!(model, "whisper-large-v3"),
        other => panic!("expected Cloud, got {other:?}"),
    }
}

#[test]
fn stt_backend_gigaam_uses_dir_and_is_local() {
    let mut d = Config::defaults();
    d.stt_provider = "gigaam".into();
    d.stt_gigaam_dir = r"C:\models\gigaam-v3".into();
    assert!(d.stt_is_local());
    match d.stt_backend() {
        SttBackendCfg::Gigaam { model_dir } => assert_eq!(model_dir, r"C:\models\gigaam-v3"),
        other => panic!("expected Gigaam, got {other:?}"),
    }
}

#[test]
fn stt_backend_whisper_uses_url_bearer_model_and_is_local() {
    let mut d = Config::defaults();
    d.stt_provider = "whisper".into();
    d.stt_whisper_url = "http://127.0.0.1:8081/v1".into();
    d.stt_whisper_bearer = "tok".into();
    d.stt_whisper_model = "whisper-large-v3-turbo".into();
    assert!(d.stt_is_local());
    match d.stt_backend() {
        SttBackendCfg::Whisper {
            base_url,
            bearer,
            model,
        } => {
            assert_eq!(base_url, "http://127.0.0.1:8081/v1");
            assert_eq!(bearer, "tok");
            assert_eq!(model, "whisper-large-v3-turbo");
        }
        other => panic!("expected Whisper, got {other:?}"),
    }
}

#[test]
fn stt_provider_defaults_from_partial_json() {
    // Old config without the STT provider fields → cloud + whisper-server
    // default URL, and thinking-off for local AI.
    let cfg: Config = serde_json::from_str(r#"{"ai_model":"x"}"#).expect("parse");
    assert_eq!(cfg.stt_provider, "cloud");
    assert_eq!(cfg.stt_whisper_url, "http://127.0.0.1:8081/v1");
    assert!(!cfg.stt_is_local());
    assert!(!cfg.ai_local_thinking);
    // GigaAM GPU (DirectML) is on by default; old configs opt in on upgrade.
    assert!(cfg.stt_gigaam_gpu);
    // Colour scheme defaults to 0 (Glacier) for configs predating the field.
    assert_eq!(cfg.color_scheme, 0);
}

#[test]
fn config_missing_provider_fields_default_cloud() {
    // An old config.json without the new fields loads as cloud + the
    // llama.cpp default URL, and resolves to the cloud endpoint.
    let cfg: Config = serde_json::from_str(r#"{"ai_model":"x"}"#).expect("parse");
    assert_eq!(cfg.ai_provider, "cloud");
    assert_eq!(cfg.ai_local_base_url, "http://127.0.0.1:8080/v1");
    assert!(!cfg.ai_endpoint(false).is_local);
}

/// REGRESSION: new config fields added in v0.0.2+ must have correct
/// defaults that are user-friendly. If we change the default, this
/// test catches it — protects against accidental "all-on-by-default"
/// surprises for upgrading users.
#[test]
fn new_v002_field_defaults() {
    let d = Config::defaults();
    // Cost cap default — 0.0 since v0.0.28 means chip is OFF.
    // Old installs (with explicit value in their config.json) keep
    // their value via the per-field serde(default=...) loader.
    assert!(
        d.max_session_cost_usd.abs() < 0.001,
        "max_session_cost_usd default should be 0.0 (chip off), got {}",
        d.max_session_cost_usd
    );
    // detector_skip_mic ON by default — fix for live regression #96
    // (candidate's own voice shouldn't trigger explanation tiles).
    assert!(
        d.detector_skip_mic,
        "detector_skip_mic default should be true (interview use-case)"
    );
    // post_meeting_debrief OFF by default — opt-in per privacy/cost.
    assert!(
        !d.post_meeting_debrief_enabled,
        "post_meeting_debrief_enabled default should be false (opt-in only)"
    );
}

/// Old config files (pre-v0.0.2) lack the new fields. Serde must
/// fill them with proper defaults via per-field #[serde(default="...")]
/// attributes — these are the source of forward compat. Struct
/// Default would also work but the per-field form is what gets used
/// during deserialization, so we assert THAT path specifically.
///
/// v0.0.28: max_session_cost_usd default flipped 1.00 → 0.0 (chip off).
#[test]
fn pre_v002_config_gets_correct_field_defaults_via_serde() {
    // Simulate a v0.0.1 config — has all fields up to v0.0.1 but no
    // max_session_cost_usd or detector_skip_mic.
    let pre_v002 = r#"{
            "ai_model": "claude-haiku-4-5",
            "stealth_enabled": false
        }"#;
    let cfg: Config = serde_json::from_str(pre_v002).expect("must parse old config");
    // Field defaults MUST be applied via serde(default=...) on the
    // field itself:
    assert!(
        cfg.max_session_cost_usd.abs() < 0.001,
        "missing field should fall to 0.0 (cap off) — v0.0.28 default"
    );
    assert!(
        cfg.detector_skip_mic,
        "missing field should fall to true (mic skipped), not false"
    );
    assert!(
        !cfg.post_meeting_debrief_enabled,
        "missing field should fall to false (opt-in)"
    );
}

/// Config with EXPLICIT positive cost cap should NOT be overridden
/// to the 0.0 default — user intent to enable the warning is preserved.
#[test]
fn explicit_positive_cost_cap_preserved() {
    let with_cap = r#"{ "max_session_cost_usd": 2.50 }"#;
    let cfg: Config = serde_json::from_str(with_cap).expect("must parse");
    assert!(
        (cfg.max_session_cost_usd - 2.50).abs() < 0.001,
        "explicit positive cap must NOT be replaced with 0.0 default"
    );
}

/// Config with EXPLICIT 0 for cost cap stays at 0 (was a meaningful
/// "I disabled the chip" signal before v0.0.28 — still works the same).
/// v0.0.28: now matches the default. Test kept to lock in the contract.
#[test]
fn explicit_zero_cost_cap_preserved() {
    let with_zero = r#"{ "max_session_cost_usd": 0.0 }"#;
    let cfg: Config = serde_json::from_str(with_zero).expect("must parse");
    assert_eq!(cfg.max_session_cost_usd, 0.0, "explicit 0 stays 0");
}

/// REGRESSION: the default models MUST be in the pricing table.
/// If someone updates Config::defaults() to a newer model but forgets
/// to add it to crate::ai::pricing_per_million, cost reporting falls
/// back to "safe upper-bound" sonnet pricing — surprise overpay.
#[test]
fn defaults_use_models_present_in_pricing_table() {
    use crate::ai::pricing_per_million;
    let d = Config::defaults();
    // Catch a typo by checking each model resolves to a non-fallback price.
    // Fallback (unknown) is sonnet's price; haiku must NOT be that.
    let (haiku_in, _) = pricing_per_million(&d.ai_model);
    assert!(
        haiku_in < 3.0,
        "default ai_model {} hit fallback pricing",
        d.ai_model
    );
    let (prep_in, _) = pricing_per_million(&d.prep_model);
    assert!(
        prep_in <= 15.0,
        "default prep_model {} unreasonably expensive",
        d.prep_model
    );
}

/// REGRESSION: trigger_keywords must include the basic terms that
/// drove every live-test trigger. Empty or stripped-down keywords =
/// missed questions during an interview.
#[test]
fn trigger_keywords_default_includes_core_devops_terms() {
    let kws = Config::defaults().trigger_keywords;
    for required in [
        "kubernetes",
        "etcd",
        "postgres",
        "linux",
        "nginx",
        "prometheus",
    ] {
        assert!(
            kws.contains(required),
            "default trigger_keywords missing core term '{required}'"
        );
    }
}

/// REGRESSION: stealth must default OFF. Live test depends on this
/// (stealth would hide tiles from screen-share, blocking debugging
/// scenarios with shared screens).
#[test]
fn stealth_defaults_off_for_safer_first_run() {
    assert!(!Config::defaults().stealth_enabled);
}

/// auto_tiles_enabled must default ON — the whole product purpose is
/// auto-suggestions on detected questions.
#[test]
fn auto_tiles_default_on() {
    assert!(Config::defaults().auto_tiles_enabled);
}

/// Malformed JSON config must NOT panic — load() returns defaults.
/// We can't test load() directly (it touches APPDATA), but we can
/// verify the error-tolerance contract via from_slice + match.
#[test]
fn malformed_json_parse_errors_caught_gracefully() {
    let bad = b"{not valid json";
    let res: Result<Config, _> = serde_json::from_slice(bad);
    assert!(
        res.is_err(),
        "must error on bad JSON (load() recovers via defaults)"
    );
}

/// Wrong field type (string instead of bool) must error — caller falls
/// back to defaults. Prevents silently accepting `"stealth_enabled":"yes"`
/// as truthy.
#[test]
fn wrong_field_type_errors_dont_coerce() {
    let bad = r#"{"stealth_enabled":"yes"}"#;
    let res: Result<Config, _> = serde_json::from_str(bad);
    assert!(res.is_err(), "string-as-bool must reject, not coerce");
}

/// REGRESSION: the snippet library is the user's "instant zero-cost
/// answer" bank. Live-test corpus + this morning's encyclopedia push
/// has it at 53 snippets. Anyone removing snippets without thinking
/// twice should hit this floor.
#[test]
fn default_snippets_cover_breadth() {
    let d = Config::defaults();
    assert!(
        d.snippets.len() >= 50,
        "snippet library shrank to {} — must stay ≥50",
        d.snippets.len()
    );
    // Domain coverage spot-check — make sure no whole category was
    // accidentally deleted.
    let keys: Vec<&str> = d.snippets.iter().map(|s| s.key.as_str()).collect();
    for domain in [
        "k8s",
        "pg",
        "incident",
        "sli", // originals
        "linux-oom",
        "linux-net",
        "tcp",
        "dns",
        "tls",
        "redis",
        "kafka",
        "oauth2",
        "docker",
        "aws-vpc",
        "prom",
        "trace",
        "saga",
        "mesh",
    ] {
        assert!(
            keys.contains(&domain),
            "default snippets missing /{domain} (domain coverage regression)"
        );
    }
}

/// REGRESSION: trigger keyword pool feeds both detector and Whisper
/// bias. After the encyclopedia push it sits at 250+ unique tokens.
/// Floor prevents accidental nuke of the list.
#[test]
fn default_trigger_keywords_breadth() {
    let kws = Config::defaults().trigger_keywords;
    let count = kws.split_whitespace().count();
    assert!(
        count >= 150,
        "trigger keyword count dropped to {count} — must stay ≥150"
    );
}

/// Default snippets must include the SRE essentials and have unique keys
/// (the expand_snippet command does case-insensitive lookup; duplicates
/// silently shadow each other).
#[test]
fn default_snippets_present_and_keys_unique() {
    let d = Config::defaults();
    let keys: Vec<String> = d.snippets.iter().map(|s| s.key.to_lowercase()).collect();
    assert!(!keys.is_empty(), "must ship default snippets");
    assert!(keys.contains(&"k8s".to_string()), "missing k8s snippet");
    assert!(keys.contains(&"pg".to_string()), "missing pg snippet");
    assert!(
        keys.contains(&"incident".to_string()),
        "missing incident snippet"
    );
    assert!(keys.contains(&"sli".to_string()), "missing sli snippet");
    let mut sorted = keys.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), keys.len(), "snippet keys must be unique");
}

/// Snippet body should be non-trivial (some content not just whitespace),
/// and title should be human-readable.
#[test]
fn default_snippets_have_content() {
    for s in Config::defaults().snippets {
        assert!(
            !s.title.trim().is_empty(),
            "snippet {} missing title",
            s.key
        );
        assert!(
            s.body.trim().len() >= 50,
            "snippet {} body too short ({} chars)",
            s.key,
            s.body.len()
        );
    }
}

/// Snippet round-trip including the new field.
#[test]
fn snippet_serialisation_roundtrip() {
    let original = Snippet {
        key: "test".into(),
        title: "Test title".into(),
        body: "**bold** body with newline\n\nand markdown".into(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let back: Snippet = serde_json::from_str(&json).unwrap();
    assert_eq!(back.key, original.key);
    assert_eq!(back.title, original.title);
    assert_eq!(back.body, original.body);
}

/// Context profile shapes round-trip with empty + non-empty.
#[test]
fn context_profile_serialisation_roundtrip() {
    let profiles = vec![
        ContextProfile {
            name: "".into(),
            context: "".into(),
        },
        ContextProfile {
            name: "interview".into(),
            context: "long\nmulti-line\ncontext".into(),
        },
    ];
    let json = serde_json::to_string(&profiles).unwrap();
    let back: Vec<ContextProfile> = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].context, "long\nmulti-line\ncontext");
}

#[test]
fn profile_lifecycle_add_select_rename_delete() {
    let mut c = Config::defaults();
    c.meeting_context = "stale".into();
    // add() creates a BLANK profile, makes it active, and clears the live
    // context (does NOT clone the previous meeting_context).
    assert_eq!(c.add_profile("A"), Some(0));
    assert_eq!(c.active_profile.as_deref(), Some("A"));
    assert_eq!(c.context_profiles[0].context, "");
    assert_eq!(c.meeting_context, "");
    // fill A's context the normal way: type + save into the active profile
    c.save_active_context("ctx A");
    assert_eq!(c.context_profiles[0].context, "ctx A");
    // blank + duplicate names are rejected
    assert!(c.add_profile("  ").is_none());
    assert!(c.add_profile("A").is_none());
    // a second profile is ALSO blank + active, regardless of current context
    assert_eq!(c.add_profile("B"), Some(1));
    assert_eq!(c.active_profile_index(), Some(1));
    assert_eq!(c.context_profiles[1].context, "");
    assert_eq!(c.meeting_context, "");
    // selecting loads that profile's context into the live field
    c.select_profile(0);
    assert_eq!(c.meeting_context, "ctx A");
    assert_eq!(c.active_profile.as_deref(), Some("A"));
    // editing + saving updates BOTH the live field and the active profile
    c.save_active_context("ctx A edited");
    assert_eq!(c.meeting_context, "ctx A edited");
    assert_eq!(c.context_profiles[0].context, "ctx A edited");
    // rename rejects duplicates, accepts a fresh name
    assert!(!c.rename_active_profile("B"));
    assert!(c.rename_active_profile("A2"));
    assert_eq!(c.context_profiles[0].name, "A2");
    assert_eq!(c.active_profile.as_deref(), Some("A2"));
    // deleting the active profile activates the next + loads its context
    c.delete_active_profile();
    assert_eq!(c.context_profiles.len(), 1);
    assert_eq!(c.active_profile.as_deref(), Some("B"));
    assert_eq!(c.meeting_context, ""); // B was created blank
                                       // deleting the last profile clears the active selection
    c.delete_active_profile();
    assert!(c.context_profiles.is_empty());
    assert!(c.active_profile.is_none());
}

// Regression for the user-reported bug: a new profile must NOT inherit the
// active profile's context. Creating "FOOT" while "ninitux" was active had
// silently copied ninitux's description into FOOT.
#[test]
fn add_profile_does_not_clone_active_profile_context() {
    let mut c = Config::defaults();
    assert_eq!(c.add_profile("ninitux"), Some(0));
    c.save_active_context("ninitux description");
    assert_eq!(c.context_profiles[0].context, "ninitux description");
    assert_eq!(c.meeting_context, "ninitux description");
    // add FOOT while ninitux is active and its context is live
    assert_eq!(c.add_profile("FOOT"), Some(1));
    assert_eq!(c.active_profile.as_deref(), Some("FOOT"));
    // FOOT must be EMPTY, and the live context cleared — not a clone
    assert_eq!(c.context_profiles[1].context, "");
    assert_eq!(c.meeting_context, "");
    // ninitux is left untouched
    assert_eq!(c.context_profiles[0].context, "ninitux description");
}
