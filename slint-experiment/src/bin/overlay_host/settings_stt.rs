//! STT (speech-to-text) Settings tab: provider switch + GigaAM/Whisper field
//! saves + the live connection test (P1 of `docs/overlay-host-gaps-and-next-checks.md`
//! — splitting the `settings_controller.rs` god-function by domain, the same way
//! Phase 2's `diagnostics.rs` and Wave 1's `settings_vision.rs` were extracted).
//!
//! This module owns the STT wiring previously inlined in `open_settings`: the
//! GigaAM GPU toggle (`on_stt_gigaam_gpu_changed` — which sat up in the AI-local
//! block region, not contiguous with the rest), the Groq/local connection test
//! (`on_stt_test_clicked`), the provider dropdown (`on_stt_provider_changed`),
//! the GigaAM model-dir save, and the Whisper url / bearer / model saves. The
//! blocks moved here VERBATIM — same captures (`cfg.clone()`, plus `win.as_weak()`
//! for the test), same bodies, byte-for-byte identical behavior. `open_settings`
//! now only CALLS `wire_stt_settings(&win, cfg)` where the main STT cluster was.
//!
//! NOT moved (different domain, left in `open_settings`): the Audio-device mic
//! callbacks `on_mic_device_selected` / `on_mic_test_clicked`.
//!
//! SECURITY (unchanged by this mechanical move): the STT test-result tile keeps
//! its GENERIC messages (`[ok] …` / `[err] …`, error chain capped at 90 chars) so
//! no `base_url` / LAN IP leaks into a screen-shared Settings window. The endpoint
//! resolves via `cfg.stt_backend()` exactly as before.
//!
//! NOTE: this extraction imports the parent crate-root via `use super::*;`
//! (reaching `SettingsWindow` / `SharedString` / the `diag!` macro / the
//! `overlay_backend` config + stt helpers). That is intentional for the move;
//! imports narrow in a later pass.
use super::{ComponentHandle, SettingsWindow, SharedString};

/// Wire the STT-tab Settings callbacks onto the Settings window. Moved VERBATIM
/// out of `open_settings` (P1 domain split) — same captures, same behavior.
/// Needs only `win` (for the closures + the test's `as_weak()`) and `cfg`
/// (cloned per closure); none of the STT blocks touch `state` / `overlay_weak`
/// / `slint_rt` / `rt_handle`, so no extra params are threaded through.
pub(crate) fn wire_stt_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) {
    {
        let cfg_c = cfg.clone();
        win.on_stt_gigaam_gpu_changed(move |on| {
            let mut c = cfg_c.write();
            c.stt_gigaam_gpu = on;
            let _ = overlay_backend::config::save(&c);
            // Apply immediately: update the global ORT accelerator + drop the
            // cached model so the next transcription reloads on the new backend.
            // (The live session pipeline reloads its own copy next session.)
            overlay_backend::stt::configure_gigaam_accelerator(on);
            overlay_backend::stt::reset_gigaam_cache();
        });
    }

    // Phase E6 v27 — STT (Groq) connection test. Same off-thread
    // pattern; hits the Groq /models endpoint with the saved key.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_stt_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_stt_test_result(SharedString::from("testing…"));
            let backend = cfg_c.read().stt_backend();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::stt::test_connection_backend(&backend)) {
                            Ok(s) => format!("[ok] {s}"),
                            Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                        }
                    }
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_stt_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Phase E10 — STT provider selector + local-engine fields.
    {
        let cfg_c = cfg.clone();
        win.on_stt_provider_changed(move |idx| {
            let provider = match idx {
                1 => "gigaam",
                2 => "whisper",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.stt_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_provider save failed: {e:#}");
                return;
            }
            diag!("stt_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_gigaam_dir_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_gigaam_dir = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_gigaam_dir save failed: {e:#}");
                return;
            }
            diag!("stt_gigaam_dir saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_url_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_url = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_url save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_url saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_bearer_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_bearer = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_bearer save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_bearer saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_model_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_model = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_model save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_model saved ({} chars)", trimmed.len());
        });
    }
}
