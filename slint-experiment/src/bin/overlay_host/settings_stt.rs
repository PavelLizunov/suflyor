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

/// The two Groq cloud model ids exposed in the STT tab combobox.
/// Index 0 = turbo (fast, recommended), index 1 = large-v3 (more accurate).
pub(crate) const CLOUD_MODELS: [&str; 2] = ["whisper-large-v3-turbo", "whisper-large-v3"];

/// Map a cloud-model combobox index to the Groq model id.
/// Unknown indices fall back to the recommended turbo model.
pub(crate) fn cloud_model_from_index(idx: i32) -> &'static str {
    match idx {
        1 => CLOUD_MODELS[1],
        _ => CLOUD_MODELS[0],
    }
}

/// Map a persisted Groq model id back to the combobox index.
/// Only the two supported values are matched; anything else (empty,
/// legacy, hand-edited) maps to 0 so the UI shows a valid selection.
pub(crate) fn cloud_model_index(model: &str) -> i32 {
    match model {
        "whisper-large-v3" => 1,
        _ => 0,
    }
}

/// Map the persisted provider to the shared dropdown index.
pub(crate) fn stt_provider_index(provider: &str) -> i32 {
    match provider {
        "gigaam" => 1,
        "whisper" => 2,
        _ => 0,
    }
}

/// Map the platform-specific dropdown index back to a persisted provider.
pub(crate) fn stt_provider_from_index(idx: i32) -> &'static str {
    match idx {
        1 => "gigaam",
        2 => "whisper",
        _ => "cloud",
    }
}

fn update_cloud_model<E>(
    config: &mut overlay_backend::config::Config,
    idx: i32,
    save: impl FnOnce(&overlay_backend::config::Config) -> Result<(), E>,
) -> Result<(), E> {
    let previous = std::mem::replace(
        &mut config.stt_model,
        cloud_model_from_index(idx).to_string(),
    );
    if let Err(error) = save(config) {
        config.stt_model = previous;
        return Err(error);
    }
    Ok(())
}

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
            let provider = stt_provider_from_index(idx);
            let mut c = cfg_c.write();
            c.stt_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_provider save failed: {e:#}");
                return;
            }
            diag!("stt_provider -> {provider}");
        });
    }
    // Cloud recognition model (stt_model): 0=turbo (fast), 1=large-v3 (accurate).
    // Handy-style rollback: on save failure the in-memory config value AND the
    // visible combobox selection revert to the previous state.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_stt_cloud_model_changed(move |idx| {
            let mut c = cfg_c.write();
            if let Err(e) = update_cloud_model(&mut c, idx, overlay_backend::config::save) {
                let prev_idx = cloud_model_index(&c.stt_model);
                drop(c);
                eprintln!("[overlay-host] stt_model save failed: {e:#}");
                if let Some(w) = weak.upgrade() {
                    w.set_stt_cloud_model_index(prev_idx);
                }
                return;
            }
            diag!("stt_model -> {}", cloud_model_from_index(idx));
        });
    }
    {
        // Recognition language (stt_language): 0=auto, 1=ru, 2=en. None = let the
        // engine auto-detect per phrase (Whisper/Groq); a forced language pins it.
        let cfg_c = cfg.clone();
        win.on_stt_language_changed(move |idx| {
            let lang = match idx {
                1 => Some("ru".to_string()),
                2 => Some("en".to_string()),
                _ => None,
            };
            let mut c = cfg_c.write();
            c.stt_language = lang.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_language save failed: {e:#}");
                return;
            }
            diag!("stt_language -> {}", lang.as_deref().unwrap_or("auto"));
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn cloud_model_mapping_is_bounded() {
        for (idx, model) in CLOUD_MODELS.iter().enumerate() {
            assert_eq!(cloud_model_from_index(idx as i32), *model);
            assert_eq!(cloud_model_index(model), idx as i32);
        }
        assert_eq!(cloud_model_from_index(-1), "whisper-large-v3-turbo");
        assert_eq!(cloud_model_from_index(2), "whisper-large-v3-turbo");
        assert_eq!(cloud_model_index(""), 0);
        assert_eq!(cloud_model_index("bogus"), 0);
    }

    #[test]
    fn provider_mapping_is_shared_by_windows_and_macos() {
        assert_eq!(stt_provider_index("cloud"), 0);
        assert_eq!(stt_provider_index("gigaam"), 1);
        assert_eq!(stt_provider_index("whisper"), 2);
        assert_eq!(stt_provider_index("retired-provider"), 0);
        assert_eq!(stt_provider_from_index(0), "cloud");
        assert_eq!(stt_provider_from_index(1), "gigaam");
        assert_eq!(stt_provider_from_index(2), "whisper");
    }

    #[test]
    fn failed_cloud_model_save_rolls_back() {
        let mut config = overlay_backend::config::Config {
            stt_model: "whisper-large-v3-turbo".into(),
            ..Default::default()
        };

        let result = update_cloud_model(&mut config, 1, |_| Err("disk full"));

        assert_eq!(result, Err("disk full"));
        assert_eq!(config.stt_model, "whisper-large-v3-turbo");
    }
}
