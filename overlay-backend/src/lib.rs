//! Tauri-free backend crate. See `Cargo.toml` for the migration
//! rationale. All 7 modules below were audited (Phase A2 agent run)
//! to have zero `tauri::*` imports and no Tauri-specific public-fn
//! parameters. They move verbatim from `src-tauri/src/` to this
//! crate's `src/`.

pub mod ai;
pub mod audio;
pub mod config;
pub mod events;
pub mod health;
pub mod journal;
pub mod kb;
pub mod runtime;
pub mod screenshot;
pub mod stt;
pub mod update;
