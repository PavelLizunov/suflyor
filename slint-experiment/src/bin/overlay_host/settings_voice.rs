//! Read-aloud (Озвучка) Settings tab: engine choice + voice chooser + speed
//! preset + a test.
//!
//! Mirrors `settings_vision.rs`'s style — a `wire_voice_settings(&win, cfg)` that
//! connects the panel callbacks. The voice list + the initial dropdown indices
//! are seeded in `open_settings` (settings_controller.rs). The neural TTS itself
//! runs in the `suflyor-tts.exe` sidecar; this module only saves config and
//! nudges the running sidecar live through the `overlay_backend::tts` client.
//!
//! RC17 adds the experimental Tera engine: an engine chooser, a Tera model
//! installer (on-demand download with SHA-verify + cancel), and namespaced
//! voice ids (`piper:<dir>` / `tera:<style>`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::{ComponentHandle, ModelRc, SettingsWindow, SharedString, VecModel};

/// Engine chooser label for the Tera option (always marked experimental).
pub(crate) fn tera_engine_label(ru: bool) -> &'static str {
    if ru {
        "Tera (экспериментально)"
    } else {
        "Tera (experimental)"
    }
}

/// Dropdown label for a Tera voice style id — the upstream style ids are the
/// stable names (`ru_f1`…), prefixed so the engine is obvious.
pub(crate) fn tera_voice_label(id: &str) -> String {
    format!("Tera {id}")
}

/// Generic screen-share-safe copy for a read-aloud request the selected
/// engine could not accept.
pub(crate) fn tts_unavailable_status(ru: bool) -> &'static str {
    if ru {
        "Озвучка сейчас недоступна. Проверьте установленный голос и повторите."
    } else {
        "Read-aloud is unavailable. Check the installed voice and try again."
    }
}

pub(crate) fn tts_test_status(accepted: bool, ru: bool) -> &'static str {
    if accepted {
        ""
    } else {
        tts_unavailable_status(ru)
    }
}

/// Localized one-line model status for the Tera section (ASCII markers only —
/// no tofu glyphs).
pub(crate) fn tera_status_line(
    state: overlay_backend::teratts_install::TeraInstalled,
    ru: bool,
) -> String {
    use overlay_backend::teratts_install::TeraInstalled;
    match (state, ru) {
        (TeraInstalled::Ready, true) => "[ок] Модель TeraTTSv2 установлена и готова".into(),
        (TeraInstalled::Ready, false) => "[ok] TeraTTSv2 model installed and ready".into(),
        (TeraInstalled::Missing, true) => {
            "[--] Модель не установлена — нажмите «Установить модель» (~370 МБ)".into()
        }
        (TeraInstalled::Missing, false) => {
            "[--] Model not installed — use Install model (~370 MB)".into()
        }
        (TeraInstalled::Broken, true) => {
            "[!] Установка повреждена — отмените и установите заново".into()
        }
        (TeraInstalled::Broken, false) => {
            "[!] Installation broken — cancel and install again".into()
        }
    }
}

/// Map a speed-preset index (the «0.75× … 2.0×» ComboBox) to the engine's
/// integer rate (-10..10, where 0 = 1.0×). Matches `tts::rate_to_speed`:
/// -5 → 0.75×, 0 → 1.0×, +3 → 1.3×, +5 → 1.5×, +10 → 2.0×.
pub(crate) fn tts_rate_for_preset(idx: i32) -> i32 {
    match idx {
        0 => -5,
        2 => 3,
        3 => 5,
        4 => 10,
        _ => 0, // index 1 = «1.0×» (also the fallback for any stray index)
    }
}

/// Inverse of [`tts_rate_for_preset`]: pick the preset index whose rate is
/// nearest the saved `tts_rate`, so the ComboBox reflects the stored speed on
/// (re)open. Defaults to «1.0×» (index 1).
pub(crate) fn preset_for_tts_rate(rate: i32) -> i32 {
    [(-5, 0), (0, 1), (3, 2), (5, 3), (10, 4)]
        .into_iter()
        .min_by_key(|(r, _)| (r - rate).abs())
        .map_or(1, |(_, idx)| idx)
}

/// Wire the Read-aloud-tab Settings callbacks onto the Settings window.
pub(crate) fn wire_voice_settings(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    // Voice chooser: index → the CURRENT engine's voice id, saved as a
    // namespaced reference (`piper:<dir>` / `tera:<style>`); apply live.
    {
        let cfg_c = cfg.clone();
        win.on_tts_voice_changed(move |idx| {
            let engine = overlay_backend::tts::parse_engine(&cfg_c.read().tts_engine);
            let id = match engine {
                overlay_backend::tts::EngineKind::Piper => {
                    let ru = cfg_c.read().ui_is_ru();
                    let voices = overlay_backend::tts::voices(ru);
                    let Some(v) = voices.get(idx.max(0) as usize) else {
                        return;
                    };
                    overlay_backend::tts::format_voice_ref(&overlay_backend::tts::VoiceRef {
                        engine,
                        id: v.id.clone(),
                    })
                }
                overlay_backend::tts::EngineKind::Tera => {
                    let ids = overlay_backend::tts::tera_voice_ids();
                    let Some(id) = ids.get(idx.max(0) as usize) else {
                        return;
                    };
                    overlay_backend::tts::format_voice_ref(&overlay_backend::tts::VoiceRef {
                        engine,
                        id: id.clone(),
                    })
                }
            };
            {
                let mut c = cfg_c.write();
                c.tts_voice = id.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tts_voice save failed: {e:#}");
                    return;
                }
            }
            overlay_backend::tts::set_voice(&id);
            diag!("tts_voice -> {id}");
        });
    }
    // Engine chooser: save `tts_engine`, switch the live client, and reseed the
    // tab so the voice list + install surface follow the new engine.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_tts_engine_changed(move |idx| {
            let engine_raw = if idx == 1 { "tera" } else { "piper" };
            {
                let mut c = cfg_c.write();
                c.tts_engine = engine_raw.to_string();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tts_engine save failed: {e:#}");
                    return;
                }
            }
            overlay_backend::tts::set_engine(engine_raw);
            if let Some(w) = weak.upgrade() {
                let c = cfg_c.read();
                super::settings_controller::populate_tts_voices(&w, &c);
            }
            diag!("tts_engine -> {engine_raw}");
        });
    }
    // Speed preset: index → integer rate; save + apply live.
    {
        let cfg_c = cfg.clone();
        win.on_tts_rate_changed(move |idx| {
            let rate = tts_rate_for_preset(idx);
            {
                let mut c = cfg_c.write();
                c.tts_rate = rate;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tts_rate save failed: {e:#}");
                    return;
                }
            }
            overlay_backend::tts::set_rate(rate);
            diag!("tts_rate -> {rate}");
        });
    }
    // Test: speak a short sample with the CURRENT voice + speed (no tile — this
    // is a quick aural check). Plays through the sidecar like any read-aloud.
    let cfg_test = cfg.clone();
    let weak_test = win.as_weak();
    win.on_tts_test_clicked(move || {
        let accepted = overlay_backend::tts::speak("Привет! Это проверка озвучки: раз, два, три.");
        let status = tts_test_status(accepted, cfg_test.read().ui_is_ru());
        if let Some(w) = weak_test.upgrade() {
            w.set_tts_test_status(SharedString::from(status));
        }
        if !status.is_empty() {
            diag!("[overlay-host] voice test unavailable");
        }
    });

    // Install the neural voices on demand (like the local-AI model installer):
    // download + SHA-verify + extract on a worker thread, then refresh the
    // chooser. The packs are large, so they are NOT bundled in the app installer.
    {
        let weak = win.as_weak();
        let cfg_install = cfg.clone();
        win.on_tts_install_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if w.get_tts_installing() {
                return; // already running
            }
            w.set_tts_installing(true);
            w.set_tts_install_phase(1); // preparing
            w.set_tts_install_label(SharedString::from(""));
            // Language for the pack labels interpolated into the @tr phase
            // templates (English UI must not show Cyrillic voice names).
            let ru = cfg_install.read().ui_is_ru();
            let weak_done = w.as_weak();
            let cfg_t = cfg_install.clone();
            std::thread::spawn(move || {
                let cancel = std::sync::atomic::AtomicBool::new(false);
                let weak_cb = weak_done.clone();
                let on = move |p: overlay_backend::tts_install::VoiceProgress| {
                    use overlay_backend::tts_install::VoiceProgress;
                    // Map the semantic variant → (phase int, label). The .slint
                    // renders the localized text via @tr from the phase; the label
                    // is the (untranslated) voice name / failed-pack list.
                    let (phase, label): (i32, String) = match p {
                        VoiceProgress::Downloading(l) => (2, l),
                        VoiceProgress::Verifying(l) => (3, l),
                        VoiceProgress::Unpacking(l) => (4, l),
                        VoiceProgress::AlreadyInstalled(l) => (5, l),
                        VoiceProgress::AllInstalled => (6, String::new()),
                        VoiceProgress::PartiallyInstalled(f) => (7, f),
                        VoiceProgress::PackFailed(l) => (9, l),
                    };
                    let weak_in = weak_cb.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = weak_in.upgrade() {
                            w.set_tts_install_phase(phase);
                            w.set_tts_install_label(SharedString::from(label));
                        }
                    });
                };
                let result = overlay_backend::tts_install::install_voices(&cancel, &on, ru);
                if let Err(e) = &result {
                    // Detail to the local log only; the Settings field stays
                    // generic (it is screen-shareable — no path/url leak).
                    diag!("[overlay-host] voice install failed: {e:#}");
                }
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else {
                        return;
                    };
                    w.set_tts_installing(false);
                    if result.is_err() {
                        w.set_tts_install_phase(8); // generic failure
                        return;
                    }
                    // Success — refresh the chooser from the freshly-installed
                    // voices, select the first, and warm the sidecar so 🔊 is
                    // prompt without restarting.
                    let voices = overlay_backend::tts::voices(ru);
                    let names: Vec<SharedString> = voices
                        .iter()
                        .map(|v| SharedString::from(v.name.as_str()))
                        .collect();
                    w.set_tts_available(!voices.is_empty());
                    w.set_tts_voice_names(ModelRc::new(VecModel::from(names)));
                    w.set_tts_voice_index(0);
                    // NB: don't overwrite the phase here — install_voices already
                    // set the final phase 6 (all installed) / 7 (partial) via the
                    // progress callback (it ran just before this).
                    if let Some(first) = voices.first() {
                        // Persist the selection so a restart resolves to the SAME
                        // voice the live session is now playing (not just whatever
                        // pick_voice_id would prefer). RC17: store the
                        // namespaced reference (`piper:<dir>`).
                        let namespaced = overlay_backend::tts::format_voice_ref(
                            &overlay_backend::tts::VoiceRef {
                                engine: overlay_backend::tts::EngineKind::Piper,
                                id: first.id.clone(),
                            },
                        );
                        {
                            let mut c = cfg_t.write();
                            c.tts_voice = namespaced.clone();
                            if let Err(e) = overlay_backend::config::save(&c) {
                                diag!("[overlay-host] tts_voice save after install failed: {e:#}");
                            }
                        }
                        overlay_backend::tts::set_voice(&namespaced);
                    }
                    overlay_backend::tts::warm();
                });
            });
        });
    }

    // RC17 — Tera model installer: download the pinned TeraTTSv2 revision with
    // SHA-verify into a staging dir + one atomic publish, on a worker thread.
    // Cancel is generation-based: each start bumps a generation and stores a
    // fresh flag; callbacks from stale generations are dropped.
    let tera_state: Arc<Mutex<(u64, Option<Arc<AtomicBool>>)>> = Arc::new(Mutex::new((0, None)));
    {
        let weak = win.as_weak();
        let cfg_install = cfg.clone();
        let state = tera_state.clone();
        win.on_tera_install_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if w.get_tera_installing() {
                return; // already running
            }
            let (generation, cancel) = {
                let mut guard = match state.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                guard.0 += 1;
                let flag = Arc::new(AtomicBool::new(false));
                guard.1 = Some(flag.clone());
                (guard.0, flag)
            };
            w.set_tera_installing(true);
            w.set_tera_install_phase(1); // preparing
            w.set_tera_install_label(SharedString::from(""));
            let weak_done = w.as_weak();
            let cfg_t = cfg_install.clone();
            let state_done = state.clone();
            std::thread::spawn(move || {
                let weak_cb = weak_done.clone();
                let state_cb = state_done.clone();
                let on = move |p: overlay_backend::teratts_install::TeraProgress| {
                    use overlay_backend::teratts_install::TeraProgress;
                    let (phase, label): (i32, String) = match p {
                        TeraProgress::Preparing => (1, String::new()),
                        TeraProgress::Downloading { file, index, total } => {
                            (2, format!("{file} ({index}/{total})"))
                        }
                        TeraProgress::Verifying { file } => (3, file),
                        TeraProgress::Publishing => (4, String::new()),
                        TeraProgress::Installed => (6, String::new()),
                    };
                    let weak_in = weak_cb.clone();
                    let state_in = state_cb.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        // Drop stale-generation callbacks (a cancelled run that
                        // reports after a newer one started).
                        let current = state_in.lock().map(|g| g.0).unwrap_or(u64::MAX);
                        if current != generation {
                            return;
                        }
                        if let Some(w) = weak_in.upgrade() {
                            w.set_tera_install_phase(phase);
                            w.set_tera_install_label(SharedString::from(label));
                        }
                    });
                };
                let result = overlay_backend::teratts_install::install(&cancel, &on);
                if let Err(e) = &result {
                    // Detail to the local log only; the Settings field stays
                    // generic (screen-shareable — no path/url leak).
                    diag!("[overlay-host] tera model install failed: {e:#}");
                }
                let cancelled = cancel.load(Ordering::Acquire);
                let _ = slint::invoke_from_event_loop(move || {
                    let current = state_done.lock().map(|g| g.0).unwrap_or(u64::MAX);
                    if current != generation {
                        return;
                    }
                    if let Ok(mut guard) = state_done.lock() {
                        guard.1 = None;
                    }
                    let Some(w) = weak_done.upgrade() else {
                        return;
                    };
                    w.set_tera_installing(false);
                    match &result {
                        Ok(()) => {
                            // Success — reseed the tab (status flips to ready,
                            // voice list becomes usable) and warm the sidecar if
                            // Tera is the selected engine.
                            let c = cfg_t.read();
                            super::settings_controller::populate_tts_voices(&w, &c);
                            overlay_backend::tts::warm();
                        }
                        Err(_) => {
                            w.set_tera_install_phase(if cancelled { 8 } else { 9 });
                        }
                    }
                });
            });
        });
    }
    {
        let state = tera_state.clone();
        win.on_tera_install_cancel_clicked(move || {
            if let Ok(guard) = state.lock() {
                if let Some(flag) = &guard.1 {
                    flag.store(true, Ordering::Release);
                    diag!("[overlay-host] tera model install cancel requested");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_rate_round_trips() {
        // Every preset index maps to a rate that maps back to the same index.
        for idx in 0..=4 {
            assert_eq!(preset_for_tts_rate(tts_rate_for_preset(idx)), idx);
        }
    }

    #[test]
    fn stray_index_and_rate_default_to_normal() {
        assert_eq!(tts_rate_for_preset(99), 0); // unknown preset → 1.0×
        assert_eq!(tts_rate_for_preset(-1), 0);
        // An arbitrary saved rate snaps to the NEAREST preset (rate 6 → 1.5×=idx3).
        assert_eq!(preset_for_tts_rate(6), 3);
        assert_eq!(preset_for_tts_rate(-10), 0); // below the slowest preset → 0.75×
                                                 // An exact tie (rate 4 is equidistant from idx2 and idx3) picks the
                                                 // first/lower preset — pinned so the behaviour is intentional.
        assert_eq!(preset_for_tts_rate(4), 2);
    }

    #[test]
    fn tera_labels_are_localized_and_ascii_safe() {
        assert_eq!(tera_engine_label(true), "Tera (экспериментально)");
        assert_eq!(tera_engine_label(false), "Tera (experimental)");
        assert_eq!(tera_voice_label("ru_f1"), "Tera ru_f1");
        // Status lines: localized, ASCII markers only (no tofu glyphs), no
        // paths/urls (screen-shareable).
        use overlay_backend::teratts_install::TeraInstalled;
        for state in [
            TeraInstalled::Ready,
            TeraInstalled::Missing,
            TeraInstalled::Broken,
        ] {
            for ru in [true, false] {
                let line = tera_status_line(state, ru);
                assert!(!line.is_empty());
                assert!(!line.contains("http"), "{line}");
                assert!(line.starts_with('['), "{line}");
            }
        }
        assert!(tera_status_line(TeraInstalled::Ready, true).contains("установлена"));
        assert!(tera_status_line(TeraInstalled::Ready, false).contains("installed"));
    }

    #[test]
    fn unavailable_status_is_localized_and_screen_share_safe() {
        let ru = tts_unavailable_status(true);
        let en = tts_unavailable_status(false);
        assert!(ru.contains("недоступна"));
        assert!(en.contains("unavailable"));
        for line in [ru, en] {
            assert!(!line.contains("http"));
            assert!(!line.contains('\\'));
            assert!(!line.contains('/'));
        }
        assert_eq!(tts_test_status(true, true), "");
        assert_eq!(tts_test_status(false, true), ru);
        assert_eq!(tts_test_status(false, false), en);
    }
}
