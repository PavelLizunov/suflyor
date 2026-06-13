//! One-click local-AI installer Settings wiring (P1 of
//! `docs/overlay-host-gaps-and-next-checks.md` — splitting the
//! `settings_controller.rs` god-function by domain, the same way Phase 2's
//! `diagnostics.rs` and Wave 1-3's `settings_vision.rs` / `settings_stt.rs` /
//! `settings_ai.rs` were extracted).
//!
//! This module owns the local-AI installer wiring previously inlined in
//! `open_settings`: the install action (`on_install_local_ai_clicked` — runs the
//! whole download + launch pipeline on a worker thread, streams progress, and on
//! success stores the server handles for kill-on-quit, writes the local config,
//! and refreshes the dropdowns + the bar's active-stack readout) and the Cancel
//! action (`on_cancel_local_ai_clicked` — flips the shared cancel flag watched by
//! the worker thread and the curl poll loop). The blocks moved here VERBATIM —
//! same captures, same bodies, byte-for-byte identical behavior. `open_settings`
//! now only CALLS `wire_local_ai(&win, cfg, state, overlay_weak)` where the two
//! install blocks sat.
//!
//! SECURITY (CRITICAL — unchanged by this mechanical move): this is a
//! download-then-execute path. The SHA-256 / allow-list verification + the spawn
//! all live in `overlay_backend::local_ai` (`install` orchestrates
//! download -> verify -> launch internally); this UI closure only CALLS
//! `install(&opts, &cancel, &on)` and then `apply_result`, so the
//! download -> backend verify -> spawn sequence is byte-for-byte identical to
//! before. Progress / error strings stay GENERIC (no `base_url` / path leak into
//! a screen-shared Settings window); a cancel is reported as "Отменено.".
//!
//! NOTE: `diag!` is reached by textual macro scope (the parent defines it before
//! the `mod settings_local_ai;` declaration); only the crate-root items are
//! imported explicitly below (`active_stack_label` stays in `overlay_host.rs`).
use super::{active_stack_label, ComponentHandle, OverlayBarWindow, SettingsWindow, SharedString};

/// Wire the local-AI installer Settings callbacks onto the Settings window.
/// Moved VERBATIM out of `open_settings` (P1 domain split) — same captures, same
/// behavior. Beyond `win` + `cfg`, the install closure captures `state` (for the
/// shared `local_ai_cancel` flag, draining previously-launched servers, and
/// storing the new server handles) and `overlay_weak` (to refresh the bar's
/// active-stack readout on success); the Cancel closure captures `state` (to
/// flip the cancel flag). Those are threaded through as extra params, matching
/// the names `open_settings` used.
pub(crate) fn wire_local_ai(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    // E10.4 — one-click in-app local-AI installer. Runs the whole
    // download + launch pipeline on a worker thread, streams progress to
    // the panel, and on success stores the server handles (for kill-on-
    // quit), writes the local config (secrets preserved), and refreshes
    // the panel dropdowns + the bar's active-stack readout.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_install_local_ai_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_local_ai_installing() {
                return; // re-entry guard
            }
            w.set_local_ai_installing(true);
            w.set_local_ai_progress(0.0);
            w.set_local_ai_gpu(SharedString::from(""));
            w.set_local_ai_status(SharedString::from("Подготовка…"));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            // Shared cancel flag (lives in AppState so the Cancel button can
            // flip it); reset before each run.
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            std::thread::spawn(move || {
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_local_ai_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    let frac = if total > 0 {
                                        (done as f64 / total as f64) as f32
                                    } else {
                                        0.0
                                    };
                                    w.set_local_ai_progress(frac);
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_local_ai_status(SharedString::from(format!(
                                        "{label}: {:.0} / {:.0} MB",
                                        mb(done),
                                        mb(total)
                                    )));
                                }
                                overlay_backend::local_ai::Progress::Gpu(s) => {
                                    w.set_local_ai_on_gpu(s.starts_with("GPU"));
                                    w.set_local_ai_gpu(SharedString::from(s));
                                }
                            }
                        });
                    }
                };
                // Re-install hardening: stop any servers we previously launched
                // so a fresh `--mmproj` llama-server can bind :8080. Without this
                // a stale vision-less server keeps the port and the new one
                // silently fails to start (wait_ready still sees the old one and
                // reports success). Fresh installs have nothing to drain.
                let opts = overlay_backend::local_ai::InstallOptions::default();
                let old_servers = {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_servers.drain(..).collect::<Vec<_>>()
                };
                overlay_backend::local_ai::stop_managed_servers(&opts.root, old_servers);
                match overlay_backend::local_ai::install(&opts, &cancel, &on) {
                    Ok(res) => {
                        let model = res.ai_local_model.clone();
                        let gigaam_dir = res.stt_gigaam_dir.clone();
                        let on_gpu = res.on_gpu;
                        {
                            let mut c = cfg_t.write();
                            overlay_backend::local_ai::apply_result(&mut c, &res);
                            if let Err(e) = overlay_backend::config::save(&c) {
                                eprintln!("[overlay-host] local-ai config save failed: {e:#}");
                            }
                            overlay_backend::ai::set_local_no_think(!c.ai_local_thinking);
                        }
                        {
                            let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                            s.local_ai_servers.extend(res.servers);
                        }
                        let weak_done = weak_t.clone();
                        let overlay_done = overlay_t.clone();
                        let cfg_done = cfg_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            diag!("local-ai installed: model={} gpu={}", model, on_gpu);
                            if let Some(w) = weak_done.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_progress(1.0);
                                w.set_local_ai_status(SharedString::from(
                                    "Готово. Локальный AI настроен и запущен.",
                                ));
                                w.set_ai_provider_index(1);
                                w.set_ai_local_base_url_input(SharedString::from(
                                    overlay_backend::local_ai::LLAMA_BASE_URL,
                                ));
                                w.set_stt_provider_index(2);
                                w.set_stt_whisper_url_input(SharedString::from(
                                    overlay_backend::local_ai::WHISPER_BASE_URL,
                                ));
                                w.set_stt_gigaam_dir_input(SharedString::from(gigaam_dir));
                            }
                            if let Some(o) = overlay_done.upgrade() {
                                o.set_active_stack(SharedString::from(active_stack_label(
                                    &cfg_done.read(),
                                )));
                            }
                        });
                    }
                    Err(e) => {
                        let cancelled = e
                            .to_string()
                            .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                        let msg = if cancelled {
                            "Отменено.".to_string()
                        } else {
                            eprintln!("[overlay-host] local-ai install failed: {e:#}");
                            format!("Ошибка установки: {e}")
                        };
                        let weak_err = weak_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_err.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_status(SharedString::from(msg));
                            }
                        });
                    }
                }
            });
        });
    }

    // E10.4 — Cancel button: flip the shared cancel flag the install worker
    // thread + the curl poll loop watch. Shared with the 12B download (same
    // flag), so one Cancel button serves both long downloads.
    {
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_cancel_local_ai_clicked(move || {
            {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(w) = weak.upgrade() {
                w.set_local_ai_status(SharedString::from("Отмена…"));
                w.set_quality_status(SharedString::from("Отмена…"));
            }
        });
    }

    // v0.18.0 — "faster (4B) ↔ smarter (12B)" switch. Persists the choice, then
    // (off the UI thread) frees :8080 owner-aware and relaunches llama-server
    // with the OTHER GGUF. STT (:8081) is left alone. The 12B button is only
    // enabled when the file is present, so the relaunch can always find a model
    // (selected_llama_gguf falls back to 4B otherwise).
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_model_quality_changed(move |want_quality| {
            let Some(w) = weak.upgrade() else { return };
            // Re-entry guard (review #3): the Slint `enabled:` bindings only
            // block the SAME button, so a fast opposite-button click during the
            // relaunch could double-launch :8080. `model-switching` gates both.
            if w.get_model_switching() {
                return;
            }
            // No-op if already on the requested model (the active button is
            // disabled, but guard anyway).
            if w.get_ai_local_quality() == want_quality {
                return;
            }
            // UI-audit 2026-06-13 (IMPORTANT): do NOT flip ai_local_quality /
            // config optimistically. If the relaunch returns PortBusy/
            // FailedToStart, an optimistic flip would leave the "●" active
            // marker + the button enabled-states pointing at a model the server
            // is NOT running, while the status says "не выполнено". We commit
            // the flip (config + UI) ONLY on a confirmed Switched outcome below;
            // until then the UI keeps showing the previous (still-running) model.
            w.set_model_switching(true);
            w.set_quality_status(SharedString::from(if want_quality {
                "Переключаю на умную модель (12B)…"
            } else {
                "Переключаю на быструю модель (4B)…"
            }));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let root = overlay_backend::local_ai::default_root();
                let want_whisper = {
                    let c = cfg_t.read();
                    c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081")
                };
                // Backend frees :8080 owner-aware, relaunches with the chosen
                // GGUF, and POLLS until it answers — returning the honest
                // outcome (review #1/#2) instead of a blind "done".
                let (outcome, started) = overlay_backend::local_ai::switch_local_model(
                    &root,
                    want_quality,
                    want_whisper,
                );
                let switched = outcome == overlay_backend::local_ai::ModelSwitch::Switched;
                {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    // Drop dead handles (the old llama we just port-killed) so
                    // the vec doesn't grow each switch; keep the live ones
                    // (whisper) (review #4).
                    s.local_ai_servers
                        .retain_mut(|c| matches!(c.try_wait(), Ok(None)));
                    s.local_ai_servers.extend(started);
                }
                // Commit the choice ONLY on a confirmed switch: persist
                // ai_local_quality + the active-stack model name (the bar reads
                // cfg.ai_local_model; the request "model" field is ignored by
                // single-model llama.cpp). On failure nothing is persisted, so
                // the next launch still starts the model that's actually running.
                if switched {
                    let mut c = cfg_t.write();
                    c.ai_local_quality = want_quality;
                    c.ai_local_model =
                        overlay_backend::local_ai::active_local_model_name(&root, want_quality);
                    if let Err(e) = overlay_backend::config::save(&c) {
                        eprintln!("[overlay-host] quality switch save failed: {e:#}");
                    }
                }
                let weak_done = weak_t.clone();
                let overlay_done = overlay_t.clone();
                let cfg_done = cfg_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_done.upgrade() {
                        w.set_model_switching(false);
                        w.set_quality_status(SharedString::from(match outcome {
                            overlay_backend::local_ai::ModelSwitch::Switched => {
                                if want_quality {
                                    "Готово: умная модель (12B). Первый ответ может грузиться ~5 с."
                                } else {
                                    "Готово: быстрая модель (4B)."
                                }
                            }
                            overlay_backend::local_ai::ModelSwitch::PortBusy => {
                                "Порт :8080 занят другим процессом — переключение не выполнено."
                            }
                            overlay_backend::local_ai::ModelSwitch::FailedToStart => {
                                "Не удалось запустить модель — проверьте установку локального AI."
                            }
                        }));
                    }
                    if let Some(o) = overlay_done.upgrade() {
                        o.set_active_stack(SharedString::from(active_stack_label(
                            &cfg_done.read(),
                        )));
                    }
                });
            });
        });
    }

    // v0.18.0 — download the optional 12B on demand (it isn't bundled). Same
    // worker/progress pattern as the installer; the backend verifies the pinned
    // SHA-256 before the file is ever loaded. On success the 12B button appears;
    // the user taps it to switch (no auto-switch, so a background download can't
    // swap the model mid-call).
    {
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_download_quality_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_quality_downloading() {
                return; // re-entry guard
            }
            w.set_quality_downloading(true);
            w.set_quality_progress(0.0);
            w.set_quality_status(SharedString::from("Подготовка…"));
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_quality_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    let frac = if total > 0 {
                                        (done as f64 / total as f64) as f32
                                    } else {
                                        0.0
                                    };
                                    w.set_quality_progress(frac);
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_quality_status(SharedString::from(format!(
                                        "{label}: {:.0} / {:.0} MB",
                                        mb(done),
                                        mb(total)
                                    )));
                                }
                                overlay_backend::local_ai::Progress::Gpu(_) => {}
                            }
                        });
                    }
                };
                let root = overlay_backend::local_ai::default_root();
                let res = overlay_backend::local_ai::download_quality_model(&root, &cancel, &on);
                let weak_done = weak_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else { return };
                    w.set_quality_downloading(false);
                    match res {
                        Ok(()) => {
                            w.set_quality_progress(1.0);
                            w.set_quality_model_present(true);
                            w.set_quality_status(SharedString::from(
                                "Умная модель загружена. Нажмите «Умнее (12B)», чтобы включить.",
                            ));
                        }
                        Err(e) => {
                            let cancelled = e
                                .to_string()
                                .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                            if cancelled {
                                w.set_quality_status(SharedString::from("Отменено."));
                            } else {
                                eprintln!("[overlay-host] 12B download failed: {e:#}");
                                w.set_quality_status(SharedString::from(format!(
                                    "Ошибка загрузки: {e}"
                                )));
                            }
                        }
                    }
                });
            });
        });
    }
}
