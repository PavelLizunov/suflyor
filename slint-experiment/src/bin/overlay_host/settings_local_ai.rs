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
#[cfg(any(windows, target_os = "linux"))]
use super::{
    active_stack_label, refresh_local_model_resource_warning, ComponentHandle, ModelRc,
    OverlayBarWindow, VecModel,
};
use super::{SettingsWindow, SharedString};

#[cfg(any(test, windows, target_os = "linux"))]
fn model_downloaded_label(
    model: overlay_backend::local_ai::ManagedModel,
    ru: bool,
) -> &'static str {
    use overlay_backend::local_ai::ManagedModel;
    match (model, ru) {
        (ManagedModel::Legacy4B, true) => "Быстрая модель (4B) загружена. Нажмите «Использовать» в её строке, чтобы включить.",
        (ManagedModel::Legacy4B, false) => "The fast model (4B) is downloaded. Click 'Use' in its row to activate it.",
        (ManagedModel::Fallback12B, true) => "Сбалансированная модель (12B QAT) загружена. Нажмите «Использовать» в её строке, чтобы включить.",
        (ManagedModel::Fallback12B, false) => "The balanced model (12B QAT) is downloaded. Click 'Use' in its row to activate it.",
        (ManagedModel::Primary26B, true) => "Основная модель 26B-A4B загружена. Нажмите «Использовать» в её строке, чтобы включить.",
        (ManagedModel::Primary26B, false) => "The primary model 26B-A4B is downloaded. Click 'Use' in its row to activate it.",
    }
}

pub(crate) fn refresh_local_context_controls(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::Config,
) {
    let root = overlay_backend::local_ai::default_root();
    let requested = overlay_backend::local_ai::ManagedModel::from_config(
        &cfg.ai_local_model,
        cfg.ai_local_quality,
    );
    let custom_name =
        overlay_backend::local_ai::custom_gguf_display_name(&cfg.ai_local_custom_gguf);
    let custom_active = !custom_name.is_empty();
    let model = overlay_backend::local_ai::effective_managed_model(&root, requested);
    let profile = if custom_active {
        overlay_backend::local_ai::HardwareModelProfile::Unknown
    } else {
        overlay_backend::local_ai::current_server_profile(model.is_quality())
    };
    let preset = overlay_backend::local_ai::LocalContextPreset::from_config(&cfg.ai_local_context);
    win.set_ai_local_quality(!custom_active && model.is_quality());
    win.set_ai_local_model_profile_index(if custom_active { -1 } else { model.index() });
    win.set_ai_local_custom_active(custom_active);
    win.set_ai_local_custom_model_name(SharedString::from(custom_name));
    win.set_ai_local_vision_available(overlay_backend::local_ai::local_vision_available(
        cfg, &root,
    ));
    win.set_legacy_model_present(overlay_backend::local_ai::legacy_model_present(&root));
    win.set_fallback_model_present(overlay_backend::local_ai::fallback_model_present(&root));
    win.set_ai_local_context_index(preset.index());
    refresh_local_context_preview(win, cfg, model, profile, preset, custom_active);
    #[cfg(any(windows, target_os = "linux"))]
    refresh_now_answering(win, cfg);
}

fn refresh_local_context_preview(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::Config,
    model: overlay_backend::local_ai::ManagedModel,
    profile: overlay_backend::local_ai::HardwareModelProfile,
    preset: overlay_backend::local_ai::LocalContextPreset,
    custom_active: bool,
) {
    win.set_ai_local_context_preview_index(preset.index());
    win.set_ai_local_hardware_profile_index(profile.index());
    win.set_ai_local_context_max_k((profile.context_tokens(false) / 1024) as i32);
    win.set_ai_local_context_auto_k(
        (overlay_backend::local_ai::LocalContextPreset::Auto.context_tokens(profile, false) / 1024)
            as i32,
    );
    let gib = f64::from(overlay_backend::local_ai::estimated_total_vram_mib(
        model, preset, profile,
    )) / 1024.0;
    let hint = if custom_active && cfg.ui_language == "ru" {
        "Для пользовательской GGUF-модели объём VRAM неизвестен; Auto использует безопасный контекст 8K.".to_string()
    } else if custom_active {
        "VRAM use is unknown for a user GGUF model; Auto uses a safe 8K context.".to_string()
    } else if cfg.ui_language == "ru" {
        format!(
            "Оценка до запуска: ~{gib:.1} ГиБ VRAM. Фактическое значение зависит от драйвера и GPU offload."
        )
    } else {
        format!(
            "Pre-launch estimate: ~{gib:.1} GiB VRAM. Actual use depends on the driver and GPU offload."
        )
    };
    win.set_ai_local_context_vram_hint(SharedString::from(hint));
}

/// Wire the local-AI installer Settings callbacks onto the Settings window.
/// Moved VERBATIM out of `open_settings` (P1 domain split) — same captures, same
/// behavior. Beyond `win` + `cfg`, the install closure captures `state` (for the
/// shared `local_ai_cancel` flag, draining previously-launched servers, and
/// storing the new server handles) and `overlay_weak` (to refresh the bar's
/// active-stack readout on success); the Cancel closure captures `state` (to
/// flip the cancel flag). Those are threaded through as extra params, matching
/// the names `open_settings` used.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn wire_local_ai(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    let pending_custom_gguf =
        std::sync::Arc::new(std::sync::Mutex::new(None::<std::path::PathBuf>));
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_now_refresh_requested(move || {
            if let Some(w) = weak.upgrade() {
                // A job finishing on the same endpoint must not replace Check.
                if w.get_ai_local_test_result().is_empty() {
                    refresh_now_answering(&w, &cfg_c.read());
                }
            }
        });
    }

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
                return; // re-entry guard (same window)
            }
            // Downloads do not resurrect the managed server under deep lock.
            // Keep the launch guard for the existing local-provider path.
            let download_only = {
                let c = cfg_c.read();
                if c.ai_provider == "local" && c.deep_lock {
                    let ru = c.ui_is_ru();
                    w.set_local_ai_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(ru),
                    ));
                    return;
                }
                c.ai_provider != "local"
            };
            // B3 — process-global dedup: the per-window bool above resets when
            // Settings is closed+reopened, so a 2nd click on a fresh window could
            // spawn a 2nd worker. This flag survives window reuse; the guard frees
            // it on every worker exit incl. panic.
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return; // another local-AI op is already running
            };
            w.set_local_ai_installing(true);
            w.set_local_ai_progress(0.0);
            w.set_local_ai_gpu(SharedString::from(""));
            w.set_local_ai_status(preparing_copy(cfg_c.read().ui_is_ru()));
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
                let _busy_guard = busy_guard; // frees local_ai_busy on exit incl. panic
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
                // Own the local-AI lifecycle lock for the WHOLE install so the
                // boot/watchdog auto-recovery can't race our free+relaunch of
                // :8080 (it try_locks and skips its tick while we hold it). RAII
                // release covers every exit path including a panic, so the lock
                // can never wedge.
                let lifecycle_lock = {
                    let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let Some(_ai_guard) =
                    overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                else {
                    return;
                };
                // Re-install hardening: stop any servers we previously launched
                // so a fresh `--mmproj` llama-server can bind :8080. Without this
                // a stale vision-less server keeps the port and the new one
                // silently fails to start (wait_ready still sees the old one and
                // reports success). Fresh installs have nothing to drain.
                // While answers stay on another provider, speech keeps its own
                // engine too: no local Whisper server is downloaded or wired.
                let mut opts = overlay_backend::local_ai::InstallOptions {
                    download_only,
                    skip_whisper: download_only,
                    ..Default::default()
                };
                let (restore_previous, restore_whisper, previous_choice) = {
                    let c = cfg_t.read();
                    opts.context =
                        overlay_backend::local_ai::LocalContextPreset::from_config(
                            &c.ai_local_context,
                        );
                    (
                        !download_only
                            && overlay_backend::local_ai::is_managed_llama_endpoint(
                                &c.ai_local_base_url,
                            )
                            && overlay_backend::local_ai::base_model_present(&opts.root),
                        !download_only
                            && c.stt_provider == "whisper"
                            && c.stt_whisper_url.contains(":8081"),
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &c.ai_local_model,
                            c.ai_local_quality,
                            &c.ai_local_custom_gguf,
                            opts.context,
                        ),
                    )
                };
                if !download_only {
                    let old_servers = {
                        let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                        std::mem::take(&mut s.local_ai_servers)
                    };
                    overlay_backend::local_ai::stop_managed_servers(&opts.root, old_servers);
                }
                match overlay_backend::local_ai::install(&opts, &cancel, &on) {
                    Ok(res) if download_only => {
                        {
                            let mut c = cfg_t.write();
                            overlay_backend::local_ai::apply_download_result(&mut c, &res);
                            if let Err(e) = overlay_backend::config::save(&c) {
                                eprintln!("[overlay-host] local-ai download config save failed: {e:#}");
                            }
                        }
                        let cfg_done = cfg_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_t.upgrade() {
                                let c = cfg_done.read();
                                w.set_local_ai_installing(false);
                                w.set_local_ai_progress(1.0);
                                w.set_local_ai_status(SharedString::from(if c.ui_is_ru() {
                                    "Загружено. Выберите «Модель на этом ПК» → «Использовать для ответов», чтобы отвечать с её помощью."
                                } else {
                                    "Downloaded. Choose «Model on this PC» → «Use for answers» to answer with it."
                                }));
                                w.set_quality_selection_allowed(
                                    overlay_backend::local_ai::primary_26b_allowed(res.hardware_profile),
                                );
                                w.set_quality_model_present(
                                    overlay_backend::local_ai::quality_model_present(&opts.root),
                                );
                                w.set_quality_vision_supported(
                                    overlay_backend::local_ai::quality_vision_supported(&opts.root),
                                );
                                w.set_quality_vision_present(
                                    overlay_backend::local_ai::quality_vision_present(&opts.root),
                                );
                                w.set_managed_local_server(
                                    overlay_backend::local_ai::is_managed_llama_endpoint(
                                        &c.ai_local_base_url,
                                    ),
                                );
                                w.set_ai_local_base_url_input(SharedString::from(
                                    c.ai_local_base_url.as_str(),
                                ));
                                refresh_local_context_controls(&w, &c);
                                refresh_local_engine_controls(&w, &c);
                            }
                        });
                    }
                    Ok(res) => {
                        let model = res.ai_local_model.clone();
                        let gigaam_dir = res.stt_gigaam_dir.clone();
                        let on_gpu = res.on_gpu;
                        let quality = res.ai_local_quality;
                        let quality_selection_allowed =
                            overlay_backend::local_ai::primary_26b_allowed(res.hardware_profile);
                        let quality_present =
                            overlay_backend::local_ai::quality_model_present(&opts.root);
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
                                    if cfg_done.read().ui_is_ru() {
                                        "Готово. Локальный AI настроен и запущен."
                                    } else {
                                        "Done. The local AI is set up and running."
                                    },
                                ));
                                w.set_ai_provider_index(1);
                                w.set_ai_local_base_url_input(SharedString::from(
                                    overlay_backend::local_ai::LLAMA_BASE_URL,
                                ));
                                w.set_managed_local_server(true);
                                w.set_ai_local_quality(quality);
                                w.set_quality_selection_allowed(quality_selection_allowed);
                                // The Settings window is reused. Replace a prior
                                // custom-server list so its selected model cannot
                                // disagree with the model this reinstall launched.
                                w.set_ai_local_models(ModelRc::new(VecModel::from(vec![
                                    SharedString::from(model.clone()),
                                ])));
                                w.set_ai_local_model_index(0);
                                w.set_quality_model_present(quality_present);
                                {
                                    let c = cfg_done.read();
                                    refresh_local_context_controls(&w, &c);
                                    refresh_local_engine_controls(&w, &c);
                                    w.set_ai_local_vision(c.ai_local_vision);
                                    w.set_vision_same_available(c.ai_local_vision);
                                    w.set_vision_provider_index(
                                        super::settings_vision::vision_provider_index_from_id(
                                            &c.vision_provider,
                                        ),
                                    );
                                }
                                refresh_local_model_resource_warning(
                                    &w,
                                    overlay_backend::local_ai::default_root(),
                                    overlay_backend::local_ai::LLAMA_BASE_URL.to_string(),
                                    model,
                                );
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
                        // A reinstall deliberately stops the old managed server
                        // before replacing files. If any later stage fails (or is
                        // cancelled), restore the last effective persisted model
                        // and keep its handles tracked instead of leaving local AI
                        // down until the next app restart.
                        let mut restored_servers = Vec::new();
                        let mut restored_label = None;
                        let mut restored_settings = None;
                        if restore_previous {
                            let (outcome, restored) =
                                overlay_backend::local_ai::restart_llama_server(
                                    &opts.root,
                                    previous_choice.clone(),
                                );
                            if matches!(
                                outcome,
                                overlay_backend::local_ai::ModelSwitch::Switched
                                    | overlay_backend::local_ai::ModelSwitch::FallbackStarted
                            ) {
                                restored_servers.extend(restored);
                                let mut c = cfg_t.write();
                                if outcome
                                    == overlay_backend::local_ai::ModelSwitch::FallbackStarted
                                {
                                    c.ai_local_quality = false;
                                }
                                if overlay_backend::local_ai::repair_managed_model_state_after_verification(
                                    &mut c,
                                    &opts.root,
                                ) {
                                    if let Err(save_error) = overlay_backend::config::save(&c) {
                                        eprintln!(
                                            "[overlay-host] restored local-AI state save failed: {save_error:#}"
                                        );
                                    }
                                }
                                restored_label = Some(active_stack_label(&c));
                                restored_settings = Some((
                                    c.ai_local_quality,
                                    c.ai_local_base_url.clone(),
                                    c.ai_local_model.clone(),
                                    c.ai_local_vision,
                                    c.vision_provider.clone(),
                                ));
                            } else {
                                overlay_backend::local_ai::terminate_servers(restored);
                            }
                        }
                        if restore_whisper {
                            restored_servers.extend(overlay_backend::local_ai::ensure_servers(
                                &opts.root,
                                false,
                                true,
                                previous_choice,
                            ));
                        }
                        if !download_only {
                            state_t
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .local_ai_servers
                                .extend(restored_servers);
                        }
                        if !e
                            .to_string()
                            .contains(overlay_backend::local_ai::CANCEL_SENTINEL)
                        {
                            eprintln!("[overlay-host] local-ai install failed: {e:#}");
                        }
                        let msg = local_ai_stop_copy(
                            &e,
                            cfg_t.read().ui_is_ru(),
                            ("Установить", "Install"),
                        );
                        let weak_err = weak_t.clone();
                        let overlay_err = overlay_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_err.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_status(SharedString::from(msg));
                                if let Some((
                                    quality,
                                    base_url,
                                    model,
                                    local_vision,
                                    vision_provider,
                                )) = restored_settings
                                {
                                    // A primary restore can downgrade to 12B.
                                    // Refresh the reused Settings window from
                                    // the persisted effective state so its
                                    // active profile, vision controls, model
                                    // list, and resource warning agree with
                                    // the server that is now running.
                                    w.set_ai_local_quality(quality);
                                    w.set_ai_local_model_profile_index(
                                        overlay_backend::local_ai::ManagedModel::from_config(
                                            &model, quality,
                                        )
                                        .index(),
                                    );
                                    w.set_ai_local_models(ModelRc::new(VecModel::from(vec![
                                        SharedString::from(model.clone()),
                                    ])));
                                    w.set_ai_local_model_index(0);
                                    w.set_ai_local_vision(local_vision);
                                    w.set_vision_same_available(local_vision);
                                    w.set_vision_provider_index(
                                        super::settings_vision::vision_provider_index_from_id(
                                            &vision_provider,
                                        ),
                                    );
                                    refresh_local_model_resource_warning(
                                        &w,
                                        opts.root.clone(),
                                        base_url,
                                        model,
                                    );
                                }
                            }
                            if let (Some(label), Some(o)) =
                                (restored_label, overlay_err.upgrade())
                            {
                                o.set_active_stack(SharedString::from(label));
                            }
                        });
                    }
                }
                // _ai_guard drops here → lifecycle lock released, watchdog re-armed.
            });
        });
    }

    // E10.4 — Cancel button: flip the shared cancel flag the install worker
    // thread + the curl poll loop watch. Shared with the 12B download (same
    // flag), so one Cancel button serves both long downloads.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_cancel_local_ai_clicked(move || {
            {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(w) = weak.upgrade() {
                let cancelling = SharedString::from(if cfg_c.read().ui_is_ru() {
                    "Отмена…"
                } else {
                    "Cancelling…"
                });
                w.set_local_ai_status(cancelling.clone());
                w.set_quality_status(cancelling);
            }
        });
    }

    {
        let pending = pending_custom_gguf.clone();
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_choose_custom_gguf_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if state_c
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .local_ai_busy
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return;
            }
            if cfg_c.read().deep_lock {
                w.set_now_notice(SharedString::from(
                    overlay_backend::deep_lock::lifecycle_guard_notice(cfg_c.read().ui_is_ru()),
                ));
                return;
            }
            let cfg_c = cfg_c.clone();
            let pending = pending.clone();
            let weak = w.as_weak();
            super::settings_dialogs::pick(
                rfd::AsyncFileDialog::new().add_filter("GGUF model", &["gguf"]),
                super::settings_dialogs::Kind::File,
                move |path| {
                    let (Some(w), Some(path)) = (weak.upgrade(), path) else { return };
                    if overlay_backend::local_ai::valid_custom_gguf_path(&path.to_string_lossy()).is_none()
                    {
                        w.set_quality_status(SharedString::from(if cfg_c.read().ui_is_ru() {
                            "Выбранный файл не является корректной GGUF-моделью."
                        } else {
                            "The chosen file is not a valid GGUF model."
                        }));
                        return;
                    }
                    if !w.get_managed_local_server() {
                        let choice = overlay_backend::local_ai::ManagedLlamaChoice::for_custom(
                            path,
                            overlay_backend::local_ai::LocalContextPreset::from_config(
                                &cfg_c.read().ai_local_context,
                            ),
                        );
                        *PENDING_BUILTIN_CHOICE
                            .lock()
                            .unwrap_or_else(|p| p.into_inner()) = Some(choice);
                        w.invoke_local_engine_activate(0);
                        return;
                    }
                    *pending.lock().unwrap_or_else(|p| p.into_inner()) = Some(path);
                    let weak_invoke = w.as_weak();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak_invoke.upgrade() {
                            w.invoke_model_profile_changed(-1);
                        }
                    });
                },
            );
        });
    }

    // Explicit bundled/user model switch. Persists the choice, then
    // (off the UI thread) frees :8080 owner-aware and relaunches llama-server
    // with the selected GGUF. STT (:8081) is left alone.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let pending_custom = pending_custom_gguf.clone();
        let weak = win.as_weak();
        win.on_model_profile_changed(move |index| {
            let Some(w) = weak.upgrade() else { return };
            let custom_path = if index < 0 {
                pending_custom
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
            } else {
                None
            };
            let want_model = overlay_backend::local_ai::ManagedModel::from_index(index);
            // Re-entry guard (review #3): the Slint `enabled:` bindings only
            // block the SAME button, so a fast opposite-button click during the
            // relaunch could double-launch :8080. `model-switching` gates both.
            if w.get_model_switching() {
                return;
            }
            // No-op if already on the requested model (the active button is
            // disabled, but guard anyway).
            if custom_path.is_none()
                && w.get_ai_local_model_profile_index() == want_model.index()
                && !w.get_ai_local_custom_active()
            {
                return;
            }
            if !w.get_managed_local_server() {
                return;
            }
            // Deep lock (v0.37): a model switch restarts :8080 — refuse while
            // the user wants the managed server unloaded.
            {
                let c = cfg_c.read();
                if c.deep_lock {
                    let ru = c.ui_is_ru();
                    w.set_quality_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(ru),
                    ));
                    return;
                }
            }
            let root = overlay_backend::local_ai::default_root();
            if custom_path.is_none()
                && !overlay_backend::local_ai::managed_model_present(&root, want_model)
            {
                w.set_quality_status(SharedString::from(if cfg_c.read().ui_is_ru() {
                    "Файл выбранной модели не установлен."
                } else {
                    "The chosen model file is not installed."
                }));
                return;
            }
            // UI-audit 2026-06-13 (IMPORTANT): do NOT flip ai_local_quality /
            // config optimistically. If the relaunch returns PortBusy/
            // FailedToStart, an optimistic flip would leave the "●" active
            // marker + the button enabled-states pointing at a model the server
            // is NOT running, while the status says "не выполнено". We commit
            // the flip (config + UI) ONLY on a confirmed Switched outcome below;
            // until then the UI keeps showing the previous (still-running) model.
            // B3 — process-global dedup (survives Settings reopen) + RAII release.
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return; // another local-AI op is already running
            };
            w.set_model_switching(true);
            let custom_name = custom_path.as_ref().and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            });
            let ru = cfg_c.read().ui_is_ru();
            w.set_quality_status(SharedString::from(
                if let Some(name) = custom_name.as_deref() {
                    if ru {
                        format!("Переключаю на пользовательскую модель {name}…")
                    } else {
                        format!("Switching to your model {name}…")
                    }
                } else {
                    match (want_model, ru) {
                        (overlay_backend::local_ai::ManagedModel::Legacy4B, true) => {
                            "Переключаю на быструю модель (4B)…".to_string()
                        }
                        (overlay_backend::local_ai::ManagedModel::Legacy4B, false) => {
                            "Switching to the fast model (4B)…".to_string()
                        }
                        (overlay_backend::local_ai::ManagedModel::Fallback12B, true) => {
                            "Переключаю на сбалансированную модель (12B QAT)…".to_string()
                        }
                        (overlay_backend::local_ai::ManagedModel::Fallback12B, false) => {
                            "Switching to the balanced model (12B QAT)…".to_string()
                        }
                        (overlay_backend::local_ai::ManagedModel::Primary26B, true) => {
                            "Переключаю на максимальную модель (26B-A4B)…".to_string()
                        }
                        (overlay_backend::local_ai::ManagedModel::Primary26B, false) => {
                            "Switching to the best model (26B-A4B)…".to_string()
                        }
                    }
                },
            ));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard; // frees local_ai_busy on exit incl. panic
                let root = overlay_backend::local_ai::default_root();
                // Own the local-AI lifecycle lock for the whole switch so the
                // boot/watchdog auto-recovery can't race our free+relaunch of
                // :8080 (it try_locks and skips while we hold it). RAII release
                // covers every exit path incl. panic.
                let lifecycle_lock = {
                    let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let Some(_ai_guard) =
                    overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                else {
                    return;
                };
                let (previous, target, want_whisper) = {
                    let c = cfg_t.read();
                    let context = overlay_backend::local_ai::LocalContextPreset::from_config(
                        &c.ai_local_context,
                    );
                    (
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &c.ai_local_model,
                            c.ai_local_quality,
                            &c.ai_local_custom_gguf,
                            context,
                        ),
                        custom_path.map_or_else(
                            || {
                                overlay_backend::local_ai::ManagedLlamaChoice::for_model(
                                    want_model, context,
                                )
                            },
                            |path| {
                                overlay_backend::local_ai::ManagedLlamaChoice::for_custom(
                                    path, context,
                                )
                            },
                        ),
                        c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081"),
                    )
                };
                // Backend frees :8080 owner-aware, relaunches with the chosen
                // GGUF, and POLLS until it answers — returning the honest
                // outcome (review #1/#2) instead of a blind "done".
                let (outcome, started) = overlay_backend::local_ai::switch_local_model(
                    &root,
                    previous,
                    target.clone(),
                    want_whisper,
                );
                let quality_present = overlay_backend::local_ai::quality_model_present(&root);
                let switched = overlay_backend::local_ai::switch_commits_choice(outcome);
                let serving = matches!(
                    outcome,
                    overlay_backend::local_ai::ModelSwitch::Switched
                        | overlay_backend::local_ai::ModelSwitch::RolledBack
                );
                let to_terminate = {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    // Reap only DEFINITIVELY-exited handles (Ok(Some)); keep
                    // running (Ok(None)) AND unknown (Err) so a live child is
                    // never lost from kill-on-quit tracking (review #3).
                    s.local_ai_servers
                        .retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
                    if serving {
                        s.local_ai_servers.extend(started);
                        Vec::new()
                    } else {
                        // Failed relaunch — don't track its dead/wedged children.
                        started
                    }
                };
                // Outside the state lock: kill the failed relaunch's children
                // (no-op when switched). No port sweep → whisper (:8081) is left
                // alone.
                overlay_backend::local_ai::terminate_servers(to_terminate);
                // Commit the choice ONLY on a confirmed switch: persist
                // ai_local_quality + the active-stack model name (the bar reads
                // cfg.ai_local_model; the request "model" field is ignored by
                // single-model llama.cpp). On failure nothing is persisted, so
                // the next launch still starts the model that's actually running.
                if switched {
                    let mut c = cfg_t.write();
                    overlay_backend::local_ai::apply_llama_choice(&mut c, &root, &target);
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
                        // A failed worker-side SHA review removes the rejected
                        // exact-size primary. Refresh this reused Settings window
                        // from disk so its normal download control is available.
                        if outcome
                            == overlay_backend::local_ai::ModelSwitch::TargetUnavailable
                        {
                            w.set_quality_model_present(quality_present);
                        }
                        let ru = cfg_done.read().ui_is_ru();
                        let status = if outcome
                            == overlay_backend::local_ai::ModelSwitch::Switched
                            && custom_name.is_some()
                        {
                            let name = custom_name.as_deref().unwrap_or_default();
                            if ru {
                                format!("Готово: пользовательская модель {name}.")
                            } else {
                                format!("Done: your model {name}.")
                            }
                        } else if !ru {
                            switch_outcome_en(outcome, want_model).to_string()
                        } else {
                            match outcome {
                                overlay_backend::local_ai::ModelSwitch::Switched => {
                                    match want_model {
                                    overlay_backend::local_ai::ManagedModel::Legacy4B => {
                                        "Готово: быстрая модель (4B)."
                                    }
                                    overlay_backend::local_ai::ManagedModel::Fallback12B => {
                                        "Готово: сбалансированная модель (12B QAT)."
                                    }
                                    overlay_backend::local_ai::ManagedModel::Primary26B => {
                                        "Готово: максимальная модель (26B-A4B)."
                                    }
                                    }
                                }
                                overlay_backend::local_ai::ModelSwitch::RolledBack => {
                                    "Новая модель не прошла проверку; предыдущая модель восстановлена."
                                }
                                overlay_backend::local_ai::ModelSwitch::PortBusy => {
                                    "Порт :8080 занят другим процессом — переключение не выполнено."
                                }
                                overlay_backend::local_ai::ModelSwitch::TargetUnavailable => {
                                    "Файл основной модели недоступен или не прошёл проверку целостности. Загрузите его заново."
                                }
                                overlay_backend::local_ai::ModelSwitch::HardwareUnsupported => {
                                    "26B-A4B доступна только для подтверждённой матрицы VRAM/RAM."
                                }
                                overlay_backend::local_ai::ModelSwitch::FallbackStarted => {
                                    "Основная модель не запустилась; включён RAM-safe fallback (12B QAT)."
                                }
                                overlay_backend::local_ai::ModelSwitch::FailedToStart => {
                                    "Не удалось запустить модель — проверьте установку локального AI."
                                }
                            }
                            .to_string()
                        };
                        w.set_quality_status(SharedString::from(status));
                        let (local_vision, vision_provider, base_url, model) = {
                            let c = cfg_done.read();
                            refresh_local_context_controls(&w, &c);
                            refresh_local_engine_state(&w, &c);
                            (
                                c.ai_local_vision,
                                c.vision_provider.clone(),
                                c.ai_local_base_url.clone(),
                                c.ai_local_model.clone(),
                            )
                        };
                        w.set_ai_local_models(ModelRc::new(VecModel::from(vec![
                            SharedString::from(model.clone()),
                        ])));
                        w.set_ai_local_model_index(0);
                        w.set_ai_local_vision(local_vision);
                        w.set_vision_same_available(local_vision);
                        w.set_vision_provider_index(
                            super::settings_vision::vision_provider_index_from_id(
                                &vision_provider,
                            ),
                        );
                        refresh_local_model_resource_warning(&w, root.clone(), base_url, model);
                    }
                    if let Some(o) = overlay_done.upgrade() {
                        o.set_active_stack(SharedString::from(active_stack_label(
                            &cfg_done.read(),
                        )));
                    }
                });
                // _ai_guard drops here → lifecycle lock released, watchdog re-armed.
            });
        });
    }

    // Slider movement is preview-only: update the estimate without persisting or
    // restarting. The Apply button calls `ai_local_context_changed` once.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_context_preview_changed(move |value| {
            let Some(w) = weak.upgrade() else { return };
            if w.get_model_switching() || !w.get_managed_local_server() {
                return;
            }
            let index = value.round().clamp(0.0, 5.0) as i32;
            if w.get_ai_local_context_preview_index() == index {
                return;
            }
            let c = cfg_c.read();
            let root = overlay_backend::local_ai::default_root();
            let requested = overlay_backend::local_ai::ManagedModel::from_config(
                &c.ai_local_model,
                c.ai_local_quality,
            );
            let model = overlay_backend::local_ai::effective_managed_model(&root, requested);
            let profile = overlay_backend::local_ai::HardwareModelProfile::from_index(
                w.get_ai_local_hardware_profile_index(),
            );
            refresh_local_context_preview(
                &w,
                &c,
                model,
                profile,
                overlay_backend::local_ai::LocalContextPreset::from_index(index),
                w.get_ai_local_custom_active(),
            );
        });
    }

    // Managed llama.cpp context preset. Auto stays compact; manual presets use
    // one fixed context for live + prep. A restart is transactional through the
    // same backend primitive as a model switch.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_ai_local_context_changed(move |index| {
            let Some(w) = weak.upgrade() else { return };
            if w.get_model_switching() || !w.get_managed_local_server() {
                return;
            }
            let target_context = overlay_backend::local_ai::LocalContextPreset::from_index(index);
            let profile = overlay_backend::local_ai::HardwareModelProfile::from_index(
                w.get_ai_local_hardware_profile_index(),
            );
            let max_k = profile.context_tokens(false) / 1024;
            let allowed = match target_context {
                overlay_backend::local_ai::LocalContextPreset::Auto => true,
                overlay_backend::local_ai::LocalContextPreset::K8 => max_k >= 8,
                overlay_backend::local_ai::LocalContextPreset::K16 => max_k >= 16,
                overlay_backend::local_ai::LocalContextPreset::K32 => max_k >= 32,
                overlay_backend::local_ai::LocalContextPreset::K64 => max_k >= 64,
                overlay_backend::local_ai::LocalContextPreset::K96 => max_k >= 96,
            };
            if !allowed {
                w.set_quality_status(SharedString::from(if cfg_c.read().ui_is_ru() {
                    "Этот контекст выше безопасного лимита текущего профиля."
                } else {
                    "This context is above the safe limit for the current profile."
                }));
                return;
            }
            let previous_context = {
                let c = cfg_c.read();
                overlay_backend::local_ai::LocalContextPreset::from_config(&c.ai_local_context)
            };
            if previous_context == target_context {
                return;
            }

            // If the live token count is unchanged (for example Auto 16K ->
            // fixed 16K), only the persisted mode changes; no restart needed.
            if previous_context.context_tokens(profile, false)
                == target_context.context_tokens(profile, false)
            {
                let mut c = cfg_c.write();
                let saved_context = c.ai_local_context.clone();
                c.ai_local_context = target_context.as_config().to_string();
                match overlay_backend::config::save(&c) {
                    Ok(()) => {
                        refresh_local_context_controls(&w, &c);
                        w.set_quality_status(SharedString::from(if c.ui_is_ru() {
                            "Контекст сохранён; новые запросы используют выбранный режим."
                        } else {
                            "Context saved; new requests use the chosen mode."
                        }));
                    }
                    Err(e) => {
                        c.ai_local_context = saved_context;
                        eprintln!("[overlay-host] local context save failed: {e:#}");
                        w.set_quality_status(SharedString::from(if c.ui_is_ru() {
                            "Не удалось сохранить настройку контекста."
                        } else {
                            "Could not save the context setting."
                        }));
                    }
                }
                return;
            }

            // Deep lock (v0.37): applying this context restarts :8080 — refuse
            // while the managed server must stay unloaded (the save-only path
            // above already returned without touching the server).
            {
                let c = cfg_c.read();
                if c.deep_lock {
                    let ru = c.ui_is_ru();
                    w.set_quality_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(ru),
                    ));
                    return;
                }
            }

            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return;
            };
            w.set_model_switching(true);
            w.set_quality_status(SharedString::from(if cfg_c.read().ui_is_ru() {
                "Применяю контекст и перезапускаю локальный AI…"
            } else {
                "Applying the context and restarting the local AI…"
            }));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard;
                let lifecycle_lock = {
                    let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let Some(_ai_guard) =
                    overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                else {
                    return;
                };
                let root = overlay_backend::local_ai::default_root();
                let (previous, want_whisper) = {
                    let c = cfg_t.read();
                    let previous_context =
                        overlay_backend::local_ai::LocalContextPreset::from_config(
                            &c.ai_local_context,
                        );
                    (
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &c.ai_local_model,
                            c.ai_local_quality,
                            &c.ai_local_custom_gguf,
                            previous_context,
                        ),
                        c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081"),
                    )
                };
                let target = previous.with_context(target_context);
                let (outcome, started) = overlay_backend::local_ai::switch_local_model(
                    &root,
                    previous,
                    target,
                    want_whisper,
                );
                let serving = matches!(
                    outcome,
                    overlay_backend::local_ai::ModelSwitch::Switched
                        | overlay_backend::local_ai::ModelSwitch::RolledBack
                        | overlay_backend::local_ai::ModelSwitch::FallbackStarted
                );
                let to_terminate = {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_servers
                        .retain_mut(|child| !matches!(child.try_wait(), Ok(Some(_))));
                    if serving {
                        s.local_ai_servers.extend(started);
                        Vec::new()
                    } else {
                        started
                    }
                };
                overlay_backend::local_ai::terminate_servers(to_terminate);

                {
                    let mut c = cfg_t.write();
                    if outcome == overlay_backend::local_ai::ModelSwitch::Switched {
                        c.ai_local_context = target_context.as_config().to_string();
                    } else if outcome == overlay_backend::local_ai::ModelSwitch::FallbackStarted {
                        c.ai_local_quality = false;
                        c.ai_local_context = target_context.as_config().to_string();
                        overlay_backend::local_ai::repair_managed_model_state_after_verification(
                            &mut c, &root,
                        );
                    }
                    if matches!(
                        outcome,
                        overlay_backend::local_ai::ModelSwitch::Switched
                            | overlay_backend::local_ai::ModelSwitch::FallbackStarted
                    ) {
                        if let Err(e) = overlay_backend::config::save(&c) {
                            eprintln!("[overlay-host] local context switch save failed: {e:#}");
                        }
                    }
                }

                let weak_done = weak_t.clone();
                let cfg_done = cfg_t.clone();
                let overlay_done = overlay_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_done.upgrade() {
                        w.set_model_switching(false);
                        let c = cfg_done.read();
                        w.set_ai_local_quality(c.ai_local_quality);
                        refresh_local_context_controls(&w, &c);
                        w.set_quality_status(SharedString::from(match outcome {
                            _ if !c.ui_is_ru() => context_outcome_en(outcome),
                            overlay_backend::local_ai::ModelSwitch::Switched => {
                                "Готово: контекст применён."
                            }
                            overlay_backend::local_ai::ModelSwitch::RolledBack => {
                                "Новый контекст не запустился; прежний режим восстановлен."
                            }
                            overlay_backend::local_ai::ModelSwitch::FallbackStarted => {
                                "26B не запустилась; включён безопасный fallback 12B."
                            }
                            overlay_backend::local_ai::ModelSwitch::PortBusy => {
                                "Порт :8080 занят другим процессом — контекст не изменён."
                            }
                            overlay_backend::local_ai::ModelSwitch::TargetUnavailable => {
                                "Файл основной модели недоступен — контекст не изменён."
                            }
                            overlay_backend::local_ai::ModelSwitch::HardwareUnsupported => {
                                "Профиль 26B не поддерживается на этом компьютере."
                            }
                            overlay_backend::local_ai::ModelSwitch::FailedToStart => {
                                "Не удалось перезапустить локальный AI."
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

    // Download EXACTLY the clicked bundled model (4B/12B/26B) on demand, on any
    // hardware. Same worker/progress pattern as the installer; the backend
    // verifies the pinned SHA-256 before the file is ever loaded. On success the
    // matching "Installed" button appears; the user taps it to switch (no
    // auto-switch, so a background download can't swap the model mid-call). The
    // generic full-stack installer (`install-local-ai-clicked`) stays separate.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_download_model_clicked(move |index| {
            let Some(w) = weak.upgrade() else { return };
            if w.get_quality_downloading() {
                return; // re-entry guard (same window)
            }
            // B3 — process-global dedup (survives Settings reopen) + RAII release.
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return; // another local-AI op is already running
            };
            let model = overlay_backend::local_ai::ManagedModel::from_index(index);
            let model_label = model.spec().label;
            w.set_local_download_profile(index);
            w.set_quality_downloading(true);
            w.set_quality_progress(0.0);
            let ru = cfg_c.read().ui_is_ru();
            w.set_quality_status(preparing_copy(ru));
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard; // frees local_ai_busy on exit incl. panic
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
                let res =
                    overlay_backend::local_ai::download_managed_model(&root, model, &cancel, &on);
                let weak_done = weak_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else { return };
                    w.set_quality_downloading(false);
                    w.set_local_download_profile(-1);
                    match res {
                        Ok(()) => {
                            w.set_quality_progress(1.0);
                            match model {
                                overlay_backend::local_ai::ManagedModel::Legacy4B => {
                                    w.set_legacy_model_present(true);
                                }
                                overlay_backend::local_ai::ManagedModel::Fallback12B => {
                                    w.set_fallback_model_present(true);
                                }
                                overlay_backend::local_ai::ManagedModel::Primary26B => {
                                    w.set_quality_model_present(true);
                                }
                            }
                            let done = model_downloaded_label(model, ru);
                            w.set_quality_status(SharedString::from(done));
                        }
                        Err(e) => {
                            if !e
                                .to_string()
                                .contains(overlay_backend::local_ai::CANCEL_SENTINEL)
                            {
                                eprintln!("[overlay-host] {model_label} download failed: {e:#}");
                            }
                            w.set_quality_status(SharedString::from(local_ai_stop_copy(
                                &e,
                                ru,
                                ("Скачать", "Download model"),
                            )));
                        }
                    }
                });
            });
        });
    }

    // Download the matching 26B vision projector. On success, relaunch :8080
    // transactionally so the projector is attached before F8 is enabled.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_download_vision12b_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_vision12b_downloading() {
                return; // re-entry guard (same window)
            }
            // B3 — process-global dedup (survives Settings reopen) + RAII release.
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return; // another local-AI op is already running
            };
            w.set_vision12b_downloading(true);
            w.set_quality_progress(0.0);
            w.set_vision12b_status(preparing_copy(cfg_c.read().ui_is_ru()));
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard; // frees local_ai_busy on exit incl. panic
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_vision12b_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    w.set_quality_progress(if total > 0 { done as f32 / total as f32 } else { 0.0 });
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_vision12b_status(SharedString::from(format!(
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
                let res = overlay_backend::local_ai::download_quality_vision(&root, &cancel, &on);
                // Hold the lifecycle lock ONLY for the restart so the long
                // download never blocks the watchdog. RAII release. A deep
                // lock keeps the server unloaded — the download still lands,
                // the restart is skipped (status offers the app-restart path).
                let restart_allowed = {
                    let c = cfg_t.read();
                    c.ai_provider == "local" && c.ai_local_quality && !c.deep_lock
                };
                let restarted = if res.is_ok() && restart_allowed {
                    let lifecycle_lock = {
                        let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                        s.local_ai_lock.clone()
                    };
                    let Some(_ai_guard) =
                        overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                    else {
                        return;
                    };
                    let (want_whisper, context) = {
                        let c = cfg_t.read();
                        (
                            c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081"),
                            overlay_backend::local_ai::LocalContextPreset::from_config(
                                &c.ai_local_context,
                            ),
                        )
                    };
                    let choice =
                        overlay_backend::local_ai::ManagedLlamaChoice::new(true, context);
                    let (outcome, started) =
                        overlay_backend::local_ai::switch_local_model(
                            &root,
                            choice.clone(),
                            choice,
                            want_whisper,
                        );
                    let ok = outcome == overlay_backend::local_ai::ModelSwitch::Switched;
                    {
                        let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                        s.local_ai_servers
                            .retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
                        if ok {
                            s.local_ai_servers.extend(started);
                        } else {
                            overlay_backend::local_ai::terminate_servers(started);
                        }
                    }
                    if ok {
                        let mut c = cfg_t.write();
                        overlay_backend::local_ai::set_local_vision(&mut c, &root, true);
                        if let Err(e) = overlay_backend::config::save(&c) {
                            eprintln!("[overlay-host] 26B vision state save failed: {e:#}");
                        }
                    }
                    ok
                } else {
                    false
                };
                let weak_done = weak_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else { return };
                    w.set_vision12b_downloading(false);
                    match res {
                        Ok(()) => {
                            w.set_quality_vision_present(true);
                            if cfg_t.read().ai_provider == "local" {
                                w.set_ai_local_vision_available(restarted);
                                w.set_ai_local_vision(restarted);
                                w.set_vision_same_available(restarted);
                            }
                            if restarted {
                                w.set_vision_provider_index(
                                    super::settings_vision::vision_provider_index_from_id("same"),
                                );
                            }
                            let ru = cfg_t.read().ui_is_ru();
                            let local = cfg_t.read().ai_provider == "local";
                            w.set_vision12b_status(SharedString::from(match (restarted, local, ru) {
                                (true, _, true) => "Зрение 26B включено — F8 теперь работает на 26B.",
                                (true, _, false) => "26B vision is on — F8 now uses 26B.",
                                (false, false, true) => "Проектор 26B загружен. Выберите локальную модель, чтобы использовать зрение.",
                                (false, false, false) => "The 26B projector is downloaded. Choose the local model to use vision.",
                                (false, true, true) => "Проектор 26B загружен. Перезапустите приложение, чтобы включить зрение.",
                                (false, true, false) => "The 26B projector is downloaded. Restart the app to turn on vision.",
                            }));
                        }
                        Err(e) => {
                            let cancelled = e
                                .to_string()
                                .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                            let ru = cfg_t.read().ui_is_ru();
                            if cancelled {
                                w.set_vision12b_status(SharedString::from(if ru {
                                    "Отменено."
                                } else {
                                    "Cancelled."
                                }));
                            } else {
                                eprintln!("[overlay-host] 26B vision projector download failed: {e:#}");
                                w.set_vision12b_status(SharedString::from(if ru {
                                    "Не удалось загрузить проектор зрения. Попробуйте ещё раз."
                                } else {
                                    "Could not download the vision projector. Try again."
                                }));
                            }
                        }
                    }
                });
            });
        });
    }

    // v0.18.2 — manual "Update engine": pull the latest llama.cpp, verify it runs
    // on this PC, then swap it in (verify-before-swap keeps a bad build from
    // breaking local AI). On a real update the live server was stopped, so we
    // relaunch it with the user's preferred model. Bypasses the weekly throttle.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        let overlay = overlay_weak.clone();
        win.on_update_engine_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_engine_updating() {
                return; // re-entry guard (same window)
            }
            // Deep lock (v0.37): a real engine swap stops :8080 and the
            // watchdog won't restart it while locked — refuse until unlocked.
            {
                let c = cfg_c.read();
                if c.deep_lock {
                    let ru = c.ui_is_ru();
                    w.set_engine_update_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(ru),
                    ));
                    return;
                }
            }
            // B3 — process-global dedup (survives Settings reopen) + RAII release.
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return; // another local-AI op is already running
            };
            w.set_engine_updating(true);
            w.set_engine_update_status(SharedString::from(if cfg_c.read().ui_is_ru() {
                "Проверяю обновление движка…"
            } else {
                "Checking for an engine update…"
            }));
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let weak_t = w.as_weak();
            let overlay_t = overlay.clone();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard; // frees local_ai_busy on exit incl. panic
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_engine_update_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_engine_update_status(SharedString::from(format!(
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
                // Own the lifecycle lock for the whole check/swap so the watchdog
                // can't race the binary swap. RAII release covers every path.
                let lifecycle_lock = {
                    let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let Some(_ai_guard) =
                    overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                else {
                    return;
                };
                let res = overlay_backend::local_ai::update_llama_engine(&root, &cancel, &on);
                overlay_backend::local_ai::mark_engine_update_checked(&root);
                // A real swap stopped :8080 — relaunch with the preferred model so
                // local AI stays up on the new engine.
                let restarted_model = if matches!(
                    res.as_ref(),
                    Ok(overlay_backend::local_ai::EngineUpdate::Updated { .. })
                ) {
                    let choice = {
                        let c = cfg_t.read();
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &c.ai_local_model,
                            c.ai_local_quality,
                            &c.ai_local_custom_gguf,
                            overlay_backend::local_ai::LocalContextPreset::from_config(
                                &c.ai_local_context,
                            ),
                        )
                    };
                    let (outcome, started) =
                        overlay_backend::local_ai::ensure_llama_serving(&root, choice);
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_servers
                        .retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
                    if matches!(
                        outcome,
                        overlay_backend::local_ai::ModelSwitch::Switched
                            | overlay_backend::local_ai::ModelSwitch::FallbackStarted
                    ) {
                        s.local_ai_servers.extend(started);
                    } else {
                        overlay_backend::local_ai::terminate_servers(started);
                    }
                    drop(s);
                    if matches!(
                        outcome,
                        overlay_backend::local_ai::ModelSwitch::Switched
                            | overlay_backend::local_ai::ModelSwitch::FallbackStarted
                    ) {
                        let mut c = cfg_t.write();
                        if outcome == overlay_backend::local_ai::ModelSwitch::FallbackStarted {
                            c.ai_local_quality = false;
                        }
                        if overlay_backend::local_ai::repair_managed_model_state_after_verification(
                            &mut c, &root,
                        ) {
                            if let Err(e) = overlay_backend::config::save(&c) {
                                eprintln!(
                                    "[overlay-host] engine-update local-AI state save failed: {e:#}"
                                );
                            }
                        }
                        Some((
                            c.ai_local_quality,
                            c.ai_local_base_url.clone(),
                            c.ai_local_model.clone(),
                            c.ai_local_vision,
                            c.vision_provider.clone(),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };
                let build = overlay_backend::local_ai::installed_engine_build(&root);
                let supported = overlay_backend::local_ai::quality_vision_supported(&root);
                let weak_done = weak_t.clone();
                let cfg_done = cfg_t.clone();
                let overlay_done = overlay_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else { return };
                    w.set_engine_updating(false);
                    if let Some(b) = build {
                        w.set_engine_build(SharedString::from(format!("b{b}")));
                    }
                    // The engine may now (or no longer) support managed vision.
                    w.set_quality_vision_supported(supported);
                    if let Some((quality, base_url, model, local_vision, vision_provider)) =
                        restarted_model
                    {
                        w.set_ai_local_quality(quality);
                        w.set_ai_local_model_profile_index(
                            overlay_backend::local_ai::ManagedModel::from_config(&model, quality)
                                .index(),
                        );
                        w.set_ai_local_models(ModelRc::new(VecModel::from(vec![
                            SharedString::from(model.clone()),
                        ])));
                        w.set_ai_local_model_index(0);
                        w.set_ai_local_vision(local_vision);
                        w.set_vision_same_available(local_vision);
                        w.set_vision_provider_index(
                            super::settings_vision::vision_provider_index_from_id(&vision_provider),
                        );
                        refresh_local_model_resource_warning(&w, root.clone(), base_url, model);
                    }
                    let ru = cfg_done.read().ui_is_ru();
                    let msg = match (res, ru) {
                        (Ok(overlay_backend::local_ai::EngineUpdate::UpToDate { .. }), true) => {
                            "Движок уже последней версии.".to_string()
                        }
                        (Ok(overlay_backend::local_ai::EngineUpdate::UpToDate { .. }), false) => {
                            "The engine is already the latest version.".to_string()
                        }
                        (Ok(overlay_backend::local_ai::EngineUpdate::Updated { to, .. }), true) => {
                            format!("Движок обновлён до b{to}.")
                        }
                        (
                            Ok(overlay_backend::local_ai::EngineUpdate::Updated { to, .. }),
                            false,
                        ) => {
                            format!("The engine is updated to b{to}.")
                        }
                        (Ok(overlay_backend::local_ai::EngineUpdate::Skipped { .. }), true) => {
                            "Обновление пропущено — оставлен текущий движок.".to_string()
                        }
                        (Ok(overlay_backend::local_ai::EngineUpdate::Skipped { .. }), false) => {
                            "Update skipped — the current engine stays.".to_string()
                        }
                        (Err(e), ru) => {
                            eprintln!("[overlay-host] engine update failed: {e:#}");
                            local_ai_stop_copy(&e, ru, ("Обновить движок", "Update engine"))
                        }
                    };
                    w.set_engine_update_status(SharedString::from(msg));
                    if let Some(o) = overlay_done.upgrade() {
                        o.set_active_stack(SharedString::from(active_stack_label(
                            &cfg_done.read(),
                        )));
                    }
                });
            });
        });
    }
}

/// "Preparing…" for the local-AI status lines.
#[cfg(any(windows, target_os = "linux"))]
fn preparing_copy(ru: bool) -> SharedString {
    SharedString::from(if ru {
        "Подготовка…"
    } else {
        "Preparing…"
    })
}

/// Status after a local-AI install, download or update stops: what happened and
/// which button to press next, in the UI language. Never echoes the raw chain
/// (it can carry a home path or URL in a screen-shared window); the full chain
/// goes to the log separately.
#[cfg(any(windows, target_os = "linux"))]
fn local_ai_stop_copy(e: &anyhow::Error, ru: bool, (button_ru, button_en): (&str, &str)) -> String {
    let chain = format!("{e:#}").to_ascii_lowercase();
    if chain.contains(overlay_backend::local_ai::CANCEL_SENTINEL) {
        return if ru {
            "Отменено."
        } else {
            "Cancelled."
        }
        .to_string();
    }
    if let Some(d) = e.downcast_ref::<overlay_backend::local_ai::DiskSpaceError>() {
        return if ru {
            format!(
                "Не хватает места на диске: нужно около {} ГБ, свободно {} ГБ. Освободите место и нажмите «{button_ru}» снова.",
                d.need_gib, d.free_gib
            )
        } else {
            format!(
                "Not enough disk space: about {} GB needed, {} GB free. Free up space, then press {button_en} again.",
                d.need_gib, d.free_gib
            )
        };
    }
    let has = |words: &[&str]| words.iter().any(|w| chain.contains(w));
    if has(&["curl not found", "curl.exe not found"]) {
        return match (ru, cfg!(windows)) {
            (true, true) => format!(
                "curl.exe отсутствует в PATH (входит в Windows 10 и новее). Добавьте его в PATH и нажмите «{button_ru}» снова."
            ),
            (false, true) => format!(
                "curl.exe is missing from PATH (included with Windows 10 and later). Add it to PATH, then press {button_en} again."
            ),
            (true, false) => format!(
                "curl не установлен. Установите curl и нажмите «{button_ru}» снова."
            ),
            (false, false) => format!(
                "curl is not installed. Install curl, then press {button_en} again."
            ),
        };
    }
    let (cause_ru, cause_en) = if has(&["sha-256", "sha256", "digest", "integrity"]) {
        (
            "Скачанный файл не прошёл проверку целостности",
            "The downloaded file failed its integrity check",
        )
    } else if has(&["archive", "extract"]) {
        (
            "Не удалось распаковать скачанный архив",
            "The downloaded archive could not be unpacked",
        )
    } else if has(&[
        "build asset",
        "build for this pc",
        "unsupported linux architecture",
    ]) {
        (
            "Для этого компьютера нет подходящей сборки движка",
            "There is no engine build for this computer",
        )
    } else if has(&[
        "did not become ready",
        "did not start",
        ":8080",
        "port busy",
    ]) {
        (
            "Локальный сервер не запустился",
            "The local server did not start",
        )
    } else if has(&[
        "curl",
        "http",
        "github",
        "attempts",
        "download",
        "timed out",
        "resolve",
    ]) {
        (
            "Загрузка прервалась — нет связи с сервером загрузки",
            "The download stopped — the download server is unreachable",
        )
    } else if ru {
        return format!(
            "Не получилось. Нажмите «{button_ru}», чтобы повторить; подробности — «Диагностика» → «Собрать логи»."
        );
    } else {
        return format!(
            "That failed. Press {button_en} to try again; details: Diagnostics → Collect logs."
        );
    };
    if ru {
        format!("{cause_ru}. Нажмите «{button_ru}», чтобы повторить.")
    } else {
        format!("{cause_en}. Press {button_en} to try again.")
    }
}

/// English model-switch outcome; the Russian copy stays at the call site.
#[cfg(any(windows, target_os = "linux"))]
fn switch_outcome_en(
    outcome: overlay_backend::local_ai::ModelSwitch,
    model: overlay_backend::local_ai::ManagedModel,
) -> &'static str {
    use overlay_backend::local_ai::{ManagedModel, ModelSwitch};
    match outcome {
        ModelSwitch::Switched => match model {
            ManagedModel::Legacy4B => "Done: the fast model (4B).",
            ManagedModel::Fallback12B => "Done: the balanced model (12B QAT).",
            ManagedModel::Primary26B => "Done: the best model (26B-A4B).",
        },
        ModelSwitch::RolledBack => "The new model failed its check; the previous model is back.",
        ModelSwitch::PortBusy => {
            "Port :8080 is used by another program — the switch did not happen."
        }
        ModelSwitch::TargetUnavailable => {
            "The main model file is missing or failed its integrity check. Download it again."
        }
        ModelSwitch::HardwareUnsupported => {
            "26B-A4B is available only on a confirmed VRAM/RAM setup."
        }
        ModelSwitch::FallbackStarted => {
            "The main model did not start; the RAM-safe fallback (12B QAT) is on."
        }
        ModelSwitch::FailedToStart => "The model did not start — check the local AI install.",
    }
}

/// English context-change outcome; the Russian copy stays at the call site.
#[cfg(any(windows, target_os = "linux"))]
fn context_outcome_en(outcome: overlay_backend::local_ai::ModelSwitch) -> &'static str {
    use overlay_backend::local_ai::ModelSwitch;
    match outcome {
        ModelSwitch::Switched => "Done: the context is applied.",
        ModelSwitch::RolledBack => "The new context did not start; the previous mode is back.",
        ModelSwitch::FallbackStarted => "26B did not start; the safe 12B fallback is on.",
        ModelSwitch::PortBusy => {
            "Port :8080 is used by another program — the context did not change."
        }
        ModelSwitch::TargetUnavailable => {
            "The main model file is missing — the context did not change."
        }
        ModelSwitch::HardwareUnsupported => "The 26B profile is not supported on this computer.",
        ModelSwitch::FailedToStart => "Could not restart the local AI.",
    }
}

/// Suflyor's own Strata steps in the UI language; Strata's setup output
/// (its own English lines) passes through unchanged.
#[cfg(any(windows, target_os = "linux"))]
fn strata_step_text(step: &str, ru: bool) -> String {
    if !ru {
        return step.to_string();
    }
    if step == "Starting Strata" {
        return "запуск".to_string();
    }
    if let Some(secs) = step
        .strip_prefix("Loading the model (")
        .and_then(|r| r.strip_suffix(" s)"))
    {
        return format!("модель загружается ({secs} с)");
    }
    if let Some(version) = step.strip_prefix("Downloading Strata ") {
        return format!("скачиваю Strata {version}");
    }
    step.to_string()
}

/// Status text for one Strata install/start progress event.
#[cfg(any(windows, target_os = "linux"))]
fn strata_progress_line(p: &overlay_backend::local_ai::Progress, ru: bool) -> String {
    match p {
        overlay_backend::local_ai::Progress::Step(s) => {
            format!("Strata: {}", strata_step_text(s, ru))
        }
        overlay_backend::local_ai::Progress::Bytes { label, done, total } => {
            let mb = |b: u64| (b as f64) / 1_048_576.0;
            if ru {
                format!("{label}: {:.0} / {:.0} МБ", mb(*done), mb(*total))
            } else {
                format!("{label}: {:.0} / {:.0} MB", mb(*done), mb(*total))
            }
        }
        _ => String::new(),
    }
}

// The engine switch commits a requested built-in choice only once it answers.
#[cfg(any(windows, target_os = "linux"))]
static PENDING_BUILTIN_CHOICE: std::sync::Mutex<
    Option<overlay_backend::local_ai::ManagedLlamaChoice>,
> = std::sync::Mutex::new(None);

/// A Strata the user installed before Suflyor: found on Settings open or
/// picked by folder. "Use" on its row adopts it.
static FOUND_STRATA: std::sync::Mutex<Option<overlay_backend::local_ai::FoundStrata>> =
    std::sync::Mutex::new(None);
/// Model servers found running on this PC, in row order after the Strata row.
static FOUND_SERVERS: std::sync::Mutex<Vec<overlay_backend::local_ai::RunningServer>> =
    std::sync::Mutex::new(Vec::new());
/// Bumped by a folder pick: a discovery that started earlier does not
/// replace the picked Strata when it finishes.
#[cfg(any(windows, target_os = "linux"))]
static FOUND_STRATA_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn listed_servers() -> Vec<overlay_backend::local_ai::RunningServer> {
    FOUND_SERVERS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

#[cfg(any(windows, target_os = "linux"))]
fn select_strata_folder(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    folder: &std::path::Path,
) {
    let weak = win.as_weak();
    let cfg = cfg.clone();
    let folder = folder.to_path_buf();
    std::thread::spawn(move || {
        let mut found = overlay_backend::local_ai::strata_in_folder(&folder);
        if found.is_empty() {
            if let Ok(entries) = std::fs::read_dir(&folder) {
                for sub in entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                    found.extend(overlay_backend::local_ai::strata_in_folder(&sub));
                }
            }
        }
        let first = found.into_iter().next();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(win) = weak.upgrade() else { return };
            let Some(first) = first else {
                win.set_strata_status(SharedString::from(if cfg.read().ui_is_ru() {
                    "Готовая Strata не найдена. Здесь нужна Linux-установка с .venv, serve/server.py и strata-<модель>.json. Папку весов Strata-data выберите в разделе «Папка моделей» выше."
                } else {
                    "No ready Strata found. Choose a Linux install with .venv, serve/server.py and strata-<model>.json. For Strata-data weights, use Model storage above."
                }));
                return;
            };
            FOUND_STRATA_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *FOUND_STRATA.lock().unwrap_or_else(|p| p.into_inner()) = Some(first);
            show_found_models(&win, &cfg.read());
            win.set_strata_status(SharedString::default());
            win.invoke_strata_choose(false);
        });
    });
}

/// The "Already on this PC" rows: the found Strata, then running servers.
/// Only folder names: Settings may be screen-shared.
fn show_found_models(win: &SettingsWindow, cfg: &overlay_backend::config::Config) {
    let ru = cfg.ui_is_ru();
    let strata = FOUND_STRATA
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let mut rows = Vec::new();
    if let Some(f) = &strata {
        let folder = f
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        rows.push(super::FoundModel {
            title: SharedString::from(f.model.clone()),
            detail: SharedString::from(if ru {
                format!("Ваша Strata · папка {folder}")
            } else {
                format!("Your Strata · folder {folder}")
            }),
            kind: 1,
            active: false,
        });
    }
    let mut external = false;
    for server in listed_servers() {
        let name = if server.name.is_empty() {
            if ru { "Сервер" } else { "Server" }.to_string()
        } else {
            server.name.clone()
        };
        let more = server.models.len().saturating_sub(1);
        let mut detail = if ru {
            format!("{name} · уже запущен · порт {}", server.port)
        } else {
            format!("{name} · already running · port {}", server.port)
        };
        if more > 0 {
            detail.push_str(&if ru {
                format!(" · ещё моделей: {more}")
            } else if more == 1 {
                " · 1 more model".to_string()
            } else {
                format!(" · {more} more models")
            });
        }
        // A Strata on :8090 is in use through the chosen folder's row when
        // one is chosen.
        let current = server.is_current(&cfg.ai_local_base_url, &cfg.ai_local_model)
            && !(server.strata_ready && overlay_backend::local_ai::adopted_strata().is_some());
        external |= current && server.strata_ready;
        rows.push(super::FoundModel {
            title: SharedString::from(server.pick_model(&cfg.ai_local_model)),
            detail: SharedString::from(detail),
            kind: 0,
            active: current,
        });
    }
    win.set_strata_external(external);
    win.set_found_models(super::ModelRc::new(super::VecModel::from(rows)));
}

/// A folder scan in flight: "Search again" does not start a second one.
#[cfg(any(windows, target_os = "linux"))]
static STRATA_SCANNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Look for running model servers and, unless one is chosen, a Strata
/// folder, off the UI thread. Each result lands on the UI thread on its own:
/// a slow disk does not hold back the servers.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn search_found_models(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    if win.get_found_models_searching() {
        return;
    }
    win.set_found_models_searching(true);
    let weak = win.as_weak();
    let cfg_t = cfg.clone();
    std::thread::spawn(move || {
        let servers: Vec<_> = overlay_backend::local_ai::find_running_servers()
            .into_iter()
            // Suflyor's own Strata already has its row in the Strata panel.
            .filter(|s| {
                !(overlay_backend::local_ai::strata_running()
                    && overlay_backend::local_ai::is_managed_strata_endpoint(&s.base_url))
            })
            .collect();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_found_models_searching(false);
            *FOUND_SERVERS.lock().unwrap_or_else(|p| p.into_inner()) = servers;
            let c = cfg_t.read();
            show_found_models(&w, &c);
            if w.get_local_connect_auto() {
                w.set_local_connect_open(
                    !overlay_backend::local_ai::is_managed_llama_endpoint(&c.ai_local_base_url)
                        && !overlay_backend::local_ai::is_managed_strata_endpoint(
                            &c.ai_local_base_url,
                        )
                        && !listed_servers()
                            .iter()
                            .any(|s| s.is_current(&c.ai_local_base_url, &c.ai_local_model)),
                );
            }
            // Discovery must not replace an explicit connection check.
            if w.get_ai_local_test_result().is_empty() {
                refresh_now_answering(&w, &c);
            }
            w.set_found_models_note(SharedString::from(if c.ui_is_ru() {
                "На этом ПК не найдено запущенных моделей и папок Strata."
            } else {
                "No running models or Strata folders found on this PC."
            }));
        });
    });
    if overlay_backend::local_ai::adopted_strata().is_some()
        || STRATA_SCANNING.swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let generation = FOUND_STRATA_GEN.load(std::sync::atomic::Ordering::SeqCst);
    let weak = win.as_weak();
    let cfg_t = cfg.clone();
    std::thread::spawn(move || {
        let root = overlay_backend::local_ai::default_root();
        let strata = overlay_backend::local_ai::find_existing_strata(&root)
            .into_iter()
            .next();
        STRATA_SCANNING.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_strata_adopted()
                || FOUND_STRATA_GEN.load(std::sync::atomic::Ordering::SeqCst) != generation
            {
                return;
            }
            *FOUND_STRATA.lock().unwrap_or_else(|p| p.into_inner()) = strata;
            show_found_models(&w, &cfg_t.read());
        });
    });
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod now_answering_tests {
    #[test]
    fn downloaded_model_names_identity_and_use_button_in_both_languages() {
        use overlay_backend::local_ai::ManagedModel;
        for (model, identity) in [
            (ManagedModel::Legacy4B, "4B"),
            (ManagedModel::Fallback12B, "12B QAT"),
            (ManagedModel::Primary26B, "26B-A4B"),
        ] {
            let en = super::model_downloaded_label(model, false);
            let ru = super::model_downloaded_label(model, true);
            assert!(en.contains(identity) && en.contains("Click 'Use'"));
            assert!(ru.contains(identity) && ru.contains("Нажмите «Использовать»"));
        }
    }

    #[test]
    fn caption_keeps_only_the_endpoint_authority() {
        for (url, expected) in [
            (
                "http://user:secret@127.0.0.1:1234/v1/private?token=secret#x",
                "127.0.0.1:1234",
            ),
            ("https://[::1]:8443/v1", "[::1]:8443"),
            ("https://localhost/v1", "localhost:443"),
            ("http://localhost:11434?secret", "localhost:11434"),
            ("C:\\private\\model.gguf", ""),
        ] {
            assert_eq!(super::now_endpoint_authority(url), expected);
        }
    }
}

#[cfg(all(test, any(windows, target_os = "linux")))]
#[allow(clippy::items_after_test_module)]
mod stop_copy_tests {
    use super::local_ai_stop_copy;
    use overlay_backend::local_ai::{DiskSpaceError, CANCEL_SENTINEL};

    const INSTALL: (&str, &str) = ("Установить", "Install");

    #[test]
    fn disk_space_names_both_sizes_and_the_button() {
        let e = anyhow::Error::from(DiskSpaceError {
            need_gib: 9,
            free_gib: 7,
            root: "/home/someone/suflyor-local-ai".into(),
        });
        let en = local_ai_stop_copy(&e, false, INSTALL);
        assert_eq!(
            en,
            "Not enough disk space: about 9 GB needed, 7 GB free. Free up space, then press Install again."
        );
        let ru = local_ai_stop_copy(&e.context("install"), true, INSTALL);
        assert!(ru.contains("нужно около 9 ГБ, свободно 7 ГБ") && ru.contains("«Установить»"));
        assert!(!ru.contains("/home") && !en.contains("/home"));
    }

    #[test]
    fn known_causes_are_named_without_the_raw_chain() {
        let e = anyhow::anyhow!("curl exit 6 (after 3 attempts)")
            .context("create install root /home/someone/suflyor-local-ai");
        let en = local_ai_stop_copy(&e, false, INSTALL);
        assert_eq!(
            en,
            "The download stopped — the download server is unreachable. Press Install to try again."
        );
        assert!(!local_ai_stop_copy(&e, true, INSTALL).contains("/home"));
        let e = anyhow::anyhow!("release asset host is not supported");
        assert!(!local_ai_stop_copy(&e, false, INSTALL).contains("server did not start"));
    }

    #[test]
    fn missing_curl_names_the_tool_and_setup_step_before_retrying() {
        for message in ["curl not found", "curl.exe not found"] {
            let e =
                anyhow::anyhow!(message).context("download engine /home/someone/suflyor-local-ai");
            let en = local_ai_stop_copy(&e, false, INSTALL);
            let ru = local_ai_stop_copy(&e, true, INSTALL);
            if cfg!(windows) {
                assert_eq!(en, "curl.exe is missing from PATH (included with Windows 10 and later). Add it to PATH, then press Install again.");
                assert_eq!(ru, "curl.exe отсутствует в PATH (входит в Windows 10 и новее). Добавьте его в PATH и нажмите «Установить» снова.");
            } else {
                assert_eq!(
                    en,
                    "curl is not installed. Install curl, then press Install again."
                );
                assert_eq!(
                    ru,
                    "curl не установлен. Установите curl и нажмите «Установить» снова."
                );
            }
        }
    }

    #[test]
    fn cancel_and_unknown_failures_say_what_to_press() {
        let cancelled = anyhow::anyhow!("{CANCEL_SENTINEL}");
        assert_eq!(local_ai_stop_copy(&cancelled, false, INSTALL), "Cancelled.");
        assert_eq!(local_ai_stop_copy(&cancelled, true, INSTALL), "Отменено.");
        let other = anyhow::anyhow!("something odd");
        assert_eq!(
            local_ai_stop_copy(&other, false, INSTALL),
            "That failed. Press Install to try again; details: Diagnostics → Collect logs."
        );
    }
}

#[cfg(any(windows, target_os = "linux"))]
static NOW_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn begin_now_check(win: &SettingsWindow) -> u64 {
    win.set_now_state(0);
    NOW_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
}

#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn now_check_is_current(generation: u64) -> bool {
    NOW_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation
}

/// No credentials, URL path, query or fragment in the screen-shared caption.
pub(crate) fn now_endpoint_authority(base: &str) -> String {
    let authority = base
        .trim()
        .strip_prefix("http://")
        .or_else(|| base.trim().strip_prefix("https://"))
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    if authority.is_empty() || (authority.contains(':') && !authority.ends_with(']')) {
        authority.to_string()
    } else {
        format!(
            "{authority}:{}",
            if base.trim().starts_with("https://") {
                443
            } else {
                80
            }
        )
    }
}

/// Seed the saved choice immediately, then check its models endpoint off-thread.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn refresh_now_answering(win: &SettingsWindow, cfg: &overlay_backend::config::Config) {
    // Asset-only refreshes (including builtin-busy changes) must not probe or
    // imply readiness of a server that downloading deliberately did not start.
    if cfg.ai_provider != "local" {
        NOW_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        return;
    }
    let ru = cfg.ui_is_ru();
    let base = cfg.ai_local_base_url.clone();
    let llama = overlay_backend::local_ai::is_managed_llama_endpoint(&base);
    let strata = overlay_backend::local_ai::is_managed_strata_endpoint(&base);
    let server = listed_servers().into_iter().find(|s| {
        s.is_current(&base, &cfg.ai_local_model)
            && !(strata && (win.get_strata_adopted() || !win.get_strata_external()))
    });
    let (title, detail) = if llama {
        let title = if cfg.ai_local_custom_gguf.is_empty() {
            overlay_backend::local_ai::ManagedModel::from_config(
                &cfg.ai_local_model,
                cfg.ai_local_quality,
            )
            .spec()
            .label
            .to_string()
        } else {
            cfg.ai_local_custom_gguf
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .to_string()
        };
        (
            title,
            if ru {
                "Встроенный движок"
            } else {
                "Built-in engine"
            }
            .to_string(),
        )
    } else if let Some(server) = server {
        let name = if server.name.is_empty() {
            if ru {
                "Сервер"
            } else {
                "Server"
            }
        } else {
            &server.name
        };
        (
            server.pick_model(&cfg.ai_local_model),
            if ru {
                format!("{name} · порт {}", server.port)
            } else {
                format!("{name} · port {}", server.port)
            },
        )
    } else if strata {
        if let Some(found) = overlay_backend::local_ai::adopted_strata() {
            let folder = found
                .dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            (
                found.model,
                if ru {
                    format!("Strata · папка {folder}")
                } else {
                    format!("Strata · folder {folder}")
                },
            )
        } else {
            (cfg.ai_local_model.clone(), "Strata".to_string())
        }
    } else {
        (
            cfg.ai_local_model.clone(),
            format!(
                "{} · {}",
                if ru { "Сервер" } else { "Server" },
                now_endpoint_authority(&base)
            ),
        )
    };
    // Preserve namespaced model IDs, but never show an absolute file path.
    let title = if title.starts_with('/')
        || title.starts_with('\\')
        || title.as_bytes().get(1) == Some(&b':')
    {
        title.rsplit(['/', '\\']).next().unwrap_or_default()
    } else {
        &title
    };
    win.set_now_title(SharedString::from(title));
    win.set_now_detail(SharedString::from(detail));
    let generation = begin_now_check(win);
    win.set_now_notice(SharedString::from(if cfg.deep_lock {
        overlay_backend::deep_lock::lifecycle_guard_notice(ru)
    } else {
        ""
    }));
    if base.trim().is_empty()
        || cfg.ai_local_model.trim().is_empty()
        || (llama && !win.get_llama_installed())
        || (strata
            && !win.get_strata_installed()
            && !win.get_strata_adopted()
            && !win.get_strata_external())
    {
        // Nothing runs yet: no model name that is not there.
        win.set_now_title(SharedString::from(if ru {
            "Модель ещё не выбрана"
        } else {
            "No model yet"
        }));
        win.set_now_detail(SharedString::from(if ru {
            "Выберите или скачайте модель ниже."
        } else {
            "Pick or download one below."
        }));
        win.set_now_state(4);
        return;
    }
    let bearer = cfg.ai_local_bearer.clone();
    let selected_model = cfg.ai_local_model.clone();
    let weak = win.as_weak();
    std::thread::spawn(move || {
        let working = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .is_some_and(|rt| {
                rt.block_on(async {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(3),
                        overlay_backend::ai::list_models(&base, &bearer),
                    )
                    .await
                    {
                        Ok(Ok(models)) => {
                            llama || strata || models.iter().any(|m| m == &selected_model)
                        }
                        _ => false,
                    }
                })
            });
        let _ = slint::invoke_from_event_loop(move || {
            if now_check_is_current(generation) {
                if let Some(w) = weak.upgrade() {
                    w.set_now_state(if working { 1 } else { 3 });
                }
            }
        });
    });
}

/// Refresh saved engine/model state without moving the user's open fold.
pub(crate) fn refresh_local_engine_state(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::Config,
) {
    let root = overlay_backend::local_ai::default_root();
    win.set_llama_installed(overlay_backend::local_ai::llama_engine_installed(&root));
    win.set_strata_installed(overlay_backend::local_ai::own_strata_installed(&root));
    #[cfg(any(windows, target_os = "linux"))]
    super::settings_strata_vision::refresh(win, cfg);
    win.set_strata_active(overlay_backend::local_ai::is_managed_strata_endpoint(
        &cfg.ai_local_base_url,
    ));
    let adopted = overlay_backend::local_ai::adopted_strata();
    win.set_strata_adopted(adopted.is_some());
    if let Some(found) = adopted {
        *FOUND_STRATA.lock().unwrap_or_else(|p| p.into_inner()) = Some(found);
    }
    show_found_models(win, cfg);
    #[cfg(any(windows, target_os = "linux"))]
    {
        win.set_ai_local_test_result(SharedString::default());
        refresh_now_answering(win, cfg);
    }
}

/// On Settings open: seed the inline server form and legacy engine index.
pub(crate) fn refresh_local_engine_controls(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::Config,
) {
    win.set_local_connect_open(
        !overlay_backend::local_ai::is_managed_llama_endpoint(&cfg.ai_local_base_url)
            && !overlay_backend::local_ai::is_managed_strata_endpoint(&cfg.ai_local_base_url)
            && !listed_servers()
                .iter()
                .any(|s| s.is_current(&cfg.ai_local_base_url, &cfg.ai_local_model)),
    );
    refresh_local_engine_state(win, cfg);
    let strata_active =
        overlay_backend::local_ai::is_managed_strata_endpoint(&cfg.ai_local_base_url);
    let llama_active = overlay_backend::local_ai::is_managed_llama_endpoint(&cfg.ai_local_base_url);
    win.set_local_engine_index(if win.get_strata_busy() || strata_active {
        1
    } else if llama_active {
        0
    } else {
        2
    });
}

/// Wire the Strata engine and the engine switch (built-in llama.cpp or Strata).
/// Both engines need the whole GPU, so a switch stops one before it starts
/// the other, under the same busy flag and lifecycle lock as a model switch.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn wire_strata(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_builtin_model_use(move |index| {
            let Some(w) = weak.upgrade() else { return };
            if !(0..=2).contains(&index) || w.get_model_switching() || w.get_strata_busy() {
                return;
            }
            let c = cfg_c.read();
            if c.deep_lock {
                w.set_now_notice(SharedString::from(
                    overlay_backend::deep_lock::lifecycle_guard_notice(c.ui_is_ru()),
                ));
                return;
            }
            if state_c
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .local_ai_busy
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return;
            }
            let root = overlay_backend::local_ai::default_root();
            let model = overlay_backend::local_ai::ManagedModel::from_index(index);
            if !overlay_backend::local_ai::managed_model_present(&root, model) {
                return;
            }
            let choice = overlay_backend::local_ai::ManagedLlamaChoice::for_model(
                model,
                overlay_backend::local_ai::LocalContextPreset::from_config(&c.ai_local_context),
            );
            drop(c);
            *PENDING_BUILTIN_CHOICE
                .lock()
                .unwrap_or_else(|p| p.into_inner()) = Some(choice);
            w.invoke_local_engine_activate(0);
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let state_c = state.clone();
        win.on_strata_choose(move |own| {
            let Some(w) = weak.upgrade() else { return };
            // The same guards as the engine switch, checked before the choice
            // is saved: a refused switch leaves the previous choice in place.
            if w.get_strata_busy() || w.get_model_switching() {
                return;
            }
            {
                let c = cfg_c.read();
                if c.deep_lock {
                    w.set_strata_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(c.ui_is_ru()),
                    ));
                    return;
                }
            }
            if state_c
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .local_ai_busy
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return;
            }
            let found = if own {
                None
            } else {
                let found = FOUND_STRATA
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .clone();
                if found.is_none() {
                    return;
                }
                found
            };
            let (dir, config) = found.as_ref().map_or_else(Default::default, |f| {
                (
                    f.dir.to_string_lossy().into_owned(),
                    f.config.to_string_lossy().into_owned(),
                )
            });
            {
                let mut c = cfg_c.write();
                c.ai_strata_dir.clone_from(&dir);
                c.ai_strata_config.clone_from(&config);
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] Strata choice save failed: {e:#}");
                }
            }
            overlay_backend::local_ai::set_adopted_strata(&dir, &config);
            w.set_strata_adopted(found.is_some());
            // Start the chosen install (it restarts a server that runs from
            // the other one).
            w.invoke_local_engine_activate(1);
        });
    }

    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_found_models_search(move || {
            if let Some(w) = weak.upgrade() {
                search_found_models(&w, &cfg_c);
            }
        });
    }

    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let state_c = state.clone();
        win.on_found_model_use(move |index| {
            let Some(w) = weak.upgrade() else { return };
            // A running install, download or switch owns the engine.
            if w.get_strata_busy()
                || w.get_model_switching()
                || state_c
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .local_ai_busy
                    .load(std::sync::atomic::Ordering::SeqCst)
            {
                return;
            }
            let has_strata_row = FOUND_STRATA
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .is_some();
            let Ok(mut index) = usize::try_from(index) else {
                return;
            };
            if has_strata_row {
                if index == 0 {
                    w.invoke_strata_choose(false);
                    return;
                }
                index -= 1;
            }
            let Some(server) = listed_servers().get(index).cloned() else {
                return;
            };
            // A Strata the user started on :8090: use it as the Strata engine
            // (3 = the running one, nothing is started).
            if server.strata_ready
                && overlay_backend::local_ai::is_managed_strata_endpoint(&server.base_url)
            {
                w.invoke_local_engine_activate(3);
                return;
            }
            let leaving_builtin = {
                let mut c = cfg_c.write();
                let leaving =
                    overlay_backend::local_ai::is_managed_llama_endpoint(&c.ai_local_base_url);
                c.ai_local_model = server.pick_model(&c.ai_local_model);
                leaving
            };
            // The URL save stores both, stops a running Strata, and reloads
            // the model list.
            w.set_local_engine_index(2);
            w.invoke_ai_local_base_url_save(SharedString::from(server.base_url.clone()));
            // The built-in engine holds the GPU and RAM the other server needs.
            // Under the lifecycle lock (a running summary or a switch keeps
            // it), and only while the choice still stands.
            if leaving_builtin {
                let lifecycle_lock = state_c
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .local_ai_lock
                    .clone();
                let cfg_t = cfg_c.clone();
                std::thread::spawn(move || {
                    let Some(_guard) =
                        overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                    else {
                        return;
                    };
                    if overlay_backend::local_ai::is_managed_llama_endpoint(
                        &cfg_t.read().ai_local_base_url,
                    ) {
                        return;
                    }
                    let root = overlay_backend::local_ai::default_root();
                    let _ = overlay_backend::local_ai::free_llama_port(&root);
                });
            }
            // A real request, not a claim: the test line shows the answer.
            w.invoke_ai_local_test_clicked();
        });
    }

    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_strata_find_folder_clicked(move || {
            let cfg_c = cfg_c.clone();
            let weak = weak.clone();
            super::settings_dialogs::pick(
                rfd::AsyncFileDialog::new(),
                super::settings_dialogs::Kind::Folder,
                move |folder| {
                    if let (Some(w), Some(folder)) = (weak.upgrade(), folder) {
                        select_strata_folder(&w, &cfg_c, &folder);
                    }
                },
            );
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_strata_folder_selected(move |folder| {
            if let Some(w) = weak.upgrade() {
                select_strata_folder(&w, &cfg_c, std::path::Path::new(folder.as_str()));
            }
        });
    }

    win.on_strata_license_clicked(|| {
        #[cfg(target_os = "linux")]
        let _ = slint_replay::native::desktop::open(std::ffi::OsStr::new(
            overlay_backend::local_ai::STRATA_MODEL_LICENSE_URL,
        ));
        #[cfg(not(target_os = "linux"))]
        let _ = std::process::Command::new("explorer")
            .arg(overlay_backend::local_ai::STRATA_MODEL_LICENSE_URL)
            .spawn();
    });

    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_strata_install_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_strata_busy() {
                return;
            }
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return;
            };
            let ru = cfg_c.read().ui_is_ru();
            w.set_local_strata_job(0);
            w.set_strata_busy(true);
            w.set_strata_status(SharedString::from(if ru {
                "Подготовка…"
            } else {
                "Preparing…"
            }));
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard;
                let root = overlay_backend::local_ai::default_root();
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let line = strata_progress_line(&p, ru);
                        if line.is_empty() {
                            return;
                        }
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_p.upgrade() {
                                w.set_strata_status(SharedString::from(line));
                            }
                        });
                    }
                };
                let res = overlay_backend::local_ai::install_strata(&root, &cancel, &on);
                if let Err(e) = &res {
                    eprintln!("[overlay-host] Strata install failed: {e:#}");
                }
                // The row of Suflyor's own install shows this, not the
                // adopted one.
                let installed = overlay_backend::local_ai::own_strata_installed(&root);
                // A failed check of Strata's own setup (no NVIDIA driver, too
                // little RAM) is the one detail the user needs to act on.
                let stopped = res.as_ref().err().and_then(|e| {
                    e.to_string()
                        .strip_prefix(overlay_backend::local_ai::STRATA_SETUP_STOPPED)
                        .map(str::to_owned)
                });
                // Otherwise generic text: setup errors can carry paths, and
                // Settings may be screen-shared.
                let msg = match &res {
                    Err(_) if stopped.is_some() => {
                        let reason = stopped.clone().unwrap_or_default();
                        if ru {
                            format!("Установка Strata остановлена: {reason}")
                        } else {
                            format!("Strata setup stopped: {reason}")
                        }
                    }
                    Ok(()) if ru => "Strata установлена. Нажмите «Использовать», чтобы запустить её.".to_string(),
                    Ok(()) => "Strata is installed. Press Use to start it.".to_string(),
                    Err(e) if e.to_string().contains(overlay_backend::local_ai::CANCEL_SENTINEL) => {
                        if ru {
                            "Отменено. Повторный запуск продолжит загрузку.".to_string()
                        } else {
                            "Cancelled. Installing again resumes the download.".to_string()
                        }
                    }
                    Err(e) if e.to_string().contains("not enough free disk space") => {
                        if ru {
                            "Недостаточно места на диске: Strata нужно около 80 ГБ.".to_string()
                        } else {
                            "Not enough free disk space: Strata needs about 80 GB.".to_string()
                        }
                    }
                    Err(_) if ru => {
                        "Установка Strata не удалась. Подробности — в suflyor-setup.log в папке Strata.".to_string()
                    }
                    Err(_) => "Strata setup failed. Details are in suflyor-setup.log in the Strata folder.".to_string(),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_t.upgrade() {
                        w.set_strata_busy(false);
                        w.set_local_strata_job(-1);
                        w.set_strata_installed(installed);
                        w.set_strata_status(SharedString::from(msg));
                    }
                });
            });
        });
    }

    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_local_engine_activate(move |index| {
            let requested_choice = PENDING_BUILTIN_CHOICE.lock().unwrap_or_else(|p| p.into_inner()).take();
            let Some(w) = weak.upgrade() else { return };
            if w.get_strata_busy() || w.get_model_switching() {
                return;
            }
            let ru = {
                let c = cfg_c.read();
                if c.deep_lock {
                    w.set_strata_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(c.ui_is_ru()),
                    ));
                    return;
                }
                c.ui_is_ru()
            };
            let Some(busy_guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire({
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_busy.clone()
            }) else {
                return;
            };
            // 1 = start Strata, 3 = use the Strata already running on :8090.
            let to_strata = index == 1 || index == 3;
            let running_strata = index == 3;
            if to_strata {
                w.set_local_strata_job(if overlay_backend::local_ai::adopted_strata().is_some() { 1 } else { 0 });
                w.set_strata_busy(true);
                w.set_strata_status(SharedString::from(if ru {
                    "Запускаю Strata: модель загружается в память 1–3 минуты…"
                } else {
                    "Starting Strata: the model takes 1-3 minutes to load…"
                }));
            } else {
                w.set_model_switching(true);
                w.set_quality_status(SharedString::from(if ru {
                    "Переключаю на встроенный движок…"
                } else {
                    "Switching to the built-in engine…"
                }));
            }
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            std::thread::spawn(move || {
                let _busy_guard = busy_guard;
                let root = overlay_backend::local_ai::default_root();
                // A boot or watchdog start of Strata may hold the lock while
                // the model loads; the user's choice wins.
                if !to_strata {
                    overlay_backend::local_ai::abort_strata_start();
                }
                // Holding the lifecycle lock keeps the llama watchdog from
                // restarting :8080 behind the switch.
                let lifecycle_lock = {
                    let s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_lock.clone()
                };
                let Some(_ai_guard) =
                    overlay_backend::local_ai::blocking_acquire_lifecycle(&lifecycle_lock)
                else {
                    return;
                };
                let previous = {
                    let c = cfg_t.read();
                    overlay_backend::local_ai::is_managed_llama_endpoint(&c.ai_local_base_url)
                        .then(|| {
                            overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                                &c.ai_local_model,
                                c.ai_local_quality,
                                &c.ai_local_custom_gguf,
                                overlay_backend::local_ai::LocalContextPreset::from_config(
                                    &c.ai_local_context,
                                ),
                            )
                        })
                };
                let mut started = Vec::new();
                let mut strata_ready = false;
                let msg = if to_strata {
                    let on = {
                        let weak_p = weak_t.clone();
                        move |p: overlay_backend::local_ai::Progress| {
                            let line = strata_progress_line(&p, ru);
                            if line.is_empty() {
                                return;
                            }
                            let weak_p = weak_p.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_p.upgrade() {
                                    w.set_strata_status(SharedString::from(line));
                                }
                            });
                        }
                    };
                    let outcome = if running_strata {
                        overlay_backend::local_ai::use_running_strata(&root)
                    } else {
                        overlay_backend::local_ai::start_strata(&root, &cancel, &on)
                    };
                    strata_ready = outcome == overlay_backend::local_ai::StrataStart::Ready;
                    if strata_ready {
                        let model = overlay_backend::local_ai::served_strata_model()
                            .unwrap_or_else(|| "strata".to_string());
                        let mut c = cfg_t.write();
                        overlay_backend::local_ai::apply_strata_choice(&mut c, &model);
                        if let Err(e) = overlay_backend::config::save(&c) {
                            eprintln!("[overlay-host] Strata choice save failed: {e:#}");
                        }
                    } else if let Some(choice) = previous {
                        // Strata stopped llama.cpp before it failed: bring the
                        // previous model back.
                        let (back, children) =
                            overlay_backend::local_ai::restart_llama_server(&root, choice);
                        if overlay_backend::local_ai::switch_has_ready_server(back) {
                            started = children;
                        } else {
                            overlay_backend::local_ai::terminate_servers(children);
                        }
                    }
                    match (outcome, ru) {
                        (overlay_backend::local_ai::StrataStart::Ready, true) => {
                            "Готово: работает Strata.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::Ready, false) => {
                            "Done: Strata is running.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::NotInstalled, true) => {
                            "Strata не установлена.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::NotInstalled, false) => {
                            "Strata is not installed.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::PortBusy, true) => {
                            "Порт :8090 занят другим процессом — Strata не запущена.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::PortBusy, false) => {
                            "Port :8090 is used by another program — Strata did not start."
                                .to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::Cancelled, true) => {
                            "Отменено.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::Cancelled, false) => {
                            "Cancelled.".to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::FailedToStart, true) => {
                            "Strata не запустилась. Подробности — в suflyor-server.log в папке Strata."
                                .to_string()
                        }
                        (overlay_backend::local_ai::StrataStart::FailedToStart, false) => {
                            "Strata did not start. Details are in suflyor-server.log in the Strata folder."
                                .to_string()
                        }
                    }
                } else {
                    overlay_backend::local_ai::stop_strata();
                    let activate_answers = requested_choice.is_some();
                    let choice = requested_choice.unwrap_or_else(|| {
                        let mut target = cfg_t.read().clone();
                        target.ai_local_base_url = overlay_backend::local_ai::LLAMA_BASE_URL.to_string();
                        overlay_backend::local_ai::repair_managed_model_state(&mut target, &root);
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &target.ai_local_model,
                            target.ai_local_quality,
                            &target.ai_local_custom_gguf,
                            overlay_backend::local_ai::LocalContextPreset::from_config(&target.ai_local_context),
                        )
                    });
                    let (outcome, children) =
                        overlay_backend::local_ai::restart_llama_server(&root, choice.clone());
                    // A cloud user chose this model, not an unrelated fallback.
                    // Keep their cloud route if only the fallback became ready.
                    let ready = overlay_backend::local_ai::switch_has_ready_server(outcome)
                        && (cfg_t.read().ai_provider == "local"
                            || overlay_backend::local_ai::switch_commits_choice(outcome));
                    if ready {
                        let choice = if outcome == overlay_backend::local_ai::ModelSwitch::FallbackStarted {
                            overlay_backend::local_ai::ManagedLlamaChoice::for_model(
                                overlay_backend::local_ai::effective_managed_model(
                                    &root, overlay_backend::local_ai::ManagedModel::Fallback12B,
                                ), choice.context,
                            )
                        } else { choice };
                        let mut c = cfg_t.write();
                        overlay_backend::local_ai::apply_llama_choice(&mut c, &root, &choice);
                        overlay_backend::local_ai::repair_managed_model_state_after_verification(&mut c, &root);
                        if activate_answers {
                            c.ai_provider = "local".to_string();
                            overlay_backend::ai::set_local_no_think(!c.ai_local_thinking);
                        }
                        if let Err(e) = overlay_backend::config::save(&c) {
                            eprintln!("[overlay-host] built-in engine save failed: {e:#}");
                        }
                        started = children;
                    } else {
                        overlay_backend::local_ai::terminate_servers(children);
                    }
                    match (ready, outcome, ru) {
                        (true, overlay_backend::local_ai::ModelSwitch::FallbackStarted, true) => "Выбранная модель не запустилась; работает резервная модель.".to_string(),
                        (true, overlay_backend::local_ai::ModelSwitch::FallbackStarted, false) => "The selected model did not start; the fallback model is running.".to_string(),
                        (true, _, true) => "Готово: работает встроенный движок.".to_string(),
                        (true, _, false) => "Done: the built-in engine is running.".to_string(),
                        (false, _, true) => {
                            "Встроенный движок не запустился — проверьте установку локального AI."
                                .to_string()
                        }
                        (false, _, false) => {
                            "The built-in engine did not start — check the local AI install."
                                .to_string()
                        }
                    }
                };
                if !started.is_empty() {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    s.local_ai_servers
                        .retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
                    s.local_ai_servers.extend(started);
                }
                let saved_url = cfg_t.read().ai_local_base_url.clone();
                let weak_done = weak_t.clone();
                let cfg_done = cfg_t.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_done.upgrade() {
                        w.set_strata_busy(false);
                        w.set_local_strata_job(-1);
                        w.set_model_switching(false);
                        w.set_strata_running(strata_ready);
                        if to_strata {
                            w.set_strata_status(SharedString::from(msg));
                        } else {
                            w.set_quality_status(SharedString::from(msg));
                        }
                        // The URL save path refreshes the managed flag, the model
                        // list, vision and the lock chip from the saved config.
                        w.invoke_ai_local_base_url_save(SharedString::from(saved_url));
                        let c = cfg_done.read();
                        if c.ai_provider == "local" {
                            w.set_ai_provider_index(1);
                        }
                        refresh_local_engine_state(&w, &c);
                        refresh_local_context_controls(&w, &c);
                        // Stay on the panel of the engine the user asked for:
                        // its status line explains the outcome.
                        w.set_local_engine_index(if to_strata { 1 } else { 0 });
                    }
                    if let Some(o) = overlay_t.upgrade() {
                        o.set_active_stack(SharedString::from(active_stack_label(
                            &cfg_done.read(),
                        )));
                    }
                });
            });
        });
    }
}
