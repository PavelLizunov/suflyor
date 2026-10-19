//! Download-only Strata encoder provisioning and explicit, image-verified F8 activation.
use super::{ComponentHandle, SettingsWindow, SharedString};
use overlay_backend::{config, local_ai};
use std::sync::atomic::Ordering;

pub(crate) fn refresh(win: &SettingsWindow, cfg: &config::Config) {
    let root = local_ai::default_root();
    win.set_strata_vision_installable(local_ai::strata_vision_installable(&root));
    win.set_strata_vision_downloaded(
        local_ai::strata_vision_downloaded(&root) || local_ai::strata_vision_available(&root),
    );
    win.set_strata_vision_enabled(
        cfg.ai_provider == "local"
            && local_ai::is_managed_strata_endpoint(&cfg.ai_local_base_url)
            && cfg.ai_local_vision
            && cfg.vision_provider == "same"
            && local_ai::strata_vision_available(&root),
    );
    if win.get_strata_vision_job() == 0 {
        win.set_strata_vision_status(SharedString::default());
    }
}

pub(crate) fn wire(
    win: &SettingsWindow,
    cfg: &config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
) {
    for activate in [false, true] {
        let cfg = cfg.clone();
        let state = state.clone();
        let weak = win.as_weak();
        let callback = move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_strata_vision_job() != 0 || w.get_strata_busy() || w.get_model_switching() {
                return;
            }
            let snapshot = cfg.read().clone();
            let ru = snapshot.ui_is_ru();
            if activate && snapshot.deep_lock {
                w.set_strata_vision_status(overlay_backend::deep_lock::lifecycle_guard_notice(ru).into());
                return;
            }
            if activate && (snapshot.ai_provider != "local" || !local_ai::is_managed_strata_endpoint(&snapshot.ai_local_base_url)) {
                w.set_strata_vision_status(if ru { "Сначала включите Strata для текстовых ответов." } else { "First use Strata for text answers." }.into());
                return;
            }
            let (busy, cancel, lock) = {
                let s = state.lock().unwrap_or_else(|p| p.into_inner());
                (s.local_ai_busy.clone(), s.local_ai_cancel.clone(), s.local_ai_lock.clone())
            };
            let Some(guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire(busy) else { return };
            cancel.store(false, Ordering::Relaxed);
            w.set_strata_vision_job(if activate { 2 } else { 1 });
            w.set_strata_vision_progress(0.0);
            w.set_strata_vision_status(if ru { "Подготовка…" } else { "Preparing…" }.into());
            let cfg = cfg.clone();
            let weak = weak.clone();
            std::thread::spawn(move || {
                let _guard = guard;
                let progress_window = weak.clone();
                let on = move |p: local_ai::Progress| {
                    let (text, fraction) = match p {
                        local_ai::Progress::Bytes { done, total, .. } => (
                            format!("Strata Vision: {} / {} MB", done / 1_000_000, total / 1_000_000),
                            Some((done as f32 / total.max(1) as f32).clamp(0.0, 1.0)),
                        ),
                        _ => (if activate {
                            if ru { "Загружаю Strata и проверяю изображение…" } else { "Loading Strata and checking image input…" }
                        } else if ru { "Скачиваю и проверяю энкодер…" } else { "Downloading and verifying the encoder…" }.to_string(), None),
                    };
                    let weak = progress_window.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak.upgrade() {
                            w.set_strata_vision_status(text.into());
                            if let Some(fraction) = fraction { w.set_strata_vision_progress(fraction); }
                        }
                    });
                };
                let root = local_ai::default_root();
                let commit = || {
                        let mut c = cfg.write();
                        // Other Settings tabs can still save fields while a worker
                        // runs. Never overwrite a newer route choice on completion.
                        if c.ai_provider != snapshot.ai_provider || c.ai_local_base_url != snapshot.ai_local_base_url || c.ai_local_model != snapshot.ai_local_model || c.vision_provider != snapshot.vision_provider || c.deep_lock {
                            anyhow::bail!("the selected routes changed during activation");
                        }
                        let mut updated = c.clone();
                        updated.ai_local_vision = true;
                        updated.vision_provider = "same".into();
                        config::save(&updated)?;
                        *c = updated;
                    Ok(())
                };
                let result = if activate {
                    match local_ai::blocking_acquire_lifecycle(&lock) {
                        Some(_permit) => local_ai::enable_strata_vision(&root, &cancel, &on, &commit),
                        None => Err(anyhow::anyhow!("local AI lifecycle unavailable")),
                    }
                } else {
                    local_ai::download_strata_vision(&root, &cancel, &on)
                };
                if let Err(e) = &result { eprintln!("[overlay-host] Strata Vision: {e:#}"); }
                let cancelled = cancel.load(Ordering::Relaxed);
                let message = match (result.is_ok(), activate, cancelled, ru) {
                    (true, true, _, true) => "Изображение проверено. Strata включена для F8.",
                    (true, true, _, false) => "Image input verified. Strata is enabled for F8.",
                    (true, false, _, true) => "Энкодер скачан и проверен. F8 не изменён.",
                    (true, false, _, false) => "Encoder downloaded and verified. F8 is unchanged.",
                    (false, _, true, true) => "Отменено. Повторная загрузка продолжится с сохранённого места.",
                    (false, _, true, false) => "Cancelled. Download again to resume.",
                    (false, true, _, true) => "Не удалось включить Vision. Маршрут F8 не изменён. Подробности в журнале.",
                    (false, true, _, false) => "Vision activation failed. F8 is unchanged. Details are in the log.",
                    (false, false, _, true) => "Загрузка не удалась. Маршруты не изменены. Повторите попытку.",
                    (false, false, _, false) => "Download failed. Routes are unchanged. Retry when ready.",
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        w.set_strata_vision_job(0);
                        let c = cfg.read();
                        refresh(&w, &c);
                        w.set_ai_local_vision(c.ai_local_vision);
                        w.set_ai_local_vision_available(local_ai::local_vision_available(&c, &root));
                        w.set_vision_same_available(super::settings_vision::same_choice_available(&c));
                        w.set_vision_provider_index(super::settings_vision::vision_provider_index_from_id(&c.vision_provider));
                        w.set_strata_vision_status(message.into());
                    }
                });
            });
        };
        if activate { win.on_strata_vision_enable(callback); } else { win.on_strata_vision_download(callback); }
    }
}
