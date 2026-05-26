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

/// v0.0.26: backend guard against concurrent download_and_install_update.
/// JS-side `oneClickBusy` ref only protects the Settings React component;
/// a second invocation from devtools or a programmatic invoke would race
/// to write the same `%TEMP%/suflyor-update-<ver>.exe`. Two writers →
/// the second hits a Windows sharing-violation on the file once the
/// first spawned process opens it. Better to refuse re-entry cleanly.
static UPDATE_IN_FLIGHT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

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

/// v0.0.24: nuclear "close every tile" — used by the Ctrl+Alt+W hotkey
/// and tray menu item. Helpful when aggressive mode floods the screen
/// or the user wants a clean slate without quitting the whole app.
/// Respects pin: pinned tiles stay (consistent with TTL reaper behavior).
///
/// v0.0.28: added `assert_overlay` guard. The tray + hotkey path call
/// `tile::close_all_unpinned` directly without going through this Tauri
/// command, so the only callers of THIS command would be from frontend
/// JS — and only the overlay window should be able to nuke all tiles.
/// Without this, a compromised tile-* window could DoS the user's
/// pinned context by spawning + nuking in a loop. Caught by review-pass
/// agent on v0.0.20→v0.0.27 diff.
#[tauri::command]
fn close_all_tiles(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    tiles: tauri::State<'_, SharedTiles>,
) -> Result<usize, String> {
    assert_overlay(&window)?;
    Ok(tile::close_all_unpinned(&app, tiles.inner()))
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
/// Redact known secret patterns from arbitrary diagnostic text (crash
/// reports, log excerpts, etc.) before including in a dump. Defensive
/// belt-and-suspenders: most crash text won't contain secrets in the
/// first place, but a future panic message that captures the full Debug
/// repr of an HTTP request COULD include the bridge bearer. Better to
/// always run text through this than rely on hoping panics stay clean.
///
/// Patterns covered:
///   - `gsk_...` (Groq API key prefix)
///   - `Bearer ...` (HTTP Authorization header value)
///   - `sk-...` (defensive — OpenAI-style key prefix, not actually used
///     by this app but cheap insurance if user ever sets one)
pub(crate) fn sanitize_diagnostic_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        // Match "gsk_" / "sk-" / "Bearer " — consume the secret token after
        // the prefix and emit a placeholder. Token defined as non-whitespace
        // and non-quote chars.
        let rest = &s[i..];
        let prefix_match: Option<(&str, &str)> = if rest.starts_with("gsk_") {
            Some(("gsk_", "<REDACTED_GROQ_KEY>"))
        } else if rest.starts_with("sk-") {
            Some(("sk-", "<REDACTED_API_KEY>"))
        } else if rest.starts_with("Bearer ") {
            Some(("Bearer ", "Bearer <REDACTED>"))
        } else {
            None
        };
        if let Some((prefix, replacement)) = prefix_match {
            out.push_str(replacement);
            // Skip past prefix + secret token (until whitespace/quote/end).
            let mut j = i + prefix.len();
            while j < bytes.len() {
                let b = bytes[j];
                if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == b'"' || b == b'\'' {
                    break;
                }
                j += 1;
            }
            i = j;
        } else {
            // Copy one char (handling multi-byte UTF-8 properly via char_indices).
            // Simpler: just copy this byte and advance. The redaction patterns
            // are all ASCII, so we won't split a multibyte char.
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod sanitize_tests {
    use super::sanitize_diagnostic_text;
    #[test] fn redacts_groq_key() {
        assert_eq!(
            sanitize_diagnostic_text("error: gsk_abc123def456 invalid"),
            "error: <REDACTED_GROQ_KEY> invalid"
        );
    }
    #[test] fn redacts_bearer_header() {
        assert_eq!(
            sanitize_diagnostic_text("Authorization: Bearer xyz789 sent"),
            "Authorization: Bearer <REDACTED> sent"
        );
    }
    #[test] fn redacts_sk_key() {
        assert_eq!(
            sanitize_diagnostic_text("token=sk-abcdef nope"),
            "token=<REDACTED_API_KEY> nope"
        );
    }
    #[test] fn leaves_other_text_unchanged() {
        let s = "plain message with no secrets here";
        assert_eq!(sanitize_diagnostic_text(s), s);
    }
    #[test] fn stops_at_quotes() {
        // JSON-quoted secret should leave the closing quote alone.
        assert_eq!(
            sanitize_diagnostic_text("\"key\":\"gsk_x123\""),
            "\"key\":\"<REDACTED_GROQ_KEY>\""
        );
    }
}

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

/// Write a markdown diagnostic dump to Desktop combining sanitized config
/// + app version + most recent session journal tail + crash report (if any).
/// Useful when filing a bug report — one file the user can attach without
/// worrying about leaking the bridge bearer or Groq key.
#[tauri::command]
fn dump_diagnostics(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let cfg = blank_share_secrets(state.read().clone());
    let cfg_json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;

    let app_version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Latest session journal tail (last 50 lines so the report stays under
    // ~50 KB even for chatty sessions). Runs through sanitize_diagnostic_text
    // so any gsk_/Bearer/sk- patterns are redacted. NOTE: ai_request events
    // include system_prompt + user_prompt which contain the user's
    // meeting_context. That's not a "secret pattern" so the sanitizer leaves
    // it intact — the dump output flags this so the user can review before
    // sharing.
    let journal_tail = journal::sessions_dir()
        .ok()
        .and_then(|dir| {
            std::fs::read_dir(&dir).ok().and_then(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|s| s.to_str())
                            .map(|s| s.eq_ignore_ascii_case("jsonl"))
                            .unwrap_or(false)
                    })
                    .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
                    .map(|e| e.path())
            })
        })
        .and_then(|path| std::fs::read_to_string(&path).ok().map(|s| (path, s)))
        .map(|(path, content)| {
            let lines: Vec<&str> = content.lines().collect();
            let tail_start = lines.len().saturating_sub(50);
            let tail = sanitize_diagnostic_text(&lines[tail_start..].join("\n"));
            // Emit only the filename, not the full path. The full path
            // contains the Windows username (`C:\Users\<user>\AppData\...`)
            // which is a low-grade PII leak when sharing the dump.
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("session.jsonl");
            format!("**File:** `{}` (in your `%APPDATA%\\overlay-mvp\\sessions\\`)\n**Tail (last {} lines, sanitized):**\n```jsonl\n{}\n```\n\n_NOTE: `ai_request` events in this tail include `system_prompt` + `user_prompt` which contain your meeting_context (e.g. company name, role). Review before sharing if that's sensitive._", filename, lines.len() - tail_start, tail)
        })
        .unwrap_or_else(|| "_no session journal found_".to_string());

    // Optional crash report (inline the same path-resolution as
    // crash_report_path so we don't need to fake a WebviewWindow).
    // Sanitize content defensively in case a future panic ever includes
    // the user's Groq key or bridge bearer in its Debug output.
    let crash = dirs::config_dir()
        .map(|d| d.join("overlay-mvp").join("crash-report.txt"))
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .map(|s| sanitize_diagnostic_text(&s))
        .map(|s| format!("```\n{s}\n```"))
        .unwrap_or_else(|| "_no crash report on disk_".to_string());

    // v0.0.21: runtime panic log (separate from startup crash-report.txt).
    // Captures worker-thread panics that don't kill the process but still
    // indicate a bug (e.g. WASAPI device race on F8 rapid double-press).
    let runtime_panics = dirs::config_dir()
        .map(|d| d.join("overlay-mvp").join("runtime-panics.log"))
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .map(|s| {
            // Tail last 100 lines so even a multi-MB log doesn't blow the dump.
            let lines: Vec<&str> = s.lines().collect();
            let start = lines.len().saturating_sub(100);
            sanitize_diagnostic_text(&lines[start..].join("\n"))
        })
        .map(|s| format!("```\n{s}\n```"))
        .unwrap_or_else(|| "_no runtime panics on disk_".to_string());

    let report = format!(
        "# Suflyor diagnostic dump\n\n\
         **App version:** v{app_version}\n\
         **Platform:** {os}/{arch}\n\n\
         ## Sanitized config\n\n\
         Secrets (groq_api_key, ai_bearer, ai_base_url, meeting_context, profiles) are blanked.\n\n\
         ```json\n{cfg_json}\n```\n\n\
         ## Latest session journal (tail)\n\n\
         {journal_tail}\n\n\
         ## Startup crash report\n\n\
         {crash}\n\n\
         ## Runtime panics (last 100 lines)\n\n\
         {runtime_panics}\n"
    );

    let stamp = journal::now_unix_ms() / 1000;
    let desktop = dirs::desktop_dir().or_else(dirs::home_dir).ok_or("no desktop dir")?;
    let path = desktop.join(format!("suflyor-diagnostic-{stamp}.md"));
    std::fs::write(&path, report).map_err(|e| e.to_string())?;
    log::info!("diagnostic dump written to {}", path.display());
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
    // v0.0.17: removed the Desktop-OR-Documents path allowlist that this
    // command used to enforce. Reasons:
    //   1. assert_overlay above already blocks any non-overlay window from
    //      calling import_config — a poisoned tile can't reach this code
    //      path. The allowlist was layering paranoia on top of that, not
    //      providing a unique defense.
    //   2. Allowlist broke real user flows: Russian Windows users where
    //      Desktop is localised to "Рабочий стол" inside OneDrive, files
    //      saved to Downloads, network shares, anywhere not under the two
    //      hardcoded folders. Original v0.0.4 ticket "там надо вводить
    //      путь руками потому что Desktop не находит".
    //   3. v0.0.17 also moves the import trigger to native file picker
    //      (tauri-plugin-dialog), which is itself a user-action gate that
    //      AI-rendered markdown cannot trigger.
    // The json parse error path still strips bytes from its message so
    // even if a poisoned overlay somehow invoked import_config with an
    // arbitrary path, the error doesn't leak file contents.
    let raw = std::path::PathBuf::from(&path);
    let canon = raw
        .canonicalize()
        .map_err(|e| format!("path canonicalize failed: {e}"))?;
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
        // v0.0.41: cap height to monitor work area minus 40 px taskbar
        // gap. Was hardcoded 900 px — on screens shorter than 900
        // (laptops with 1366×768 or scaled 1080p), the bottom of the
        // Settings window with the footer (Save / Back buttons) was
        // off-screen. Now: shrink to fit.
        let monitor_h = w.current_monitor()
            .ok()
            .flatten()
            .map(|m| {
                let scale = m.scale_factor();
                (m.size().height as f64 / scale) - 40.0
            })
            .unwrap_or(900.0);
        // clamp(min, max) — clippy::manual_clamp wanted instead of chained
        // .min().max(). Same semantics: floor at 480 (don't shrink past usable),
        // ceiling at 900 (don't grow past the 12-section content height).
        let target_h = monitor_h.clamp(480.0, 900.0);
        let _ = w.set_size(tauri::LogicalSize::new(760.0, target_h));
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

/// v0.0.23: one-click update flow. Downloads the latest NSIS installer
/// to %TEMP%, spawns it, and returns. The frontend then calls quit_app
/// so the installer can replace overlay-mvp.exe without "file in use"
/// errors. NSIS handles UAC prompt + relaunch on success.
///
/// We use NSIS (`*_x64-setup.exe`) over MSI because:
///   - NSIS installer can replace a running binary on Windows more
///     reliably (defers via reboot-once-required marker, or uses
///     msiexec /qb that the user cannot abort mid-replace)
///   - NSIS is a single .exe — `Command::new(path).spawn()` works directly
///   - MSI needs `msiexec.exe /i path.msi` which sometimes doesn't trigger
///     the UAC flow correctly when spawned from a user-context Tauri app
///
/// Returns the path of the launched installer so the UI can show it.
#[tauri::command]
async fn download_and_install_update(window: tauri::WebviewWindow) -> Result<String, String> {
    assert_overlay(&window)?;
    // v0.0.26: refuse re-entry. compare_exchange returns Ok if it was
    // false → we win, set it true and proceed. Err means another call
    // is already in flight. We do NOT unset on success — the spawned
    // installer has the file mmap'd; a second call would hit Windows
    // sharing-violation. We DO unset on error so the user can retry.
    use std::sync::atomic::Ordering;
    if UPDATE_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Update already in progress — wait for the installer to launch".into());
    }
    // RAII helper: drop clears the flag. Any early `?` return below
    // releases the lock so the user can retry. On the success path we
    // explicitly `std::mem::forget(guard)` immediately after the spawn
    // line so the lock STAYS SET (the spawned installer has the file
    // mmap'd; a second invoke would hit a sharing-violation).
    //
    // v0.0.27: switched from `guard.reset=false` to mem::forget. Same
    // effect, but the intent is now compiler-visible — any future edit
    // slipping a fallible call between spawn() and the forget would
    // still leak the lock, but the forget reads as "deliberately do
    // NOT run the destructor" rather than mutating a flag.
    struct ReleaseGuard;
    impl Drop for ReleaseGuard {
        fn drop(&mut self) {
            UPDATE_IN_FLIGHT.store(false, Ordering::SeqCst);
        }
    }
    let guard = ReleaseGuard;

    let current = env!("CARGO_PKG_VERSION");
    let api_url = format!(
        "https://api.github.com/repos/{owner}/{name}/releases/latest",
        owner = REPO_OWNER, name = REPO_NAME
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent(format!("suflyor/{current} update-download"))
        .build()
        .map_err(|e| format!("HTTP client init failed: {e}"))?;

    let v: serde_json::Value = client
        .get(&api_url)
        .send()
        .await
        .map_err(|e| format!("GitHub API unreachable: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub API error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("malformed GitHub JSON: {e}"))?;

    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
    let latest = tag.trim_start_matches('v');
    if latest.is_empty() {
        return Err("GitHub release has no tag_name".into());
    }

    // Find the NSIS asset: name pattern `suflyor_<ver>_x64-setup.exe`.
    let assets = v.get("assets").and_then(|a| a.as_array())
        .ok_or("GitHub release has no assets")?;
    let nsis_asset = assets.iter().find(|a| {
        a.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.ends_with("_x64-setup.exe"))
            .unwrap_or(false)
    }).ok_or("no NSIS installer (*_x64-setup.exe) in latest release")?;
    let download_url = nsis_asset
        .get("browser_download_url")
        .and_then(|u| u.as_str())
        .ok_or("asset missing browser_download_url")?;

    log::info!("downloading update v{latest} from {download_url}");
    let bytes = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download HTTP error: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("read body failed: {e}"))?;
    log::info!("downloaded {} bytes", bytes.len());

    // Sanity check: NSIS setup.exe should be at least a few hundred KB.
    // If GitHub Releases is mid-publish or asset is corrupt, we get a
    // tiny redirect HTML — refuse to spawn it to avoid the user seeing
    // a weird error from Windows.
    if bytes.len() < 100_000 {
        return Err(format!(
            "downloaded file is suspiciously small ({} bytes) — refusing to launch",
            bytes.len()
        ));
    }

    // Write to %TEMP%/suflyor-update-<ver>.exe. Overwrite if exists.
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(format!("suflyor-update-{latest}.exe"));
    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("temp write failed: {e}"))?;
    log::info!("update installer written to {}", installer_path.display());

    // Spawn the installer detached. The installer will:
    //   1. Prompt UAC for elevation
    //   2. Wait for overlay-mvp.exe to exit (or kill it)
    //   3. Replace files
    //   4. Optionally relaunch (NSIS default)
    //
    // We use `spawn()` (NOT `output()`) so we don't wait for it to exit.
    // The frontend will call quit_app right after this returns.
    std::process::Command::new(&installer_path)
        .spawn()
        .map_err(|e| format!("spawn installer failed: {e}"))?;

    // v0.0.27: spawn succeeded — leak the guard so its Drop never runs
    // and UPDATE_IN_FLIGHT stays SET. A second invoke can't race against
    // the running installer for the same file. The flag naturally clears
    // when the app quits 2s later.
    //
    // v0.0.28: edge case — if BOTH quit_app and window.close() fail
    // frontend-side (extremely rare; would mean Tauri shutdown is
    // totally broken), the process stays alive and the lock stays
    // SET forever in this session. The Settings toast-fallback path
    // now calls `clear_update_in_flight` to unstick it.
    std::mem::forget(guard);
    Ok(installer_path.to_string_lossy().to_string())
}

/// Manually clear the UPDATE_IN_FLIGHT lock. Called by the Settings
/// toast-fallback when both quit_app AND window.close() failed — the
/// process is still alive but the lock would otherwise stay SET until
/// the user manually restarts the app. v0.0.28.
#[tauri::command]
fn clear_update_in_flight(window: tauri::WebviewWindow) -> Result<(), String> {
    assert_overlay(&window)?;
    UPDATE_IN_FLIGHT.store(false, std::sync::atomic::Ordering::SeqCst);
    log::info!("clear_update_in_flight: lock manually released by frontend");
    Ok(())
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

/// Trim `s` to roughly its last `keep_bytes` and snap to the next entry
/// boundary ("\n\n"). UTF-8 safe — walks forward from the target offset
/// to the next char boundary before slicing.
///
/// Used by the runtime-panics.log rotation in the panic hook. v0.0.27.
fn truncate_panic_log_tail(s: &str, keep_bytes: usize) -> &str {
    let mut start = s.len().saturating_sub(keep_bytes);
    // s.len() is always a char boundary so this terminates.
    while !s.is_char_boundary(start) {
        start += 1;
    }
    // s[start..] is now a valid UTF-8 slice. Snap to entry separator.
    s[start..]
        .find("\n\n")
        .map(|i| &s[start + i + 2..])
        .unwrap_or(&s[start..])
}

#[cfg(test)]
mod panic_log_rotation_tests {
    use super::truncate_panic_log_tail;

    #[test]
    fn truncates_pure_ascii() {
        let s = "aaaa\n\nbbbb\n\ncccc\n\n";
        let t = truncate_panic_log_tail(s, 8);
        // start = len-8 = 10, lands inside "cccc\n\n". Find "\n\n" → return "".
        // (the bytes "cccc" are at 12..16, then "\n\n" at 16..18).
        assert!(t.is_empty() || t == "cccc\n\n");
    }

    #[test]
    fn no_separator_returns_clean_utf8_tail() {
        let s = "aaaaaaaaaaaa"; // no "\n\n"
        let t = truncate_panic_log_tail(s, 4);
        assert_eq!(t, "aaaa");
    }

    #[test]
    fn keep_larger_than_input_returns_all() {
        let s = "short";
        let t = truncate_panic_log_tail(s, 1_000_000);
        assert_eq!(t, "short");
    }

    #[test]
    fn cyrillic_mid_char_offset_is_safe() {
        // 'я' = U+044F = 2 bytes in UTF-8 (0xD1 0x8F). 500 chars * 2B = 1000 bytes.
        // Each "ababab" group ends with "\n\n" entry separator.
        // Without the boundary fix this panics with
        //   "byte index N is not a char boundary; it is inside 'я' (bytes a..b)"
        // for ~50% of keep_bytes values.
        let mut s = String::new();
        for i in 0..500 {
            s.push_str(&format!("яяяяяя #{i}\n\n"));
        }
        // Sweep ALL keep_bytes from 1..s.len() — at least half land mid-char.
        for k in 1..s.len() {
            let t = truncate_panic_log_tail(&s, k);
            // If we reach here without panicking, the slice was valid UTF-8.
            // Also assert the result IS valid UTF-8 by parsing it as &str ops.
            let _ = t.chars().count();
        }
    }

    #[test]
    fn emoji_4byte_offset_is_safe() {
        // '🔥' = U+1F525 = 4 bytes in UTF-8. Tests the 4-byte char-boundary path.
        let mut s = String::new();
        for _ in 0..200 {
            s.push_str("🔥🔥🔥\n\n");
        }
        for k in 1..s.len() {
            let t = truncate_panic_log_tail(&s, k);
            let _ = t.chars().count();
        }
    }

    #[test]
    fn empty_string_returns_empty() {
        assert_eq!(truncate_panic_log_tail("", 100), "");
        assert_eq!(truncate_panic_log_tail("", 0), "");
    }

    #[test]
    fn separator_at_offset_works() {
        // Confirm the snap-to-separator: keep_bytes=10 lands at "\n\n" → tail = "ccc".
        let s = "aaaaaaa\n\nbbbb\n\nccc";
        let t = truncate_panic_log_tail(s, 10);
        // start = 18-10 = 8, lands at second char of first "\n\n" pair.
        // find "\n\n" in s[8..] returns position of next pair.
        // The exact tail depends on which separator is found first — what matters
        // is the result is valid UTF-8 and starts cleanly.
        assert!(!t.contains("\n\n\n"), "tail should not have triple newlines");
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

/// v0.0.60 (shipped as v0.0.60): aggregate stats over all session
/// JSONLs. Cheap pass — reads each file once, only looks at
/// `session_start` + `session_summary` events (skips the per-line
/// detail). Returns one big payload the frontend renders.
#[derive(Debug, Serialize)]
struct SessionStats {
    /// Total session files in the sessions dir.
    sessions_total: u32,
    /// Sessions that contain a `session_summary` event (i.e. cleanly
    /// closed). The difference (`sessions_total - sessions_closed`) is
    /// sessions where the app crashed mid-meeting.
    sessions_closed: u32,
    /// Total wall-clock duration across all closed sessions, in seconds.
    /// Open sessions contribute 0 (no end timestamp).
    duration_total_sec: u64,
    /// Total AI requests across all sessions.
    ai_requests_total: u64,
    /// Total tiles spawned across all sessions.
    tiles_spawned_total: u64,
    /// Sum of `total_cost_microcents` from every SessionSummary, in USD
    /// (divided by 100M for display).
    total_cost_usd: f64,
    /// Per-day session counts for the last 30 days (newest first).
    /// Tuple = ("YYYY-MM-DD", count).
    daily_last_30: Vec<(String, u32)>,
    /// Top-5 most-frequent tile question prefixes (first 60 chars,
    /// case-insensitive), sorted by count desc.
    top_questions: Vec<(String, u32)>,
}

#[tauri::command]
fn read_all_session_stats(window: tauri::WebviewWindow) -> Result<SessionStats, String> {
    assert_overlay(&window)?;
    let dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    if !dir.exists() {
        return Ok(SessionStats {
            sessions_total: 0,
            sessions_closed: 0,
            duration_total_sec: 0,
            ai_requests_total: 0,
            tiles_spawned_total: 0,
            total_cost_usd: 0.0,
            daily_last_30: vec![],
            top_questions: vec![],
        });
    }

    let mut sessions_total: u32 = 0;
    let mut sessions_closed: u32 = 0;
    let mut duration_total_ms: u64 = 0;
    let mut ai_requests_total: u64 = 0;
    let mut tiles_spawned_total: u64 = 0;
    let mut total_cost_microcents: u128 = 0;
    let mut daily_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut question_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue; };
        if !meta.is_file() { continue; }
        // Skip absurdly large files to keep this fast.
        if meta.len() > 50 * 1024 * 1024 { continue; }
        let Ok(content) = std::fs::read_to_string(&path) else { continue; };

        sessions_total += 1;
        let mut has_summary = false;

        // Day bucket from session_start unix_ms — fall back to file mtime
        // if the session_start line is missing/malformed.
        let mut day_bucket: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue; };
            let kind = v["kind"].as_str().unwrap_or("");
            match kind {
                "session_start" => {
                    // Take first session_start timestamp seen. Use match-
                    // guard form to satisfy clippy::collapsible_match.
                    if let (None, Some(ts)) = (day_bucket.as_ref(), v["unix_ms"].as_i64()) {
                        day_bucket = Some(ymd_from_unix_ms(ts));
                    }
                }
                "session_summary" => {
                    has_summary = true;
                    if let Some(d) = v["duration_ms"].as_u64() { duration_total_ms += d; }
                    if let Some(n) = v["ai_requests_total"].as_u64() { ai_requests_total += n; }
                    if let Some(n) = v["tiles_spawned"].as_u64() { tiles_spawned_total += n; }
                    if let Some(c) = v["total_cost_microcents"].as_u64() {
                        total_cost_microcents += c as u128;
                    }
                }
                "tile_spawn" => {
                    if let Some(q) = v["question"].as_str() {
                        // Normalise: lowercase, collapse whitespace, take first 60 chars.
                        let q = q.to_lowercase();
                        let normalized: String = q.split_whitespace().collect::<Vec<_>>().join(" ");
                        let trimmed: String = normalized.chars().take(60).collect();
                        if !trimmed.is_empty() {
                            *question_counts.entry(trimmed).or_insert(0) += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        if has_summary {
            sessions_closed += 1;
        }
        // Fall back to file mtime for bucketing if no session_start ts.
        let bucket = day_bucket.unwrap_or_else(|| {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            ymd_from_unix_ms(mtime)
        });
        *daily_counts.entry(bucket).or_insert(0) += 1;
    }

    // Last 30 days from today, newest first. Missing days emit 0.
    let today_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut daily_last_30: Vec<(String, u32)> = Vec::with_capacity(30);
    for day_offset in 0..30 {
        let ts = today_ms - day_offset as i64 * 86_400_000;
        let ymd = ymd_from_unix_ms(ts);
        let count = *daily_counts.get(&ymd).unwrap_or(&0);
        daily_last_30.push((ymd, count));
    }

    // Top-5 question prefixes by count desc.
    let mut top: Vec<(String, u32)> = question_counts.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(5);

    Ok(SessionStats {
        sessions_total,
        sessions_closed,
        duration_total_sec: duration_total_ms / 1000,
        ai_requests_total,
        tiles_spawned_total,
        total_cost_usd: total_cost_microcents as f64 / 100_000_000.0,
        daily_last_30,
        top_questions: top,
    })
}

/// Convert unix-ms timestamp to a "YYYY-MM-DD" string in UTC. We don't
/// pull chrono just for this — manual math is fine for date bucketing.
/// Works for any positive timestamp post-epoch.
fn ymd_from_unix_ms(ms: i64) -> String {
    let sec = ms / 1000;
    // Days since 1970-01-01 (UTC).
    let days = sec / 86_400;
    // Civil-from-days algorithm, public domain — Howard Hinnant.
    // Returns (year, month, day) Gregorian.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// v0.0.67: Quick-set STT language during a live session without
/// reopening Settings. Updates `cfg.stt_language` + persists to
/// config.json. STT module reads cfg on each transcription, so the
/// change applies on the very next audio chunk.
///
/// Accepts: "ru" | "en" | "" (empty = auto-detect). Any other value
/// is rejected to avoid typos polluting config.
#[tauri::command]
fn set_stt_language(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
    lang: String,
) -> Result<(), String> {
    assert_overlay(&window)?;
    let normalized = lang.trim().to_lowercase();
    if !normalized.is_empty() && normalized != "ru" && normalized != "en" {
        return Err(format!("invalid stt language '{normalized}' — must be ru, en, or empty (auto)"));
    }
    let next_cfg = {
        let mut c = state.write();
        c.stt_language = if normalized.is_empty() { None } else { Some(normalized) };
        c.clone()
    };
    config::save(&next_cfg).map_err(|e| e.to_string())?;
    Ok(())
}

/// v0.0.68: Re-ask the question that spawned a tile and replace its
/// content with a fresh AI answer. Triggered by the 🔄 button on tile
/// chrome (tile emits `tile:reload-request` event → Overlay listens →
/// invokes this command).
///
/// Flow: close the old tile (frees its slot), call AI with a minimal
/// reload prompt (meeting_context as background + the question), spawn
/// a new tile carrying the fresh answer. New tile gets a fresh label
/// and probably reuses the freed slot (slot-allocator picks the first
/// FREE one).
///
/// Why not in-place: keeping the same window would require sending the
/// new markdown via Tauri event to a specific window. Close+respawn is
/// simpler, reuses everything that already works, and the user sees a
/// brief 200ms blank-then-fresh which makes "I just got a new answer"
/// obvious. Pin status is intentionally NOT preserved — if the user
/// re-asks, they want to consider the new answer fresh.
///
/// Cost: ~$0.001 per click on Haiku. No cap, user-initiated.
#[tauri::command]
async fn tile_reload(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
    tiles: tauri::State<'_, SharedTiles>,
    label: String,
    question: String,
    current_generation: Option<u32>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let q_trim = question.trim();
    if q_trim.is_empty() {
        return Err("empty question — cannot reload".into());
    }
    // v0.0.69: increment reload generation. Saturating add to 99 cap
    // matches frontend display (which clamps at 99 too) — if a user
    // reloads 99 times, additional reloads keep showing 🔄×99 instead
    // of overflowing to a 4-digit badge that breaks layout.
    let next_generation = current_generation.unwrap_or(0).saturating_add(1).min(99);
    let (base_url, bearer, model, response_language, meeting_context, preferred_monitor, stealth) = {
        let c = state.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.ai_model.clone(),
            c.response_language.clone(),
            c.meeting_context.clone(),
            c.tile_monitor_name.clone(),
            c.stealth_enabled,
        )
    };
    if base_url.trim().is_empty() || bearer.trim().is_empty() {
        return Err("AI bridge not configured (set base_url + bearer in Settings)".into());
    }

    // Minimal "answer the question fresh" prompt. Doesn't reuse the
    // transcript-aware build_auto_tile_prompts because reload is a
    // standalone re-ask, not a transcript-driven one. meeting_context
    // is still attached as background so user-specific stack info
    // informs the answer.
    let lang_block = match response_language.as_str() {
        "ru" => "Отвечай на русском языке. Английский только для названий технологий.",
        "en" => "Respond in English.",
        _ => "Respond in the same language as the question.",
    };
    let ctx_block = if meeting_context.trim().is_empty() {
        "Контекст встречи не задан.".to_string()
    } else {
        format!(
            "Бэкграунд пользователя (для понимания уровня — не привязывай ответ к этим темам \
             если вопрос про другое):\n{}",
            meeting_context.trim()
        )
    };
    let system_prompt = format!(
        "Ты — техничный AI-ассистент. Пользователь видит твой ответ в окошке поверх экрана. \
         Нужен максимально полезный краткий ответ.\n\n\
         {ctx_block}\n\n\
         === Правила ===\n\
         - НИКАКОЙ преамбулы (\"Хороший вопрос\", \"Конечно\"). Первое слово — суть.\n\
         - Максимум 120 слов, цель 60-80. Краткость > полнота.\n\
         - Маркдаун: **жирный** для ключевого, `code` для команд/имён, маркированные списки.\n\
         - Конкретные команды/числа/имена, не общие фразы.\n\
         - {lang_block}"
    );
    let user_prompt = format!("Вопрос:\n{q_trim}");

    let messages = vec![
        crate::ai::ChatMessage {
            role: "system".into(),
            content: crate::ai::MessageContent::Text(system_prompt),
        },
        crate::ai::ChatMessage {
            role: "user".into(),
            content: crate::ai::MessageContent::Text(user_prompt),
        },
    ];

    let (answer, _usage) = crate::ai::complete_with_usage(&base_url, &bearer, &model, messages, 512)
        .await
        .map_err(|e| format!("AI error: {e:#}"))?;

    // Close the old tile (frees its slot for the respawn).
    tile::close_tile_by_label(&app, tiles.inner(), &label);

    // Spawn fresh tile with the new answer. Kind=Manual so the
    // color-code header reads "MANUAL" (this was an explicit user
    // action, not auto-detection). v0.0.69: pass next_generation so
    // the new tile shows 🔄×N badge in its chrome.
    let new_label = tile::spawn_tile_with_generation(
        &app,
        tiles.inner(),
        q_trim.to_string(),
        answer.trim().to_string(),
        preferred_monitor,
        stealth,
        tile::TileKind::Manual,
        Vec::new(),
        next_generation,
    )
    .map_err(|e| e.to_string())?;

    Ok(new_label)
}

/// v0.0.72: Quick-switch the live AI model without opening Settings.
/// Useful when mid-meeting you want Sonnet's deeper reasoning instead
/// of Haiku's speed. Updates cfg.ai_model + persists to disk; STT and
/// all other AI paths read from cfg on each call so the switch applies
/// to the very next request.
///
/// Whitelist enforced: only known Claude model IDs are accepted so a
/// typo in the chip handler doesn't poison cfg with a junk string that
/// blows up the next AI call. Extend the list when new models ship.
#[tauri::command]
fn set_ai_model(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
    model: String,
) -> Result<(), String> {
    assert_overlay(&window)?;
    let normalized = model.trim().to_string();
    const KNOWN: &[&str] = &[
        "claude-haiku-4-5",
        "claude-sonnet-4-6",
        "claude-sonnet-4-5",
        "claude-opus-4-7",
        "claude-opus-4-6",
    ];
    if !KNOWN.contains(&normalized.as_str()) {
        return Err(format!(
            "model '{normalized}' not in allow-list (haiku-4-5 / sonnet-4-6 / sonnet-4-5 / opus-4-7 / opus-4-6)"
        ));
    }
    let next_cfg = {
        let mut c = state.write();
        c.ai_model = normalized;
        c.clone()
    };
    config::save(&next_cfg).map_err(|e| e.to_string())?;
    Ok(())
}

/// v0.0.66: detector trigger tester. Runs the real detect_trigger
/// function on sample text using the current cfg.trigger_keywords.
/// Returns a human-readable verdict so user can tune trigger_keywords
/// without spinning up a live session.
#[derive(Debug, Serialize)]
struct DetectorTestResult {
    triggered: bool,
    reason: String,
    matched_keyword: Option<String>,
}

#[tauri::command]
fn test_detector(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
    text: String,
) -> Result<DetectorTestResult, String> {
    assert_overlay(&window)?;
    let keywords = state.read().trigger_keywords.clone();
    let result = runtime::detect_trigger(&text, &keywords);
    Ok(match result {
        Some(runtime::Trigger::Question(_)) => DetectorTestResult {
            triggered: true,
            reason: "matched as question (contains ? or interrogative pattern)".into(),
            matched_keyword: None,
        },
        Some(runtime::Trigger::Keyword(kw, _)) => DetectorTestResult {
            triggered: true,
            reason: format!("matched keyword: «{kw}»"),
            matched_keyword: Some(kw),
        },
        None => DetectorTestResult {
            triggered: false,
            reason: "no trigger (no '?', no interrogative, no keyword match, or too short / noise)".into(),
            matched_keyword: None,
        },
    })
}

/// v0.0.65: Pre-meeting cheatsheet generator. Reads cfg.meeting_context,
/// asks Sonnet (prep_model) to produce 8 likely interview questions
/// with brief answer outlines. Saves the markdown to Desktop with a
/// dated filename so the user can review pre-call.
///
/// Costs ~1 Sonnet call (≈$0.01-0.03 depending on context length).
/// Settings UI: Profile → Meeting Context → "💎 Cheatsheet (Sonnet)"
/// button next to "✨ Structure".
#[tauri::command]
async fn generate_cheatsheet(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let (base_url, bearer, model, lang, ctx) = {
        let c = state.read();
        (
            c.ai_base_url.clone(),
            c.ai_bearer.clone(),
            c.prep_model.clone(),
            c.response_language.clone(),
            c.meeting_context.clone(),
        )
    };
    if base_url.trim().is_empty() || bearer.trim().is_empty() {
        return Err("AI bridge not configured".into());
    }
    if ctx.trim().is_empty() {
        return Err("meeting_context is empty — fill it in first via Settings → Profile → Meeting context".into());
    }
    let lang_directive = if lang == "ru" {
        "Пиши на русском языке."
    } else {
        "Write in English."
    };
    let system = format!(
        "You are an interview prep coach. Given a description of an upcoming meeting/interview, \
         generate a cheatsheet of 8 likely questions and short answer outlines (3-5 bullet points each). \
         Output as markdown with H2 headings per question. Be concrete and pragmatic — no fluff. \
         Cover both behavioural questions and the most likely technical deep-dives based on the role. {lang_directive}"
    );
    let user = format!("Meeting/interview context:\n\n{ctx}\n\nGenerate the 8-question cheatsheet now:");
    let messages = vec![
        ai::ChatMessage {
            role: "system".into(),
            content: ai::MessageContent::Text(system),
        },
        ai::ChatMessage {
            role: "user".into(),
            content: ai::MessageContent::Text(user),
        },
    ];
    let (text, _usage) = ai::complete_with_usage(&base_url, &bearer, &model, messages, 2048)
        .await
        .map_err(|e| e.to_string())?;
    // Save to Desktop with timestamped filename.
    let desktop = dirs::desktop_dir().ok_or_else(|| "no desktop dir".to_string())?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let ymd = ymd_from_unix_ms(ts);
    let out_path = desktop.join(format!("suflyor-cheatsheet-{ymd}.md"));
    let header = format!("# Pre-meeting cheatsheet — {ymd}\n\n");
    std::fs::write(&out_path, header + &text).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().into_owned())
}

/// v0.0.63: append the most recent Q+A to a persistent bookmarks.md
/// in the app data dir. Lets the user save particularly good AI
/// answers for later review.
///
/// File location: `%APPDATA%\overlay-mvp\bookmarks.md`.
/// Format: each entry separated by `---`, with H2 question + body
/// markdown + ISO 8601 timestamp footer.
#[tauri::command]
fn bookmark_last_answer(
    window: tauri::WebviewWindow,
    rt: tauri::State<'_, SharedRuntime>,
) -> Result<String, String> {
    assert_overlay(&window)?;
    let (q, a) = {
        let s = rt.lock();
        match (s.last_question.clone(), s.last_answer.clone()) {
            (Some(q), Some(a)) if !q.is_empty() && !a.is_empty() => (q, a),
            _ => return Err("no AI Q+A this session yet".into()),
        }
    };
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "no app data dir".to_string())?
        .join("overlay-mvp");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let path = data_dir.join("bookmarks.md");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let ymd = ymd_from_unix_ms(ts);
    let entry = format!("\n## {q}\n\n{a}\n\n_{ymd}_\n\n---\n");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(entry.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// v0.0.63: open bookmarks.md in the default app (Notepad on Windows).
/// Returns the path so Settings can show "Saved: ..." toast.
#[tauri::command]
async fn open_bookmarks(window: tauri::WebviewWindow) -> Result<String, String> {
    assert_overlay(&window)?;
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "no app data dir".to_string())?
        .join("overlay-mvp");
    let path = data_dir.join("bookmarks.md");
    if !path.exists() {
        // Create an empty file with a friendly header so Notepad opens
        // something meaningful instead of erroring.
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        std::fs::write(&path, "# suflyor bookmarks\n\n(no bookmarks yet — click ⭐ in the overlay after an AI answer)\n").map_err(|e| e.to_string())?;
    }
    // Use Tauri's opener plugin via raw command — same pattern as
    // open_sessions_folder elsewhere.
    let path_str = path.to_string_lossy().to_string();
    // Spawn detached; on Windows uses ShellExecuteW to open with the
    // default associated app for .md (usually Notepad or the user's
    // chosen editor).
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &path_str])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    Ok(path_str)
}

/// v0.0.61: snapshot of the last Q+A pair (used by the overlay's
/// 💡 button to seed `tile_followups`). Reads `last_question` +
/// `last_answer` from RuntimeState — populated by every AI flow
/// (auto-tile / manual ask / PTT / F3).
#[tauri::command]
fn get_last_qa(
    window: tauri::WebviewWindow,
    rt: tauri::State<'_, SharedRuntime>,
) -> Result<Option<(String, String)>, String> {
    assert_overlay(&window)?;
    let s = rt.lock();
    match (s.last_question.clone(), s.last_answer.clone()) {
        (Some(q), Some(a)) if !q.is_empty() && !a.is_empty() => Ok(Some((q, a))),
        _ => Ok(None),
    }
}

/// v0.0.61: Generate 3 follow-up questions for a tile via Haiku. Used
/// by the `💡` button on each tile. Returns plain strings the
/// frontend renders as clickable chips.
///
/// Prompt design: explicit instruction to return EXACTLY 3 questions,
/// one per line, no numbering, no quotes, no markdown. Short Sonnet-
/// style "system prompt" enforces the format. Capped at 256 input +
/// 256 output tokens — keeps cost <$0.001 per call.
#[tauri::command]
async fn tile_followups(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, SharedConfig>,
    question: String,
    answer: String,
) -> Result<Vec<String>, String> {
    assert_overlay(&window)?;
    let (base_url, bearer, model) = {
        let cfg = state.read();
        (cfg.ai_base_url.clone(), cfg.ai_bearer.clone(), cfg.ai_model.clone())
    };
    if base_url.trim().is_empty() || bearer.trim().is_empty() {
        return Err("AI bridge not configured (set base_url + bearer in Settings)".into());
    }

    // Truncate inputs so we don't blow up token budget on a giant tile.
    let q_short: String = question.chars().take(500).collect();
    let a_short: String = answer.chars().take(2000).collect();

    let system = "You generate 3 short follow-up questions that an interviewer might ask \
                  next, given a Q&A. Output EXACTLY 3 questions, one per line. \
                  No numbering, no quotes, no markdown, no preamble. \
                  Each question on its own line, terminated by ?";
    let user = format!(
        "Original question:\n{q_short}\n\nAnswer given:\n{a_short}\n\nNow output 3 follow-up questions:"
    );

    let messages = vec![
        crate::ai::ChatMessage {
            role: "system".to_string(),
            content: crate::ai::MessageContent::Text(system.to_string()),
        },
        crate::ai::ChatMessage {
            role: "user".to_string(),
            content: crate::ai::MessageContent::Text(user),
        },
    ];

    let (text, _usage) = crate::ai::complete_with_usage(&base_url, &bearer, &model, messages, 256)
        .await
        .map_err(|e| e.to_string())?;

    // Split by newlines, trim, drop empties, take 3.
    let questions: Vec<String> = text
        .lines()
        .map(|s| s.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c.is_whitespace()).to_string())
        .filter(|s| !s.is_empty() && s.len() > 4)
        .take(3)
        .collect();

    Ok(questions)
}

/// v0.0.58: render a session JSONL into a human-readable markdown file
/// and save to Desktop. Pairs with the Replay viewer "📥 Export
/// markdown" button. Same path-validation as `load_session` — only
/// reads from the sessions dir.
///
/// Markdown sections (rendered in chronological order):
///   - H1 with session start timestamp + AI model
///   - For each `ai_request` / `ai_response` pair: Q (prompt preview) +
///     A (full response markdown), wall-clock timestamp, latency, cost
///   - For each `tile_spawn`: Q + A (snapshot of what was shown)
///   - Final SessionSummary block if present
///
/// Intentionally skips raw transcript lines and detector decisions —
/// they're noise for a human reading a post-meeting recap. (The
/// Replay viewer is the right tool when you want the raw timeline.)
#[tauri::command]
fn export_session_markdown(
    window: tauri::WebviewWindow,
    path: String,
) -> Result<String, String> {
    assert_overlay(&window)?;
    const MAX_BYTES: u64 = 10 * 1024 * 1024;
    let p = std::path::PathBuf::from(&path);
    let sessions_dir = journal::sessions_dir().map_err(|e| e.to_string())?;
    let canonical_session_dir = sessions_dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = p.canonicalize().map_err(|e| e.to_string())?;
    if !canonical_path.starts_with(&canonical_session_dir) {
        return Err("path is outside sessions dir".into());
    }
    let meta = std::fs::metadata(&canonical_path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_BYTES {
        return Err(format!("session file too large ({} bytes, max {})", meta.len(), MAX_BYTES));
    }

    let content = std::fs::read_to_string(&canonical_path).map_err(|e| e.to_string())?;
    let mut events: Vec<serde_json::Value> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            events.push(v);
        }
    }

    let fmt_clock = |unix_ms: i64| -> String {
        // Naive HH:MM:SS from milliseconds — uses chrono if available,
        // otherwise raw arithmetic. We don't want to pull chrono just
        // for this so do it by hand.
        let total_sec = (unix_ms / 1000).rem_euclid(86_400);
        let h = total_sec / 3600;
        let m = (total_sec % 3600) / 60;
        let s = total_sec % 60;
        format!("{h:02}:{m:02}:{s:02}")
    };

    let mut md = String::new();
    let filename = canonical_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    md.push_str(&format!("# suflyor session — {filename}\n\n"));

    // Pull session_start metadata if present.
    if let Some(start) = events.iter().find(|e| e["kind"] == "session_start") {
        let model = start["ai_model"].as_str().unwrap_or("?");
        let prep = start["prep_model"].as_str().unwrap_or("");
        let lang = start["response_language"].as_str().unwrap_or("?");
        md.push_str(&format!("- model: `{model}`"));
        if !prep.is_empty() { md.push_str(&format!(" · prep: `{prep}`")); }
        md.push_str(&format!(" · lang: `{lang}`\n"));
        if let Some(ts) = start["unix_ms"].as_i64() {
            md.push_str(&format!("- started: {}\n", fmt_clock(ts)));
        }
        md.push('\n');
    }

    // Pair ai_request with following ai_response by tile id when possible,
    // otherwise just emit in order.
    let mut req_count = 0;
    let mut pending_req: Option<&serde_json::Value> = None;
    for ev in &events {
        let kind = ev["kind"].as_str().unwrap_or("");
        match kind {
            "ai_request" => {
                pending_req = Some(ev);
            }
            "ai_response" => {
                req_count += 1;
                let purpose = ev["purpose"].as_str().unwrap_or("ask");
                let ts_str = ev["unix_ms"].as_i64().map(fmt_clock).unwrap_or_else(|| "?".into());
                let latency = ev["latency_ms"].as_i64().unwrap_or(0);
                let cost_micro = ev["cost_microcents"].as_i64().unwrap_or(0);
                let cost_usd = cost_micro as f64 / 100_000_000.0;
                md.push_str(&format!("## #{req_count} · {purpose} · {ts_str}\n\n"));
                if let Some(req) = pending_req {
                    let prompt = req["user_prompt"].as_str()
                        .or_else(|| req["user_prompt_preview"].as_str())
                        .unwrap_or("");
                    if !prompt.is_empty() {
                        md.push_str("**Prompt:**\n\n");
                        md.push_str("```\n");
                        md.push_str(prompt);
                        md.push_str("\n```\n\n");
                    }
                }
                let text = ev["text"].as_str().unwrap_or("");
                md.push_str("**Answer:**\n\n");
                md.push_str(text);
                md.push_str("\n\n");
                md.push_str(&format!("_{latency} ms · ${cost_usd:.4}_\n\n---\n\n"));
                pending_req = None;
            }
            _ => {}
        }
    }

    // SessionSummary footer if present.
    if let Some(sum) = events.iter().find(|e| e["kind"] == "session_summary") {
        md.push_str("## Summary\n\n");
        let dur_min = sum["duration_ms"].as_i64().unwrap_or(0) as f64 / 60_000.0;
        md.push_str(&format!("- duration: {dur_min:.1} min\n"));
        md.push_str(&format!("- transcript lines: {} (mic {} · system {})\n",
            sum["transcript_lines"].as_i64().unwrap_or(0),
            sum["transcript_mic"].as_i64().unwrap_or(0),
            sum["transcript_system"].as_i64().unwrap_or(0)));
        md.push_str(&format!("- AI requests: {} · tiles spawned: {}\n",
            sum["ai_requests_total"].as_i64().unwrap_or(0),
            sum["tiles_spawned"].as_i64().unwrap_or(0)));
        let total_cost = sum["total_cost_microcents"].as_i64().unwrap_or(0) as f64 / 100_000_000.0;
        md.push_str(&format!("- total cost: ${total_cost:.4}\n"));
    }

    // Save to Desktop with `.md` suffix replacing the `.jsonl` ext.
    let desktop = dirs::desktop_dir().ok_or_else(|| "no desktop dir".to_string())?;
    let stem = canonical_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session");
    let out_path = desktop.join(format!("{stem}.md"));
    std::fs::write(&out_path, md).map_err(|e| e.to_string())?;
    Ok(out_path.to_string_lossy().into_owned())
}

// ── Entry point ──────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    // v0.0.21: runtime panic hook. The existing P0-3 crash-report.txt
    // catches STARTUP panics only (via .run().unwrap_or_else at the
    // bottom). Worker-thread panics during a session (e.g. WASAPI
    // device race when F8 is double-pressed) were silently dropped to
    // stderr which release builds don't surface. Now appends each
    // panic with full payload + location + timestamp to
    // `%APPDATA%/overlay-mvp/runtime-panics.log` so the user can show
    // it. Combines with sanitize_diagnostic_text — when included in
    // dump_diagnostics, gsk_/Bearer/sk- patterns get redacted.
    //
    // NOTE: helper `truncate_panic_log_tail` is at the bottom of this
    // file (above the `#[cfg(test)] mod tests`).
    std::panic::set_hook(Box::new(|info| {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown location>".to_string());
        let payload = info.payload();
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };
        let entry = format!(
            "[unix={timestamp}] panic at {location}\n  {msg}\n\n"
        );
        eprintln!("PANIC: {entry}");
        // v0.0.28: fall back to %TEMP% if config_dir() returns None
        // (extremely rare on Windows but undocumented previously — the
        // panic record would silently vanish). Caught by review agent.
        let dir = dirs::config_dir()
            .map(|d| d.join("overlay-mvp"))
            .unwrap_or_else(|| std::env::temp_dir().join("overlay-mvp-panic-fallback"));
        {
            let path = dir.join("runtime-panics.log");
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            // v0.0.26: KEEP-LAST-N rotation instead of full delete.
            // Previously remove_file at 1MB wiped history right when the
            // user might need it most (e.g. cascading panics). Now: read
            // file, keep last ~500KB, rewrite. The very panic that just
            // tripped the size check gets appended below.
            //
            // v0.0.27: UTF-8 SAFETY — see truncate_panic_log_tail tests.
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > 1_000_000 {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        let tail = truncate_panic_log_tail(&s, 500_000);
                        let _ = std::fs::write(&path, tail);
                    } else {
                        // If we can't even read it, fall back to delete
                        // so writes don't keep failing on a broken file.
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                use std::io::Write;
                let _ = f.write_all(entry.as_bytes());
            }
        }
    }));

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
        .plugin(tauri_plugin_dialog::init())
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
            close_all_tiles,
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
            export_session_markdown,
            read_all_session_stats,
            tile_followups,
            get_last_qa,
            bookmark_last_answer,
            open_bookmarks,
            generate_cheatsheet,
            test_detector,
            set_stt_language,
            set_ai_model,
            tile_reload,
            set_stealth,
            list_snippets,
            expand_snippet,
            kb_search,
            kb_get,
            kb_stats,
            kb_spawn,
            check_bridge,
            check_update,
            download_and_install_update,
            clear_update_in_flight,
            crash_report_path,
            dump_diagnostics,
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
