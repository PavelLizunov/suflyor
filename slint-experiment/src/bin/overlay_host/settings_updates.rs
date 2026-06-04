//! In-app auto-update Settings tab: GitHub check + download-then-run installer
//! (P1 of `docs/overlay-host-gaps-and-next-checks.md` — splitting the
//! `settings_controller.rs` god-function by domain, the same way Phase 2's
//! `diagnostics.rs` and Wave 1-3's `settings_vision.rs` / `settings_stt.rs` /
//! `settings_ai.rs` were extracted).
//!
//! This module owns the Updates-tab wiring previously inlined in `open_settings`:
//! the GitHub release check (`on_check_updates_clicked`) and the download +
//! launch-installer-then-quit action (`on_install_update_clicked`). The blocks
//! moved here VERBATIM — same captures (only `win.as_weak()`; neither closure
//! reads `cfg`/`state`), same bodies, byte-for-byte identical behavior.
//! `open_settings` now only CALLS `wire_updates(&win)` where the two Updates
//! blocks sat.
//!
//! SECURITY (CRITICAL — unchanged by this mechanical move): this is a
//! download-then-execute path. The SHA-256 / allow-list verification lives in
//! `overlay_backend::update` (`download_installer` then `run_installer`); these
//! UI closures only CALL it, so the download -> backend verify -> spawn
//! sequence is byte-for-byte identical to before. On a spawn failure the app
//! does NOT quit (stays open + shows a generic message). The status/error
//! strings stay GENERIC (no `download_url` / path leak into a screen-shared
//! Settings window).
//!
//! NOTE: `diag!` is reached by textual macro scope (the parent defines it before
//! the `mod settings_updates;` declaration); only the crate-root types are
//! imported explicitly below.
use super::{ComponentHandle, SettingsWindow, SharedString};

/// Wire the Updates-tab Settings callbacks onto the Settings window. Moved
/// VERBATIM out of `open_settings` (P1 domain split) — same captures, same
/// behavior. Needs only `win`: both closures capture just `win.as_weak()` (the
/// GitHub check + the download/run installer run on a detached thread with a
/// local current-thread tokio runtime — `open_settings` has no `rt_handle` — and
/// neither reads `cfg` or `state`), so no extra params are threaded through.
pub(crate) fn wire_updates(win: &SettingsWindow) {
    // Phase E8 — in-app auto-update (Updates tab). Network calls run on a
    // detached thread with a local current-thread tokio runtime (same
    // pattern as the AI/STT test buttons — open_settings has no rt_handle).
    {
        let weak = win.as_weak();
        win.on_check_updates_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_update_checking(true);
            w.set_update_available(false);
            w.set_update_status(SharedString::from("Проверка GitHub…"));
            diag!("update: checking GitHub for newer release");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::check_latest(env!(
                        "CARGO_PKG_VERSION"
                    ))),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak2.upgrade() else {
                        return;
                    };
                    w.set_update_checking(false);
                    match res {
                        Ok(info) if info.newer && !info.download_url.is_empty() => {
                            w.set_update_download_url(SharedString::from(info.download_url));
                            w.set_update_available(true);
                            w.set_update_status(SharedString::from(format!(
                                "Доступна версия {} — нажмите «Обновить сейчас»",
                                info.latest_version
                            )));
                        }
                        Ok(info) if info.newer => w.set_update_status(SharedString::from(format!(
                            "Есть версия {}, но в релизе нет установщика",
                            info.latest_version
                        ))),
                        Ok(info) => w.set_update_status(SharedString::from(format!(
                            "У вас последняя версия ({})",
                            info.latest_version
                        ))),
                        Err(e) => {
                            w.set_update_status(SharedString::from(format!("Ошибка проверки: {e}")))
                        }
                    }
                });
            });
        });
    }
    {
        let weak = win.as_weak();
        win.on_install_update_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let url = w.get_update_download_url().to_string();
            if url.is_empty() {
                return;
            }
            w.set_update_checking(true);
            w.set_update_status(SharedString::from("Скачивание установщика…"));
            diag!("update: downloading installer");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::download_installer(&url)),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                match res {
                    Ok(path) => match overlay_backend::update::run_installer(&path) {
                        Ok(()) => {
                            // Installer launched — quit so it can overwrite the
                            // running binary (its first page is interactive, so
                            // the app is gone before it reaches the File step).
                            diag!("update: installer launched, quitting app");
                            let _ = slint::invoke_from_event_loop(|| {
                                let _ = slint::quit_event_loop();
                            });
                        }
                        Err(e) => {
                            // P0.3: the installer failed to spawn (blocked exe /
                            // deleted file). Do NOT quit — stay open + show why.
                            diag!("update: installer spawn FAILED: {e:#}");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak2.upgrade() {
                                    w.set_update_checking(false);
                                    w.set_update_status(SharedString::from(
                                        "Не удалось запустить установщик — приложение оставлено открытым (см. лог)",
                                    ));
                                }
                            });
                        }
                    },
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak2.upgrade() {
                                w.set_update_checking(false);
                                w.set_update_status(SharedString::from(format!(
                                    "Ошибка обновления: {e}"
                                )));
                            }
                        });
                    }
                }
            });
        });
    }
}
