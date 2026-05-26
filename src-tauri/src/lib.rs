mod ai;
mod audio;
mod config;
mod hotkeys;
mod journal;
mod kb;
mod runtime;
mod screenshot;
mod stt;
mod tile;
mod tray;

use config::{Config, SharedConfig};
use runtime::{SharedRuntime, TranscriptLine};
use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;
use tile::SharedTiles;

/// Remembers the overlay window's pre-settings position so close_settings
/// can restore it instead of always snapping back to (200, 40). Lazy-init
/// from None — first open_settings call sets it; close_settings reads + clears.
static PRE_SETTINGS_POS: Mutex<Option<(f64, f64)>> = Mutex::new(None);

// ── Caller-window guard ──────────────────────────────────────────────────
//
// Sensitive commands (config read/write, session lifecycle, screenshot,
// mic capture, stealth toggle, filesystem export/import) MUST only be
// callable from the trusted overlay window. Tile-* windows render AI
// answers — that markdown can include strings sourced from interviewer
// transcript or external pages, and an AI-rendered tile is in scope for
// markdown-driven prompt injection. Without this guard a poisoned tile
// could `invoke("export_config")` to leak the bearer + Groq key, or
// `invoke("set_stealth", { enabled: false })` to reveal the overlay
// during a screenshare.
//
// Tauri 2 auto-injects `tauri::WebviewWindow` as a command argument
// without the JS side needing to pass anything, so adding the guard is
// a non-breaking change.
//
// Companion fix: `src-tauri/capabilities/tile.json` removes the
// `opener:default` + `global-shortcut:*` plugin perms from tile-* so
// even arbitrary `invoke("plugin:...")` calls from a tile are denied
// at the ACL layer before reaching Rust.
fn assert_overlay(window: &tauri::WebviewWindow) -> Result<(), String> {
    let label = window.label();
    if label == "overlay" {
        Ok(())
    } else {
        log::warn!(
            "blocked sensitive command from non-overlay window: label={label}"
        );
        Err(format!(
            "permission denied: this command is restricted to the overlay window (caller={label})"
        ))
    }
}

// ── Config commands ──────────────────────────────────────────────────────

#[tauri::command]
fn get_config(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
) -> Result<Config, String> {
    assert_overlay(&window)?;
    Ok(state.read().clone())
}

#[tauri::command]
fn save_config(
    window: tauri::WebviewWindow,
    new_cfg: Config,
    state: tauri::State<'_, SharedConfig>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    {
        let mut g = state.write();
        *g = new_cfg.clone();
    }
    config::save(&new_cfg).map_err(|e| e.to_string())
}

// ── Audio device enumeration ─────────────────────────────────────────────

#[tauri::command]
fn list_audio_devices() -> Result<audio::DeviceList, String> {
    audio::list_devices().map_err(|e| e.to_string())
}

// ── Capture session lifecycle ────────────────────────────────────────────

#[tauri::command]
async fn start_session(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    let cfg = cfg.inner().clone();
    let rt = rt.inner().clone();
    let tiles = tiles.inner().clone();
    runtime::start_session(app, cfg, rt, tiles)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_session(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    runtime::stop_session(
        app,
        cfg.inner().clone(),
        rt.inner().clone(),
        tiles.inner().clone(),
    );
    Ok(())
}

// ── AI ask + screenshots ─────────────────────────────────────────────────

#[tauri::command]
async fn ask_ai(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    let cfg = cfg.inner().clone();
    let rt = rt.inner().clone();
    runtime::ask(app, cfg, rt).await;
    Ok(())
}

#[tauri::command]
fn take_screenshot(
    window: tauri::WebviewWindow,
    rt: tauri::State<'_, SharedRuntime>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let url = screenshot::capture_primary_jpeg().map_err(|e| e.to_string())?;
    runtime::stash_screenshot(rt.inner().clone(), url.clone());
    Ok(url)
}

#[tauri::command]
fn get_transcript(
    window: tauri::WebviewWindow,
    rt: tauri::State<'_, SharedRuntime>,
) -> Result<Vec<TranscriptLine>, String> {
    assert_overlay(&window)?;
    Ok(runtime::snapshot_transcript(rt.inner()))
}

// ── Pre-meeting prep flow ────────────────────────────────────────────────

#[tauri::command]
async fn prep_record(
    window: tauri::WebviewWindow,
    duration_secs: u32,
    cfg: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let (mic_device, groq_key, language, whisper_prompt, stt_model) = {
        let c = cfg.read();
        (
            c.mic_device.clone(),
            c.groq_api_key.clone(),
            c.stt_language.clone(),
            // Same biasing applies to prep dictation — the user is
            // talking about the same domain they'll then meet about.
            stt::build_whisper_prompt(&c.trigger_keywords, &c.meeting_context),
            c.stt_model.clone(),
        )
    };
    if groq_key.trim().is_empty() {
        return Err("Groq API key not set".into());
    }
    let duration_ms = (duration_secs as u64) * 1000;

    // Record off the async runtime so the WASAPI blocking loop doesn't
    // stall other tokio tasks.
    let pcm = tokio::task::spawn_blocking(move || {
        audio::record_mic_blocking(duration_ms, mic_device)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
    .map_err(|e| format!("record error: {e:#}"))?;

    if pcm.is_empty() {
        return Err("recording produced no audio (mic silent?)".into());
    }

    let text = stt::transcribe_once(&pcm, &groq_key, language.as_deref(), whisper_prompt.as_deref(), &stt_model)
        .await
        .map_err(|e| format!("stt error: {e:#}"))?;
    Ok(text)
}

#[tauri::command]
async fn prep_structure(
    window: tauri::WebviewWindow,
    raw_text: String,
    cfg: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let (base_url, bearer, model, response_language) = {
        let c = cfg.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.prep_model.clone(),
            c.response_language.clone(),
        )
    };

    let lang_directive = if response_language == "ru" {
        "Отвечай на русском языке."
    } else {
        "Respond in English."
    };

    let system_prompt = format!(
        "Ты — помощник по подготовке к разговорам. На вход — сырой текст (надиктовка \
         или конспект пользователя) с описанием предстоящей встречи. Преврати его в \
         компактный структурированный контекст, который AI-ассистент будет использовать \
         как system prompt во время живого разговора.\n\n\
         Используй только релевантные секции (если для какой-то нет данных — опусти):\n\
         # Роль и фон пользователя\n\
         # О собеседнике/компании\n\
         # Цель встречи\n\
         # Возможные вопросы\n\
         # Ключевые термины и технологии\n\
         # Тон ответов\n\n\
         Будь конкретным, без воды. Маркдаун с # заголовками. {}",
        lang_directive
    );

    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system_prompt),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(raw_text),
        },
    ];

    ai::complete(&base_url, &bearer, &model, messages, 2048)
        .await
        .map_err(|e| format!("ai error: {e:#}"))
}

// ── Tiles (auto-question windows) ────────────────────────────────────────

#[tauri::command]
fn spawn_tile(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    tiles: tauri::State<'_, SharedTiles>,
    question: String,
    answer: String,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let (preferred_monitor, stealth) = {
        let c = cfg.read();
        (c.tile_monitor_name.clone(), c.stealth_enabled)
    };
    tile::spawn_tile_with_stealth(&app, tiles.inner(), question, answer, preferred_monitor, stealth, tile::TileKind::Auto)
        .map_err(|e| e.to_string())
}

// ── Knowledge Base (embedded glossary + commands + patterns) ────────

/// Search the embedded KB. UI calls on input change. Returns up to `limit`
/// results ranked by exact-match > prefix > heading > body.
#[tauri::command]
fn kb_search(query: String, limit: Option<usize>) -> Vec<kb::KBEntry> {
    kb::search(&query, limit.unwrap_or(20))
}

/// Fast exact-key lookup. Used by the `/keyname` palette flow when user
/// types a known shortcut. Returns None if no exact match.
#[tauri::command]
fn kb_get(key: String) -> Option<kb::KBEntry> {
    kb::get(&key).cloned()
}

/// Counts per source — for Settings UI status banner.
#[tauri::command]
fn kb_stats() -> kb::KBStats {
    kb::stats()
}

/// Spawn a tile carrying a KB entry's body. Equivalent to expand_snippet
/// but for the KB rather than user-editable snippet config. Returns the
/// spawned tile label so UI can show it landed.
#[tauri::command]
fn kb_spawn(
    window: tauri::WebviewWindow,
    key: String,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let entry = kb::get(&key).ok_or_else(|| format!("kb entry '{key}' not found"))?.clone();
    let (preferred_monitor, stealth) = {
        let c = cfg.read();
        (c.tile_monitor_name.clone(), c.stealth_enabled)
    };
    tile::spawn_tile_with_stealth(
        &app,
        tiles.inner(),
        entry.heading,
        entry.body,
        preferred_monitor,
        stealth,
        tile::TileKind::Manual,
    )
    .map_err(|e| e.to_string())
}

// ── Snippets (pre-written templated answers, zero AI cost) ──────────

/// Return the configured snippet list. Frontend uses this to render the
/// palette + the Settings management UI.
#[tauri::command]
fn list_snippets(cfg: tauri::State<'_, SharedConfig>) -> Vec<config::Snippet> {
    cfg.read().snippets.clone()
}

/// Look up a snippet by its trigger key (case-insensitive) and spawn a
/// tile carrying its body. No AI call, no STT call — instant.
/// Returns the spawned tile's label so the UI can ack.
#[tauri::command]
fn expand_snippet(
    window: tauri::WebviewWindow,
    key: String,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let lookup = key.trim().to_lowercase();
    let (snip_title, snip_body, preferred_monitor, stealth) = {
        let c = cfg.read();
        let snip = c
            .snippets
            .iter()
            .find(|s| s.key.trim().to_lowercase() == lookup)
            .ok_or_else(|| format!("snippet '{key}' not found"))?
            .clone();
        (
            snip.title,
            snip.body,
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
        )
    };
    tile::spawn_tile_with_stealth(
        &app,
        tiles.inner(),
        snip_title,
        snip_body,
        preferred_monitor,
        stealth,
        tile::TileKind::Manual,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn ask_from_mic(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    runtime::manual_ask_source(
        app, cfg.inner().clone(), rt.inner().clone(), tiles.inner().clone(),
        audio::AudioSource::Mic,
    )
    .await;
    Ok(())
}

#[tauri::command]
async fn ask_from_system(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    runtime::manual_ask_source(
        app, cfg.inner().clone(), rt.inner().clone(), tiles.inner().clone(),
        audio::AudioSource::System,
    )
    .await;
    Ok(())
}

// ── Push-to-talk (hold mode) ─────────────────────────────────────────────

/// Start the push-to-talk window for a source. Opens a dedicated WASAPI
/// capture (separate from main always-on) so we get one clean WAV blob
/// without VAD chunking. Returns start timestamp for UI tracking.
#[tauri::command]
fn manual_ask_hold_start(
    window: tauri::WebviewWindow,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    source: String,
) -> Result<u64, String> {
    assert_overlay(&window)?;
    let src = parse_source(&source)?;
    Ok(runtime::manual_ask_window_start(
        rt.inner().clone(),
        cfg.inner().clone(),
        src,
    ))
}

/// End the push-to-talk window: slice transcript lines that arrived
/// during the hold, ask AI, spawn tile.
#[tauri::command]
async fn manual_ask_hold_end(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
    source: String,
) -> Result<(), String> {
    assert_overlay(&window)?;
    let src = parse_source(&source)?;
    runtime::manual_ask_window_end(
        app, cfg.inner().clone(), rt.inner().clone(), tiles.inner().clone(), src,
    )
    .await;
    Ok(())
}

fn parse_source(s: &str) -> Result<audio::AudioSource, String> {
    match s {
        "mic" | "Mic" => Ok(audio::AudioSource::Mic),
        "system" | "System" => Ok(audio::AudioSource::System),
        other => Err(format!("unknown source: {other}")),
    }
}

#[tauri::command]
fn close_tile(
    app: tauri::AppHandle,
    tiles: tauri::State<'_, SharedTiles>,
    label: String,
) {
    tile::close_tile_by_label(&app, tiles.inner(), &label);
}

#[tauri::command]
fn pin_tile(
    tiles: tauri::State<'_, SharedTiles>,
    label: String,
    pinned: bool,
) -> bool {
    tile::set_tile_pinned(tiles.inner(), &label, pinned)
}

#[tauri::command]
fn list_monitors(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    app.available_monitors()
        .map_err(|e| e.to_string())
        .map(|ms| {
            ms.into_iter()
                .map(|m| m.name().cloned().unwrap_or_else(|| "unnamed".to_string()))
                .collect()
        })
}

// ── Window control ───────────────────────────────────────────────────────

/// Scan sessions/ for the most recent journal and try to recover the
/// meeting_context that was active at SessionStart. Useful after a crash —
/// the user sees their pre-meeting notes still in place.
#[tauri::command]
fn last_session_summary(
    window: tauri::WebviewWindow,
) -> Result<Option<serde_json::Value>, String> {
    assert_overlay(&window)?;
    let dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Ok(None);
    };
    let mut files: Vec<std::path::PathBuf> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .collect();
    files.sort();
    let Some(last) = files.last() else { return Ok(None) };

    // Read first ~3 lines for SessionStart event.
    let Ok(content) = std::fs::read_to_string(last) else { return Ok(None) };
    let mut start: Option<serde_json::Value> = None;
    let mut transcript_count: usize = 0;
    let mut tile_count: usize = 0;
    for line in content.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        match kind {
            "session_start" if start.is_none() => start = Some(v),
            "transcript_line" => transcript_count += 1,
            "tile_spawn" => tile_count += 1,
            _ => {}
        }
    }
    let Some(start) = start else { return Ok(None) };
    Ok(Some(serde_json::json!({
        "path": last.to_string_lossy(),
        "unix_ms": start.get("unix_ms"),
        "meeting_context_chars": start.get("meeting_context_chars"),
        "ai_model": start.get("ai_model"),
        "transcript_lines": transcript_count,
        "tiles_spawned": tile_count,
    })))
}

/// Toggle stealth (screen-share invisibility) at runtime.
/// Affects overlay + all existing tile windows.
#[tauri::command]
fn set_stealth(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    enabled: bool,
    cfg: tauri::State<'_, SharedConfig>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    {
        let mut g = cfg.write();
        g.stealth_enabled = enabled;
    }
    let new_cfg = cfg.read().clone();
    if let Err(e) = config::save(&new_cfg) {
        log::warn!("set_stealth save failed: {e:#}");
    }
    // Apply to overlay window.
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_content_protected(enabled);
    }
    // Apply to all existing tile windows.
    for label in app.webview_windows().keys().filter(|l| l.starts_with("tile-")) {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.set_content_protected(enabled);
        }
    }
    log::info!("stealth toggled to {enabled}");
    Ok(())
}

#[tauri::command]
fn export_config(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    // PERSONAL-USE app: export keeps secrets (groq_api_key + ai_bearer)
    // so user can import on another machine and have everything just work.
    // Previously stripped them "for security" but that defeated the user's
    // actual flow (live regression 2026-05-25: "Импорт не сработал на токены").
    // Backup file lands on Desktop — user's responsibility not to share it.
    let cfg = state.read().clone();
    let bytes = serde_json::to_vec_pretty(&cfg).map_err(|e| e.to_string())?;
    let stamp = journal::now_unix_ms() / 1000;
    let desktop = dirs::desktop_dir().or_else(dirs::home_dir).ok_or("no desktop dir")?;
    let path = desktop.join(format!("suflyor-backup-{stamp}.json"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// "Shareable" export — blanks secrets + personal data so the file can be
/// safely sent to a friend via messenger / committed to git / posted in a
/// gist. The recipient gets snippets + keywords + hotkeys + UI prefs but
/// must wire up their OWN bridge URL, API key, and meeting context.
///
/// Blanked fields:
/// - groq_api_key       (Whisper STT secret)
/// - ai_bearer          (BRIDGE_SECRET — Claude proxy auth)
/// - ai_base_url        (your LAN IP / network topology)
/// - meeting_context    (resume excerpts, company names, salary)
/// - context_profiles   (named meeting contexts — personal)
///
/// Kept fields: snippets, trigger_keywords, hotkeys, audio device prefs,
/// tile_monitor_name, ai_model choice, response_language, stt_model.
/// Pure helper for `export_config_safe`: zeroes out fields that would
/// leak personal data or secrets if the backup file were shared.
/// Extracted so we can unit-test field semantics without touching disk.
///
/// Blanked fields:
///   - groq_api_key, ai_bearer       (secrets)
///   - ai_base_url                   (your LAN IP / network topology)
///   - meeting_context               (resume, salary, company names)
///   - context_profiles + active_profile (named personal contexts)
///
/// All other fields (snippets, trigger_keywords, ai_model, hotkeys, etc.)
/// are KEPT so the recipient gets a useful starting point.
pub(crate) fn blank_share_secrets(mut cfg: Config) -> Config {
    cfg.groq_api_key = String::new();
    cfg.ai_bearer = String::new();
    cfg.ai_base_url = String::new();
    cfg.meeting_context = String::new();
    cfg.context_profiles = Vec::new();
    cfg.active_profile = None;
    cfg
}

#[tauri::command]
fn export_config_safe(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let cfg = blank_share_secrets(state.read().clone());
    let bytes = serde_json::to_vec_pretty(&cfg).map_err(|e| e.to_string())?;
    let stamp = journal::now_unix_ms() / 1000;
    let desktop = dirs::desktop_dir().or_else(dirs::home_dir).ok_or("no desktop dir")?;
    let path = desktop.join(format!("suflyor-share-{stamp}.json"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    log::info!(
        "exported shareable config (no secrets, no personal context) to {}",
        path.display()
    );
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod export_safe_tests {
    use super::blank_share_secrets;
    use crate::config::{Config, ContextProfile};

    fn loaded_cfg() -> Config {
        let mut c = Config::defaults();
        c.groq_api_key = "gsk_realsecret".into();
        c.ai_bearer = "Bearer_REAL".into();
        c.ai_base_url = "http://192.168.0.142:18902/v1".into();
        c.meeting_context = "Senior SRE @ MyCorp, salary $200k, target $250k".into();
        c.context_profiles = vec![
            ContextProfile { name: "K8s interview".into(), context: "kubernetes stuff".into() },
        ];
        c.active_profile = Some("K8s interview".into());
        c
    }

    #[test]
    fn blanks_groq_key() {
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.groq_api_key, "");
    }

    #[test]
    fn blanks_ai_bearer() {
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.ai_bearer, "");
    }

    #[test]
    fn blanks_ai_base_url() {
        // LAN topology leak — friend doesn't need our internal IP.
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.ai_base_url, "");
    }

    #[test]
    fn blanks_meeting_context() {
        // Personal — may contain salary, company name, resume excerpts.
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.meeting_context, "");
    }

    #[test]
    fn blanks_context_profiles_and_active() {
        let c = blank_share_secrets(loaded_cfg());
        assert!(c.context_profiles.is_empty());
        assert!(c.active_profile.is_none());
    }

    #[test]
    fn keeps_snippets() {
        // Generic technical templates — safe to share.
        let c = blank_share_secrets(loaded_cfg());
        assert!(!c.snippets.is_empty(), "snippets should be retained");
    }

    #[test]
    fn keeps_trigger_keywords() {
        // Generic DevOps vocab — safe to share.
        let c = blank_share_secrets(loaded_cfg());
        assert!(!c.trigger_keywords.is_empty(), "trigger_keywords should be retained");
    }

    #[test]
    fn keeps_ai_model_and_response_language() {
        // Recipient may want to follow same model preferences.
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.ai_model, "claude-haiku-4-5");
        assert_eq!(c.response_language, "ru");
    }

    #[test]
    fn keeps_hotkeys() {
        let c = blank_share_secrets(loaded_cfg());
        assert_eq!(c.hotkey_ask, "F9");
    }

    #[test]
    fn idempotent_safe_export() {
        // Running twice gives same blanked output — no leak on re-export.
        let c1 = blank_share_secrets(loaded_cfg());
        let c2 = blank_share_secrets(c1.clone());
        assert_eq!(c2.groq_api_key, "");
        assert_eq!(c2.ai_bearer, "");
        assert_eq!(c2.ai_base_url, "");
    }
}

#[tauri::command]
fn import_config(
    window: tauri::WebviewWindow,
    path: String,
    state: tauri::State<'_, SharedConfig>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    // SECURITY: confine reads to the user's Desktop / Documents directories.
    // Without this gate the renderer (which can be poisoned by AI-rendered
    // markdown in a tile, prompt-injected through interviewer transcript)
    // could exfiltrate ANY file on disk via this command — the json-parse
    // error path leaks part of the contents back via the error string.
    // S0 finding from night-run security audit (2026-05-25).
    let raw = std::path::PathBuf::from(&path);
    let canon = raw
        .canonicalize()
        .map_err(|e| format!("path canonicalize failed: {e}"))?;
    let desktop = dirs::desktop_dir();
    let docs = dirs::document_dir();
    let allowed = [&desktop, &docs]
        .iter()
        .filter_map(|d| d.as_ref())
        .any(|d| canon.starts_with(d));
    if !allowed {
        return Err(format!(
            "import_config refused: path must be under Desktop or Documents (got {})",
            canon.display()
        ));
    }
    let bytes = std::fs::read(&canon).map_err(|e| e.to_string())?;
    // Don't leak parse-error contents to the renderer — strip to the error
    // category only. (serde sometimes echoes raw bytes in its message.)
    let mut imported: Config = serde_json::from_slice(&bytes).map_err(|e| {
        let cat = match e.classify() {
            serde_json::error::Category::Io => "I/O",
            serde_json::error::Category::Syntax => "syntax",
            serde_json::error::Category::Data => "schema mismatch",
            serde_json::error::Category::Eof => "unexpected EOF",
        };
        format!("json parse failed: {cat}")
    })?;
    // PRESERVE existing secrets if import doesn't carry them (export strips them).
    {
        let current = state.read();
        if imported.ai_bearer.trim().is_empty() {
            imported.ai_bearer = current.ai_bearer.clone();
        }
        if imported.groq_api_key.trim().is_empty() {
            imported.groq_api_key = current.groq_api_key.clone();
        }
    }
    // VALIDATE critical fields after merge.
    if imported.ai_base_url.trim().is_empty() {
        return Err("Imported config missing ai_base_url — refused".into());
    }
    {
        let mut g = state.write();
        *g = imported.clone();
    }
    config::save(&imported).map_err(|e| e.to_string())
}

/// Returns the path to crash-report.txt if it exists, None otherwise.
/// Used by Settings UI to surface a "📨 View crash report" button only
/// when there's actually a report to show. (Created by the `.run()` panic
/// handler in `pub fn run()` — see P0-3 fix in v0.0.2.)
#[tauri::command]
fn crash_report_path(window: tauri::WebviewWindow) -> Result<Option<String>, String> {
    assert_overlay(&window)?;
    let dir = dirs::config_dir().ok_or("no config dir")?;
    let path = dir.join("overlay-mvp").join("crash-report.txt");
    if path.exists() {
        Ok(Some(path.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn open_sessions_folder(window: tauri::WebviewWindow) -> Result<String, String> {
    assert_overlay(&window)?;
    let dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(&path)
            .spawn();
    }
    Ok(path)
}

/// Hard-exit the whole app. Wired to the "✕ Выйти" button in Settings
/// header (which already wraps it in a confirm modal). Same effect as
/// tray-icon → Quit. **First closes any active session** so the journal
/// gets a proper SessionStop + SessionSummary event (P0-1 fix from review
/// 2026-05-25 — previously the journal was orphaned mid-session if user
/// quit during recording, losing the final summary stats).
#[tauri::command]
fn quit_app(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    cfg: tauri::State<'_, SharedConfig>,
    rt: tauri::State<'_, SharedRuntime>,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<(), String> {
    assert_overlay(&window)?;
    log::info!("quit_app: user requested exit from Settings — closing session first");
    // stop_session is synchronous (no .await needed) — it spawns the debrief
    // fire-and-forget so app.exit(0) below won't kill the in-flight Sonnet
    // call mid-flight. That's intentional: debrief is best-effort, and the
    // bigger win is making sure the JSONL closes cleanly.
    runtime::stop_session(
        app.clone(),
        cfg.inner().clone(),
        rt.inner().clone(),
        tiles.inner().clone(),
    );
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn open_settings(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<(), String> {
    assert_overlay(&window)?;
    // Settings open inline in the overlay window — also grow window so
    // the form is usable, then navigate to ?settings=1. 900px tall
    // accommodates 12 sections without scroll on 1080p (live regression
    // 2026-05-25: "там еще скрол есть"). On smaller screens the window
    // still resizes naturally — user can shrink via window edge.
    if let Some(w) = app.get_webview_window("overlay") {
        // Remember current outer position so close_settings can restore it
        // (instead of always snapping back to the default 200,40). Skip if
        // outer_position errors — we'll just fall back to the default on close.
        if let Ok(pos) = w.outer_position() {
            if let Ok(scale) = w.scale_factor() {
                let logical = pos.to_logical::<f64>(scale);
                if let Ok(mut guard) = PRE_SETTINGS_POS.lock() {
                    *guard = Some((logical.x, logical.y));
                }
            }
        }
        let _ = w.set_size(tauri::LogicalSize::new(760.0, 900.0));
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.eval("window.location.search = '?settings=1'");
    }
    Ok(())
}

#[tauri::command]
fn close_settings(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<(), String> {
    assert_overlay(&window)?;
    // Restore overlay window to compact bar size + (pre-settings position
    // OR default 200,40) + clear ?settings. The remembered position is
    // taken from open_settings; if that failed we fall back to default.
    if let Some(w) = app.get_webview_window("overlay") {
        let (x, y) = PRE_SETTINGS_POS
            .lock()
            .ok()
            .and_then(|mut g| g.take())
            .unwrap_or((200.0, 40.0));
        let _ = w.set_size(tauri::LogicalSize::new(520.0, 96.0));
        let _ = w.set_position(tauri::LogicalPosition::new(x, y));
        let _ = w.eval("window.location.search = ''");
    }
    Ok(())
}

// ── Bridge health probe ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct BridgeStatus {
    reachable: bool,
    /// HTTP status returned (if we got one). 0 = no HTTP response at all.
    status: u16,
    /// Round-trip in ms (HEAD/GET on /models if available, else GET base).
    /// Capped at our 5s probe timeout — if we hit the cap, `reachable=false`.
    latency_ms: u64,
    /// Human-readable hint: empty on success, otherwise tells the user
    /// what to check (DNS, port, auth, model name).
    hint: String,
}

/// Probe the configured AI bridge with a fast, cheap request. Used by
/// Settings to give a "🟢 reachable / 🔴 unreachable" indicator next to
/// the base_url field, fixing P2-7 from review 2026-05-25 where the user
/// got a confusing "HTTP timeout" with no explanation when the default
/// hardcoded LAN IP wasn't running their bridge.
///
/// Tries POST `/chat/completions` with a minimal payload (1 token). Most
/// OpenAI-compat bridges accept this even on bad model names — the 4xx
/// response still proves the bridge is alive. Bug-hunt 2026-05-25: we now
/// pass the user's CONFIGURED model name instead of a hardcoded one, so
/// bridges that don't ship "claude-haiku-4-5" alias (e.g. local Ollama,
/// older proxy forks) don't get misreported as broken when they're fine.
#[tauri::command]
async fn check_bridge(
    window: tauri::WebviewWindow,
    base_url: String,
    bearer: String,
    model: Option<String>,
) -> Result<BridgeStatus, String> {
    assert_overlay(&window)?;
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(BridgeStatus {
                reachable: false,
                status: 0,
                latency_ms: 0,
                hint: format!("HTTP client init failed: {e}"),
            });
        }
    };
    let user_model = model.unwrap_or_else(|| "claude-haiku-4-5".to_string());
    // Two-phase probe: first try user's configured model. If we get a 400
    // that looks like "model not found", retry with a universal fallback
    // so we can distinguish "bridge is broken" from "bridge is fine but
    // doesn't ship this model alias". Hint message says exactly that.
    let t0_outer = std::time::Instant::now();
    let probe_once = |model_name: String| {
        let client = client.clone();
        let url = url.clone();
        let bearer = bearer.clone();
        async move {
            let body = serde_json::json!({
                "model": model_name,
                "messages": [{"role": "user", "content": "."}],
                "max_tokens": 1,
                "stream": false,
            });
            let t = std::time::Instant::now();
            let r = client.post(&url).bearer_auth(&bearer).json(&body).send().await?;
            let status = r.status().as_u16();
            let body_text = r.text().await.unwrap_or_default();
            Ok::<(u16, String, u64), reqwest::Error>((status, body_text, t.elapsed().as_millis() as u64))
        }
    };

    let first = probe_once(user_model.clone()).await;
    match first {
        Ok((status, body_text, latency_ms)) => {
            // 400 + body mentions "model" → likely model-not-found.
            // Retry with universal fallback to confirm bridge itself is OK.
            // Extracted as pure fn `is_model_not_found_response` so we can
            // unit-test the matrix of bridge error messages.
            let looks_like_model_404 = is_model_not_found_response(status, &body_text);
            if looks_like_model_404 {
                let fallback = "claude-3-5-sonnet-latest".to_string(); // most universal OpenAI-compat alias
                if let Ok((fb_status, _fb_body, fb_latency)) = probe_once(fallback).await {
                    if fb_status < 500 && fb_status != 400 {
                        // Bridge IS healthy, just doesn't speak user's model name.
                        return Ok(BridgeStatus {
                            reachable: true,
                            status,
                            latency_ms: fb_latency,
                            hint: format!(
                                "bridge alive (HTTP {} with fallback model), but rejected `{}` — check ai_model setting",
                                fb_status, user_model
                            ),
                        });
                    }
                }
            }
            // 200 = full success. 4xx = bridge is alive but rejected our
            // payload (bad model name / auth) — still counts as "reachable".
            // 5xx = bridge is sick.
            let reachable = status < 500;
            let hint = if status == 0 {
                "no HTTP response — DNS or connection refused".into()
            } else if status == 401 || status == 403 {
                "Bearer token rejected — check ai_bearer (BRIDGE_SECRET)".into()
            } else if status == 404 {
                format!("404 — endpoint /chat/completions not found on {base_url} (typo in URL?)")
            } else if status == 400 {
                format!("HTTP 400 — bridge rejected payload (body: {})", body_text.chars().take(120).collect::<String>())
            } else if status >= 500 {
                format!("HTTP {status} — bridge is reachable but failing")
            } else {
                String::new()
            };
            Ok(BridgeStatus { reachable, status, latency_ms, hint })
        }
        Err(e) => {
            let msg = format!("{e}");
            let latency_ms = t0_outer.elapsed().as_millis() as u64;
            let hint = if msg.contains("timed out") {
                format!("no response in 5s — wrong IP/port? (probed {url})")
            } else if msg.contains("dns") || msg.contains("name resolution") {
                "DNS failed — check that ai_base_url is a valid host".into()
            } else if msg.contains("connection refused") || msg.contains("ConnectRefused") {
                format!("connection refused — is bridge running on {base_url}?")
            } else {
                msg
            };
            Ok(BridgeStatus { reachable: false, status: 0, latency_ms, hint })
        }
    }
}

/// Pure helper for `check_bridge`: identifies HTTP 400 responses whose
/// body suggests "model not found / unknown / invalid" — used to decide
/// whether to retry the probe with a universal fallback model name.
///
/// Loose matching by design: a bridge that returns "invalid request body —
/// contact model team" will ALSO trigger the fallback retry. Cost is one
/// extra 1-token POST; benefit is correctly distinguishing "bridge is fine,
/// just doesn't speak this model alias" from "bridge is broken".
fn is_model_not_found_response(status: u16, body: &str) -> bool {
    if status != 400 {
        return false;
    }
    let body_lower = body.to_lowercase();
    body_lower.contains("model")
        && (body_lower.contains("not found")
            || body_lower.contains("unknown")
            || body_lower.contains("invalid"))
}

#[cfg(test)]
mod bridge_probe_tests {
    use super::is_model_not_found_response;

    #[test]
    fn non_400_never_matches() {
        assert!(!is_model_not_found_response(200, "model not found"));
        assert!(!is_model_not_found_response(401, "model not found"));
        assert!(!is_model_not_found_response(500, "model unknown"));
        assert!(!is_model_not_found_response(0, ""));
    }

    #[test]
    fn ollama_unknown_model_matches() {
        // Ollama returns this exact pattern
        let body = r#"{"error":"model 'claude-haiku-4-5' not found, try `ollama pull`"}"#;
        assert!(is_model_not_found_response(400, body));
    }

    #[test]
    fn openai_invalid_model_matches() {
        // OpenAI proxy returns "model_not_found" error code with text
        let body = r#"{"error":{"message":"The model `claude-haiku-4-5` does not exist","type":"invalid_request_error","code":"model_not_found"}}"#;
        assert!(is_model_not_found_response(400, body));
    }

    #[test]
    fn anthropic_passthrough_unknown_model_matches() {
        let body = r#"{"error":{"type":"invalid_request_error","message":"Unknown model: claude-haiku-4-5"}}"#;
        assert!(is_model_not_found_response(400, body));
    }

    #[test]
    fn unrelated_400_does_not_match() {
        // 400 from rate limit / quota / etc. — no "model" word in error
        let body = r#"{"error":"Rate limit exceeded"}"#;
        assert!(!is_model_not_found_response(400, body));
    }

    #[test]
    fn malformed_request_400_does_not_match() {
        let body = r#"{"error":"Invalid JSON in request body"}"#;
        assert!(!is_model_not_found_response(400, body));
    }

    #[test]
    fn case_insensitive_match() {
        // Some bridges uppercase "Model" or "MODEL"
        assert!(is_model_not_found_response(400, r#"{"error":"MODEL NOT FOUND"}"#));
        assert!(is_model_not_found_response(400, r#"{"error":"Model Unknown"}"#));
    }

    #[test]
    fn empty_body_does_not_match() {
        // Defensive: 400 with no body shouldn't trigger fallback probe.
        assert!(!is_model_not_found_response(400, ""));
    }

    #[test]
    fn known_false_positive_documented() {
        // Per fn docstring: legitimate non-model-related errors that happen
        // to mention "model" in their text will trigger fallback. Accepted.
        let body = r#"{"error":"Request invalid: please contact the model team"}"#;
        assert!(is_model_not_found_response(400, body),
            "documented false-positive: matcher is loose by design");
    }
}

// ── Update checker ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct UpdateInfo {
    /// Currently-installed version (from Cargo.toml at compile time).
    current: String,
    /// Newest published GitHub release tag (without leading "v"), if any.
    latest: Option<String>,
    /// True iff `latest` is strictly newer than `current` (semver-ish
    /// numeric compare). False means "you're up to date" or "couldn't tell".
    update_available: bool,
    /// URL to open in browser for download.
    download_url: String,
    /// Release notes / changelog from GitHub release body.
    notes: String,
    /// Empty on success, else explanation.
    error: String,
}

const REPO_OWNER: &str = "PavelLizunov";
const REPO_NAME: &str = "suflyor";

/// Check the suflyor GitHub repo for a newer release. Lightweight: one
/// HTTP GET to api.github.com/releases/latest, ~1 KB JSON response. UI
/// shows a button "Скачать v0.0.2" when update_available=true; clicking
/// opens the download URL via the existing opener plugin.
///
/// Note: this does NOT auto-download or auto-install. That would require
/// code signing (we don't have certs) AND it's risky for a personal tool
/// to silently swap its own binary. The user reviews + clicks.
#[tauri::command]
async fn check_update(window: tauri::WebviewWindow) -> Result<UpdateInfo, String> {
    assert_overlay(&window)?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let download_default = format!(
        "https://github.com/{owner}/{name}/releases/latest",
        owner = REPO_OWNER, name = REPO_NAME
    );
    let url = format!(
        "https://api.github.com/repos/{owner}/{name}/releases/latest",
        owner = REPO_OWNER, name = REPO_NAME
    );
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("suflyor/{current} update-check"))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Ok(UpdateInfo {
            current, latest: None, update_available: false,
            download_url: download_default,
            notes: String::new(),
            error: format!("HTTP client init failed: {e}"),
        }),
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return Ok(UpdateInfo {
            current, latest: None, update_available: false,
            download_url: download_default,
            notes: String::new(),
            error: format!("GitHub unreachable: {e}"),
        }),
    };
    if !resp.status().is_success() {
        return Ok(UpdateInfo {
            current, latest: None, update_available: false,
            download_url: download_default,
            notes: String::new(),
            error: format!("GitHub returned HTTP {}", resp.status()),
        });
    }
    let v: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return Ok(UpdateInfo {
            current, latest: None, update_available: false,
            download_url: download_default,
            notes: String::new(),
            error: format!("malformed JSON from GitHub: {e}"),
        }),
    };
    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
    let latest_str = tag.trim_start_matches('v').to_string();
    let notes = v.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string();
    let release_url = v.get("html_url").and_then(|u| u.as_str()).unwrap_or(&download_default).to_string();
    // Edge case: GitHub returned HTTP 200 but body is missing `tag_name` or
    // it's an empty string. Don't pretend everything is fine — tell the
    // user the API responded weirdly so they don't get a false "up to date"
    // when the response is actually broken.
    if latest_str.is_empty() {
        return Ok(UpdateInfo {
            current,
            latest: None,
            update_available: false,
            download_url: release_url,
            notes,
            error: "GitHub API returned no tag_name — releases page may be empty or response malformed".into(),
        });
    }
    let update_available = is_strictly_newer(&latest_str, &current);
    Ok(UpdateInfo {
        current,
        latest: Some(latest_str),
        update_available,
        download_url: release_url,
        notes,
        error: String::new(),
    })
}

/// Tiny semver-ish compare: split by '.' and compare numeric components
/// left-to-right. Non-numeric components compared as strings. Returns true
/// iff `candidate` is strictly newer than `current`. Pre-release suffixes
/// (e.g. "-rc1") are ignored for now.
fn is_strictly_newer(candidate: &str, current: &str) -> bool {
    let strip = |s: &str| -> String {
        let s = s.trim_start_matches('v').to_string();
        // Drop pre-release / build metadata.
        s.split(['-', '+']).next().unwrap_or("").to_string()
    };
    let a = strip(candidate);
    let b = strip(current);
    if a.is_empty() {
        return false;
    }
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect()
    };
    let av = parse(&a);
    let bv = parse(&b);
    for i in 0..av.len().max(bv.len()) {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        if ai > bi { return true; }
        if ai < bi { return false; }
    }
    false // equal
}

#[cfg(test)]
mod update_tests {
    use super::is_strictly_newer;
    #[test] fn equal_is_not_newer() {
        assert!(!is_strictly_newer("0.0.1", "0.0.1"));
        assert!(!is_strictly_newer("1.2.3", "1.2.3"));
    }
    #[test] fn lower_is_not_newer() {
        assert!(!is_strictly_newer("0.0.0", "0.0.1"));
        assert!(!is_strictly_newer("1.0.0", "2.0.0"));
    }
    #[test] fn higher_is_newer() {
        assert!(is_strictly_newer("0.0.2", "0.0.1"));
        assert!(is_strictly_newer("0.1.0", "0.0.1"));
        assert!(is_strictly_newer("1.0.0", "0.99.99"));
    }
    #[test] fn v_prefix_ignored() {
        assert!(is_strictly_newer("v0.0.2", "0.0.1"));
        assert!(is_strictly_newer("v0.0.2", "v0.0.1"));
    }
    #[test] fn prerelease_suffix_ignored() {
        // 0.0.2-rc1 treated as 0.0.2 — still newer than 0.0.1.
        assert!(is_strictly_newer("0.0.2-rc1", "0.0.1"));
        // 0.0.1+build5 same as 0.0.1.
        assert!(!is_strictly_newer("0.0.1+build5", "0.0.1"));
    }
    #[test] fn empty_candidate_is_not_newer() {
        assert!(!is_strictly_newer("", "0.0.1"));
    }
    #[test] fn unequal_segment_counts() {
        // "1" is treated as "1.0.0" via unwrap_or(0) padding.
        assert!(!is_strictly_newer("1", "1.0.0"));
        assert!(!is_strictly_newer("1.0", "1.0.0"));
        // "1.0.0.5" — the 4th segment is ignored only via comparison
        // (av[3]=5, bv[3]=0 → av>bv, return true). Documents the actual
        // behavior even if 4-segment versions are unusual.
        assert!(is_strictly_newer("1.0.0.5", "1.0.0"));
    }
    #[test] fn non_numeric_segments_treated_as_zero() {
        // Each segment parse-or-zero. "abc" → [0] vs "0.0.0" → [0,0,0]
        // → padded loop is all zeros. Returns false (equal).
        assert!(!is_strictly_newer("abc", "0.0.0"));
        // "0.x.1" → [0, 0, 1] vs "0.0.0" → [0, 0, 0] → 0=0, 0=0, 1>0 → true.
        // Garbage middle segment doesn't trip the loop.
        assert!(is_strictly_newer("0.x.1", "0.0.0"));
    }
}

// ── Replay viewer (read session journals) ────────────────────────────────

#[derive(Debug, Serialize)]
struct SessionInfo {
    path: String,
    filename: String,
    size_bytes: u64,
    modified_unix: u64,
}

/// List all `.jsonl` session journals in the sessions dir, sorted by
/// modified time desc (newest first). Used by in-app replay viewer.
#[tauri::command]
fn list_sessions(window: tauri::WebviewWindow) -> Result<Vec<SessionInfo>, String> {
    assert_overlay(&window)?;
    let dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let read = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let mut out: Vec<SessionInfo> = read
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let size_bytes = meta.len();
            let modified_unix = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            Some(SessionInfo {
                path: path.to_string_lossy().to_string(),
                filename,
                size_bytes,
                modified_unix,
            })
        })
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.modified_unix));
    Ok(out)
}

/// Read a JSONL session journal, parse each line as a Value, return an
/// array. Refuses files >10MB to keep the renderer responsive.
#[tauri::command]
fn load_session(
    window: tauri::WebviewWindow,
    path: String,
) -> Result<Vec<serde_json::Value>, String> {
    assert_overlay(&window)?;
    const MAX_BYTES: u64 = 10 * 1024 * 1024;

    let p = std::path::PathBuf::from(&path);
    // Restrict reads to the sessions dir so a malicious caller can't slurp
    // arbitrary files via the renderer.
    let sessions_dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    let canonical_session_dir = sessions_dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = p.canonicalize().map_err(|e| e.to_string())?;
    if !canonical_path.starts_with(&canonical_session_dir) {
        return Err("path is outside sessions dir".into());
    }

    let meta = std::fs::metadata(&canonical_path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_BYTES {
        return Err(format!(
            "session file too large ({} bytes, max {})",
            meta.len(),
            MAX_BYTES
        ));
    }

    let content = std::fs::read_to_string(&canonical_path).map_err(|e| e.to_string())?;
    let mut events = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(v) => events.push(v),
            Err(e) => log::warn!("load_session: skip malformed line: {e}"),
        }
    }
    Ok(events)
}

// ── Entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let shared_cfg = config::shared();
    let shared_rt = runtime::shared();
    let shared_tiles = tile::shared();

    tauri::Builder::default()
        // Single-instance guard — if another overlay-mvp.exe is already
        // running, this new launch will FOCUS the existing window and
        // exit cleanly. Prevents orphan processes that hold global
        // hotkeys (caught live 2026-05-25: F3/F4/F8/F9 silently failed
        // to register on the second instance, blocking critical UX).
        // The closure receives args + cwd from the second launch attempt
        // — we just bring the existing overlay forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("single-instance: second launch attempted, focusing existing overlay");
            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.show();
                let _ = w.set_focus();
                let _ = w.unminimize();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(shared_cfg.clone())
        .manage(shared_rt.clone())
        .manage(shared_tiles.clone())
        .setup(move |app| {
            let _warnings = hotkeys::register_all(app.handle(), shared_cfg.clone());
            if let Err(e) = tray::setup(app.handle()) {
                log::warn!("tray setup failed: {e}");
            }
            // DEBUG: auto-open devtools on launch so we can see JS console.
            // SECURITY: this MUST be debug-only. The Tauri command surface
            // (incl. get_config which returns groq_api_key + ai_bearer) is
            // reachable from any JS context. A devtools console in release
            // = anyone with momentary local access can dump every secret.
            // The unconditional duplicate that lived here previously was a
            // S0 finding from the night-run security audit (2026-05-25).
            #[cfg(debug_assertions)]
            if let Some(w) = app.get_webview_window("overlay") {
                w.open_devtools();
            }
            // STEALTH: optional — controlled by config.stealth_enabled.
            let stealth_on = shared_cfg.read().stealth_enabled;
            if stealth_on {
                if let Some(w) = app.get_webview_window("overlay") {
                    match w.set_content_protected(true) {
                        Ok(_) => log::info!("overlay window content protection ENABLED"),
                        Err(e) => log::warn!("set_content_protected failed: {e}"),
                    }
                }
            } else {
                log::info!("stealth OFF — overlay will appear in screen-share");
            }

            // Reaper task: defense-in-depth cleanup of leaked tiles. The
            // per-tile TTL task should always handle close, but if a tile's
            // task ever panics or is dropped, this 30s tick will sweep the
            // zombie tile from the active list AND close its webview.
            //
            // MUST use tauri::async_runtime — Tauri's setup() runs BEFORE
            // the main tokio runtime is installed, so a plain tokio::spawn
            // panics with "there is no reactor running".
            let tiles_for_reaper = shared_tiles.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));
                ticker.tick().await; // discard first immediate tick
                loop {
                    ticker.tick().await;
                    tile::reaper_tick(&app_handle, &tiles_for_reaper);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            list_audio_devices,
            start_session,
            stop_session,
            ask_ai,
            take_screenshot,
            get_transcript,
            open_settings,
            close_settings,
            quit_app,
            prep_record,
            prep_structure,
            spawn_tile,
            close_tile,
            pin_tile,
            ask_from_mic,
            ask_from_system,
            manual_ask_hold_start,
            manual_ask_hold_end,
            list_monitors,
            open_sessions_folder,
            last_session_summary,
            export_config,
            export_config_safe,
            import_config,
            list_sessions,
            load_session,
            set_stealth,
            list_snippets,
            expand_snippet,
            kb_search,
            kb_get,
            kb_stats,
            kb_spawn,
            check_bridge,
            check_update,
            crash_report_path,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            // P0-3: graceful degradation instead of process-level panic.
            // Log to env_logger, then write a crash report next to the
            // exe so the user can email it. Exiting non-zero so a wrapper
            // (Task Scheduler, systemd, future auto-restart) can detect.
            log::error!("fatal: tauri run failed: {e:#}");
            let report = format!(
                "suflyor crashed at startup.\n\
                 ---\n\
                 {e:#}\n\
                 ---\n\
                 Likely causes:\n\
                 - WebView2 runtime missing (Microsoft Edge runtime)\n\
                 - Capability/permission file malformed\n\
                 - Another suflyor process holds the single-instance lock\n\
                 - Anti-virus blocked the bundled WebView\n\
                 Send this file to the maintainer if you can't tell which one.\n"
            );
            if let Some(dir) = dirs::config_dir() {
                let path = dir.join("overlay-mvp").join("crash-report.txt");
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, report);
                eprintln!("crash report written to: {}", path.display());
            }
            std::process::exit(1);
        });
}
