//! User config persisted as JSON in OS data dir.
//! Path: %APPDATA%\suflyor\config.json

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// The curated `/key` snippet pack (the ~1000-line default data literal) lives
/// in its own file to keep this module navigable; `default_snippets` is the
/// only item it exposes. Unit tests live in `config/tests.rs` (declared at the
/// bottom of this file).
mod repair;
mod snippets;
use snippets::default_snippets;

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

    /// Read-aloud (TTS, read-aloud feature): the SAPI voice id to speak with.
    /// Empty = auto-pick a Russian voice. A machine-local OUTPUT preference
    /// (like `vision_phonetics`) — deliberately NOT transferred on a
    /// server-settings import.
    #[serde(default)]
    pub tts_voice: String,

    /// Read-aloud speech rate (SAPI range −10…+10, 0 = normal). Machine-local.
    #[serde(default)]
    pub tts_rate: i32,

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

    /// When true, LIVE auto-tile hints are phrased as ready-to-read-aloud lines
    /// (no filler words, confident/assertive, short) so the user can voice them
    /// verbatim during a call. Independent of `post_meeting_debrief_enabled`.
    /// Default OFF — opt-in via the Coaching tab (Фича1).
    #[serde(default)]
    pub live_coaching_tiles_enabled: bool,

    /// v0.13.0 — record the raw session audio (mic + system, separate 16 kHz
    /// mono WAVs) under `%APPDATA%\suflyor\recordings\<session_id>\`. Kept
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

    /// "Reader mode" tile mute — suppress AUTO-detected AI tiles (the
    /// `TileKind::Ai` answers the detector spawns on each transcript line) so
    /// the user can record + just listen without tiles popping up. Manual asks
    /// (F6/F9/PTT), KB/snippets, summaries, and error tiles are NOT affected.
    /// Default OFF.
    #[serde(default)]
    pub suppress_tiles: bool,

    /// Collapse the wide (~1200px) overlay bar to a compact read-aloud pill
    /// (read-aloud status + Stop + Expand). For using the app purely as a
    /// text-to-speech reader. Persisted so the bar reopens in the chosen size.
    /// Default OFF (full bar).
    #[serde(default)]
    pub compact_bar: bool,

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

    /// ТЗ 2026-07-06 (C) — last user-dragged position of the text-ask input
    /// window (physical px, top-left). `None` = never dragged → centered as
    /// before. Validated against visible monitors on restore (a stale position
    /// from an unplugged monitor falls back to center).
    pub text_ask_pos: Option<(i32, i32)>,

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
            tts_voice: String::new(),
            tts_rate: 0,
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
            text_ask_pos: None,
            color_scheme: 0,
            tile_font_size: default_tile_font_size(),
            snippets: default_snippets(),
            post_meeting_debrief_enabled: default_post_meeting_debrief_enabled(),
            live_coaching_tiles_enabled: false,
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
            suppress_tiles: false,
            compact_bar: false,
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
    // Heal an externally-mangled config: a non-UTF-8 tool (Notepad "ANSI" save,
    // PowerShell without -Encoding utf8, a cp1252 paste) can round-trip
    // config.json through Windows-1252, leaving the profile context as
    // valid-UTF-8 mojibake ("**Ð Ð¾Ð»ÑŒ:**" = "**Роль:**") that strict-UTF-8 load
    // then accepts. Detect that exact signature and reverse it so the user never
    // sees garbled Cyrillic. Conservative — only repairs strings that reconstruct
    // to valid UTF-8 GAINING Cyrillic, so legitimate text is untouched.
    if let Some(fixed) = repair::repair_cp1252_mojibake(&cfg.meeting_context) {
        log::warn!("config: repaired cp1252-mojibaked meeting_context on load");
        cfg.meeting_context = fixed;
        dirty = true;
    }
    for prof in &mut cfg.context_profiles {
        if let Some(fixed) = repair::repair_cp1252_mojibake(&prof.context) {
            log::warn!("config: repaired cp1252-mojibaked context for a profile");
            prof.context = fixed;
            dirty = true;
        }
    }
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
    // Keep the :port suffix of the authority if any, blank the host. A bracketed
    // IPv6 literal ([fd00::abcd]) has colons INSIDE the brackets that are part of
    // the address, not a port — so for those keep a port ONLY when ':<digits>'
    // follows the closing ']'; otherwise nothing. (P0-2: a plain rfind(':') kept
    // ':abcd]' and leaked the IPv6 tail for a no-port bracketed host.)
    let port = if authority.starts_with('[') {
        match authority.find(']') {
            Some(close) => {
                let after = &authority[close + 1..];
                if after.starts_with(':')
                    && after.len() > 1
                    && after.as_bytes()[1..].iter().all(u8::is_ascii_digit)
                {
                    after
                } else {
                    ""
                }
            }
            None => "", // malformed (no closing bracket) — blank the whole authority
        }
    } else {
        authority.rfind(':').map(|i| &authority[i..]).unwrap_or("")
    };
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
mod tests;
