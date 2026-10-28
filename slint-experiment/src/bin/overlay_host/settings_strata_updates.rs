//! Settings actions for Strata's engine-only signed updater.
use super::{ComponentHandle, SettingsWindow};
use overlay_backend::{config, local_ai};
use std::sync::atomic::Ordering;

pub(crate) fn refresh(win: &SettingsWindow) {
    let root = local_ai::default_root();
    win.set_strata_update_supported(local_ai::strata_update_available(&root));
    win.set_strata_engine_version(
        local_ai::strata_engine_version(&root)
            .unwrap_or_default()
            .into(),
    );
}

pub(crate) fn wire(
    win: &SettingsWindow,
    cfg: &config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
) {
    for check in [true, false] {
        let cfg = cfg.clone();
        let state = state.clone();
        let weak = win.as_weak();
        let action = move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_strata_busy() || w.get_model_switching() || w.get_strata_vision_job() != 0 {
                return;
            }
            let snapshot = cfg.read().clone();
            let ru = snapshot.ui_is_ru();
            if !check && snapshot.deep_lock {
                w.set_strata_update_status(
                    overlay_backend::deep_lock::lifecycle_guard_notice(ru).into(),
                );
                return;
            }
            let (busy, cancel, lock) = {
                let s = state.lock().unwrap_or_else(|p| p.into_inner());
                (
                    s.local_ai_busy.clone(),
                    s.local_ai_cancel.clone(),
                    s.local_ai_lock.clone(),
                )
            };
            let Some(guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire(busy) else {
                return;
            };
            cancel.store(false, Ordering::Relaxed);
            w.set_strata_update_job(if check { 1 } else { 2 });
            w.set_strata_busy(true);
            w.set_strata_update_status(
                if ru {
                    "Подготовка…"
                } else {
                    "Preparing…"
                }
                .into(),
            );
            let weak = weak.clone();
            let cfg = cfg.clone();
            std::thread::spawn(move || {
                let _guard = guard;
                let progress = weak.clone();
                let on = move |event: local_ai::Progress| {
                    let text = match event {
                        local_ai::Progress::Step(step)
                            if step == "Starting Strata"
                                || step.starts_with("Loading the model") =>
                        {
                            if ru {
                                "Запускаю Strata с установленным движком…"
                            } else {
                                "Starting Strata with the installed engine…"
                            }
                        }
                        _ if check => {
                            if ru {
                                "Проверяю официальный канал обновлений Strata…"
                            } else {
                                "Checking the official Strata update feed…"
                            }
                        }
                        _ => {
                            if ru {
                                "Обновляю движок Strata. Веса модели остаются на диске…"
                            } else {
                                "Updating the Strata engine. Model weights are kept…"
                            }
                        }
                    };
                    let weak = progress.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak.upgrade() {
                            w.set_strata_update_status(text.into());
                        }
                    });
                };
                let root = local_ai::default_root();
                let result = match local_ai::blocking_acquire_lifecycle(&lock) {
                    Some(_permit) if !check && cfg.read().deep_lock => {
                        Err(anyhow::anyhow!("local AI is deep-locked"))
                    }
                    Some(_permit) => local_ai::update_strata_engine(&root, check, &cancel, &on),
                    None => Err(anyhow::anyhow!("local AI lifecycle unavailable")),
                };
                if let Err(error) = &result {
                    eprintln!("[overlay-host] Strata updater: {error:#}");
                }
                let message = match result {
                    Ok(report) if check => {
                        let release = report
                            .release_version
                            .map(|version| {
                                if ru {
                                    format!("Релиз Strata {version}. ")
                                } else {
                                    format!("Strata release {version}. ")
                                }
                            })
                            .unwrap_or_default();
                        let engine = if report.update_available {
                            if ru {
                                format!(
                                    "Доступен движок {}. Нажмите «Обновить движок Strata».",
                                    report.version
                                )
                            } else {
                                format!(
                                    "Engine {} is available. Click Update Strata engine.",
                                    report.version
                                )
                            }
                        } else if ru {
                            format!("В последнем официальном канале нет более нового совместимого движка. Установлен {}.", report.version)
                        } else {
                            format!("The latest official feed has no newer compatible engine. Installed: {}.", report.version)
                        };
                        format!("{release}{engine}")
                    }
                    Ok(report) => {
                        if ru {
                            format!(
                                "Установлен движок Strata {}. Веса модели не изменены.",
                                report.version
                            )
                        } else {
                            format!(
                                "Strata engine {} is installed. Model weights are unchanged.",
                                report.version
                            )
                        }
                    }
                    Err(_) if cancel.load(Ordering::Relaxed) => {
                        if ru {
                            "Отменено. Веса модели сохранены.".into()
                        } else {
                            "Cancelled. Model weights are kept.".into()
                        }
                    }
                    Err(_) => {
                        if ru {
                            "Не удалось завершить обновление. Веса модели сохранены. Подробности в журнале.".into()
                        } else {
                            "Could not complete the engine update. Model weights are kept. Details are in the log.".into()
                        }
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        w.set_strata_update_job(0);
                        w.set_strata_busy(false);
                        refresh(&w);
                        w.set_strata_update_status(message.into());
                    }
                });
            });
        };
        if check {
            win.on_strata_engine_check(action);
        } else {
            win.on_strata_engine_update(action);
        }
    }
}
