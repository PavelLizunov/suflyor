//! Tauri-free backend crate. See `Cargo.toml` for the migration
//! rationale. All 7 modules below were audited (Phase A2 agent run)
//! to have zero `tauri::*` imports and no Tauri-specific public-fn
//! parameters. They move verbatim from `src-tauri/src/` to this
//! crate's `src/`.

pub mod ai;
#[cfg(windows)]
pub mod audio;
// macOS seam: real microphone capture on the default input through the tiny
// AVAudioEngine bridge (native/macos/mic_capture.m). System audio stays
// unsupported there — logged once as degraded, never faked.
#[cfg(target_os = "macos")]
#[path = "audio_macos.rs"]
pub mod audio;
// Remaining non-Windows seam: same public `overlay_backend::audio` surface,
// but capture / device enumeration / recording entry points fail with
// explicit unsupported errors (no OS calls, no fake success).
#[cfg(all(not(windows), not(target_os = "macos")))]
#[path = "audio_unavailable.rs"]
pub mod audio;
pub mod audio_metrics;
// WASAPI route-change watcher; only the Windows audio.rs consumes it.
#[cfg(windows)]
pub(crate) mod audio_route;
pub mod bridge;
pub mod capabilities;
pub mod codex_subscription;
pub mod components;
pub mod config;
pub mod conspect;
pub mod credentials;
pub mod deep_lock;
pub mod diar_install;
pub mod diarize;
pub(crate) mod download;
pub mod events;
pub mod health;
pub mod hermes_install;
pub(crate) mod http_log;
pub mod journal;
pub mod kb;
pub mod local_ai;
pub mod memory;
pub mod ocr;
pub mod ocr_install;
pub mod paths;
pub mod persistence;
pub mod re_transcribe;
pub mod recorder;
pub mod runtime;
pub mod session_admin;
pub mod session_audio;
pub mod session_names;
pub mod stt;
pub mod summary_source;
pub mod teratts_install;
pub mod text;
pub mod tts;
pub mod tts_install;
pub mod tts_normalize;
pub mod update;
pub mod vision;
