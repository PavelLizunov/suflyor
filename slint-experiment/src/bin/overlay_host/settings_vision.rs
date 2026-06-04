//! Vision (screenshot) Settings tab: provider switch + field saves + live test
//! (P1 of `docs/overlay-host-gaps-and-next-checks.md` — splitting the
//! `settings_controller.rs` god-function by domain, the same way Phase 2's
//! `diagnostics.rs` was extracted).
//!
//! This module owns the V4 "vision channel" wiring previously inlined in
//! `open_settings`: the provider dropdown (`on_vision_provider_changed`), the
//! phonetics toggle, the cloud + local field saves (base_url / bearer / model),
//! and the `on_vision_test_clicked` connection test. The blocks moved here
//! VERBATIM — same captures (`cfg.clone()` + `win.as_weak()` for the test),
//! same bodies, byte-for-byte identical behavior. `open_settings` now only
//! CALLS `wire_vision_settings(&win, cfg)` at the same spot.
//!
//! SECURITY (unchanged by this mechanical move): the vision test-result tile
//! keeps its GENERIC messages (`[ok] …` / `[err] …` / `[--] vision is off`) so
//! no `base_url` / LAN IP leaks into a screen-shared Settings window. The
//! endpoint resolves via `cfg.vision_endpoint()` exactly as before.
//!
//! NOTE: this extraction imports the parent crate-root via `use super::*;`
//! (reaching `SettingsWindow` / `SharedString` / the `diag!` macro / the
//! `overlay_backend` config + ai helpers). That is intentional for the move;
//! imports narrow in a later pass.
use super::{ComponentHandle, SettingsWindow, SharedString};

/// Wire the Vision-tab Settings callbacks onto the Settings window. Moved
/// VERBATIM out of `open_settings` (P1 domain split) — same captures, same
/// behavior. Needs only `win` (for the closures + the test's `as_weak()`) and
/// `cfg` (cloned per closure); none of the Vision blocks touch `state` /
/// `slint_rt` / `rt_handle`, so no extra params are threaded through.
pub(crate) fn wire_vision_settings(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    // ===== V4 — vision (screenshot) channel: provider switch + field saves + test =====
    {
        let cfg_c = cfg.clone();
        win.on_vision_provider_changed(move |idx| {
            let provider = match idx {
                0 => "off",
                1 => "same",
                3 => "local",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.vision_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_provider save failed: {e:#}");
                return;
            }
            diag!("vision_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_phonetics_changed(move |on| {
            let mut c = cfg_c.write();
            c.vision_phonetics = on;
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_phonetics save failed: {e:#}");
                return;
            }
            diag!("vision_phonetics -> {on}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_model save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_model save failed: {e:#}");
            }
        });
    }
    {
        // Vision connection test — resolve the vision endpoint, reuse the AI
        // bridge tester. Off-thread so the HTTP round-trip can't freeze the UI.
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_vision_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_vision_test_result(SharedString::from("testing…"));
            let Some(ep) = cfg_c.read().vision_endpoint() else {
                w.set_vision_test_result(SharedString::from("[--] vision is off"));
                return;
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(
                        ep.base_url,
                        ep.bearer,
                        ep.model,
                    )) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_vision_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }
}
