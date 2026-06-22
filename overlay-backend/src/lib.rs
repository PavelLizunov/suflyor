//! Tauri-free backend crate. See `Cargo.toml` for the migration
//! rationale. All 7 modules below were audited (Phase A2 agent run)
//! to have zero `tauri::*` imports and no Tauri-specific public-fn
//! parameters. They move verbatim from `src-tauri/src/` to this
//! crate's `src/`.

pub mod ai;
pub mod audio;
pub mod components;
pub mod config;
pub mod conspect;
pub mod events;
pub mod health;
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
pub mod session_names;
pub mod stt;
pub mod tts;
pub mod tts_install;
pub mod update;
pub mod vision;
