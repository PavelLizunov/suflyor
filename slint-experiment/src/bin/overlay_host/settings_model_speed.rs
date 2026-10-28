//! User-triggered local-model speed sample; hiding Settings never cancels it.
use super::{ComponentHandle, SettingsWindow};
use overlay_backend::{ai::benchmark, config, deep_lock};
use std::sync::atomic::Ordering;

pub(crate) fn refresh(win: &SettingsWindow, cfg: &config::Config) {
    if win.get_model_speed_running() { return; }
    win.set_model_speed_status("".into());
    let text = benchmark::load_last_sample(cfg).map(|saved| {
        describe(&saved.sample, &saved.context_preset, saved.engine_version.as_deref(), cfg.ui_is_ru())
    }).unwrap_or_default();
    win.set_model_speed_result(text.into());
}

fn describe(sample: &benchmark::ModelSpeedSample, context: &str, engine: Option<&str>, ru: bool) -> String {
    let unavailable = if ru { "нет данных" } else { "unavailable" };
    let rate = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0)
        .map(|v| format!("{v:.1}")).unwrap_or_else(|| unavailable.into());
    let count = |v: Option<u64>| v.map(|n| n.to_string()).unwrap_or_else(|| unavailable.into());
    let first = sample.first_token_ms.map(|ms| format!("{:.2}", ms / 1000.0))
        .unwrap_or_else(|| unavailable.into());
    let stamp = overlay_backend::journal::format_msk_label(sample.measured_at_ms as i64, ru);
    let name = &sample.model;
    let engine = engine.unwrap_or(if ru { "версия движка неизвестна" } else { "engine version unknown" });
    if ru {
        format!("{name} · {engine} · профиль контекста: {context}\nПоследний замер: {stamp}\nPrefill: {} ток/с · Decode: {} ток/с\nПервый токен: {first} с · всего: {:.2} с\nВход: {} · из кэша: {} · выход: {} токенов",
            rate(sample.prefill_tps), rate(sample.decode_tps), sample.total_ms / 1000.0,
            count(sample.input_tokens), count(sample.cached_tokens), count(sample.output_tokens))
    } else {
        format!("{name} · {engine} · context preset: {context}\nLast measured: {stamp}\nPrefill: {} tok/s · Decode: {} tok/s\nFirst token: {first} s · total: {:.2} s\nInput: {} · cached: {} · output: {} tokens",
            rate(sample.prefill_tps), rate(sample.decode_tps), sample.total_ms / 1000.0,
            count(sample.input_tokens), count(sample.cached_tokens), count(sample.output_tokens))
    }
}

pub(crate) fn wire(
    win: &SettingsWindow,
    cfg: &config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
) {
    {
        let state = state.clone();
        let weak = win.as_weak();
        win.on_cancel_model_speed(move || {
            if weak.upgrade().is_some_and(|w| w.get_model_speed_running()) {
                state.lock().unwrap_or_else(|p| p.into_inner()).local_ai_cancel.store(true, Ordering::Release);
            }
        });
    }
    let weak = win.as_weak();
    let cfg = cfg.clone();
    let state = state.clone();
    win.on_measure_model_speed(move || {
        let Some(w) = weak.upgrade() else { return };
        if w.get_model_speed_running() { return; }
        let snapshot = cfg.read().clone();
        let ru = snapshot.ui_is_ru();
        if snapshot.ai_provider != "local" {
            w.set_model_speed_status(if ru { "Сначала выберите локальную модель." } else { "Select a local model first." }.into());
            return;
        }
        if deep_lock::endpoint_blocked(snapshot.deep_lock, &snapshot.ai_local_base_url) {
            w.set_model_speed_status(deep_lock::lifecycle_guard_notice(ru).into());
            return;
        }
        let (busy, cancel, session) = {
            let state = state.lock().unwrap_or_else(|p| p.into_inner());
            (state.local_ai_busy.clone(), state.local_ai_cancel.clone(), state.timer_active)
        };
        if session {
            w.set_model_speed_status(if ru { "Завершите сессию перед замером, чтобы не задерживать ответы." } else { "End the session before measuring to avoid delaying answers." }.into());
            return;
        }
        let Some(guard) = slint_replay::app_state::LocalAiBusyGuard::try_acquire(busy) else { return };
        cancel.store(false, Ordering::Release);
        let size = w.get_model_speed_size().clamp(0, 2) as u32;
        w.set_model_speed_running(true);
        w.set_model_speed_status(if ru { "Измеряю скорость. Окно можно закрыть; «Отмена» остановит замер." } else { "Measuring speed. You can close Settings; Cancel stops the measurement." }.into());
        let cfg = cfg.clone();
        let weak = weak.clone();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<benchmark::ModelSpeedSample> {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                rt.block_on(benchmark::measure_model_speed(snapshot.ai_endpoint(false), size, &cancel))
            })();
            if let Err(error) = &result { eprintln!("[overlay-host] Model speed: {error}"); }
            let _ = slint::invoke_from_event_loop(move || {
                let _guard = guard;
                let Some(w) = weak.upgrade() else { return };
                w.set_model_speed_running(false);
                let current = cfg.read();
                if current.ai_provider != snapshot.ai_provider
                    || current.ai_local_base_url != snapshot.ai_local_base_url
                    || current.ai_local_model != snapshot.ai_local_model
                    || current.ai_local_context != snapshot.ai_local_context
                    || current.ai_local_custom_gguf != snapshot.ai_local_custom_gguf
                {
                    refresh(&w, &current);
                    w.set_model_speed_status(if ru { "Модель или контекст изменились; запустите новый замер." } else { "The model or context changed; run a new measurement." }.into());
                    return;
                }
                let status = match result {
                    Ok(_) if cancel.load(Ordering::Acquire) => if ru { "Замер отменён. Предыдущий результат сохранён." } else { "Measurement cancelled. The previous result is kept." },
                    Ok(sample) => {
                        let missing = sample.prefill_tps.is_none() || sample.decode_tps.is_none();
                        let saved = benchmark::save_last_sample(&snapshot, sample.clone()).is_ok();
                        if saved {
                            refresh(&w, &current);
                        } else {
                            w.set_model_speed_result(describe(&sample, &snapshot.ai_local_context, None, ru).into());
                        }
                        if !saved {
                            if ru { "Замер завершён, но сохранить результат не удалось." } else { "Measurement finished, but could not be saved." }
                        } else if missing {
                            if ru { "Замер сохранён. Сервер не передал часть метрик; они не заменены оценкой по общему времени." } else { "Measurement saved. The server omitted some metrics; total time is not used to invent them." }
                        } else if ru { "Замер сохранён. Скорость — по данным движка; первый токен и общее время могут включать ожидание и загрузку модели." }
                        else { "Measurement saved. Speed is engine-reported; first-token and total time may include waiting and model loading." }
                    }
                    Err(_) if cancel.load(Ordering::Acquire) => if ru { "Замер отменён. Предыдущий результат сохранён." } else { "Measurement cancelled. The previous result is kept." },
                    Err(error) if deep_lock::is_blocked_error(&error.to_string()) => deep_lock::lifecycle_guard_notice(ru),
                    Err(_) => if ru { "Замер не завершён. Убедитесь, что модель загружена и сервер отвечает. Подробности в журнале." } else { "Measurement failed. Make sure the model is loaded and the server responds. Details are in the log." },
                };
                w.set_model_speed_status(status.into());
            });
        });
    });
}
