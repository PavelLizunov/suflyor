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
        // v0.11.0 — Test Practice toggle for plain F8 (study / self-check).
        win.on_vision_test_practice_changed(move |on| {
            let mut c = cfg_c.write();
            c.vision_test_practice = on;
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_test_practice save failed: {e:#}");
                return;
            }
            diag!("vision_test_practice -> {on}");
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
        // Vision connection test — resolve the vision endpoint, then send the
        // SYNTHETIC test image (never the user's screen) so this actually
        // exercises the IMAGE path, not just text reachability. Off-thread so the
        // HTTP round-trip can't freeze the UI.
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
                    Ok(rt) => match rt.block_on(overlay_backend::vision::test_connection(
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

    // ===== OCR engine (Tesseract) — download-on-demand install button =====
    // Mirrors the voice installer: download + SHA-verify + extract on a worker
    // thread (the ~53 MB engine is NOT bundled). On success the OCR path
    // (Shift+Alt+2 / Ctrl+F8) starts using Tesseract instead of the VLM.
    {
        let weak = win.as_weak();
        win.on_ocr_install_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if w.get_ocr_installing() {
                return;
            }
            w.set_ocr_installing(true);
            w.set_ocr_install_phase(1); // preparing
            let weak_done = w.as_weak();
            std::thread::spawn(move || {
                let weak_cb = weak_done.clone();
                let on = move |p: overlay_backend::ocr_install::OcrProgress| {
                    use overlay_backend::ocr_install::OcrProgress;
                    // Semantic variant → phase int; the .slint renders the
                    // localized text via @tr (no label needed for OCR).
                    let phase: i32 = match p {
                        OcrProgress::Downloading => 2,
                        OcrProgress::Verifying => 3,
                        OcrProgress::Unpacking => 4,
                        OcrProgress::AlreadyInstalled => 5,
                        OcrProgress::Installed => 6,
                    };
                    let weak_in = weak_cb.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak_in.upgrade() {
                            w.set_ocr_install_phase(phase);
                        }
                    });
                };
                let result = overlay_backend::ocr_install::install(&on);
                if let Err(e) = &result {
                    diag!("[overlay-host] OCR install failed: {e:#}");
                }
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else {
                        return;
                    };
                    w.set_ocr_installing(false);
                    if result.is_err() {
                        w.set_ocr_install_phase(8); // generic failure
                    } else {
                        // Final phase was set by install() via the progress
                        // callback; just flip the installed flag so the button
                        // disappears.
                        w.set_ocr_installed(true);
                    }
                });
            });
        });
    }
}
