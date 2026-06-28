//! Read-aloud (Озвучка) Settings tab: voice chooser + speed preset + a test.
//!
//! Mirrors `settings_vision.rs`'s style — a `wire_voice_settings(&win, cfg)` that
//! connects the panel callbacks. The voice list + the initial dropdown indices
//! are seeded in `open_settings` (settings_controller.rs). The neural TTS itself
//! runs in the `suflyor-tts.exe` sidecar; this module only saves config and
//! nudges the running sidecar live through the `overlay_backend::tts` client.
use super::{ComponentHandle, ModelRc, SettingsWindow, SharedString, VecModel};

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
    // Voice chooser: index → the installed voice's id; save + apply live.
    {
        let cfg_c = cfg.clone();
        win.on_tts_voice_changed(move |idx| {
            let voices = overlay_backend::tts::voices();
            let Some(v) = voices.get(idx.max(0) as usize) else {
                return;
            };
            let id = v.id.clone();
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
    win.on_tts_test_clicked(move || {
        overlay_backend::tts::speak("Привет! Это проверка озвучки: раз, два, три.");
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
                let result = overlay_backend::tts_install::install_voices(&cancel, &on);
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
                    let voices = overlay_backend::tts::voices();
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
                        // pick_voice_id would prefer).
                        {
                            let mut c = cfg_t.write();
                            c.tts_voice = first.id.clone();
                            if let Err(e) = overlay_backend::config::save(&c) {
                                diag!("[overlay-host] tts_voice save after install failed: {e:#}");
                            }
                        }
                        overlay_backend::tts::set_voice(&first.id);
                    }
                    overlay_backend::tts::warm();
                });
            });
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
}
