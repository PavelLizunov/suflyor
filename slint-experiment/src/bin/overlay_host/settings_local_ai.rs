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
use super::{
    active_stack_label, refresh_local_model_resource_warning, ComponentHandle, ModelRc,
    OverlayBarWindow, SettingsWindow, SharedString, VecModel,
};

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
pub(crate) fn wire_local_ai(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    let pending_custom_gguf =
        std::sync::Arc::new(std::sync::Mutex::new(None::<std::path::PathBuf>));

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
            // Deep lock (v0.37): the managed server is deliberately unloaded —
            // an install would silently resurrect it. Unlock from the bar first.
            {
                let c = cfg_c.read();
                if c.deep_lock {
                    let ru = c.ui_is_ru();
                    w.set_local_ai_status(SharedString::from(
                        overlay_backend::deep_lock::lifecycle_guard_notice(ru),
                    ));
                    return;
                }
            }
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
                let mut opts = overlay_backend::local_ai::InstallOptions::default();
                let (restore_previous, restore_whisper, previous_choice) = {
                    let c = cfg_t.read();
                    opts.context =
                        overlay_backend::local_ai::LocalContextPreset::from_config(
                            &c.ai_local_context,
                        );
                    (
                        overlay_backend::local_ai::is_managed_llama_endpoint(&c.ai_local_base_url)
                            && overlay_backend::local_ai::base_model_present(&opts.root),
                        c.stt_provider == "whisper" && c.stt_whisper_url.contains(":8081"),
                        overlay_backend::local_ai::ManagedLlamaChoice::from_config(
                            &c.ai_local_model,
                            c.ai_local_quality,
                            &c.ai_local_custom_gguf,
                            opts.context,
                        ),
                    )
                };
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
                                    "Готово. Локальный AI настроен и запущен.",
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
                        state_t
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .local_ai_servers
                            .extend(restored_servers);
                        let cancelled = e
                            .to_string()
                            .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                        let msg = if cancelled {
                            "Отменено.".to_string()
                        } else {
                            eprintln!("[overlay-host] local-ai install failed: {e:#}");
                            "Ошибка установки локального AI. Подробности в журнале.".to_string()
                        };
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

    {
        let pending = pending_custom_gguf.clone();
        let weak = win.as_weak();
        win.on_choose_custom_gguf_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let Some(path) = rfd::FileDialog::new()
                .add_filter("GGUF model", &["gguf"])
                .pick_file()
            else {
                return;
            };
            if overlay_backend::local_ai::valid_custom_gguf_path(&path.to_string_lossy()).is_none()
            {
                w.set_quality_status(SharedString::from(
                    "Выбранный файл не является корректной GGUF-моделью.",
                ));
                return;
            }
            *pending.lock().unwrap_or_else(|p| p.into_inner()) = Some(path);
            let weak_invoke = w.as_weak();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = weak_invoke.upgrade() {
                    w.invoke_model_profile_changed(-1);
                }
            });
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
                w.set_quality_status(SharedString::from(
                    "Файл выбранной модели не установлен.",
                ));
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
            w.set_quality_status(SharedString::from(
                if let Some(name) = custom_name.as_deref() {
                    format!("Переключаю на пользовательскую модель {name}…")
                } else {
                    match want_model {
                        overlay_backend::local_ai::ManagedModel::Legacy4B => {
                            "Переключаю на быструю модель (4B)…".to_string()
                        }
                        overlay_backend::local_ai::ManagedModel::Fallback12B => {
                            "Переключаю на сбалансированную модель (12B QAT)…".to_string()
                        }
                        overlay_backend::local_ai::ManagedModel::Primary26B => {
                            "Переключаю на максимальную модель (26B-A4B)…".to_string()
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
                        let status = if outcome
                            == overlay_backend::local_ai::ModelSwitch::Switched
                            && custom_name.is_some()
                        {
                            format!(
                                "Готово: пользовательская модель {}.",
                                custom_name.as_deref().unwrap_or_default()
                            )
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
                w.set_quality_status(SharedString::from(
                    "Этот контекст выше безопасного лимита текущего профиля.",
                ));
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
                        w.set_quality_status(SharedString::from(
                            "Контекст сохранён; новые запросы используют выбранный режим.",
                        ));
                    }
                    Err(e) => {
                        c.ai_local_context = saved_context;
                        eprintln!("[overlay-host] local context save failed: {e:#}");
                        w.set_quality_status(SharedString::from(
                            "Не удалось сохранить настройку контекста.",
                        ));
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
            w.set_quality_status(SharedString::from(
                "Применяю контекст и перезапускаю локальный AI…",
            ));
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
                            let done = match model {
                                overlay_backend::local_ai::ManagedModel::Legacy4B => {
                                    "Быстрая модель (4B) загружена. Нажмите её профиль, чтобы включить."
                                }
                                overlay_backend::local_ai::ManagedModel::Fallback12B => {
                                    "Сбалансированная модель (12B QAT) загружена. Нажмите её профиль, чтобы включить."
                                }
                                overlay_backend::local_ai::ManagedModel::Primary26B => {
                                    "Основная модель 26B-A4B загружена. Нажмите её профиль, чтобы включить."
                                }
                            };
                            w.set_quality_status(SharedString::from(done));
                        }
                        Err(e) => {
                            let cancelled = e
                                .to_string()
                                .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                            if cancelled {
                                w.set_quality_status(SharedString::from("Отменено."));
                            } else {
                                eprintln!("[overlay-host] {model_label} download failed: {e:#}");
                                w.set_quality_status(SharedString::from(
                                    "Ошибка загрузки модели. Подробности в журнале.",
                                ));
                            }
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
            w.set_vision12b_status(SharedString::from("Подготовка…"));
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
                    c.ai_local_quality && !c.deep_lock
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
                            w.set_ai_local_vision_available(restarted);
                            w.set_ai_local_vision(restarted);
                            w.set_vision_same_available(restarted);
                            if restarted {
                                w.set_vision_provider_index(
                                    super::settings_vision::vision_provider_index_from_id("same"),
                                );
                            }
                            w.set_vision12b_status(SharedString::from(if restarted {
                                "Зрение 26B включено — F8 теперь работает на 26B."
                            } else {
                                "Проектор 26B загружен. Перезапустите приложение, чтобы включить зрение."
                            }));
                        }
                        Err(e) => {
                            let cancelled = e
                                .to_string()
                                .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                            if cancelled {
                                w.set_vision12b_status(SharedString::from("Отменено."));
                            } else {
                                eprintln!("[overlay-host] 26B vision projector download failed: {e:#}");
                                w.set_vision12b_status(SharedString::from(
                                    "Не удалось загрузить проектор зрения. Попробуйте ещё раз.",
                                ));
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
            w.set_engine_update_status(SharedString::from("Проверяю обновление движка…"));
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
                    let msg = match res {
                        Ok(overlay_backend::local_ai::EngineUpdate::UpToDate { .. }) => {
                            "Движок уже последней версии.".to_string()
                        }
                        Ok(overlay_backend::local_ai::EngineUpdate::Updated { to, .. }) => {
                            format!("Движок обновлён до b{to}.")
                        }
                        Ok(overlay_backend::local_ai::EngineUpdate::Skipped { .. }) => {
                            "Обновление пропущено — оставлен текущий движок.".to_string()
                        }
                        Err(e) => {
                            // Full chain (incl. os errors) → stderr only; the UI
                            // shows a generic message (no path/URL leak).
                            eprintln!("[overlay-host] engine update failed: {e:#}");
                            "Не удалось обновить движок. Подробности в журнале.".to_string()
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
