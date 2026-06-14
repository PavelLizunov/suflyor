//! User config persisted as JSON in OS data dir.
//! Path: %APPDATA%\overlay-mvp\config.json

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Current on-disk config schema version. Bumped only on a BREAKING layout
/// change (a field renamed/removed/re-typed in a way serde defaults can't
/// paper over). [`load`] stamps any older/unstamped file up to this number and
/// is the single place a number-keyed migration would run. Additive fields do
/// NOT need a bump — `#[serde(default)]` already loads old files.
const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// On-disk schema version (see [`CURRENT_CONFIG_VERSION`]). 0 = a file
    /// written before versioning existed; [`load`] stamps it to the current
    /// number on first read. Serialized first so it's visible at the top of
    /// config.json.
    #[serde(default)]
    pub config_version: u32,

    /// Pre-meeting context (system prompt prefix), free-form.
    /// e.g. "Это собеседование на Senior SRE position. Мой опыт: 7 лет K8s..."
    pub meeting_context: String,

    /// Named profiles for meeting_context (swap quickly).
    pub context_profiles: Vec<ContextProfile>,

    /// Active profile name (matches one of context_profiles[].name), or None.
    pub active_profile: Option<String>,

    /// Audio device names (exact match against WASAPI enumeration).
    pub mic_device: Option<String>, // e.g. "Headset Microphone (A50 Mic)"
    pub system_audio_device: Option<String>, // e.g. "Line (A50 Stream Out)"

    /// AI proxy (OpenAI-compatible) — your Linux bridge.
    pub ai_base_url: String, // e.g. "http://192.168.0.142:18902/v1"
    pub ai_bearer: String,  // BRIDGE_SECRET
    pub ai_model: String,   // Live answers — fast, default claude-haiku-4-5
    pub prep_model: String, // Pre-meeting context structuring — smart, default claude-sonnet-4-5

    /// EXPERIMENTAL — when true, the system prompt is sent with Anthropic
    /// `cache_control: ephemeral` so a pass-through bridge can prompt-cache
    /// it (faster repeat/follow-up asks). Default OFF: some OpenAI-compat
    /// bridges reject the unknown field, so enable + test against YOUR
    /// bridge. `#[serde(default)]` keeps old config.json files loading.
    #[serde(default)]
    pub ai_prompt_cache: bool,

    /// AI provider for live answers: "cloud" (default — the bridge above) or
    /// "local" (an OpenAI-compatible local server like Ollama / llama-server).
    /// Lets a local model fully REPLACE Claude. (Auto-fallback is a later
    /// phase.) `#[serde(default)]` → old configs default to "cloud".
    #[serde(default = "default_ai_provider")]
    pub ai_provider: String,
    /// Local server base URL (OpenAI-compatible). Default is llama.cpp's
    /// "http://127.0.0.1:8080/v1" (the shipped setup pipeline); Ollama uses
    /// "http://127.0.0.1:11434/v1".
    #[serde(default = "default_ai_local_base_url")]
    pub ai_local_base_url: String,
    /// Bearer for the local server (Ollama ignores it; some servers want one).
    #[serde(default)]
    pub ai_local_bearer: String,
    /// Local model id for live answers, e.g. "qwen2.5:7b-instruct". Empty
    /// until the user sets it.
    #[serde(default)]
    pub ai_local_model: String,
    /// Local model for prep/structuring; empty → falls back to ai_local_model.
    #[serde(default)]
    pub ai_local_prep_model: String,
    /// True if the local model can accept screenshots (a vision model). When
    /// false, screenshots are dropped for local asks (a text model would error).
    #[serde(default)]
    pub ai_local_vision: bool,
    /// When the LOCAL AI model is a hybrid "thinking" model (e.g. Gemma 4
    /// E4B), `false` (default) makes us send `chat_template_kwargs.enable_thinking
    /// = false` so it answers directly instead of emitting long hidden
    /// reasoning (≈5× faster for interview answers). Set `true` to allow the
    /// model to think. Only affects the LOCAL provider.
    #[serde(default)]
    pub ai_local_thinking: bool,
    /// Local LLM size preference: `false` (default) = the fast ~4B model
    /// (Gemma 4 E4B), `true` = the smarter but ~2× slower 12B (Gemma 4 12B
    /// QAT). The local server loads ONE GGUF; flipping this restarts
    /// llama-server with the other file. `true` is honoured only when the 12B
    /// GGUF is actually present on disk (downloaded on demand from Settings),
    /// else we transparently fall back to the 4B so the server always starts.
    #[serde(default)]
    pub ai_local_quality: bool,

    /// Screenshot/vision channel — resolved INDEPENDENTLY of the text AI (via
    /// [`Config::vision_endpoint`]) so a local text model can keep answering
    /// while screenshots go to a vision-capable model. Provider: "off"
    /// (disabled), "same" (reuse the text endpoint), "cloud" (the `vision_*`
    /// bridge fields), or "local" (a 2nd local vision server). Default "cloud"
    /// → F8 capture works out of the box through the configured bridge with a
    /// Sonnet vision model.
    #[serde(default = "default_vision_provider")]
    pub vision_provider: String,
    /// Cloud vision endpoint. Empty `base_url`/`bearer` fall back to the text
    /// bridge (`ai_base_url`/`ai_bearer`); empty `model` falls back to Sonnet
    /// (`DEFAULT_VISION_MODEL`).
    #[serde(default)]
    pub vision_base_url: String,
    #[serde(default)]
    pub vision_bearer: String,
    #[serde(default)]
    pub vision_model: String,
    /// Local vision endpoint — a 2nd OpenAI-compatible server running a vision
    /// model (e.g. Qwen2-VL on another port) while text stays on the primary
    /// local model. Empty fields fall back to the text-local ones (`ai_local_*`).
    #[serde(default)]
    pub vision_local_base_url: String,
    #[serde(default)]
    pub vision_local_bearer: String,
    #[serde(default)]
    pub vision_local_model: String,

    /// Feature #4 — append an IPA transcription to each non-trivial word in the
    /// F8 TRANSLATE-mode output (e.g. schedule [ˈʃedjuːl]). OFF by default: it's a
    /// power feature and keeps short subtitles clean. Only affects translate-mode
    /// captures, never the normal describe prompt.
    #[serde(default)]
    pub vision_phonetics: bool,

    /// Test Practice mode for the F8 capture (v0.11.0). When ON, plain F8 (NOT
    /// Shift+F8, which stays translate) treats the screenshot as a PRACTICE quiz
    /// question and returns the answer + a short explanation, for study /
    /// self-check. OFF by default = F8 keeps the normal Describe behaviour. A
    /// deliberate, persisted, non-hidden opt-in (it is NOT for graded exams).
    #[serde(default)]
    pub vision_test_practice: bool,

    /// Language tag (ISO 639-1) the assistant should ALWAYS respond in.
    /// Injected into the system prompt at runtime.
    pub response_language: String, // e.g. "ru"

    /// Groq Whisper STT.
    pub groq_api_key: String,
    pub stt_language: Option<String>, // None = auto-detect, "ru" = forced Russian
    /// Groq Whisper model: "whisper-large-v3" (most accurate, slower) vs
    /// "whisper-large-v3-turbo" (~3× faster, slightly less accurate).
    /// Default: large-v3 — accuracy beats latency for interview use.
    pub stt_model: String,

    /// STT provider: "cloud" (default — Groq Whisper), "gigaam" (local
    /// in-process GigaAM-v3 via ONNX — Russian-specialised, runs on CPU), or
    /// "whisper" (local whisper.cpp server, OpenAI-compatible — multilingual,
    /// best for mixed RU+EN). `#[serde(default)]` → old configs stay "cloud".
    #[serde(default = "default_stt_provider")]
    pub stt_provider: String,
    /// Directory holding the local GigaAM model (`model.int8.onnx` + `vocab.txt`).
    /// Used when `stt_provider == "gigaam"`. Empty until the user sets it.
    #[serde(default)]
    pub stt_gigaam_dir: String,
    /// Run the local GigaAM model on the GPU via the ONNX Runtime DirectML
    /// execution provider (Windows, vendor-agnostic DX12). Falls back to CPU
    /// automatically if no compatible GPU / DirectML runtime is present.
    /// ~7x faster on long audio; ~1s one-time shader-compile on first use.
    #[serde(default = "default_stt_gigaam_gpu")]
    pub stt_gigaam_gpu: bool,
    /// Local whisper.cpp server base URL (OpenAI-compatible), e.g.
    /// "http://127.0.0.1:8081/v1". Used when `stt_provider == "whisper"`.
    #[serde(default = "default_stt_whisper_url")]
    pub stt_whisper_url: String,
    /// Bearer for the local whisper server (usually empty; whisper.cpp ignores it).
    #[serde(default)]
    pub stt_whisper_bearer: String,
    /// Model id sent to the local whisper server. whisper.cpp serves the loaded
    /// model regardless of this, but OpenAI-compat clients require the field.
    #[serde(default = "default_stt_whisper_model")]
    pub stt_whisper_model: String,

    /// Preferred monitor name for tile windows. None = first non-primary, fallback to primary.
    pub tile_monitor_name: Option<String>,

    /// Whitespace-separated trigger keywords for auto-tile spawn (case-insensitive).
    /// Example: "kubernetes etcd terraform postgres". Plus any "?" sentence.
    pub trigger_keywords: String,

    /// Enable auto-detect of questions/keywords in transcript → spawn tiles.
    pub auto_tiles_enabled: bool,

    /// When true, overlay + tile windows call set_content_protected(true) and
    /// become invisible to screen-share / capture APIs. OFF by default for
    /// easier debugging and use cases where stealth is not needed.
    pub stealth_enabled: bool,

    /// When true, on session stop the full mic transcript is sent to the
    /// prep_model (Sonnet by default) for a 3-point coaching debrief, which
    /// spawns as a Manual tile. Costs ~1 Sonnet call per session. Skipped
    /// when the session was shorter than 30s or had fewer than 5 mic lines.
    ///
    /// Default is OFF (opt-in via Settings). A privacy/cost-conscious tool
    /// shouldn't silently start spending money on Sonnet just because the
    /// user upgraded.
    #[serde(default = "default_post_meeting_debrief_enabled")]
    pub post_meeting_debrief_enabled: bool,

    /// v0.13.0 — record the raw session audio (mic + system, separate 16 kHz
    /// mono WAVs) under `%APPDATA%\overlay-mvp\recordings\<session_id>\`. Kept
    /// locally; nothing is uploaded. Enables a future "re-transcribe + re-summary
    /// from the archive" flow (the live STT transcript is real-time-bounded;
    /// re-running offline over the saved audio yields a better transcript).
    /// Default ON — the recordings power the "re-summary from the archive"
    /// flow, so they should accumulate by default. The Settings → Audio toggle
    /// and the retention bound let a user turn it off or cap disk use.
    /// See `default_record_audio_enabled`.
    #[serde(default = "default_record_audio_enabled")]
    pub record_audio_enabled: bool,

    /// v0.13.0 — how many of the most-recent recorded sessions to keep on disk
    /// (older `recordings\<id>\` dirs are pruned at session start). ~230 MB/hr
    /// for both channels, so a bound matters. 0 = keep everything (unbounded).
    #[serde(default = "default_record_retention_sessions")]
    pub record_retention_sessions: u32,

    /// v0.15.0 — age-based retention for recorded audio: `recordings\<id>\`
    /// dirs older than this many days are pruned at session start. 0 = no age
    /// limit (default — old configs behave exactly as before). Combines with
    /// `record_retention_sessions`: when both are non-zero BOTH bounds apply.
    #[serde(default)]
    pub record_retention_days: u32,

    /// v0.15.0 — how many session JOURNALS (`sessions\*.jsonl` — the
    /// transcripts + AI turns behind the 🗄 archive) to keep on disk. Was a
    /// hard-coded 100, which a 10-calls-a-day user exhausts in two weeks.
    /// 0 = unlimited. Pruned at the NEXT session start.
    #[serde(default = "default_journal_retention_sessions")]
    pub journal_retention_sessions: u32,

    /// v0.15.0 — total disk budget for session journals, in MB (was a
    /// hard-coded 500). After the count prune, oldest journals are deleted
    /// until under this budget. 0 = unlimited.
    #[serde(default = "default_journal_max_total_mb")]
    pub journal_max_total_mb: u32,

    /// fs-audit #5 — total disk budget for raw audio recordings, in MB. After
    /// the count/age prunes, oldest `recordings\<id>\` dirs are deleted until the
    /// recordings tree is under this budget. Applies REGARDLESS of the count/age
    /// policy (mirrors `journal_max_total_mb`, which caps journals even in
    /// "keep all" mode), so even the "all (no limit)" retention choice can't fill
    /// the disk. 0 = truly unlimited. Default 20 GB (~87 h of dual-channel audio)
    /// — a safety backstop a normal user never reaches.
    #[serde(default = "default_record_max_total_mb")]
    pub record_max_total_mb: u32,

    /// P2 — index finished JSONL sessions into the local SQLite archive
    /// (searchable interview history). ON by default; the JSONL journals stay the
    /// source of truth either way, and the catalog can be deleted + rebuilt.
    /// Disabling stops the startup indexing. Old configs without this key default
    /// ON.
    #[serde(default = "default_session_archive_enabled")]
    pub session_archive_enabled: bool,

    /// v0.0.73: when true, `quit_app` exports the most recent session's
    /// JSONL journal to a Markdown file on the user's Desktop right
    /// before exiting. Filename: `suflyor-session-YYYY-MM-DD-HHmm.md`.
    /// Same rendering as the Replay viewer's "📥 Export markdown" button
    /// — only Q+A pairs + final summary, no raw transcript clutter.
    ///
    /// Default OFF (opt-in). Users who want every session captured
    /// without thinking enable it once and forget. Failure to write is
    /// logged but never blocks the quit (avoids "I want to leave but
    /// the app won't let me" UX nightmares).
    #[serde(default)]
    pub auto_export_on_quit: bool,

    /// Soft budget hint per session, in USD. When session cost crosses
    /// this number, a yellow "💰 over $X budget" chip appears in the
    /// overlay — but AI calls still go through. Blocking mid-interview
    /// would be terrible UX (you can't get help precisely when you need
    /// it). The rate-limit (15 auto-tiles/min) already prevents real
    /// runaway-spend scenarios.
    ///
    /// Set to 0 to disable the warning entirely.
    /// Default 1.00 USD ≈ 200 Haiku tile spawns. Counter resets on
    /// start_session.
    ///
    /// Live regression 2026-05-25: original v0.0.2 design was a HARD
    /// block, which user rightfully called "странное решение" — pivoted
    /// to soft warning in v0.0.5.
    #[serde(default = "default_max_session_cost_usd")]
    pub max_session_cost_usd: f64,

    /// When true, the auto-tile detector ignores transcript lines that
    /// came from the MICROPHONE (your own voice). Only system-audio lines
    /// (interviewer questions) can trigger an auto-tile. Live regression
    /// 2026-05-25: detector kept firing on the candidate's own statements
    /// ("Я работал с Kubernetes …") and spawned redundant explanation tiles.
    ///
    /// Default ON — interview use-case is "they ask, I answer; AI helps
    /// the answer." If you want both sides considered, turn this off.
    #[serde(default = "default_detector_skip_mic")]
    pub detector_skip_mic: bool,

    /// **AGGRESSIVE MODE** (v0.0.18). When true, `maybe_spawn_tile` skips
    /// the question/keyword detector entirely and treats EVERY transcript
    /// line as a trigger. Combined with `detector_skip_mic=false` this
    /// effectively spawns a tile for every audio chunk Whisper produces.
    ///
    /// Use cases:
    ///   - You're paying for AI and want maximum coverage regardless of
    ///     whether the line "sounds like a question"
    ///   - You're testing the pipeline end-to-end
    ///   - Whisper is dropping `?` and the candidate's monologue is what
    ///     you actually want suggestions on
    ///
    /// Trade-off: cost. With this on, expect 30-50 tiles per minute of
    /// continuous speech, each = one Haiku call. Soft cost cap chip still
    /// fires but doesn't block. Also bumps internal MAX_TILES_PER_MIN from
    /// 15 to 60 so the rate-limiter doesn't strangle aggressive mode.
    ///
    /// Default OFF — out of the box behaviour stays the same.
    #[serde(default)]
    pub auto_tile_every_line: bool,

    // NOTE: the legacy `hotkey_*` (F9/F10/F11/F12) and `manual_ask_mode` and
    // `custom_css` fields were REMOVED (P1.3). They were dead config that never
    // matched runtime behaviour — the app uses FIXED hotkeys (F1/F3/F4/F6/F8/F9/
    // Shift+F8/Shift+F9), push-to-talk is hard-wired, and there is no CSS surface
    // in the Slint build. `#[serde(default)]` on the struct means old config.json
    // files carrying these keys still load fine (serde ignores the unknown keys).
    /// UI language for Settings + Overlay + Tile chrome strings (NOT
    /// AI response language — that's `response_language` above). v0.0.42.
    /// Supported: "ru" (default, current primary), "en". Anything else
    /// falls back to "ru" at the t() lookup level.
    ///
    /// Stored in config.json. Loaded once per window mount; switching
    /// re-renders via React state. Tray menu remains Russian (Rust-side
    /// menu builder doesn't observe this field — separate concern).
    #[serde(default = "default_ui_language")]
    pub ui_language: String,

    /// Active colour scheme for the Slint design tokens (`theme.slint`).
    /// 0 = Glacier (default, cool graphite + blue accent), 1 = Graphite
    /// (warm charcoal + teal), 2 = Obsidian (blue-black + violet), 3 =
    /// Light Frost (daytime light mode). Out-of-range values clamp to 0
    /// at the `Theme.scheme` set site. `#[serde(default)]` → old configs
    /// land on Glacier. v0.1.2.
    #[serde(default)]
    pub color_scheme: i32,

    /// Tile body font size in px. Default 12. Reasonable range 11-18.
    /// Stored here (not localStorage) because tile windows can't read
    /// localStorage from the overlay window — has to be passed via
    /// URL param (`&fs=14`) at spawn time. v0.0.55.
    #[serde(default = "default_tile_font_size")]
    pub tile_font_size: u32,

    /// Pre-written answer snippets. Each snippet has a short trigger key
    /// (e.g. "k8s", "pg") that the user can invoke via the palette to
    /// instantly spawn a tile with the body text — zero AI latency,
    /// zero cost. Great for the 5-6 "give me the template" questions
    /// that come up every interview (incident-response framework, SLI
    /// design, postgres tuning checklist, etc.).
    pub snippets: Vec<Snippet>,

    /// Phase E6 v20 — tile body opacity (0.5..1.0). Lets the user see
    /// THROUGH tiles to the meeting window underneath. Default 1.0 =
    /// opaque (current behaviour). Cherry-picked from the design
    /// bundle 2 `body-opacity` prop; only this one design change was
    /// adopted — see cycle 26 chat thread for risk-analysis rationale.
    #[serde(default = "default_tile_body_opacity")]
    pub tile_body_opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snippet {
    /// Short trigger key, case-insensitive (e.g. "k8s", "pg-tune").
    pub key: String,
    /// Human-readable title shown as the tile's question text.
    pub title: String,
    /// Body — full markdown rendered in the tile.
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProfile {
    pub name: String,
    pub context: String,
}

/// Name given to the auto-migrated profile seeded from a pre-profiles
/// `meeting_context` (see `load`).
pub const DEFAULT_PROFILE_NAME: &str = "Основной";

impl Config {
    /// Index of the active profile within `context_profiles`, if one is set and
    /// still present.
    #[must_use]
    pub fn active_profile_index(&self) -> Option<usize> {
        let name = self.active_profile.as_deref()?;
        self.context_profiles.iter().position(|p| p.name == name)
    }

    /// Select a profile by index: make it active and load its context into the
    /// live `meeting_context`. No-op if `idx` is out of range.
    pub fn select_profile(&mut self, idx: usize) {
        if let Some(p) = self.context_profiles.get(idx) {
            self.active_profile = Some(p.name.clone());
            self.meeting_context = p.context.clone();
        }
    }

    /// Add a new BLANK profile and make it active, clearing the live
    /// `meeting_context` so the editor starts empty. Rejects a blank or
    /// duplicate name; returns the new index on success.
    ///
    /// NOTE: a new profile deliberately does NOT inherit the previously-active
    /// profile's context. Before, it cloned `self.meeting_context` (which held
    /// the active profile's text), so creating "FOOT" while "ninitux" was
    /// active silently copied ninitux's description into FOOT — surprising and
    /// reported as a bug. Clearing `meeting_context` also keeps it in sync with
    /// the new (empty) active profile, so the next AI call doesn't use the old
    /// profile's context while a fresh profile is "active".
    pub fn add_profile(&mut self, name: &str) -> Option<usize> {
        let name = name.trim();
        if name.is_empty() || self.context_profiles.iter().any(|p| p.name == name) {
            return None;
        }
        self.context_profiles.push(ContextProfile {
            name: name.to_string(),
            context: String::new(),
        });
        self.active_profile = Some(name.to_string());
        self.meeting_context = String::new();
        Some(self.context_profiles.len() - 1)
    }

    /// Rename the active profile. Rejects a blank or duplicate name; returns
    /// whether a rename happened.
    pub fn rename_active_profile(&mut self, new_name: &str) -> bool {
        let new_name = new_name.trim();
        if new_name.is_empty() || self.context_profiles.iter().any(|p| p.name == new_name) {
            return false;
        }
        if let Some(idx) = self.active_profile_index() {
            self.context_profiles[idx].name = new_name.to_string();
            self.active_profile = Some(new_name.to_string());
            return true;
        }
        false
    }

    /// Delete the active profile. The profile that slides into its slot (or the
    /// first remaining one) becomes active and loads into `meeting_context`; if
    /// none remain, the active selection clears (live context is left as-is).
    pub fn delete_active_profile(&mut self) {
        if let Some(idx) = self.active_profile_index() {
            self.context_profiles.remove(idx);
            let next = self
                .context_profiles
                .get(idx)
                .or_else(|| self.context_profiles.first())
                .map(|p| (p.name.clone(), p.context.clone()));
            match next {
                Some((name, ctx)) => {
                    self.active_profile = Some(name);
                    self.meeting_context = ctx;
                }
                None => self.active_profile = None,
            }
        }
    }

    /// Persist edited context into the live `meeting_context` AND, if a profile
    /// is active, into that profile so the picker and the live field never drift.
    pub fn save_active_context(&mut self, text: &str) {
        self.meeting_context = text.to_string();
        if let Some(idx) = self.active_profile_index() {
            self.context_profiles[idx].context = text.to_string();
        }
    }
}

impl Config {
    pub fn defaults() -> Self {
        Self {
            config_version: CURRENT_CONFIG_VERSION,
            meeting_context: String::new(),
            context_profiles: vec![],
            active_profile: None,
            mic_device: None,
            system_audio_device: None,
            ai_base_url: "http://192.168.0.142:18902/v1".into(),
            ai_bearer: String::new(),
            ai_model: "claude-haiku-4-5".into(),
            prep_model: "claude-sonnet-4-6".into(),
            ai_prompt_cache: false,
            ai_provider: default_ai_provider(),
            ai_local_base_url: default_ai_local_base_url(),
            ai_local_bearer: String::new(),
            ai_local_model: String::new(),
            ai_local_prep_model: String::new(),
            ai_local_vision: false,
            ai_local_thinking: false,
            ai_local_quality: false,
            vision_provider: default_vision_provider(),
            vision_base_url: String::new(),
            vision_bearer: String::new(),
            vision_model: String::new(),
            vision_local_base_url: String::new(),
            vision_local_bearer: String::new(),
            vision_local_model: String::new(),
            vision_phonetics: false,
            vision_test_practice: false,
            response_language: "ru".into(),
            groq_api_key: String::new(),
            stt_language: Some("ru".into()),
            stt_model: "whisper-large-v3".into(),
            stt_provider: default_stt_provider(),
            stt_gigaam_dir: String::new(),
            stt_gigaam_gpu: default_stt_gigaam_gpu(),
            stt_whisper_url: default_stt_whisper_url(),
            stt_whisper_bearer: String::new(),
            stt_whisper_model: default_stt_whisper_model(),
            tile_monitor_name: None,
            stealth_enabled: false, // OFF by default — easier to debug & not every use case needs stealth
            trigger_keywords: default_trigger_keywords(),
            auto_tiles_enabled: true,
            ui_language: default_ui_language(),
            color_scheme: 0,
            tile_font_size: default_tile_font_size(),
            snippets: default_snippets(),
            post_meeting_debrief_enabled: default_post_meeting_debrief_enabled(),
            record_audio_enabled: default_record_audio_enabled(),
            record_retention_sessions: default_record_retention_sessions(),
            record_retention_days: 0,
            journal_retention_sessions: default_journal_retention_sessions(),
            journal_max_total_mb: default_journal_max_total_mb(),
            record_max_total_mb: default_record_max_total_mb(),
            session_archive_enabled: default_session_archive_enabled(),
            auto_export_on_quit: false,
            max_session_cost_usd: default_max_session_cost_usd(),
            detector_skip_mic: default_detector_skip_mic(),
            auto_tile_every_line: false,
            tile_body_opacity: default_tile_body_opacity(),
        }
    }
}

/// Resolved AI endpoint for a single call (live answer or prep/structuring).
#[derive(Debug, Clone)]
pub struct AiEndpoint {
    pub base_url: String,
    pub bearer: String,
    pub model: String,
    /// True when the LOCAL provider is active. Callers zero out cost and
    /// gate screenshots (text-only local models) on this flag.
    pub is_local: bool,
}

/// Default cloud vision model when `vision_model` is unset — Sonnet (strong
/// vision + present in the ai.rs pricing table).
pub const DEFAULT_VISION_MODEL: &str = "claude-sonnet-4-6";

/// One subsystem's config-only readiness for the diagnostics panel (#131).
/// `detail` carries only NEUTRAL values (provider tag, URL, model, device) —
/// never a bearer / API-key value.
#[derive(Debug, Clone)]
pub struct ReadinessItem {
    /// True if the active config for this subsystem is complete enough to use.
    pub configured: bool,
    /// Short neutral detail (e.g. "local · http://… · gemma-4"); may be empty.
    pub detail: String,
}

/// Config-only (no network / no audio) readiness snapshot for the diagnostics
/// panel. Built by [`Config::readiness`]; the live AI/STT pings are layered on
/// top by the UI via the existing test handlers.
#[derive(Debug, Clone)]
pub struct ReadinessReport {
    pub ai: ReadinessItem,
    pub stt: ReadinessItem,
    pub mic: ReadinessItem,
    pub sys: ReadinessItem,
    /// Separate vision channel (F8 screenshots). `configured` is false when the
    /// provider is "off" (feature intentionally disabled) — the UI renders that
    /// as a neutral "off", not an error.
    pub vision: ReadinessItem,
    pub stealth_on: bool,
}

impl Config {
    /// Resolve which AI endpoint to use. `prep=true` selects the structuring
    /// model, else the live-answer model. Provider `"local"` uses the
    /// `ai_local_*` fields (a local prep model falls back to the local live
    /// model); anything else (default `"cloud"`) uses the bridge fields — so
    /// existing configs behave exactly as before.
    #[must_use]
    pub fn ai_endpoint(&self, prep: bool) -> AiEndpoint {
        if self.ai_provider == "local" {
            let model = if prep && !self.ai_local_prep_model.trim().is_empty() {
                self.ai_local_prep_model.clone()
            } else {
                self.ai_local_model.clone()
            };
            AiEndpoint {
                base_url: self.ai_local_base_url.clone(),
                bearer: self.ai_local_bearer.clone(),
                model,
                is_local: true,
            }
        } else {
            let model = if prep {
                self.prep_model.clone()
            } else {
                self.ai_model.clone()
            };
            AiEndpoint {
                base_url: self.ai_base_url.clone(),
                bearer: self.ai_bearer.clone(),
                model,
                is_local: false,
            }
        }
    }

    /// V0.8.0 (Поток D) — one-shot CLOUD escalation endpoint. Always the cloud
    /// bridge with the SMART model (`prep_model`, default Sonnet), IGNORING
    /// `ai_provider`. Used when the user escalates a single hard question (a
    /// Shift+F9 ask, a per-tile "↑ ask the smart model", or a 🧠 follow-up)
    /// without flipping the persistent provider — so the default stays local
    /// (free, private) and there is no "forgot to switch back" trap.
    ///
    /// NOTE this gives a STRONGER model (deeper reasoning), NOT live web access —
    /// the cloud model still has a training cutoff. Live "2026 facts" would need
    /// a separate web-search / tool-use feature (out of scope here).
    ///
    /// `is_local` is forced false so callers bill it + allow screenshots as for
    /// any cloud call. The bridge fields are always present in config (even when
    /// the active provider is local), so this is safe regardless of provider.
    #[must_use]
    pub fn ai_endpoint_cloud(&self) -> AiEndpoint {
        AiEndpoint {
            base_url: self.ai_base_url.clone(),
            bearer: self.ai_bearer.clone(),
            model: self.prep_model.clone(),
            is_local: false,
        }
    }

    /// Resolve the SEPARATE vision endpoint, or `None` when vision is "off".
    /// "same" reuses the text endpoint; "cloud"/"local" use the `vision_*` /
    /// `vision_local_*` fields but fall back to the corresponding text fields
    /// when left empty, so a configured bridge works without re-entering creds.
    /// Cloud model falls back to Sonnet ([`DEFAULT_VISION_MODEL`]).
    #[must_use]
    pub fn vision_endpoint(&self) -> Option<AiEndpoint> {
        let pick = |specific: &str, fallback: &str| {
            if specific.trim().is_empty() {
                fallback.to_string()
            } else {
                specific.to_string()
            }
        };
        match self.vision_provider.as_str() {
            "same" => Some(self.ai_endpoint(false)),
            "cloud" => Some(AiEndpoint {
                base_url: pick(&self.vision_base_url, &self.ai_base_url),
                bearer: pick(&self.vision_bearer, &self.ai_bearer),
                model: pick(&self.vision_model, DEFAULT_VISION_MODEL),
                is_local: false,
            }),
            "local" => Some(AiEndpoint {
                base_url: pick(&self.vision_local_base_url, &self.ai_local_base_url),
                bearer: pick(&self.vision_local_bearer, &self.ai_local_bearer),
                model: pick(&self.vision_local_model, &self.ai_local_model),
                is_local: true,
            }),
            _ => None, // "off" (or unknown) → feature disabled
        }
    }

    /// Config-only readiness snapshot for the diagnostics panel (#131). Pure
    /// (no network, no audio) so it's instant + testable; the UI layers live
    /// AI/STT pings on top. `detail` strings carry NO secrets.
    #[must_use]
    pub fn readiness(&self) -> ReadinessReport {
        // AI — resolve the ACTIVE provider (local vs cloud) via the resolver.
        let ep = self.ai_endpoint(false);
        let ai_configured = !ep.base_url.trim().is_empty()
            && !ep.model.trim().is_empty()
            && (ep.is_local || !ep.bearer.trim().is_empty());
        let ai_detail = if ai_configured {
            format!(
                "{} · {} · {}",
                if ep.is_local { "local" } else { "cloud" },
                ep.base_url,
                ep.model
            )
        } else {
            String::new()
        };

        // STT — the active backend, keyed by `stt_provider`.
        let (stt_configured, stt_detail) = match self.stt_provider.as_str() {
            "gigaam" => {
                let ok = !self.stt_gigaam_dir.trim().is_empty();
                let d = if ok {
                    format!("gigaam · {}", self.stt_gigaam_dir)
                } else {
                    String::new()
                };
                (ok, d)
            }
            "whisper" => {
                let ok = !self.stt_whisper_url.trim().is_empty();
                let d = if ok {
                    format!("whisper · {}", self.stt_whisper_url)
                } else {
                    String::new()
                };
                (ok, d)
            }
            _ => {
                let ok = !self.groq_api_key.trim().is_empty();
                let d = if ok {
                    "groq cloud".to_string()
                } else {
                    String::new()
                };
                (ok, d)
            }
        };

        // Mic / system audio — a None / empty device means "system default",
        // a valid config (configured = true); the live signal check lives on
        // the Audio tab. Empty detail → the UI renders the localized "default".
        let device_detail = |d: &Option<String>| match d.as_deref() {
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => String::new(),
        };

        // Vision (F8) — resolve the SEPARATE vision channel. "off" → not
        // configured (intentional). detail carries provider + url + model only,
        // never a bearer (mirrors the AI line).
        let (vision_configured, vision_detail) = match self.vision_endpoint() {
            Some(ep) => {
                let ok = !ep.base_url.trim().is_empty() && !ep.model.trim().is_empty();
                let d = if ok {
                    format!("{} · {} · {}", self.vision_provider, ep.base_url, ep.model)
                } else {
                    String::new()
                };
                (ok, d)
            }
            None => (false, String::new()),
        };

        ReadinessReport {
            ai: ReadinessItem {
                configured: ai_configured,
                detail: ai_detail,
            },
            stt: ReadinessItem {
                configured: stt_configured,
                detail: stt_detail,
            },
            mic: ReadinessItem {
                configured: true,
                detail: device_detail(&self.mic_device),
            },
            sys: ReadinessItem {
                configured: true,
                detail: device_detail(&self.system_audio_device),
            },
            vision: ReadinessItem {
                configured: vision_configured,
                detail: vision_detail,
            },
            stealth_on: self.stealth_enabled,
        }
    }
}

/// Resolved STT backend for a session. Mirrors [`AiEndpoint`]: the config's
/// `stt_provider` string selects which transcription engine `stt::spawn` uses.
/// "cloud" (Groq) is the default so existing configs are unchanged.
#[derive(Debug, Clone)]
pub enum SttBackendCfg {
    /// Groq Whisper cloud API.
    Cloud { api_key: String, model: String },
    /// Local whisper.cpp server (OpenAI-compatible `/audio/transcriptions`).
    Whisper {
        base_url: String,
        bearer: String,
        model: String,
    },
    /// Local in-process GigaAM-v3 (ONNX). `model_dir` holds `model.int8.onnx` + `vocab.txt`.
    Gigaam { model_dir: String },
}

impl Config {
    /// Resolve which STT backend to use from `stt_provider`. Unknown / "cloud"
    /// → Groq, so old configs behave exactly as before.
    #[must_use]
    pub fn stt_backend(&self) -> SttBackendCfg {
        match self.stt_provider.as_str() {
            "gigaam" => SttBackendCfg::Gigaam {
                model_dir: self.stt_gigaam_dir.clone(),
            },
            "whisper" => SttBackendCfg::Whisper {
                base_url: self.stt_whisper_url.clone(),
                bearer: self.stt_whisper_bearer.clone(),
                model: self.stt_whisper_model.clone(),
            },
            _ => SttBackendCfg::Cloud {
                api_key: self.groq_api_key.clone(),
                model: self.stt_model.clone(),
            },
        }
    }

    /// True when STT runs locally (GigaAM or local Whisper) — callers skip the
    /// Groq-key "configured" check.
    #[must_use]
    pub fn stt_is_local(&self) -> bool {
        matches!(self.stt_provider.as_str(), "gigaam" | "whisper")
    }
}

fn default_ai_provider() -> String {
    "cloud".into()
}

fn default_vision_provider() -> String {
    // F8 capture works out of the box via the already-configured cloud bridge.
    "cloud".into()
}

fn default_ai_local_base_url() -> String {
    // llama.cpp (the shipped setup-local-ai.ps1 pipeline) serves on :8080.
    // Ollama users can change this to :11434 in Settings.
    "http://127.0.0.1:8080/v1".into()
}

fn default_stt_provider() -> String {
    "cloud".into()
}

fn default_stt_gigaam_gpu() -> bool {
    true
}

fn default_stt_whisper_url() -> String {
    "http://127.0.0.1:8081/v1".into()
}

fn default_stt_whisper_model() -> String {
    "whisper-large-v3-turbo".into()
}

fn default_tile_body_opacity() -> f32 {
    1.0
}

fn default_post_meeting_debrief_enabled() -> bool {
    false // opt-in — surprise Sonnet calls are bad UX
}

fn default_record_audio_enabled() -> bool {
    // ON by default — the saved audio is what a later "re-transcribe + re-summary
    // from the archive" flow needs; every un-recorded call is lost to it. The
    // Settings toggle + retention give the user control over privacy + disk.
    true
}

fn default_record_retention_sessions() -> u32 {
    10 // ~ last 10 sessions; user-adjustable, 0 = unbounded
}

fn default_journal_retention_sessions() -> u32 {
    100 // matches the pre-v0.15 hard-coded journal::KEEP_LAST_SESSIONS
}

fn default_journal_max_total_mb() -> u32 {
    500 // matches the pre-v0.15 hard-coded journal::MAX_TOTAL_BYTES
}

fn default_record_max_total_mb() -> u32 {
    20_000 // ~20 GB backstop on raw audio even in "keep all" mode; 0 = unlimited
}

fn default_session_archive_enabled() -> bool {
    true // the JSONL journals already exist; the catalog just indexes them
}

fn default_max_session_cost_usd() -> f64 {
    // v0.0.28: flipped 1.00 → 0.0 (chip disabled by default). Pet-project
    // user explicitly opted out of cost guard rails — said «по костам не
    // важно, у меня безлимитные деньги». Old installs keep their existing
    // value (serde reads from file); the chip only stops appearing for
    // fresh installs OR users who explicitly set 0. The cost-cap field
    // remains in Settings for users who DO want it.
    //
    // The hard-block path was already removed in v0.0.5 (soft warning
    // only), so this is purely a UX guilt-trip removal, not a behaviour
    // change for live AI calls.
    0.0
}

fn default_detector_skip_mic() -> bool {
    true // candidate's own voice shouldn't trigger explanation tiles
}

fn default_ui_language() -> String {
    // v0.0.42: default RU because that's the current primary language
    // (user is Russian-speaking; original Settings copy is Russian).
    // EN is opt-in via Settings → Interface → Язык интерфейса.
    "ru".into()
}

fn default_tile_font_size() -> u32 {
    // v0.0.55: default 12 matches the historic `--fs-12` CSS var that
    // .tile-body.markdown had previously hardcoded. Range 11-18 keeps
    // tiles readable without breaking grid math.
    12
}

/// Massive default trigger-keyword pool — 250+ DevOps/SRE/Cloud/Linux
/// terms across every common interview domain. Detector fires on any
/// whole-word match. User-configurable in Settings.
///
/// Note: this string also feeds Whisper's bias prompt (alongside
/// CANONICAL_TECH_VOCAB) — heavy users on the 800-char prompt budget
/// may want to trim. The detector ignores prompt budget — match against
/// the full list always.
///
/// Curated 2026-05-25 by domain (line-grouped for readability).
fn default_trigger_keywords() -> String {
    "\
        kubernetes k8s k3s etcd helm kustomize argocd flux istio linkerd cilium calico \
        kubectl kubeadm kubelet ingress configmap daemonset statefulset deployment \
        \
        docker containerd podman runc crio buildkit dockerfile compose multistage \
        registry distroless oci namespace cgroup \
        \
        linux bash zsh systemd journalctl strace ltrace lsof tcpdump iptables \
        nftables ufw firewalld selinux apparmor iotop htop dstat sar perf flamegraph \
        \
        postgres pgbouncer mysql mariadb mongo mongodb redis memcached rabbitmq \
        kafka nats activemq pulsar cassandra clickhouse cockroachdb elasticsearch \
        influxdb timescaledb prometheus opensearch \
        \
        grafana loki tempo jaeger zipkin opentelemetry alertmanager fluentd \
        fluentbit vector datadog newrelic splunk pagerduty observability tracing \
        sli slo sla errorbudget runbook postmortem chaos \
        \
        terraform ansible puppet chef saltstack pulumi crossplane vagrant packer \
        consul vault nomad opa rego sentinel \
        \
        jenkins gitlab github bitbucket teamcity bamboo circleci travis drone \
        argo flux helm tekton skaffold spinnaker gitops cicd pipeline \
        \
        aws gcp azure ec2 s3 rds eks gke aks lambda dynamodb sqs sns kinesis \
        cloudwatch cloudfront route53 elb alb nlb vpc subnet iam sts kms \
        bigquery pubsub cloudsql functions appengine cloudrun \
        eventhub servicebus cosmosdb storage blob queues vmss aks app-service \
        \
        nginx haproxy envoy traefik caddy apache varnish istio linkerd \
        gateway service-mesh sidecar canary blue-green rolling \
        \
        tcp udp http https grpc rest graphql websocket dns bgp ospf vpn vxlan \
        mpls nat dhcp dhcpv6 ipv4 ipv6 mtu mss tls ssl mtls handshake \
        certificate ca pki acme letsencrypt \
        \
        load balancer latency throughput jitter packet-loss bandwidth pps \
        connection pool keepalive timeout retry backoff circuit-breaker \
        ratelimit deadlock contention concurrency parallelism \
        \
        cpu memory disk ram nvme ssd hdd iops queue swap ballooning hugepages \
        oom segfault corefile coredump panic kernel module driver \
        \
        ci cd cicd devops sre devsecops gitops trunk-based mvc microservices \
        monolith serverless event-driven cqrs saga eventsourcing \
        \
        python golang rust java kotlin scala swift typescript javascript \
        nodejs npm pnpm yarn cargo maven gradle webpack vite esbuild \
        \
        oauth oidc jwt saml sso mtls rbac abac ldap kerberos zerotrust \
        encryption hashing bcrypt argon2 hmac signing certificate-pinning \
        secrets rotation \
        \
        cache invalidation cdn write-through write-back write-around eviction \
        lru lfu ttl stampede coherence consistency partition replication sharding \
        \
        scaling autoscaling vertical horizontal hpa vpa keda spot-instance \
        capacity provisioning forecasting throughput-test \
        \
        backup snapshot restore disaster-recovery rto rpo failover failback \
        active-passive active-active region availability-zone\
    "
    .into()
}

/// Massive default snippet library — 50+ pre-written templates covering
/// every common SRE / DevOps interview topic. Each is dense (~500-1000
/// chars), Russian-language, markdown-formatted, ready to spawn as a tile
/// via `/key` palette. Zero AI call, zero cost, ~50ms latency.
///
/// Curated 2026-05-25 from real interview question pool + production
/// runbook commons. Edit `%APPDATA%\overlay-mvp\config.json` array
/// `snippets` to customise per-user.
fn default_snippets() -> Vec<Snippet> {
    vec![
                Snippet {
                    key: "k8s".into(),
                    title: "Kubernetes troubleshoot — 5-step framework".into(),
                    body: "**Шаги диагностики (по убыванию частоты):**\n\n\
                           1. `kubectl get pods -A | grep -v Running` — что не Running?\n\
                           2. `kubectl describe pod X` — Events внизу: ImagePullBackOff / CrashLoopBackOff / OOMKilled / Pending?\n\
                           3. `kubectl logs X --previous` — последний exit, особенно для CrashLoop\n\
                           4. `kubectl get events --sort-by=.lastTimestamp` — cluster-wide контекст\n\
                           5. **Node-level:** `kubectl top node`, `df -h`, `dmesg` — диск/память/OOM?\n\n\
                           **Корень причин в нашей практике (топ 5):**\n\
                           - readiness/liveness probe слишком агрессивная → kill loop\n\
                           - ImagePullSecret истёк / private registry\n\
                           - Resource requests > capacity → Pending forever\n\
                           - PVC stuck (PV из другой AZ)\n\
                           - DNS внутри cluster: `nslookup kubernetes.default` в Pod'е".into(),
                },
                Snippet {
                    key: "pg".into(),
                    title: "PostgreSQL slow query — что проверять".into(),
                    body: "**Чеклист (порядок имеет значение):**\n\n\
                           1. **`EXPLAIN (ANALYZE, BUFFERS)`** — seq scan на большой таблице? индекса нет?\n\
                           2. `pg_stat_statements` — кто топ-10 по total_time?\n\
                           3. **Bloat:** `pg_stat_user_tables.n_dead_tup` — autovacuum успевает?\n\
                           4. **Locks:** `pg_stat_activity` где `wait_event_type='Lock'`\n\
                           5. **Config sanity:** `shared_buffers` (~25% RAM), `effective_cache_size` (~75%), `work_mem` × max_connections не превышает свободную RAM\n\n\
                           **Частые подставы:**\n\
                           - `SET random_page_cost = 1.1` для NVMe (default 4.0 — ложь на современных дисках)\n\
                           - JIT включён на маленьких запросах → ↑latency. `jit=off` для OLTP\n\
                           - Connection pooler отсутствует → 1000+ idle процессов жрут RAM. **PgBouncer transaction mode**".into(),
                },
                Snippet {
                    key: "incident".into(),
                    title: "Incident response — первые 5 минут".into(),
                    body: "**Order of operations:**\n\n\
                           1. **Признать:** «вижу алерт X, начинаю расследование». Без этого все ждут.\n\
                           2. **Stop the bleed (не root cause!):** rollback / failover / scale up. Лечим симптом сначала.\n\
                           3. **Open war room** + один **incident commander** (только координирует, не дебажит)\n\
                           4. **Timeline в realtime:** `T+0 alert, T+2 rollback started, T+5 mitigated…`\n\
                           5. **Communication on schedule:** статус каждые 15 мин даже если «still investigating»\n\n\
                           **NEVER** в первые 5 минут:\n\
                           - искать виноватого\n\
                           - чинить config in-place без бэкапа\n\
                           - молча копаться 30 минут «я почти нашёл»\n\n\
                           **Post-mortem:** blameless, 5 whys, action items с owner+due date".into(),
                },
                Snippet {
                    key: "sli".into(),
                    title: "SLI/SLO design — что измерять, что НЕ измерять".into(),
                    body: "**Хорошие SLI** (user-visible):\n\
                           - **Availability:** % успешных HTTP 200-399 за окно\n\
                           - **Latency:** p99 ≤ X ms для critical path\n\
                           - **Throughput:** requests/sec для batch жоб\n\
                           - **Correctness:** % правильных ответов (для ML/search)\n\n\
                           **Плохие SLI** (proxy-метрики, не user-pain):\n\
                           - CPU usage, RAM usage — никого не волнует пока система работает\n\
                           - Pod restarts — может быть «правильно» (rolling deploy)\n\n\
                           **Error budget:** SLO 99.9% = 43min downtime/month. Если бюджет сгорел — **stop feature releases, focus on reliability**. Не «прибавим строгости» — продакшн уже сгорел.\n\n\
                           **SLO ≠ SLA.** SLA = договорное обещание клиенту (с штрафами). SLO = внутренний таргет, обычно строже SLA.".into(),
                },
                // ── Kubernetes deep cuts ──────────────────────────────
                Snippet { key: "k8s-net".into(), title: "K8s networking — Service / Ingress / CNI".into(), body:
                    "**Service types (от меньшего scope):**\n\
                     - **ClusterIP** — внутри cluster, default\n\
                     - **NodePort** — открывает 30000-32767 на каждом node (dev/staging)\n\
                     - **LoadBalancer** — облачный LB (AWS NLB, GCP TCP LB)\n\
                     - **ExternalName** — CNAME alias, без proxy\n\n\
                     **Ingress vs Service:** Service = L4 (TCP/UDP), Ingress = L7 (HTTP host/path routing, TLS termination). Без Ingress controller (nginx-ingress, traefik, contour) ресурс Ingress ничего не делает.\n\n\
                     **CNI plugins (для interview):** Calico (BGP, NetworkPolicy), Cilium (eBPF, observability), Flannel (простой, VXLAN), Weave (mesh, encrypted). Выбор зависит от: NetworkPolicy support, performance, encryption needs.\n\n\
                     **Debug:** `kubectl exec -it pod -- nslookup svc-name`, `iptables -L -n -t nat | grep PORT`, `cilium monitor` для eBPF cluster.".into() },
                Snippet { key: "k8s-rbac".into(), title: "K8s RBAC — Roles, Bindings, SA".into(), body:
                    "**4 главных объекта:**\n\
                     - **Role / ClusterRole** — *что можно* (verbs: get/list/watch/create/update/patch/delete)\n\
                     - **RoleBinding / ClusterRoleBinding** — *кому* (subjects: User, Group, ServiceAccount)\n\
                     - **ServiceAccount** — идентичность для Pod'а (по умолчанию `default` в namespace)\n\
                     - **API Group** в Role — `\"\"` для core (pods, services), `apps` для deployments\n\n\
                     **Принципы:**\n\
                     - **Least privilege:** Role > ClusterRole когда хватает namespace\n\
                     - Default SA НЕ давать прав — создавать отдельный `app-sa` для каждого workload\n\
                     - `automountServiceAccountToken: false` если Pod не использует API\n\n\
                     **Debug:** `kubectl auth can-i create pods --as=system:serviceaccount:default:app-sa`".into() },
                Snippet { key: "k8s-storage".into(), title: "K8s storage — PV / PVC / StorageClass".into(), body:
                    "**Chain:** App → **PVC** (запрос storage) → **PV** (фактический volume) → **StorageClass** (provisioner).\n\n\
                     **Access modes:**\n\
                     - `ReadWriteOnce (RWO)` — один node (CSI block, EBS, GP3)\n\
                     - `ReadWriteMany (RWX)` — несколько nodes (NFS, EFS, CephFS)\n\
                     - `ReadOnlyMany (ROX)` — read-only несколько nodes\n\n\
                     **StorageClass важные параметры:**\n\
                     - `reclaimPolicy: Retain` — НЕ удалять PV при удалении PVC (важные данные)\n\
                     - `volumeBindingMode: WaitForFirstConsumer` — создавать PV в той же zone что Pod\n\n\
                     **Частые pain points:**\n\
                     - PVC `Pending` — нет StorageClass или provisioner не запущен\n\
                     - Pod `Pending` — PV в другой AZ от node\n\
                     - StatefulSet с RWO + node failure → manual recovery нужна".into() },
                Snippet { key: "k8s-autoscale".into(), title: "K8s autoscaling — HPA / VPA / Cluster Autoscaler".into(), body:
                    "**Три уровня:**\n\
                     - **HPA** (Horizontal Pod Autoscaler) — увеличивает количество Pod'ов по CPU/memory/custom metric\n\
                     - **VPA** (Vertical Pod Autoscaler) — меняет requests/limits существующих Pod'ов\n\
                     - **CA** (Cluster Autoscaler) — добавляет/убирает nodes когда Pod'ы Pending\n\n\
                     **HPA подводные камни:**\n\
                     - Metric server должен быть установлен (`kubectl top pod` работает?)\n\
                     - Stabilization window: scale-up быстрый, scale-down медленный (5 мин default)\n\
                     - Custom metrics — нужен Prometheus Adapter\n\n\
                     **VPA + HPA конфликт:** один меняет requests, другой решает по % использования. Использовать вместе только с external metric, не CPU.\n\n\
                     **KEDA** — event-driven autoscaling (Kafka lag, SQS queue depth, etc.). Альтернатива HPA когда CPU не отражает нагрузку.".into() },
                Snippet { key: "k8s-secrets".into(), title: "K8s secrets — что хранить, как защищать".into(), body:
                    "**Default Secret = base64**, не шифрование. На диске etcd лежит как plaintext.\n\n\
                     **Защита:**\n\
                     - **Encryption at rest:** `--encryption-provider-config` в API server (AES-CBC / KMS provider)\n\
                     - **External secret store:** Vault, AWS Secrets Manager, GCP Secret Manager — через External Secrets Operator (ESO) или Vault Agent Injector\n\
                     - **Sealed Secrets** (Bitnami) — шифруем secret через public key, безопасно коммитим в Git\n\
                     - **SOPS + age** — encrypted YAML в Git, GitOps friendly\n\n\
                     **Принципы:**\n\
                     - НЕ хранить secrets в env vars видимых через `kubectl describe pod`\n\
                     - Mount как files (volumeMount), `defaultMode: 0400`\n\
                     - Rotation: short TTL + автоматическая инъекция (Vault dynamic secrets)\n\
                     - RBAC: `kubectl auth can-i get secrets -n prod` от service account".into() },
                // ── Linux troubleshooting ─────────────────────────────
                Snippet { key: "linux-oom".into(), title: "Linux OOM killer — кто и почему".into(), body:
                    "**Симптомы:** процесс пропал без stacktrace, в `dmesg` строка `Out of memory: Killed process X (name)`.\n\n\
                     **Расследование:**\n\
                     1. `dmesg -T | grep -i 'killed process'` — кто, когда, score\n\
                     2. `cat /proc/<pid>/oom_score` — оценка перед kill (выше = первый кандидат)\n\
                     3. `/var/log/messages` или `journalctl -k --since '1 hour ago'`\n\
                     4. **Контекст:** общая память до момента — `sar -r 1`, `free -h`\n\n\
                     **Профилактика:**\n\
                     - **cgroup memory limits** — контейнер OOM'ится первым, не всё на хосте\n\
                     - `vm.overcommit_memory=2` + `overcommit_ratio` — строгий contract вместо optimistic\n\
                     - `oom_score_adj` критичным процессам (database, prometheus): `-1000` = неубиваемый\n\
                     - Включить **swap** (даже на K8s nodes — `--fail-swap-on=false`)\n\n\
                     **K8s context:** OOMKilled в `kubectl describe pod` — увеличить `resources.limits.memory`.".into() },
                Snippet { key: "linux-disk".into(), title: "Linux диск переполнен — как разобраться".into(), body:
                    "**Симптомы:** `Write failed: No space left on device`, app падает.\n\n\
                     **Что проверять:**\n\
                     1. **`df -h`** — какой раздел? (часто `/var/log` или `/tmp`)\n\
                     2. **`df -i`** — inodes! Маленькие файлы могут забить inodes, не block usage\n\
                     3. **`du -hx --max-depth=1 /var | sort -h`** — найти жирный subdir\n\
                     4. **`lsof | grep deleted | sort -k7 -h`** — открытые удалённые файлы (rotated logs, что app держит) — занимают место до restart\n\
                     5. **`ncdu /`** — интерактивный TUI, быстрее `du`\n\n\
                     **Частые причины:**\n\
                     - Logs без logrotate (rotated) → growing forever\n\
                     - `journalctl --vacuum-size=500M` — systemd journal раздулся\n\
                     - `docker system prune -a` — image cache, build cache\n\
                     - Core dumps в `/var/lib/systemd/coredump/`\n\
                     - **`lsof +L1`** — файлы с link-count 0 (deleted, still held)".into() },
                Snippet { key: "linux-net".into(), title: "Linux network debug — что-то не отвечает".into(), body:
                    "**Слой за слоем, снизу вверх:**\n\n\
                     **L1-L2 (link):** `ip link show` — UP/DOWN? `ethtool eth0` — скорость/duplex.\n\
                     **L3 (IP):** `ip addr show`, `ip route get 8.8.8.8` — какой интерфейс/gateway.\n\
                     **L3 connectivity:** `ping -c 4 <gateway>` → `ping 8.8.8.8` → `ping google.com`. Где сломалось?\n\
                     **DNS:** `dig +short example.com`, `getent hosts example.com` (учитывает /etc/hosts).\n\
                     **L4 connectivity:** `nc -vz host 443`, `curl -v https://host`, `traceroute -n -T -p 443 host`.\n\n\
                     **Полезные tools:**\n\
                     - `ss -tnp` — кто слушает (быстрее netstat)\n\
                     - `ss -tn state established` — активные коннекшены\n\
                     - `tcpdump -i any -nn host X.X.X.X` — что реально летит\n\
                     - `iptables -L -n -v`, `nft list ruleset` — фаервол блокирует?\n\
                     - `mtr <host>` — combination traceroute + ping, видит intermittent loss".into() },
                Snippet { key: "linux-perf".into(), title: "Linux performance — USE method (Brendan Gregg)".into(), body:
                    "**USE = Utilization · Saturation · Errors** для каждого ресурса:\n\n\
                     **CPU:**\n\
                     - Utilization: `top`, `mpstat -P ALL 1`\n\
                     - Saturation: load average / cores > 1.0\n\
                     - Errors: `dmesg | grep -i 'mce\\|cpu'`\n\n\
                     **Memory:**\n\
                     - U: `free -h`, `vmstat 1`\n\
                     - S: swap I/O (`si/so` в vmstat) — не нулевые? oom-kill recent?\n\
                     - E: ECC errors (`edac-util`)\n\n\
                     **Disk:**\n\
                     - U: `iostat -xz 1` (%util)\n\
                     - S: avgqu-sz (queue), await/svctm — wait time\n\
                     - E: `smartctl -a /dev/sda`, `dmesg | grep -i error`\n\n\
                     **Network:**\n\
                     - U: `sar -n DEV 1`, `iftop`\n\
                     - S: `ss -s` (overflow, retransmits), `nstat | grep -i drop`\n\
                     - E: `ip -s link` (errors/dropped)\n\n\
                     **Профайлеры:** `perf top`, `perf record/report`, `bcc-tools` (eBPF), `flamegraph.pl`.".into() },
                Snippet { key: "linux-systemd".into(), title: "systemd — основные команды + unit files".into(), body:
                    "**Status / control:**\n\
                     - `systemctl status <unit>` — текущее состояние\n\
                     - `systemctl start/stop/restart/reload <unit>`\n\
                     - `systemctl enable/disable <unit>` — boot persistence\n\
                     - `systemctl list-units --failed` — что упало\n\n\
                     **Journals:**\n\
                     - `journalctl -u <unit> -f` — tail\n\
                     - `journalctl -u <unit> --since '1 hour ago'`\n\
                     - `journalctl -p err -b` — errors с последнего boot\n\n\
                     **Unit file (`/etc/systemd/system/myapp.service`):**\n\
                     ```ini\n\
                     [Unit]\n\
                     Description=My App\n\
                     After=network-online.target\n\
                     Wants=network-online.target\n\n\
                     [Service]\n\
                     Type=notify\n\
                     ExecStart=/usr/bin/myapp\n\
                     Restart=on-failure\n\
                     RestartSec=5s\n\
                     User=myapp\n\
                     MemoryMax=2G\n\
                     CPUQuota=200%\n\n\
                     [Install]\n\
                     WantedBy=multi-user.target\n\
                     ```\n\n\
                     После правок: `systemctl daemon-reload && systemctl restart myapp`.".into() },
                // ── Networking deep cuts ──────────────────────────────
                Snippet { key: "tcp".into(), title: "TCP states + 3-way handshake + проблемы".into(), body:
                    "**3-way handshake:** SYN → SYN+ACK → ACK. После — `ESTABLISHED`.\n\
                     **Close:** FIN → ACK → FIN → ACK. Между FIN+ACK и финальным ACK — `TIME_WAIT` (~60s).\n\n\
                     **Состояния которые видишь в `ss`:**\n\
                     - `LISTEN` — server слушает\n\
                     - `ESTAB` — рабочее соединение\n\
                     - `TIME_WAIT` — много = частые короткие коннекты, нужен keep-alive\n\
                     - `CLOSE_WAIT` — твой код не закрыл socket после remote FIN. **Bug в app**\n\
                     - `SYN_SENT` зависший — firewall дропает или пакеты теряются\n\n\
                     **TCP tuning для high-throughput:**\n\
                     - `net.core.somaxconn=65535` — backlog accept queue\n\
                     - `net.ipv4.tcp_max_syn_backlog=65535`\n\
                     - `net.ipv4.tcp_fin_timeout=15` — короче TIME_WAIT (если backend behind LB)\n\
                     - `net.ipv4.tcp_tw_reuse=1` — переиспользовать TIME_WAIT sockets\n\
                     - `net.core.netdev_max_backlog=5000` — pre-routing queue\n\n\
                     **MTU issues:** `tracepath host`, `ping -M do -s 1472 host` — если фрагментация ломает MSS clamping, проблема в туннеле.".into() },
                Snippet { key: "dns".into(), title: "DNS — как работает + диагностика".into(), body:
                    "**Иерархия резолвинга (от хоста):**\n\
                     1. `/etc/hosts` (статический)\n\
                     2. **NSS** (`/etc/nsswitch.conf` — `hosts: files dns`)\n\
                     3. `systemd-resolved` (если активен) — кеширует, читает `/etc/systemd/resolved.conf`\n\
                     4. `/etc/resolv.conf` — recursive resolvers (8.8.8.8, 1.1.1.1)\n\
                     5. Recursive resolver обходит: root → TLD (.com) → authoritative для example.com\n\n\
                     **Tools (используй в этом порядке):**\n\
                     - `getent hosts example.com` — учитывает /etc/hosts + nsswitch\n\
                     - `dig +short example.com` — pure DNS query\n\
                     - `dig +trace example.com` — полный обход иерархии\n\
                     - `dig @8.8.8.8 example.com` — конкретный resolver\n\
                     - `nslookup -debug` — старый, иногда полезен для verbose response\n\n\
                     **Частые проблемы:**\n\
                     - TTL = 0 → каждый запрос пересчитывается → latency\n\
                     - search-domain в resolv.conf → лишние NXDOMAIN запросы\n\
                     - Coredns в K8s: `kubectl exec -it pod -- nslookup kubernetes.default`\n\
                     - DNS-over-HTTPS (DoH) — Cloudflare/Quad9 для приватности".into() },
                Snippet { key: "tls".into(), title: "TLS handshake + сертификаты + типичные ошибки".into(), body:
                    "**TLS 1.3 handshake (упрощённо):**\n\
                     1. Client → Server: `ClientHello` (поддерживаемые ciphers, SNI hostname, key share)\n\
                     2. Server → Client: `ServerHello` + cert + key share. Уже шифровано после этого\n\
                     3. Client verify cert → derive shared key → `Finished`. Готово, 1-RTT.\n\n\
                     **TLS 1.2 = 2 RTT** (старый, не используй для новых сервисов).\n\n\
                     **Cert chain:** leaf → intermediate(s) → root CA. **Сервер ДОЛЖЕН отдавать leaf + intermediates** (не root — он у клиента).\n\n\
                     **Debug:**\n\
                     - `openssl s_client -connect host:443 -servername host` — handshake debug, видит весь chain\n\
                     - `curl -vI https://host` — verbose с TLS info\n\
                     - `ssllabs.com/ssltest` — внешняя проверка\n\n\
                     **Типичные ошибки:**\n\
                     - `unable to verify the first certificate` — не отдан intermediate\n\
                     - `Hostname mismatch` — cert на `www.x.com`, ходишь на `x.com` (нужен SAN)\n\
                     - `certificate has expired` — поставь `cert-manager` + ACME (Let's Encrypt)\n\
                     - `wrong version number` — кто-то говорит HTTP вместо HTTPS на port 443".into() },
                Snippet { key: "lb".into(), title: "Load balancers — типы, алгоритмы, sticky sessions".into(), body:
                    "**L4 vs L7:**\n\
                     - **L4** (TCP/UDP) — AWS NLB, HAProxy mode TCP. Быстро, не знает HTTP. Можно балансить gRPC, MQTT, Postgres.\n\
                     - **L7** (HTTP) — AWS ALB, nginx, Envoy. Видит headers/paths → routing rules, TLS termination, rewrite. Дороже CPU.\n\n\
                     **Алгоритмы:**\n\
                     - **Round-robin** — простой, не учитывает нагрузку\n\
                     - **Least connections** — лучше для long-lived (websocket, БД pool)\n\
                     - **IP hash / consistent hash** — кеш friendly (один user → один backend), но плохой spread\n\
                     - **Random with two choices (P2C)** — на удивление хорошо работает\n\
                     - **Weighted** — backend с разным CPU\n\n\
                     **Sticky sessions:**\n\
                     - Cookie-based (L7): LB вставляет `AWSALB=xxx`\n\
                     - Source-IP (L4): hash (ip, port) → backend. Ломается за NAT\n\
                     - **Избегай если можно** — stateless app + Redis session > sticky\n\n\
                     **Health checks:** `/health` endpoint, interval 5-10s, threshold 2-3 fails. **Не путать с liveness/readiness в K8s.**".into() },
                Snippet { key: "http".into(), title: "HTTP коды — что значат + когда используются".into(), body:
                    "**2xx success:**\n\
                     - `200 OK` — стандартный успех\n\
                     - `201 Created` — POST создал ресурс (Location header указывает на новый)\n\
                     - `204 No Content` — успех но тела нет (DELETE, PUT без response)\n\n\
                     **3xx redirect:**\n\
                     - `301` — permanent (SEO friendly, кеш forever)\n\
                     - `302` — temporary (default Express/Flask `redirect`)\n\
                     - `304 Not Modified` — ETag/If-Modified-Since совпали\n\n\
                     **4xx client error:**\n\
                     - `400` — невалидный request (badly formed JSON)\n\
                     - `401` — нет credentials (отдай `WWW-Authenticate`)\n\
                     - `403` — credentials есть, но прав нет\n\
                     - `404` — ресурс не существует\n\
                     - `409` — конфликт (optimistic lock, version mismatch)\n\
                     - `422 Unprocessable Entity` — JSON валиден но семантически ломан\n\
                     - `429 Too Many Requests` — rate limit (отдай `Retry-After`)\n\n\
                     **5xx server error:**\n\
                     - `500` — что-то сломалось внутри (не пиши stack trace в body!)\n\
                     - `502 Bad Gateway` — proxy не достучался до upstream\n\
                     - `503 Service Unavailable` — temporary, scheduled maintenance, отдай `Retry-After`\n\
                     - `504 Gateway Timeout` — upstream timeout".into() },
                // ── Databases ─────────────────────────────────────────
                Snippet { key: "pg-replica".into(), title: "PostgreSQL replication — streaming, logical, варианты".into(), body:
                    "**Streaming replication (binary, физический WAL):**\n\
                     - Setup: `pg_basebackup -h primary -U replicator -D /var/lib/postgresql -R`\n\
                     - Replica = read-only (default `hot_standby = on`)\n\
                     - Полная копия cluster — нельзя реплицировать одну DB\n\
                     - Async (default) или sync (`synchronous_standby_names`)\n\n\
                     **Logical replication (per-table, начиная с PG10):**\n\
                     - Publisher: `CREATE PUBLICATION pub FOR TABLE users, orders;`\n\
                     - Subscriber: `CREATE SUBSCRIPTION sub CONNECTION '...' PUBLICATION pub;`\n\
                     - Можно cross-version (10 → 16), можно частично, можно writeable\n\
                     - НЕ реплицирует DDL (схемы должны совпадать manually)\n\n\
                     **HA paterns:**\n\
                     - **Patroni** — leader election через etcd/consul/zk + auto-failover\n\
                     - **repmgr** — старый, manual switchover\n\
                     - **PgBouncer / Pgpool-II** — pooling + read/write split\n\n\
                     **Подводные камни:**\n\
                     - WAL bloat если replica отстаёт → `max_slot_wal_keep_size` (PG13+) спасает\n\
                     - Split-brain при failover без fencing — terminate old primary жёстко".into() },
                Snippet { key: "mysql".into(), title: "MySQL replication + InnoDB ключевые особенности".into(), body:
                    "**Replication типы:**\n\
                     - **Statement-based** — replicate SQL текст. Проблемы с non-deterministic (`NOW()`, `RAND()`)\n\
                     - **Row-based** (default 5.7+) — реплицируем сами row changes\n\
                     - **Mixed** — Statement когда безопасно, иначе Row\n\
                     - **GTID** (Global Transaction ID) — упрощает failover, обязателен для group replication\n\n\
                     **InnoDB важное:**\n\
                     - **Buffer pool** — главный cache. `innodb_buffer_pool_size = 70-80% RAM`\n\
                     - **Redo log** (`ib_logfile0/1`) — write-ahead, recovery после crash\n\
                     - **Undo log** — MVCC read consistency, rollback\n\
                     - **Clustered index** — таблица физически отсортирована по PK. Без PK MySQL создаст hidden\n\
                     - **Secondary index** содержит PK, не row pointer → wide PK = wide indexes\n\n\
                     **Tuning:**\n\
                     - `innodb_flush_log_at_trx_commit=1` (durability) vs `=2` (perf, риск 1s loss)\n\
                     - `innodb_io_capacity=2000` для SSD (default 200)\n\
                     - `sync_binlog=1` для prod (производительность ↓, durability ↑)".into() },
                Snippet { key: "redis".into(), title: "Redis — persistence, cluster, типичные паттерны".into(), body:
                    "**Persistence:**\n\
                     - **RDB** — snapshot периодически (`save 300 10`). Fast restart, может потерять последние секунды\n\
                     - **AOF** — append-only log каждой write op. `appendfsync everysec` (compromise) или `always` (slow)\n\
                     - **Both** — Redis читает AOF при старте. Recommended для prod\n\n\
                     **HA / scaling:**\n\
                     - **Sentinel** — мониторинг master/replicas + auto-failover (HA только)\n\
                     - **Cluster** — 16384 slots, sharded (data разбита по nodes). Min 3 masters + 3 replicas\n\
                     - **Cluster ограничения:** multi-key ops только если все keys в одном slot (`{user:1}:foo`, `{user:1}:bar`)\n\n\
                     **Паттерны:**\n\
                     - **Cache-aside:** app сам читает/пишет cache. Простой, fault-tolerant.\n\
                     - **Rate limit:** `INCR + EXPIRE` или token bucket через Lua\n\
                     - **Distributed lock:** Redlock алгоритм (controversial; `SET key val NX PX 30000` достаточно для большинства)\n\
                     - **Pub/Sub** — fire-and-forget, без persistence. Для гарантий → Streams\n\n\
                     **Anti-patterns:** `KEYS *` в prod (заблокирует), большие values (>10MB), expensive Lua scripts.".into() },
                Snippet { key: "mongo".into(), title: "MongoDB — replica set, sharding, indexing".into(), body:
                    "**Replica set:**\n\
                     - Min 3 nodes (primary + 2 secondary) — для election quorum\n\
                     - Primary только пишет, secondary читают (если `readPreference != primary`)\n\
                     - Election при недоступности primary, ~10s timeout\n\
                     - **Oplog** — capped collection, source-of-truth для replication\n\n\
                     **Sharding (для huge datasets):**\n\
                     - **Shard key** — главное решение. Низкая cardinality = hotspot. Не меняется после установки.\n\
                     - **Hashed shard key** — равномерный spread, но range queries разбиваются на все shards\n\
                     - **Compound shard key** — лучше, но всё равно immutable\n\
                     - Components: `mongos` (router), `config servers` (3-node replica set с metadata), `shards`\n\n\
                     **Indexes:**\n\
                     - Compound: ESR-правило (Equality, Sort, Range) — порядок полей\n\
                     - **Covered query** — все нужные fields есть в index → не trip to documents\n\
                     - **Partial index** — `{partialFilterExpression: {active: true}}` экономит место\n\
                     - `db.collection.explain('executionStats').find(...)` — есть IXSCAN или COLLSCAN?".into() },
                Snippet { key: "ch".into(), title: "ClickHouse — для interview SRE/data".into(), body:
                    "**Что это:** column-oriented OLAP DB. Optimized для агрегаций по миллиардам строк. **НЕ заменяет** OLTP (Postgres/MySQL).\n\n\
                     **Ключевые особенности:**\n\
                     - **Columnar storage** — читает только нужные columns, компрессия высокая (LZ4/ZSTD)\n\
                     - **MergeTree** family — основной engine. Данные иммутабельны, periodic background merge\n\
                     - **Sharding + replication** через ZooKeeper / ClickHouse Keeper (встроенный, PG-like)\n\
                     - **Materialized views** — pre-aggregations, обновляются по INSERT\n\n\
                     **Подходит для:** logs (Loki alternative), metrics (Prometheus long-term), analytics, observability backend (Datadog using CH internally).\n\n\
                     **НЕ подходит для:** транзакций, UPDATE-heavy workloads, primary key lookups мелких rows.\n\n\
                     **Tuning:**\n\
                     - `ORDER BY` — outer partition key (часто timestamp + dimension)\n\
                     - `PARTITION BY toYYYYMM(date)` — manageable parts\n\
                     - **TTL** — `TTL date + INTERVAL 90 DAY DELETE` для retention\n\
                     - Profile: `SELECT * FROM system.query_log WHERE query LIKE '%table%'`".into() },
                // ── Observability ─────────────────────────────────────
                Snippet { key: "prom".into(), title: "Prometheus + Alertmanager — основное".into(), body:
                    "**Архитектура:** Pull-based — Prometheus сам ходит за метриками на `/metrics` endpoint targets (полная противоположность InfluxDB push).\n\n\
                     **Service discovery:** static, file_sd, kubernetes_sd, consul_sd, ec2_sd. Targets находятся автоматом.\n\n\
                     **PromQL базовые:**\n\
                     - `rate(http_requests_total[5m])` — qps в last 5 min\n\
                     - `histogram_quantile(0.99, sum by(le) (rate(latency_bucket[5m])))` — p99\n\
                     - `up{job=\"api\"} == 0` — target down\n\
                     - `avg by(instance) (node_cpu_seconds_total{mode!=\"idle\"})` — CPU\n\n\
                     **Recording rules:** pre-compute expensive queries → быстрые dashboards.\n\n\
                     **Alertmanager:**\n\
                     - Group by `alertname, severity` — один email на 50 firing alerts\n\
                     - Inhibition: critical inhibits warning (на той же машине)\n\
                     - Silence: maintenance window\n\
                     - Routes: разные команды → разные channels (PagerDuty/Slack/email)\n\n\
                     **Retention:** local 15d default. Для long-term — Thanos / Cortex / VictoriaMetrics / Mimir.".into() },
                Snippet { key: "grafana".into(), title: "Grafana — dashboards + alerting do/don't".into(), body:
                    "**Dashboard design:**\n\
                     - **USE method** для resource-based (CPU/Memory/Disk/Net): Utilization, Saturation, Errors\n\
                     - **RED method** для service-based (HTTP/gRPC): Rate, Errors, Duration\n\
                     - Не делай 50-панельный «full overview». Лучше 3 dashboards: overview / drill-down / debug\n\
                     - Время в верхнем углу, variables в panel-affecting toolbar\n\n\
                     **Variables (templating):**\n\
                     - `$cluster`, `$namespace`, `$pod` — каскадные queries\n\
                     - `__rate_interval` — built-in, sane default для `rate()`\n\n\
                     **Alerting (Grafana 9+ unified alerting):**\n\
                     - Multi-dimensional (одно правило → много альертов по labels)\n\
                     - Связывай alert с dashboard через annotation `runbook_url`\n\
                     - `for: 5m` — пожар должен гореть 5 мин, иначе flapping\n\
                     - Notification policy = маршрутизация (как Alertmanager routes)\n\n\
                     **Anti-patterns:**\n\
                     - Alert per pod restart — false positives, used to be CrashLoopBackOff better\n\
                     - Single-value metric «is service alive» — лучше `up == 0 for 1m`\n\
                     - Hardcoded thresholds — % CPU зависит от размера node, лучше anomaly detection".into() },
                Snippet { key: "logs".into(), title: "Logging stack — ELK vs Loki vs ClickHouse".into(), body:
                    "**ELK (Elasticsearch + Logstash + Kibana):**\n\
                     - Full-text search, mature ecosystem\n\
                     - Дорогой по RAM/диску (inverted index на каждое поле)\n\
                     - Сложный operational toll (cluster master split-brain, shard rebalance)\n\
                     - Используй когда нужен **поиск по содержимому**\n\n\
                     **Loki (Grafana Labs):**\n\
                     - **Индексирует только labels**, не содержимое — дёшево\n\
                     - LogQL syntax напоминает PromQL\n\
                     - Идеален для **K8s + Prometheus stack** (одни labels)\n\
                     - Slower для grep по content больших volumes\n\
                     - Storage backend = S3/GCS, ~10× дешевле ES\n\n\
                     **ClickHouse:**\n\
                     - Топ по скорости aggregations\n\
                     - Materialized views для pre-computed metrics-from-logs\n\
                     - Используется Cloudflare, Uber, Datadog внутри\n\
                     - Steeper learning curve, нет нативного UI (но Grafana plugin есть)\n\n\
                     **Шиппинг:**\n\
                     - **Fluent Bit** — lightweight, K8s native, C-based\n\
                     - **Vector** (Datadog) — Rust, more flexible transforms\n\
                     - **Fluentd** — старая школа, Ruby, медленнее\n\
                     - **Promtail** — официальный шиппер для Loki".into() },
                Snippet { key: "trace".into(), title: "Distributed tracing — Jaeger / Tempo / OpenTelemetry".into(), body:
                    "**Зачем:** проследить один запрос через 10+ микросервисов. См. где p99 latency, где error.\n\n\
                     **Концепции:**\n\
                     - **Trace** = вся цепочка (один user request)\n\
                     - **Span** = одна операция (HTTP call, DB query). Имеет start_time, duration, parent_span_id\n\
                     - **Context propagation** — trace_id передаётся через `traceparent` header (W3C) или старый `X-B3-*` (Zipkin)\n\n\
                     **OpenTelemetry (стандарт):**\n\
                     - SDK для каждого языка → отправляет в **OTel Collector** → дальше в Jaeger/Tempo/Datadog\n\
                     - Auto-instrumentation для популярных libs (HTTP servers, gRPC, DB drivers)\n\
                     - Заменил OpenTracing + OpenCensus\n\n\
                     **Backends:**\n\
                     - **Jaeger** — старший, full-featured, in-memory или Cassandra/ES backend\n\
                     - **Tempo** (Grafana) — cheap storage в S3, integration с Loki/Mimir\n\
                     - **Zipkin** — самый старый, простой\n\n\
                     **Sampling:**\n\
                     - **Head-based** (% sampling прямо в SDK) — простой, тебе может не повезти не зацепить incident\n\
                     - **Tail-based** (в Collector, по error/latency) — дороже но «правильнее»".into() },
                // ── CI/CD ─────────────────────────────────────────────
                Snippet { key: "deploy".into(), title: "Deploy strategies — blue/green vs canary vs rolling".into(), body:
                    "**Rolling update (default K8s Deployment):**\n\
                     - Постепенно заменяем N pods, `maxSurge` + `maxUnavailable`\n\
                     - Плюс: простой, нет дополнительной инфры\n\
                     - Минус: смешанный traffic на старую+новую версии, hard rollback (нужен обратный rolling)\n\n\
                     **Blue/Green:**\n\
                     - Поднимаем целиком новый «green» environment\n\
                     - Переключаем traffic через LB / Ingress switch — атомарно\n\
                     - Плюс: instant rollback (вернуть LB обратно)\n\
                     - Минус: 2× ресурсов\n\n\
                     **Canary:**\n\
                     - 1% → 10% → 50% → 100% постепенно\n\
                     - Метрики (error rate, latency, business KPIs) — автоматический abort\n\
                     - Tools: **Argo Rollouts**, **Flagger** (Flux)\n\
                     - Лучший для prod high-traffic\n\n\
                     **A/B testing** ≠ canary:\n\
                     - A/B = product experiment (feature change)\n\
                     - Canary = infra deploy (same feature, новая версия binary)\n\n\
                     **Feature flags** (LaunchDarkly, Unleash) — orthogonal: код задеплоен, фича скрыта.".into() },
                Snippet { key: "argo".into(), title: "GitOps + ArgoCD — push vs pull деплой".into(), body:
                    "**GitOps принципы (Weaveworks):**\n\
                     1. Git = source of truth для всего (manifests, configs)\n\
                     2. Декларативные манифесты (K8s YAML, Terraform, Crossplane)\n\
                     3. Auto-sync: agent в cluster постоянно сверяет реальное состояние с Git\n\
                     4. Pull-based — cluster сам тянет, не CI пушит\n\n\
                     **ArgoCD:**\n\
                     - **Application** = (Git repo + path) → (K8s cluster + namespace)\n\
                     - Auto-sync polling 3min или webhook trigger\n\
                     - Sync waves для ordered deploy (CRD → operator → instance)\n\
                     - UI показывает diff между Git и live, drift detection\n\n\
                     **Argo Rollouts** (отдельный controller):\n\
                     - Заменяет Deployment на Rollout (CRD)\n\
                     - Canary / Blue-Green с analysis templates (Prometheus query → abort)\n\n\
                     **Flux v2** (CNCF, конкурент):\n\
                     - Более модульный (Source + Kustomize + Helm controllers)\n\
                     - Лучше для multi-tenancy и multi-cluster\n\
                     - Менее красивый UI\n\n\
                     **Tradeoff push vs pull:**\n\
                     - Push (CI → cluster) — нужны cluster creds в CI, проще для одного env\n\
                     - Pull (GitOps) — credentials только у agent, лучше security boundary".into() },
                Snippet { key: "ci".into(), title: "CI pipeline — что должно быть на каждом step".into(), body:
                    "**Стандартный pipeline для backend (порядок важен):**\n\n\
                     1. **Lint + format check** — fail fast, < 30s. golangci-lint, ruff, eslint\n\
                     2. **Unit tests** — параллелизуй, coverage gate (≥70% обычно sane)\n\
                     3. **Build** — cache dependencies. Docker multi-stage для slim images\n\
                     4. **Security scan:** image (Trivy/Grype), secrets (gitleaks), SAST (semgrep)\n\
                     5. **Integration tests** — нужны живые БД (Testcontainers, docker-compose)\n\
                     6. **Push artifact** — image в registry, tag = git SHA (не `latest`)\n\
                     7. **Deploy to dev** — auto на main branch\n\
                     8. **E2E tests** — Playwright/Cypress против dev env\n\
                     9. **Deploy to staging/prod** — manual approval или canary auto\n\n\
                     **Принципы:**\n\
                     - **Каждый PR проходит pipeline до build** (минимум)\n\
                     - **Каждый commit на main** = автоматический deploy в dev\n\
                     - **Артефакт неизменен** — образ собран один раз, проходит env-to-env\n\
                     - **Pipeline = код** (Jenkinsfile, .github/workflows, .gitlab-ci.yml) — в репо, ревьювится\n\n\
                     **Cache:** Docker layer cache (BuildKit `--cache-from`), npm/cargo/pip cache в `~/.cache`. Может ускорить 5-10×.".into() },
                Snippet { key: "secrets-ci".into(), title: "Secrets в CI/CD — где НЕ хранить + где можно".into(), body:
                    "**Где НЕЛЬЗЯ:**\n\
                     - В код / `.env` файлах в репо (даже private!)\n\
                     - В Dockerfile (`ENV API_KEY=...`) — попадёт в image layer навсегда\n\
                     - В CI job logs — `set -x` в shell или `echo $TOKEN` спалит\n\
                     - В deployment manifests как plaintext\n\n\
                     **Где можно (по убыванию security):**\n\
                     1. **External vault** (HashiCorp Vault, AWS Secrets Manager) — dynamic secrets (Vault выдаёт DB cred на 1 час)\n\
                     2. **Sealed Secrets / SOPS** — зашифрованные YAML в Git, расшифровка только в cluster\n\
                     3. **OIDC federation** — CI получает короткоживущий token от AWS/GCP по trust relationship (без долгоживущих keys)\n\
                     4. **GitHub Actions secrets / GitLab CI variables** — масked в логах, но всё ещё доступно maintainer'у\n\
                     5. **K8s Secret** (encrypted-at-rest!) — для уже задеплоенного app\n\n\
                     **Best practices:**\n\
                     - **Rotation** — short TTL + автоматическая ротация\n\
                     - **Audit log** — кто/когда читал каждый secret\n\
                     - **Least privilege** — отдельный SA на каждый workload\n\
                     - **Не передавай secrets через args** (видны в `ps`) — только env или mounted files".into() },
                // ── Cloud ─────────────────────────────────────────────
                Snippet { key: "aws-vpc".into(), title: "AWS VPC — subnets / routing / connectivity".into(), body:
                    "**Структура VPC:**\n\
                     - **VPC** = appname + CIDR (10.0.0.0/16)\n\
                     - **Subnets** = AZ-specific (10.0.1.0/24 в us-east-1a). Public vs Private отличается route table\n\
                     - **Route table:** Public ─→ IGW (Internet Gateway). Private ─→ NAT Gateway (для outbound) или Endpoints\n\
                     - **VPC Endpoints** — приватный доступ к AWS services (S3, DynamoDB Gateway endpoints бесплатные)\n\n\
                     **Связь между VPC:**\n\
                     - **VPC Peering** — point-to-point, transitive routing НЕТ\n\
                     - **Transit Gateway** — hub-and-spoke, scales до тысяч VPCs, поддерживает SD-WAN\n\
                     - **PrivateLink** — service exposure без peering (cross-account SaaS)\n\n\
                     **Security:**\n\
                     - **Security Group** = stateful firewall (Pod-level), на ENI\n\
                     - **NACL** = stateless ACL (subnet-level), и in и out нужны\n\
                     - **Flow Logs** — pcap-style traffic log в S3 или CloudWatch\n\n\
                     **Подводные камни:**\n\
                     - 5 SG per ENI default — можно увеличить через quota\n\
                     - NAT Gateway = $0.045/hr + $0.045/GB processed — много трафика = дорого. Vpce S3 спасает\n\
                     - IPv6 поддерживается, но Dual-stack настраивать руками".into() },
                Snippet { key: "aws-iam".into(), title: "AWS IAM — Users / Roles / Policies — кратко".into(), body:
                    "**4 главных объекта:**\n\
                     - **User** — человек/external system с долговременными credentials\n\
                     - **Group** — набор политик на множество users\n\
                     - **Role** — переключаемая identity (assume-role), short-lived creds. Используй для EC2/Lambda/cross-account\n\
                     - **Policy** — JSON document с Statements (`Effect`, `Action`, `Resource`, `Condition`)\n\n\
                     **Принципы:**\n\
                     - **НЕ ИСПОЛЬЗОВАТЬ root account** — только для billing + начальный setup\n\
                     - **НЕ хранить access keys** в EC2 — Instance Profile (Role) даёт STS creds автоматически\n\
                     - **Least privilege** — `Action: \"s3:GetObject\", Resource: \"arn:aws:s3:::my-bucket/*\"` (не `s3:*`)\n\
                     - **MFA на всё** — особенно root и IAM с привилегиями\n\n\
                     **Conditions:**\n\
                     - `aws:SourceIp` — restrict by IP\n\
                     - `aws:MultiFactorAuthPresent` — требовать MFA для critical actions\n\
                     - `aws:RequestTag/Project` — tag-based authorization\n\n\
                     **Permission boundary** — max permissions для созданных пользователем roles (для delegating IAM admin developers).".into() },
                Snippet { key: "s3".into(), title: "S3 — consistency, storage classes, типичные паттерны".into(), body:
                    "**Consistency (с 2020):** strong read-after-write для всех ops (включая overwrites + deletes). Раньше eventual для overwrites.\n\n\
                     **Storage classes:**\n\
                     - **Standard** — default, multi-AZ, hot\n\
                     - **Intelligent-Tiering** — auto переключение по access patterns ($)\n\
                     - **Standard-IA / One Zone-IA** — infrequent access, дёшево read но retrieval fee\n\
                     - **Glacier Instant Retrieval** — milliseconds retrieval, минимум 90 дней\n\
                     - **Glacier Flexible / Deep Archive** — часы/дни retrieval, минимум 90/180 дней. Архив compliance\n\n\
                     **Lifecycle policies:** auto-transition `Standard → IA → Glacier → Delete` по возрасту objects.\n\n\
                     **Производительность:**\n\
                     - 3500 PUT/COPY/POST/DELETE per second per prefix\n\
                     - 5500 GET/HEAD per second per prefix\n\
                     - Prefix sharding для high-throughput (`/2024/01/01/...` vs `<hash>/...`)\n\
                     - Multipart upload для >100 MB файлов (parallelism + resume)\n\n\
                     **Security:**\n\
                     - **Bucket policy** + **ACL** + **Block Public Access** (последнее — fail-safe)\n\
                     - **SSE-S3 / SSE-KMS** — encryption at rest, KMS даёт audit log\n\
                     - **Object Lock + WORM** — compliance (нельзя удалить N дней)\n\
                     - **Versioning** + **MFA Delete** — защита от ransomware/accidental delete".into() },
                // ── Containers ────────────────────────────────────────
                Snippet { key: "docker".into(), title: "Docker — layers, multi-stage, dockerfile best practices".into(), body:
                    "**Layers:**\n\
                     - Каждая `RUN` / `COPY` / `ADD` создаёт новый layer\n\
                     - Layers immutable, кешируются → меняешь нижний layer = пересобираешь всё выше\n\
                     - **Order matters:** редко-меняющиеся (`apt install`) ВВЕРХУ, часто-меняющиеся (`COPY src/`) ВНИЗУ\n\n\
                     **Multi-stage build:**\n\
                     ```dockerfile\n\
                     FROM golang:1.22 AS builder\n\
                     WORKDIR /src\n\
                     COPY go.mod go.sum ./\n\
                     RUN go mod download\n\
                     COPY . .\n\
                     RUN CGO_ENABLED=0 go build -o /app\n\n\
                     FROM gcr.io/distroless/static:nonroot\n\
                     COPY --from=builder /app /app\n\
                     ENTRYPOINT [\"/app\"]\n\
                     ```\n\
                     Финальный image ~10 MB вместо 800 MB.\n\n\
                     **Best practices:**\n\
                     - `USER nonroot` — не root внутри контейнера\n\
                     - `HEALTHCHECK` — Docker / orchestrator знает что app живой\n\
                     - `.dockerignore` — не пихай `.git`, `node_modules` в context\n\
                     - **Don't run as PID 1 без init** — `tini` или `--init` для signal handling\n\
                     - **Pin versions:** `python:3.11.7-slim`, не `python:latest`\n\
                     - **Cache mounts** (BuildKit): `RUN --mount=type=cache,target=/root/.cache/go-build go build` — ускоряет 5-10×".into() },
                // ── Security ──────────────────────────────────────────
                Snippet { key: "oauth2".into(), title: "OAuth 2.0 / OIDC — потоки + когда какой".into(), body:
                    "**Базовые роли:**\n\
                     - **Resource Owner** — user\n\
                     - **Client** — приложение (web, mobile, CLI)\n\
                     - **Authorization Server** — выдаёт tokens (Auth0, Keycloak, Okta)\n\
                     - **Resource Server** — API, валидирует tokens\n\n\
                     **Flows (выбирай по типу client):**\n\n\
                     - **Authorization Code + PKCE** — для web/mobile/SPA (modern default). Browser → auth → exchange code на token.\n\
                     - **Client Credentials** — machine-to-machine (cron job, microservice). Только client_id+secret, нет user.\n\
                     - **Device Code** — для CLI без браузера / smart TV. Показывает URL+code на одном устройстве, login на другом.\n\
                     - **Refresh Token** — продление access_token без re-login. Храни SECURE (httpOnly cookie или secure storage).\n\n\
                     **❌ Deprecated:** Implicit (XSS-prone), Resource Owner Password Credentials (нарушает разделение ответственности).\n\n\
                     **OIDC vs OAuth:** OIDC = OAuth 2.0 + identity layer. Возвращает **id_token** (JWT с claims о user). OAuth = authorization (access), OIDC = authentication (who).\n\n\
                     **JWT валидация:** проверять signature, `iss`, `aud`, `exp`. Public key через JWKS endpoint (`.well-known/jwks.json`).".into() },
                Snippet { key: "owasp".into(), title: "OWASP Top 10 (2021) — что чаще всего ломают".into(), body:
                    "1. **Broken Access Control** — `/admin` без RBAC, IDOR (`/users/123` → меняешь на 124), missing function-level checks\n\
                     2. **Cryptographic Failures** — секреты в логах, weak ciphers (MD5/SHA1 для passwords), no TLS\n\
                     3. **Injection** — SQL/NoSQL/Command/LDAP. **Parameterized queries**, ORM. Никогда string concat!\n\
                     4. **Insecure Design** — отсутствие threat modeling. Например, password reset → token в URL → log → утечка\n\
                     5. **Security Misconfiguration** — default creds, debug mode in prod, verbose error pages, открытые порты\n\
                     6. **Vulnerable Components** — устаревшие libs. Tools: `npm audit`, `pip-audit`, Dependabot, Trivy, Snyk\n\
                     7. **Identification/Auth Failures** — weak passwords allowed, нет rate limit на login, predictable session IDs\n\
                     8. **Software/Data Integrity Failures** — unsigned updates, npm/pip packages from random source, CI без integrity check\n\
                     9. **Security Logging Failures** — не логировать auth events; **или** логировать sensitive data\n\
                     10. **SSRF** — Server-Side Request Forgery. App fetches `?url=...` без validation → атакующий достаёт `http://169.254.169.254/` (metadata)\n\n\
                     **Defense in depth:** WAF + secure code + monitoring + patch cadence. Никогда **одна** мера.".into() },
                // ── SRE ───────────────────────────────────────────────
                Snippet { key: "capacity".into(), title: "Capacity planning — формулы + что учитывать".into(), body:
                    "**Базовый расчёт:**\n\n\
                     `Required capacity = peak_qps × avg_response_time × safety_factor`\n\n\
                     Пример: 10k qps peak × 50ms response × 1.5 safety = 750 concurrent requests. При 100 RPS/instance → 8 instances.\n\n\
                     **Что учитывать:**\n\
                     - **Headroom** — никогда 100% utilization. SRE practice: 60-70% peak\n\
                     - **Growth** — Q-over-Q business metric forecast (если product растёт 20% QoQ — capacity тоже)\n\
                     - **Failover scenario** — если одна AZ упала, оставшиеся должны вынести 100%. Значит 3 AZ × 50% normal load = 150% capacity\n\
                     - **Burst pattern** — peak/avg ratio. Black Friday = 10× normal. Что делать?\n\
                     - **Resource limits** не только CPU/RAM:\n\
                       - DB connections (PgBouncer max?)\n\
                       - File descriptors (ulimit)\n\
                       - Port range (ephemeral ports)\n\
                       - SNAT ports на NAT gateway (AWS limit 55k per public IP)\n\n\
                     **Load testing:**\n\
                     - `k6`, `locust`, `wrk`, `vegeta` — gradual ramp до breakpoint\n\
                     - **Найди где деградирует** (response time / error rate / queue depth) — это твой real capacity, не теоретический.\n\
                     - **Chaos engineering** (Gremlin, Litmus) — что если узкое место упадёт?".into() },
                Snippet { key: "runbook".into(), title: "Runbook — структура для on-call".into(), body:
                    "**Каждый алерт = runbook с linkable URL** в Alert annotations.\n\n\
                     **Структура runbook:**\n\n\
                     1. **Алерт name + summary** (что значит этот алерт)\n\
                     2. **Severity:** SEV1 (page) / SEV2 (slack) / SEV3 (ticket)\n\
                     3. **First actions** (≤5 шагов, конкретные команды):\n\
                        - `kubectl logs deployment/api -n prod --tail=200`\n\
                        - `curl https://api.example.com/health`\n\
                        - dashboard URL\n\
                     4. **Common causes** (с диагностикой каждой):\n\
                        - DB connection pool exhausted → `SELECT count(*) FROM pg_stat_activity`\n\
                        - Upstream slow → `grep upstream_response_time access.log`\n\
                     5. **Mitigation** (что делать, в порядке от safest):\n\
                        - Auto-restart pod\n\
                        - Scale up replicas\n\
                        - Failover to standby\n\
                        - Rollback last deploy\n\
                     6. **Escalation:** когда призывать оригинального owner / senior\n\
                     7. **Post-mortem template link**\n\n\
                     **Принципы:**\n\
                     - **Каждый алерт должен иметь runbook** (или: алерт удалить)\n\
                     - **Junior on-call** должен пройти runbook без помощи\n\
                     - **Update после каждого incident** — что нового узнали? Add to runbook.\n\
                     - Версионирование в Git, ревью изменений\n\
                     - Test раз в квартал — chaos drill «прокликай как на page»".into() },
                Snippet { key: "errorbudget".into(), title: "Error budget — как использовать на практике".into(), body:
                    "**Базовая формула:**\n\
                     - SLO `99.9% availability` → budget `0.1%` = 43.2 min downtime/month\n\
                     - SLO `99.95%` → 21.6 min/month\n\
                     - SLO `99.99%` (\"four nines\") → 4.3 min/month — **серьёзная стоимость**\n\n\
                     **Что значит \"бюджет сгорел\":**\n\
                     - **Stop feature releases** — freezing deploys на N дней\n\
                     - Focus engineers на reliability: chaos drills, runbooks, alerting тuning\n\
                     - Не «давайте увеличим SLO до 99.99%» — это игнорирует реальность\n\n\
                     **Что значит \"бюджет в запасе\":**\n\
                     - **Take risks:** deploy чаще, agressive canary, experimental features\n\
                     - Plan maintenance windows — не отнимай у бюджета unplanned outages\n\
                     - Run intentional failure tests (Gameday)\n\n\
                     **Multi-window burn rate (Google SRE book):**\n\
                     - Slow burn: за 6 часов сгорело 5% бюджета → page on-call\n\
                     - Fast burn: за 5 минут сгорело 2% → page + escalate\n\
                     - Avoids paging on transient spike, но не upset for sustained issue\n\n\
                     **Дискуссия с PM:**\n\
                     - SLO = договор между **infra и product** team\n\
                     - Если product хочет deploy 10× в день → нужен ОБЪЕКТИВНЫЙ budget tracker\n\
                     - Нет budget tracker = SLO = wishful thinking".into() },
                // ── Microservices ─────────────────────────────────────
                Snippet { key: "saga".into(), title: "Saga pattern — распределённые транзакции".into(), body:
                    "**Проблема:** один бизнес-процесс трогает 3 сервиса (Order → Payment → Inventory). 2PC дорогой и хрупкий.\n\n\
                     **Saga = последовательность local транзакций с compensating actions.**\n\n\
                     **Choreography (decentralised):**\n\
                     - Каждый сервис emit event, остальные subscribe\n\
                     - OrderCreated → Payment subscribes → reserves\n\
                     - PaymentReserved → Inventory subscribes → reserves\n\
                     - InventoryReserved → Order subscribes → finalises\n\
                     - **Плюс:** loose coupling, нет центральной точки отказа\n\
                     - **Минус:** сложно дебажить (где мы в saga?), implicit dependency graph\n\n\
                     **Orchestration (central coordinator):**\n\
                     - Saga orchestrator (state machine) вызывает services по очереди\n\
                     - Tools: Temporal, Camunda, AWS Step Functions\n\
                     - **Плюс:** explicit flow, visualization, retry/timeout встроены\n\
                     - **Минус:** SPOF coordinator (HA нужен), tight coupling от orchestrator\n\n\
                     **Compensating actions ОБЯЗАТЕЛЬНЫ:**\n\
                     - Payment failed → emit OrderCancelled → Inventory releases reservation\n\
                     - **НЕ ВСЕ операции откатываются** — отправил email? Compensate = «sorry» email\n\n\
                     **Idempotency** критична — message broker может deliver дважды.\n\
                     **Outbox pattern** — атомарность \"DB write + event publish\".".into() },
                Snippet { key: "mesh".into(), title: "Service mesh — Istio / Linkerd, когда нужен".into(), body:
                    "**Что делает mesh:** sidecar (Envoy/proxy) рядом с каждым app handle:\n\
                     - **mTLS** автоматически между всеми services (zero-trust networking)\n\
                     - **Traffic management** — canary, A/B, retries, timeouts, circuit breakers\n\
                     - **Observability** — automatic metrics/traces без правок app кода\n\
                     - **Policy** — кто может звать кого (authorization)\n\n\
                     **Istio:**\n\
                     - Feature-rich, complex. Envoy data plane + Istiod control plane\n\
                     - VirtualService / DestinationRule / Gateway — CRDs\n\
                     - Steep learning curve, but unmatched flexibility\n\
                     - Ambient mode (новый) — без sidecars, ztunnel + waypoint\n\n\
                     **Linkerd:**\n\
                     - Simpler, Rust-based proxy (быстрее, меньше памяти чем Envoy)\n\
                     - Лучше для smaller / starting teams\n\
                     - Менее feature-богат\n\n\
                     **Когда НЕ нужен:**\n\
                     - <10 микросервисов — overhead не оправдан\n\
                     - Если auth/TLS уже делается на app level (libraries)\n\
                     - Один namespace — простой Network Policy достаточно\n\n\
                     **Когда нужен:**\n\
                     - 50+ services, multi-team\n\
                     - Compliance: \"all traffic encrypted\"\n\
                     - Cross-cluster / multi-region routing\n\
                     - Платформенная команда стандартизирует observability".into() },
                Snippet { key: "circuit".into(), title: "Circuit breaker + retry — паттерны устойчивости".into(), body:
                    "**Circuit breaker состояния:**\n\
                     - **Closed** — нормальное прохождение запросов\n\
                     - **Open** — открыт после N failures, request fails fast БЕЗ обращения к upstream\n\
                     - **Half-Open** — после cooldown пробует ОДИН запрос. Success → Closed. Fail → Open\n\n\
                     **Параметры:**\n\
                     - `failure_threshold = 50%` за окно 10s\n\
                     - `request_volume_threshold = 20` (минимум для статистики)\n\
                     - `sleep_window = 5s` (Open → Half-Open delay)\n\n\
                     **Retry правила:**\n\
                     - **Exponential backoff with jitter:** `delay = min(cap, base * 2^attempt) + rand(0, base)`\n\
                     - **НЕ retry на 4xx** (твоя ошибка, не upstream)\n\
                     - **Retry на:** 502, 503, 504, network timeout, connection refused\n\
                     - **Retry budget** — макс N retries за окно (не битьём в стенку весь pool)\n\
                     - **Idempotency!** Не retry POST без идемпотентного key\n\n\
                     **Timeout каскад:**\n\
                     - Client timeout (e.g. 30s) > сумма всех downstream timeouts + retries\n\
                     - Иначе retry сработает после того как client уже отвалился\n\n\
                     **Libraries:**\n\
                     - **resilience4j** (Java), **polly** (.NET), **tenacity** (Python)\n\
                     - **Envoy** делает это в service mesh без кода\n\
                     - **Hystrix** deprecated — see resilience4j".into() },
                // ── Message Queues ────────────────────────────────────
                Snippet { key: "kafka".into(), title: "Kafka — partitions, consumer groups, semantics".into(), body:
                    "**Базовые концепции:**\n\
                     - **Topic** = log of messages\n\
                     - **Partition** = ordered immutable sequence (parallel unit)\n\
                     - **Offset** = position в partition (consumer tracks)\n\
                     - **Replication factor** = N брокеров хранят копию (typical 3)\n\
                     - **Producer key** → hash(key) % partitions = always same partition (ordering per key)\n\n\
                     **Consumer groups:**\n\
                     - Один consumer group получает каждое сообщение РАЗ\n\
                     - Partitions распределяются между consumers в группе\n\
                     - `# consumers ≤ # partitions` (лишние idle)\n\
                     - **Rebalance** при join/leave — pause traffic\n\n\
                     **Delivery semantics:**\n\
                     - **At-most-once** — commit offset BEFORE process → может потерять при crash\n\
                     - **At-least-once** (default) — process THEN commit → может дублировать\n\
                     - **Exactly-once** — `transactional.id` + idempotent producer + `isolation.level=read_committed` consumer\n\n\
                     **Tuning производительности:**\n\
                     - Producer: `batch.size=64KB`, `linger.ms=10` — batching\n\
                     - `compression.type=lz4` (хороший trade-off speed/ratio)\n\
                     - `acks=all` (durability) vs `acks=1` (throughput) vs `acks=0` (fire-and-forget)\n\
                     - Consumer: `max.poll.records=500`, `fetch.min.bytes=1MB`\n\n\
                     **Retention:** `retention.ms=7d` (time) или `retention.bytes=10GB` (size). Compacted topic = только последнее значение per key.".into() },
                Snippet { key: "rabbit".into(), title: "RabbitMQ — exchanges, queues, когда vs Kafka".into(), body:
                    "**4 типа exchanges:**\n\
                     - **Direct** — routing key == binding key (exact match)\n\
                     - **Topic** — pattern match (`logs.*.error`, wildcard)\n\
                     - **Fanout** — broadcast в все bound queues (ignores routing key)\n\
                     - **Headers** — match по message headers (редко используется)\n\n\
                     **Queue types:**\n\
                     - **Classic** — single-node, replicas через mirroring (deprecated in 4.0)\n\
                     - **Quorum** (recommended) — Raft consensus, HA, persistent\n\
                     - **Streams** (3.9+) — Kafka-like append-only log\n\n\
                     **RabbitMQ vs Kafka:**\n\
                     - **RabbitMQ:** flexible routing, per-message ACK, push model, lower latency для small messages, лучше для task queues / job dispatch\n\
                     - **Kafka:** high throughput, replay-able log, partitioned scale, event streaming / log aggregation\n\n\
                     **Делегирование выбора:**\n\
                     - \"Worker pool делает email-отправку\" → RabbitMQ + work queue\n\
                     - \"Event sourcing 1M events/sec\" → Kafka\n\
                     - \"Pub/sub микросервисов\" → оба подходят, выбирай team familiarity\n\
                     - \"Order processing, последовательность важна per-customer\" → Kafka (partition by customer_id)\n\n\
                     **Anti-patterns:** RabbitMQ как long-term storage (TTL maxes out), Kafka как RPC bus (overkill).".into() },
                // ── Performance / Caching ─────────────────────────────
                Snippet { key: "cache".into(), title: "Cache strategies — write-through / -back / -around".into(), body:
                    "**Read patterns:**\n\
                     - **Cache-aside** (lazy loading): app сам проверяет cache → miss → fetch DB → populate cache. Простой, fault-tolerant\n\
                     - **Read-through:** cache provider сам fetches DB на miss. Cleaner code, но cache становится SPOF\n\n\
                     **Write patterns:**\n\
                     - **Write-through:** write идёт в cache И в DB sync. Slow writes, fresh cache\n\
                     - **Write-back / write-behind:** write только в cache, async flush в DB. Fast writes, риск потерь\n\
                     - **Write-around:** write в DB, cache игнорируется. Cache miss на следующий read\n\n\
                     **Eviction:**\n\
                     - **LRU** — Least Recently Used (default Redis `allkeys-lru`)\n\
                     - **LFU** — Least Frequently Used (Redis `allkeys-lfu`, лучше для stable access patterns)\n\
                     - **FIFO** — простой queue, плохо для cache\n\
                     - **Random** — surprisingly competitive\n\
                     - **TTL** — time-based, complementary к eviction\n\n\
                     **Invalidation (\"two hardest problems in CS\"):**\n\
                     - **TTL** — простой но stale data до expiry\n\
                     - **Explicit invalidate** — write path удаляет cache key. Хрупкий (легко забыть)\n\
                     - **Event-driven** — DB change → publish → cache subscribers invalidate\n\
                     - **Versioned keys** — `user:42:v3` — release new version = effectively new cache\n\n\
                     **Cache stampede:** thundering herd когда expired key fetched 1000× одновременно. Lock + double-check или probabilistic early refresh.".into() },
                // ── Search ────────────────────────────────────────────
                Snippet { key: "es".into(), title: "Elasticsearch basics — index, mapping, query".into(), body:
                    "**Inverted index:** для каждого term → список documents где он встречается. Это база full-text search.\n\n\
                     **Иерархия:**\n\
                     - **Cluster** ⊃ **Indices** ⊃ **Shards** ⊃ **Segments** (Lucene level)\n\
                     - **Document** = JSON object с auto-assigned `_id`\n\n\
                     **Mapping** (= schema):\n\
                     - `keyword` — exact match (фильтр, aggregation)\n\
                     - `text` — full-text, analyzed (stems, lowercase, stop words)\n\
                     - Часто хочешь оба: `\"name\": {\"type\":\"text\", \"fields\":{\"keyword\":{\"type\":\"keyword\"}}}`\n\
                     - **Dynamic mapping** — ES угадывает типы. Опасно в prod, делай explicit\n\n\
                     **Query DSL основное:**\n\
                     - `match` — full-text query (analyze + search)\n\
                     - `term` — exact match (НЕ для text fields — будет искать analyzed token)\n\
                     - `bool { must, should, must_not, filter }` — composite\n\
                     - `aggs` — Elasticsearch's group by + analytics\n\n\
                     **Sharding:**\n\
                     - Number of shards задаётся при создании index, **не меняется** (нужен reindex)\n\
                     - Replicas меняются live (`PUT /_settings`)\n\
                     - Rule of thumb: shard size 10-50 GB. 200 shards on small index = wasted overhead\n\n\
                     **Anti-patterns:**\n\
                     - Использовать как primary DB (нет transactions, eventual consistency)\n\
                     - Indexing 10M docs за раз без bulk API + refresh tuning\n\
                     - `wildcard` queries (`*foo*`) на больших indexes — full scan".into() },
                // ── Streaming / ML-Ops ────────────────────────────────
                Snippet { key: "mlops".into(), title: "ML-Ops basics — model serving + monitoring".into(), body:
                    "**ML lifecycle:**\n\
                     1. **Data ingestion + validation** (Great Expectations, TFDV)\n\
                     2. **Feature engineering** — feature store (Feast, Tecton) для re-use\n\
                     3. **Training** — track experiments (MLflow, W&B), versioned data (DVC)\n\
                     4. **Validation** — accuracy / fairness / robustness checks\n\
                     5. **Serving** (см. ниже)\n\
                     6. **Monitoring** — data drift, model drift, business metrics\n\n\
                     **Serving patterns:**\n\
                     - **Batch:** scheduled job предсказывает на all customers за ночь, results в DB\n\
                     - **Real-time online:** REST/gRPC endpoint, low latency (<100ms p99)\n\
                     - **Streaming:** Kafka → consumer применяет модель → новый topic\n\
                     - **Edge:** TFLite / ONNX / CoreML на устройстве\n\n\
                     **Tools:**\n\
                     - **TF Serving, TorchServe** — фреймворк-specific\n\
                     - **NVIDIA Triton** — multi-framework, GPU optimized\n\
                     - **BentoML, KServe** — Kubernetes-native, abstracts framework\n\
                     - **Seldon Core** — advanced (canary, A/B, explainers)\n\n\
                     **Monitoring (новые типы ошибок vs обычный app):**\n\
                     - **Data drift** — input distribution меняется (PSI / KL-divergence per feature)\n\
                     - **Concept drift** — relationship X→Y меняется\n\
                     - **Model performance в проде** — нужны ground-truth labels (delayed feedback)\n\
                     - **Shadow deployment** — новая модель работает рядом, results сравниваются offline".into() },
                // ── Diagnostic checklist ──────────────────────────────
                Snippet { key: "slow".into(), title: "«Сайт тормозит» — общий чеклист 5 минут".into(), body:
                    "**Step 1: где именно медленно** (узнать ДО digging):\n\
                     - DevTools Network tab → TTFB или waterfall?\n\
                     - Server-side timing (`Server-Timing` header) — DB / cache / template render?\n\
                     - APM trace (Datadog / New Relic / Jaeger) — какой span главный contributor?\n\n\
                     **Step 2: типичные подозреваемые:**\n\n\
                     **DB-related:**\n\
                     - Slow query (`pg_stat_statements`, `mysql slow_query_log`)\n\
                     - Connection pool exhausted (`pg_stat_activity` показывает 1000 idle)\n\
                     - Lock contention (long-running transaction)\n\
                     - Missing index (после deploy ALTER TABLE без индекса)\n\n\
                     **Cache-related:**\n\
                     - Cache miss rate взлетел (Redis: `INFO stats` → `keyspace_misses`)\n\
                     - Cache stampede после mass eviction\n\n\
                     **App-related:**\n\
                     - GC pause (Java: `-Xlog:gc*`, Node: `--inspect`)\n\
                     - CPU pegged (`top`, profiler)\n\
                     - Memory leak → swap → 10× slowdown\n\
                     - N+1 queries (ORM lazy loading)\n\n\
                     **External:**\n\
                     - Third-party API slow (logs upstream_response_time)\n\
                     - DNS resolution slow (resolver fails → app retries)\n\
                     - Network packet loss (mtr)\n\
                     - CDN cache miss → origin overload\n\n\
                     **Step 3: метрики глобально:**\n\
                     - Дашборд RED (Rate, Errors, Duration) — где аномалия?\n\
                     - Compare с baseline неделю назад\n\
                     - Recent deploys? Rollback if matches start time".into() },
                Snippet { key: "memleak".into(), title: "Memory leak debug — Linux + основные runtimes".into(), body:
                    "**Симптомы:** RAM растёт монотонно во времени, без plateau. После N часов — OOM или swap thrash.\n\n\
                     **Базовая диагностика:**\n\
                     - `ps aux --sort=-%mem | head` — кто жрёт\n\
                     - `cat /proc/<pid>/status | grep -E 'VmRSS|VmPeak'`\n\
                     - `smem -tk` — учитывает shared memory правильно\n\
                     - `pmap -x <pid>` — детальный breakdown\n\n\
                     **JVM (Java/Kotlin/Scala):**\n\
                     - **Heap dump:** `jcmd <pid> GC.heap_dump /tmp/heap.hprof`\n\
                     - **Анализ:** Eclipse MAT, VisualVM, IntelliJ profiler — ищи **dominator tree**\n\
                     - **Live analysis:** `jcmd <pid> VM.native_memory summary` (если NMT enabled)\n\
                     - Частые источники: ThreadLocal'ы, кеши без bound, classloader leaks\n\n\
                     **Node.js:**\n\
                     - `--inspect` flag → Chrome DevTools Memory tab → heap snapshot\n\
                     - **Compare 2 snapshots** — найти что появилось\n\
                     - Closure-related, EventEmitter listeners без `off()`, Promises держат references\n\n\
                     **Go:**\n\
                     - `import _ \"net/http/pprof\"` → `go tool pprof http://...:6060/debug/pprof/heap`\n\
                     - `top -cum`, `list <fn>`, `web` — flame graph\n\
                     - Goroutine leaks: `pprof/goroutine` — растут ли\n\n\
                     **Python:**\n\
                     - `tracemalloc.start()` → `tracemalloc.take_snapshot()` → diff\n\
                     - `objgraph.show_growth()` — что новых instances\n\
                     - `memory_profiler` decorator для line-level".into() },
                // ── Misc one-liners ───────────────────────────────────
                Snippet { key: "jvm".into(), title: "JVM tuning — флаги + GC выбор".into(), body:
                    "**Heap size:**\n\
                     - `-Xms4G -Xmx4G` — установи min=max чтоб JVM не resize'ил\n\
                     - **Container-aware:** `-XX:MaxRAMPercentage=75` (Java 10+, проще чем считать MB)\n\n\
                     **GC выбор (Java 17+):**\n\
                     - **G1GC** (default) — balance latency/throughput, default <=4 GB\n\
                     - **ZGC** (`-XX:+UseZGC`) — pause < 1ms, для latency-sensitive, поддерживает терабайтные heaps\n\
                     - **Shenandoah** (RedHat) — конкурент ZGC\n\
                     - **Parallel GC** — old-school, max throughput, длинные паузы. Для batch.\n\n\
                     **Observability:**\n\
                     - `-Xlog:gc*:file=/var/log/gc.log:time,uptime,level,tags` — structured GC log\n\
                     - **GCEasy.io** — paste log → визуализация\n\
                     - `jcmd <pid> GC.heap_info` — runtime heap state\n\n\
                     **JIT:**\n\
                     - **Tiered compilation** (default) — quick C1 → optimal C2\n\
                     - `-XX:+PrintCompilation` — что инлайнится / деоптимизируется\n\
                     - **GraalVM** — alternative JIT, иногда быстрее, иногда медленнее\n\n\
                     **Container gotchas:**\n\
                     - JVM до Java 10 не видел cgroup limits → heap > container memory → OOMKilled\n\
                     - **Java 10+:** `-XX:+UseContainerSupport` (default on)\n\
                     - CPU: `-XX:ActiveProcessorCount=N` если container limits < hostspecific".into() },
                Snippet { key: "git".into(), title: "Git advanced — rebase, bisect, reflog, hooks".into(), body:
                    "**Rebase vs merge:**\n\
                     - **Merge** — сохраняет история, делает merge commit. History видит \"когда был merged\"\n\
                     - **Rebase** — переносит твои commits на свежий main. Linear history\n\
                     - **Golden rule:** не rebase pushed branches которые юзают другие\n\n\
                     **Interactive rebase** (`git rebase -i HEAD~5`):\n\
                     - `pick / reword / edit / squash / drop` — clean up история перед PR\n\
                     - Полезно сводить 12 \"fix typo\" → 1 logical commit\n\n\
                     **`git bisect`** — найти когда баг introduced:\n\
                     ```\n\
                     git bisect start\n\
                     git bisect bad HEAD\n\
                     git bisect good v1.2.0\n\
                     # git checkout автоматически между N коммитами\n\
                     # ты тестишь, git bisect good|bad\n\
                     git bisect reset\n\
                     ```\n\
                     С `git bisect run ./test.sh` — fully automated.\n\n\
                     **`git reflog`** — Time machine. Всегда восстанавливай через reflog:\n\
                     - `git reflog` — список всех HEAD movements\n\
                     - `git reset --hard HEAD@{2}` — undo последнюю операцию\n\n\
                     **Hooks** (`.git/hooks/`):\n\
                     - `pre-commit` — lint/format перед commit\n\
                     - `commit-msg` — conventional commits валидация\n\
                     - `pre-push` — run tests перед push\n\
                     - **`pre-commit framework`** (Python) — shared hooks между разработчиками\n\n\
                     **`git worktree`** — несколько checked-out branches одновременно без clone:\n\
                     - `git worktree add ../hotfix hotfix-branch`".into() },
                Snippet { key: "regex".into(), title: "Regex — частые паттерны для логов".into(), body:
                    "**Базовые классы:**\n\
                     - `\\d` digit, `\\w` word char (letter/digit/_), `\\s` whitespace\n\
                     - `[^abc]` — НЕ a/b/c\n\
                     - `\\b` — word boundary (важно для match слов в тексте)\n\n\
                     **Quantifiers:**\n\
                     - `*` 0+, `+` 1+, `?` 0-1, `{n,m}` range\n\
                     - **Lazy:** `*?`, `+?` — match минимум (для `<.*?>`)\n\n\
                     **Useful patterns:**\n\
                     - IP: `\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b`\n\
                     - Email (rough): `[\\w.+-]+@[\\w-]+\\.[\\w.-]+`\n\
                     - URL: `https?://\\S+`\n\
                     - Hex color: `#[0-9a-fA-F]{6}`\n\
                     - UUID: `[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`\n\
                     - ISO timestamp: `\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}`\n\n\
                     **Lookarounds (продвинутое):**\n\
                     - `foo(?=bar)` — foo с bar после (не consume)\n\
                     - `foo(?!bar)` — foo БЕЗ bar после\n\
                     - `(?<=bar)foo` — foo с bar перед\n\
                     - `(?<!bar)foo` — foo БЕЗ bar перед\n\n\
                     **Производительность:**\n\
                     - Избегай **catastrophic backtracking**: `(a+)+b` на `aaaaaaaaa!` зависнет на час\n\
                     - Anchor: `^foo` лучше `foo` для matches с начала строки\n\
                     - В Python: `re.compile(...)` для repeated use".into() },
                Snippet { key: "perf-tips".into(), title: "Web app perf — 10 квик-винов".into(), body:
                    "1. **Включи gzip/brotli** на nginx/CDN — 3-5× меньше HTTP payload\n\
                     2. **Cache-Control headers** для статики (`max-age=31536000, immutable` для hashed assets)\n\
                     3. **HTTP/2 или HTTP/3** — мультиплексинг, header compression\n\
                     4. **CDN** для всего static — Cloudflare/Fastly/CloudFront\n\
                     5. **DB connection pool** (PgBouncer / Hikari) — переиспользование TCP+SSL handshakes\n\
                     6. **N+1 → JOIN или batch fetch** — самый частый бекенд-bottleneck\n\
                     7. **Index missing columns** в часто-фильтруемых WHERE/JOIN\n\
                     8. **Eager-load** для known relations (Rails `includes`, Django `select_related`)\n\
                     9. **Lazy-load** images / iframes (`loading=\"lazy\"`)\n\
                     10. **Bundle splitting** на front-end — отдельный bundle per route\n\n\
                     **Метрики которые юзер реально чувствует:**\n\
                     - **LCP** (Largest Contentful Paint) — когда основной контент виден. Цель <2.5s\n\
                     - **FID/INP** — interaction latency. Цель <100ms\n\
                     - **CLS** — layout shift score. Цель <0.1\n\
                     - **TTFB** — time to first byte. Цель <800ms\n\
                     - Real User Monitoring (RUM): web-vitals.js + send to analytics".into() },
                Snippet { key: "interview-tips".into(), title: "Interview tips — как структурировать ответ на behavioral".into(), body:
                    "**STAR framework:**\n\
                     - **Situation** — короткий контекст (1-2 предложения, не саговая ёлка)\n\
                     - **Task** — твоя ответственность в этой ситуации\n\
                     - **Action** — что ИМЕННО ты сделал (\"я\", не \"мы\"). Конкретика\n\
                     - **Result** — измеримый исход + что узнал\n\n\
                     **Анти-паттерны:**\n\
                     - **\"Мы переделали систему\"** — кто конкретно ты? Что делал?\n\
                     - **30-минутная сага** без структуры — интервьюер потеряется\n\
                     - **Только успехи** — спросят failure, не готов = красный флаг\n\
                     - **\"Я попросил у команды помощи\"** как finale — что СДЕЛАЛ потом?\n\n\
                     **Типичные вопросы (заготовь 2-3 истории):**\n\
                     - Tell me about a conflict с коллегой\n\
                     - Project что failed / scope creep\n\
                     - Time you had to learn что-то быстро\n\
                     - Difficult technical decision\n\
                     - Когда не соглашался с менеджером\n\
                     - Самый proud project\n\
                     - Mistake / regret\n\n\
                     **Reverse interview questions (ты к интервьюеру):**\n\
                     - Что для вас был самый интересный technical challenge here last quarter?\n\
                     - Как выглядит typical week для этой роли?\n\
                     - On-call rotation / pager hygiene?\n\
                     - Career path / promotion criteria?\n\
                     - Как принимаются технические решения (RFC? консенсус? CTO декрет?)".into() },
                Snippet { key: "salary".into(), title: "Salary negotiation — как обсуждать".into(), body:
                    "**До интервью:**\n\
                     - **Сам узнай рынок** — Levels.fyi / Glassdoor / habr salaries / индустриальные опросы\n\
                     - Знай свой **walk-away number** (минимум за который пойдёшь) и **target**\n\
                     - Compensation = base + bonus + equity + sign-on + relocation + benefits — не путай!\n\n\
                     **\"Ваши ожидания?\":**\n\
                     - **Никогда не называй первое число** if you can avoid\n\
                     - Try: \"я ищу что в рынке для senior X роли в этом регионе, что вы готовы предложить?\"\n\
                     - Если давят: дай **range, не точку**, и **anchor 10-15% выше target**\n\
                     - Format: \"$210k-$240k base, ожидаю total comp $X с учётом equity\"\n\n\
                     **При offer:**\n\
                     - **Не отвечай сразу** — \"Спасибо, мне нужно подумать, отвечу до X\". Стандартная практика.\n\
                     - **Counter offer письменно** — конкретные числа, fact-based justification (\"конкурент X offer мне Y\")\n\
                     - **Total comp** компонентам — base vs bonus vs equity vs sign-on. Иногда легче сдвинуть один\n\
                     - **Sign-on bonus** — обычно компенсирует unvested equity со старого места\n\n\
                     **Никогда:**\n\
                     - Не врать про competing offers (могут проверить через recruiter network)\n\
                     - Не accept на word — wait for written offer letter\n\
                     - Не сжигать мосты — даже если walking away".into() },
    ]
}

pub fn config_path() -> Result<PathBuf> {
    let dir = crate::paths::data_root().context("no config dir")?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create config dir {}", dir.display()))?;
    Ok(dir.join("config.json"))
}

/// Parse config bytes, tolerating a leading UTF-8 BOM. Notepad's "UTF-8 with
/// BOM" and PowerShell JSON round-trips prepend EF BB BF, which
/// serde_json::from_slice rejects; without stripping it we'd fall back to
/// defaults and silently wipe the user's hand-edited config (profiles /
/// devices / keys). Shared by load() AND both import paths so a BOM-prefixed
/// export imports identically (audit: import_* used to reject what load() took).
fn parse_config_bytes(raw: &[u8]) -> Result<Config> {
    let bytes = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    serde_json::from_slice(bytes).map_err(|e| {
        // SECURITY: do NOT surface serde_json's Display — on an `invalid type` /
        // trailing-garbage error it echoes the offending TOKEN verbatim, and
        // config.json interleaves live secrets (ai_bearer / groq_api_key /
        // ai_base_url) with the numeric/bool fields. Keep only the secret-free
        // location so a bad hand-edit is still locatable. Shared by load() AND
        // the import paths, so every parse error is sanitised at the root.
        anyhow::anyhow!(
            "config JSON is not valid (line {}, column {})",
            e.line(),
            e.column()
        )
    })
}

/// P1.4 — best-effort preserve an unparseable config.json before [`load`] falls
/// back to defaults. A corrupt file, a bad hand-edit, or a truncated write from
/// an old pre-atomic-save build must never silently destroy the user's live
/// keys / profiles / devices: we RENAME the bytes aside to
/// `config.json.broken-<unix_secs>` (off the load path but recoverable) and only
/// then continue with defaults. All failures are logged and swallowed — load()
/// must still return so the app can start.
fn preserve_corrupt_config(path: &std::path::Path) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = path.with_extension(format!("json.broken-{ts}"));
    match std::fs::rename(path, &backup) {
        Ok(()) => log::warn!("preserved corrupt config as {}", backup.display()),
        Err(e) => log::warn!("could not preserve corrupt config ({e})"),
    }
}

pub fn load() -> Config {
    let path = match config_path() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("config dir unavailable ({e}), using defaults");
            return Config::defaults();
        }
    };
    let mut cfg = match std::fs::read(&path) {
        Ok(raw) => match parse_config_bytes(&raw) {
            Ok(cfg) => cfg,
            Err(_) => {
                // The file exists but won't parse. Preserve it (P1.4) instead of
                // letting the next save() overwrite the user's keys / profiles.
                // SECURITY: the error is intentionally dropped, not logged — even
                // post-sanitisation we keep the corrupt-path log (emitted by
                // preserve_corrupt_config) as the only breadcrumb, so no config
                // byte can ever reach the log on this secrets-bearing file.
                preserve_corrupt_config(&path);
                log::warn!("config parse failed; preserved a copy, using defaults");
                Config::defaults()
            }
        },
        // Fresh install — no config yet. Expected, so don't cry wolf.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::defaults(),
        Err(e) => {
            log::warn!("config read failed ({e}), using defaults");
            Config::defaults()
        }
    };
    let mut dirty = false;
    // NB: we deliberately do NOT re-seed default snippets when the list is
    // empty. A fresh install already gets them via Config::defaults() on the
    // error arms above; re-seeding on every empty list clobbered a user who had
    // intentionally deleted all their snippets (and rewrote config.json on
    // every launch). To restore the canned set, use Settings → reset. (#134)
    // Migrate a pre-profiles config: if the user already has a meeting_context
    // but no named profiles, seed it as their first profile so the new
    // multi-profile picker has something to show + select. Non-destructive: the
    // live meeting_context is unchanged, just mirrored into a profile.
    if cfg.context_profiles.is_empty() && !cfg.meeting_context.trim().is_empty() {
        cfg.context_profiles.push(ContextProfile {
            name: DEFAULT_PROFILE_NAME.to_string(),
            context: cfg.meeting_context.clone(),
        });
        cfg.active_profile = Some(DEFAULT_PROFILE_NAME.to_string());
        log::info!("migrated meeting_context into a default profile");
        dirty = true;
    }
    // P1.3 — schema-versioning anchor. Stamp the file with the current schema
    // version so a FUTURE release can detect an older layout (config_version <
    // CURRENT) and run a one-time, number-keyed migration right here. Every
    // field today is additive (serde fills missing ones from defaults), so the
    // only action now is to stamp; the hook exists for the first breaking change.
    if cfg.config_version < CURRENT_CONFIG_VERSION {
        cfg.config_version = CURRENT_CONFIG_VERSION;
        dirty = true;
    }
    if dirty {
        // v0.17.1 (мега-аудит): was a silent `let _` — a failed migration save
        // (full disk, permissions, AV lock) lost the stamped changes with zero
        // trace. Non-fatal either way (the in-memory cfg is correct for this
        // run), but now the tester log says WHY settings didn't persist.
        if let Err(e) = save(&cfg) {
            log::warn!("config migration save failed (changes not persisted): {e:#}");
        }
    }
    cfg
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    let bytes = serde_json::to_vec_pretty(cfg)?;
    // Atomic write: stream into a sibling temp file, then rename over the
    // target. A rename within the same directory is atomic on NTFS, so a crash
    // / power loss / full disk mid-write leaves either the OLD complete config
    // or the NEW complete one on disk — never a truncated file that the next
    // load() would fail to parse and silently replace with Config::defaults()
    // (wiping the user's live keys / profiles / devices / hotkeys). On Windows
    // std::fs::rename overwrites the destination (MoveFileEx replace-existing).
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &bytes).context("write config (tmp)")?;
    // Keep ONE generation of the previous on-disk config as config.json.bak
    // before we replace it. Atomic-save already guarantees we never see a torn
    // file; the .bak adds a manual escape hatch when a future *valid* write
    // stores something the user wants to undo (a bad import, a fat-fingered
    // settings change). Best-effort: a missing source (first-ever save) or a
    // copy error must not fail the real save.
    if path.exists() {
        let bak = path.with_extension("json.bak");
        // fs-audit — the .bak is a SINGLE never-pruned generation, so a raw copy
        // left the user's groq_api_key / ai_bearer in plaintext on disk forever
        // (and a key rotation left the OLD key behind in it). Snapshot a
        // SECRET-REDACTED clone of the previous config instead: undo still
        // restores profiles / hotkeys / devices / settings; the live secrets stay
        // only in config.json (where the user can re-enter them). Best-effort —
        // any read / parse / serialize error just skips the .bak, exactly as the
        // prior copy-error path did.
        match std::fs::read(&path)
            .ok()
            .and_then(|b| parse_config_bytes(&b).ok())
            .and_then(|old| serde_json::to_vec_pretty(&secret_redacted(&old)).ok())
        {
            Some(redacted) => {
                if let Err(e) = std::fs::write(&bak, redacted) {
                    log::debug!("config .bak snapshot skipped ({e})");
                }
            }
            None => log::debug!("config .bak snapshot skipped (unreadable/unparseable)"),
        }
    }
    std::fs::rename(&tmp, &path).context("replace config")?;
    Ok(())
}

/// A clone of `cfg` with every at-rest secret blanked, for the `config.json.bak`
/// undo snapshot — so a never-pruned backup never holds plaintext credentials
/// (fs-audit). Undo still restores profiles / hotkeys / devices / settings; only
/// the bearer tokens + API key are dropped (they live in config.json, and the
/// user can re-enter them). Keep in sync with the secret `String` fields.
fn secret_redacted(cfg: &Config) -> Config {
    let mut c = cfg.clone();
    c.ai_bearer.clear();
    c.ai_local_bearer.clear();
    c.vision_bearer.clear();
    c.vision_local_bearer.clear();
    c.groq_api_key.clear();
    c.stt_whisper_bearer.clear();
    c
}

/// Phase E6 v28 — export the full config (INCLUDING ai_bearer +
/// groq_api_key) to an arbitrary path the user picks. Pretty JSON so
/// it's human-editable. The caller is responsible for warning that
/// the file contains secrets.
pub fn export_to(path: &std::path::Path, cfg: &Config) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(cfg).context("serialize config")?;
    std::fs::write(path, bytes).context("write export")?;
    Ok(())
}

/// Phase E6 v28 — import a config from an arbitrary path, validate by
/// deserializing into `Config` (unknown fields ignored, missing fields
/// filled by serde defaults), then persist to the canonical location.
/// Returns the imported Config so the caller can re-apply live state.
pub fn import_from(path: &std::path::Path) -> Result<Config> {
    let bytes = std::fs::read(path).context("read import file")?;
    let cfg: Config = parse_config_bytes(&bytes).context("parse import JSON")?;
    save(&cfg).context("persist imported config")?;
    Ok(cfg)
}

/// Server-only settings merge (#125): copy ONLY the AI/STT server fields
/// (providers, base URLs, models, bearer/API keys, prompt-cache + the local
/// model knobs) from `imported` onto a clone of `current`. EVERY local field —
/// profiles, audio devices, monitor pin, trigger keywords, hotkeys, UI
/// language, theme, snippets, etc. — is preserved from `current`. Pure (no IO)
/// so it's trivially testable; [`import_server_settings_from`] wraps it with
/// read + save.
#[must_use]
pub fn merge_server_settings(current: &Config, imported: Config) -> Config {
    let mut next = current.clone();
    // Cloud AI provider/endpoint.
    next.ai_provider = imported.ai_provider;
    next.ai_base_url = imported.ai_base_url;
    next.ai_bearer = imported.ai_bearer;
    next.ai_model = imported.ai_model;
    next.prep_model = imported.prep_model;
    next.ai_prompt_cache = imported.ai_prompt_cache;
    // Local AI provider/endpoint.
    next.ai_local_base_url = imported.ai_local_base_url;
    next.ai_local_bearer = imported.ai_local_bearer;
    next.ai_local_model = imported.ai_local_model;
    next.ai_local_prep_model = imported.ai_local_prep_model;
    next.ai_local_vision = imported.ai_local_vision;
    next.ai_local_thinking = imported.ai_local_thinking;
    // Vision channel (separate endpoint).
    next.vision_provider = imported.vision_provider;
    next.vision_base_url = imported.vision_base_url;
    next.vision_bearer = imported.vision_bearer;
    next.vision_model = imported.vision_model;
    next.vision_local_base_url = imported.vision_local_base_url;
    next.vision_local_bearer = imported.vision_local_bearer;
    next.vision_local_model = imported.vision_local_model;
    // NOTE: vision_phonetics is deliberately NOT transferred — it's a per-user
    // OUTPUT preference (append IPA to a translation), not a server endpoint, so it
    // stays machine-local like response_language / stt_language / ui_language. Do not
    // add it here: an import would then overwrite the local user's phonetics choice.
    // STT provider + per-backend server settings.
    next.stt_provider = imported.stt_provider;
    next.groq_api_key = imported.groq_api_key;
    next.stt_language = imported.stt_language;
    next.stt_model = imported.stt_model;
    next.stt_gigaam_dir = imported.stt_gigaam_dir;
    next.stt_gigaam_gpu = imported.stt_gigaam_gpu;
    next.stt_whisper_url = imported.stt_whisper_url;
    next.stt_whisper_bearer = imported.stt_whisper_bearer;
    next.stt_whisper_model = imported.stt_whisper_model;
    next
}

/// Server-only import from a user-picked file (#125). Reads + validates the
/// JSON (full `Config` shape — unknown fields ignored, missing filled by serde
/// defaults), merges ONLY the AI/STT server fields onto `current` via
/// [`merge_server_settings`], persists, and returns the merged Config. Lets a
/// user carry their AI/STT server setup to another PC WITHOUT clobbering that
/// PC's local profiles, devices, UI, hotkeys, or snippets — unlike the full
/// [`import_from`], which replaces the whole config.
pub fn import_server_settings_from(path: &std::path::Path, current: &Config) -> Result<Config> {
    let bytes = std::fs::read(path).context("read server settings import file")?;
    let imported: Config = parse_config_bytes(&bytes).context("parse server settings JSON")?;
    let next = merge_server_settings(current, imported);
    save(&next).context("persist imported server settings")?;
    Ok(next)
}

// ---------------------------------------------------------------------------
// P1.7 — server-settings EXPORT (server fields only) + import PREVIEW.
// ---------------------------------------------------------------------------

/// P1.7 — export ONLY the AI/STT/vision SERVER fields to a user-picked path.
/// Built from [`Config::defaults`] (so meeting_context, context_profiles,
/// snippets, audio devices, monitor pin, trigger keywords, UI/theme, etc. are
/// blank/default) with exactly the fields [`merge_server_settings`] copies
/// overlaid from `cfg`. Pretty JSON, human-editable.
///
/// SECURITY: this file DOES contain the server creds (`ai_bearer`,
/// `groq_api_key`, the vision/local bearers) and the private `ai_base_url` —
/// that is intentional (the whole point is transferring a server setup to
/// another PC). It must NOT contain `meeting_context`, `context_profiles`,
/// `snippets`, audio devices, or any other machine-local field — the caller is
/// responsible for warning the user the file holds secrets.
pub fn export_server_settings_to(path: &std::path::Path, cfg: &Config) -> Result<()> {
    // `merge_server_settings(current, imported)` copies the server fields of
    // `imported` onto a clone of `current`. Feed defaults as `current` and the
    // live config as `imported` → a Config whose ONLY non-default fields are the
    // server ones. Single source of truth for "what is a server field".
    let server_only = merge_server_settings(&Config::defaults(), cfg.clone());
    let bytes = serde_json::to_vec_pretty(&server_only).context("serialize server settings")?;
    std::fs::write(path, bytes).context("write server-settings export")?;
    Ok(())
}

/// One AI/STT/vision endpoint group in a redacted import preview. Carries
/// NEUTRAL fields only — provider tag, base URL, model — plus key PRESENCE as a
/// bool. NEVER the secret VALUE of a bearer / API key. Both the `*_old` and
/// `*_new` sides are populated so the UI can render an "old -> new" diff.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewGroup {
    /// Human-readable group label, already English (UI translates via @tr on
    /// the fixed label it pairs this data with; this string is data only and is
    /// not itself shown untranslated — kept for tests / logging).
    pub label: String,
    pub provider_old: String,
    pub provider_new: String,
    /// Base URL. May be a private LAN IP — shown because it already appears in
    /// the Settings URL fields. See [`mask_host`] for the copyable/loggable form.
    pub base_url_old: String,
    pub base_url_new: String,
    pub model_old: String,
    pub model_new: String,
    /// Whether this group HAS a credential (bearer / API key). PRESENCE only —
    /// the value is never carried. `false` for groups with no credential field
    /// (e.g. local AI when no bearer is set), which the UI renders as "—".
    pub key_present_old: bool,
    pub key_present_new: bool,
    /// True when this group exposes a credential field at all (so the UI can
    /// distinguish "no key field here" from "key field, currently empty").
    pub has_key_field: bool,
}

impl PreviewGroup {
    /// True when any visible field differs old -> new (drives a "changed" marker).
    #[must_use]
    pub fn changed(&self) -> bool {
        self.provider_old != self.provider_new
            || self.base_url_old != self.base_url_new
            || self.model_old != self.model_new
            || self.key_present_old != self.key_present_new
    }
}

/// Redacted, human-readable diff of what a server-settings import WOULD change,
/// produced by [`preview_server_settings`]. Contains NO secret value anywhere —
/// only provider tags, base URLs, models, and key-presence booleans, plus the
/// machine-local model paths flagged for review. The UI shows this and requires
/// an explicit Apply before any merge happens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerSettingsPreview {
    /// Cloud AI endpoint (`ai_*`).
    pub cloud_ai: PreviewGroup,
    /// Local AI endpoint (`ai_local_*`).
    pub local_ai: PreviewGroup,
    /// Vision channel (`vision_*` / `vision_local_*`, by provider).
    pub vision: PreviewGroup,
    /// STT — Groq cloud key + per-backend (`groq_api_key`, whisper server).
    pub stt: PreviewGroup,
    /// Machine-local GigaAM model dir on THIS PC (kept, not imported). Shown so
    /// the user knows the imported file's path was ignored. May be empty.
    pub gigaam_dir_current: String,
    /// The GigaAM dir the imported file CARRIED (informational only — NOT
    /// applied by the default Apply). May be empty.
    pub gigaam_dir_incoming: String,
}

/// Mask the host of a URL for a COPYABLE / loggable string, keeping the scheme,
/// port and path so it's still recognisable without leaking the private LAN IP
/// or hostname. `http://192.168.0.142:18902/v1` -> `http://***:18902/v1`. Empty
/// input -> empty output. Best-effort: anything it can't parse is returned with
/// the authority blanked rather than echoed.
#[must_use]
pub fn mask_host(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return String::new();
    }
    // Split off scheme:// if present.
    let (scheme, rest) = match url.find("://") {
        Some(i) => (&url[..i + 3], &url[i + 3..]),
        None => ("", url),
    };
    // authority is up to the first '/', the remainder is the path.
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    // Keep the :port suffix of the authority if any, blank the host.
    let port = authority.rfind(':').map(|i| &authority[i..]).unwrap_or("");
    format!("{scheme}***{port}{path}")
}

/// P1.7 — PURE, REDACTED preview of a server-settings import. For each endpoint
/// group (cloud AI, local AI, vision, STT) it reports provider / base_url /
/// model old->new and key PRESENCE (bool) old->new — NEVER a secret value — plus
/// the machine-local GigaAM model path (kept from THIS PC, not imported). No IO;
/// trivially testable. The UI renders this and requires an explicit Apply.
///
/// SECURITY: by construction this NEVER reads `*_bearer` / `groq_api_key` VALUES
/// into any output field — only `!value.trim().is_empty()` booleans. The redaction
/// guard test asserts no secret value appears in any produced string.
#[must_use]
pub fn preview_server_settings(current: &Config, imported: &Config) -> ServerSettingsPreview {
    let present = |s: &str| !s.trim().is_empty();
    ServerSettingsPreview {
        cloud_ai: PreviewGroup {
            label: "Cloud AI".into(),
            provider_old: current.ai_provider.clone(),
            provider_new: imported.ai_provider.clone(),
            base_url_old: current.ai_base_url.clone(),
            base_url_new: imported.ai_base_url.clone(),
            model_old: current.ai_model.clone(),
            model_new: imported.ai_model.clone(),
            key_present_old: present(&current.ai_bearer),
            key_present_new: present(&imported.ai_bearer),
            has_key_field: true,
        },
        local_ai: PreviewGroup {
            label: "Local AI".into(),
            provider_old: current.ai_provider.clone(),
            provider_new: imported.ai_provider.clone(),
            base_url_old: current.ai_local_base_url.clone(),
            base_url_new: imported.ai_local_base_url.clone(),
            model_old: current.ai_local_model.clone(),
            model_new: imported.ai_local_model.clone(),
            key_present_old: present(&current.ai_local_bearer),
            key_present_new: present(&imported.ai_local_bearer),
            has_key_field: true,
        },
        vision: PreviewGroup {
            label: "Vision".into(),
            provider_old: current.vision_provider.clone(),
            provider_new: imported.vision_provider.clone(),
            // Show the cloud vision URL/model by default; the local vision
            // fields fall back to it in vision_endpoint(), and showing both
            // would crowd the preview. The provider line tells the user which
            // one is active.
            base_url_old: current.vision_base_url.clone(),
            base_url_new: imported.vision_base_url.clone(),
            model_old: current.vision_model.clone(),
            model_new: imported.vision_model.clone(),
            key_present_old: present(&current.vision_bearer)
                || present(&current.vision_local_bearer),
            key_present_new: present(&imported.vision_bearer)
                || present(&imported.vision_local_bearer),
            has_key_field: true,
        },
        stt: PreviewGroup {
            label: "STT".into(),
            provider_old: current.stt_provider.clone(),
            provider_new: imported.stt_provider.clone(),
            // For STT the "base URL" that matters for a server transfer is the
            // local whisper server; Groq cloud has no user URL. Show whisper.
            base_url_old: current.stt_whisper_url.clone(),
            base_url_new: imported.stt_whisper_url.clone(),
            model_old: current.stt_model.clone(),
            model_new: imported.stt_model.clone(),
            // STT credentials: Groq API key OR the local whisper bearer.
            key_present_old: present(&current.groq_api_key) || present(&current.stt_whisper_bearer),
            key_present_new: present(&imported.groq_api_key)
                || present(&imported.stt_whisper_bearer),
            has_key_field: true,
        },
        gigaam_dir_current: current.stt_gigaam_dir.clone(),
        gigaam_dir_incoming: imported.stt_gigaam_dir.clone(),
    }
}

/// P1.7 — read + parse a picked file and build a REDACTED import preview
/// against `current`, returning the preview AND the parsed `Config` (so the
/// caller can stash it and apply on confirm — no save happens here). Reuses the
/// BOM-tolerant, value-free [`parse_config_bytes`], so a malformed file yields a
/// secret-free error. Mirrors [`import_server_settings_from`] but stops BEFORE
/// merging/persisting.
pub fn preview_server_settings_from(
    path: &std::path::Path,
    current: &Config,
) -> Result<(ServerSettingsPreview, Config)> {
    let bytes = std::fs::read(path).context("read server settings file")?;
    let imported: Config = parse_config_bytes(&bytes).context("parse server settings JSON")?;
    let preview = preview_server_settings(current, &imported);
    Ok((preview, imported))
}

/// P1.7 — apply a server-settings import the way the UI's "Apply" does:
/// [`merge_server_settings`] (all server fields) EXCEPT the machine-local
/// `stt_gigaam_dir`, which is kept from `current` because a GigaAM model
/// directory is a path on THIS PC and the imported value almost never exists
/// here (silently clobbering it would break local STT). Pure (no IO); the UI
/// wrapper persists via [`save`] and re-applies live state.
#[must_use]
pub fn apply_server_settings(current: &Config, imported: Config) -> Config {
    let mut next = merge_server_settings(current, imported);
    // Keep the machine-local GigaAM model path from THIS PC.
    next.stt_gigaam_dir = current.stt_gigaam_dir.clone();
    next
}

/// Global, thread-safe handle.
pub type SharedConfig = Arc<RwLock<Config>>;

pub fn shared() -> SharedConfig {
    Arc::new(RwLock::new(load()))
}

/// Wrap an in-memory [`Config`] as a [`SharedConfig`] WITHOUT touching the
/// on-disk config — for tests and callers that already hold a `Config`.
#[must_use]
pub fn shared_from(cfg: Config) -> SharedConfig {
    Arc::new(RwLock::new(cfg))
}

#[cfg(test)]
mod tests {
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
}
