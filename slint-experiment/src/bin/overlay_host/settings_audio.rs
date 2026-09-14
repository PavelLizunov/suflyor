//! Windows audio settings. Device names remain the backend's existing contract.

use super::SettingsWindow;
use overlay_backend::{audio, config};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

/// Index zero is always the default, never an endpoint or a placeholder.
struct DeviceChoices {
    names: Vec<String>,
    selected: i32,
    missing: bool,
    empty: bool,
}

impl DeviceChoices {
    fn new(devices: Vec<String>, saved: Option<&str>, default_label: &str) -> Self {
        let mut names = vec![default_label.to_owned()];
        for name in devices {
            if !name.trim().is_empty() && !names[1..].contains(&name) {
                names.push(name);
            }
        }
        let empty = names.len() == 1;
        let saved = saved.filter(|name| !name.trim().is_empty());
        let mut missing = false;
        let selected = match saved {
            None => 0,
            Some(name) => match names[1..].iter().position(|item| item == name) {
                Some(index) => index + 1,
                None => {
                    missing = true;
                    names.push(name.to_owned());
                    names.len() - 1
                }
            },
        };
        Self { names, selected: selected as i32, missing, empty }
    }
}

fn selection(names: &[String], index: i32, missing: bool) -> Option<Option<String>> {
    let index = usize::try_from(index).ok()?;
    if index >= names.len() || (missing && index == names.len() - 1) {
        return None;
    }
    Some(if index == 0 { None } else { Some(names[index].clone()) })
}

fn saved_index(names: &[String], saved: Option<&str>) -> i32 {
    saved.filter(|name| !name.trim().is_empty()).and_then(|name| {
        names.iter().skip(1).position(|item| item == name).map(|index| index + 1)
    }).unwrap_or(0) as i32
}

fn persist_selection(
    cfg: &config::SharedConfig,
    system: bool,
    selected: Option<String>,
    persist: impl FnOnce(&config::Config) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut current = cfg.write();
    let mut candidate = current.clone();
    if system {
        candidate.system_audio_device = selected;
    } else {
        candidate.mic_device = selected;
    }
    // Keep the write lock until the atomic save finishes: another Settings
    // callback must not be overwritten by an older asynchronous snapshot.
    persist(&candidate)?;
    *current = candidate;
    Ok(())
}

fn model(names: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(names.into_iter().map(SharedString::from).collect::<Vec<_>>()))
}

fn show_devices(win: &SettingsWindow, cfg: &config::SharedConfig, inputs: Vec<String>, outputs: Vec<String>) {
    let current = cfg.read();
    let label = win.get_audio_default_label();
    let mic = DeviceChoices::new(inputs.clone(), current.mic_device.as_deref(), label.as_str());
    // Some headsets expose their mixed system stream as a Capture endpoint
    // (A50 Stream Out). The backend already resolves Render, then Capture.
    let system = DeviceChoices::new(outputs.into_iter().chain(inputs).collect(), current.system_audio_device.as_deref(), label.as_str());
    win.set_mic_devices(model(mic.names));
    win.set_mic_device_index(mic.selected);
    win.set_mic_device_missing(mic.missing);
    win.set_mic_devices_empty(mic.empty);
    win.set_system_devices(model(system.names));
    win.set_system_device_index(system.selected);
    win.set_system_device_missing(system.missing);
    win.set_system_devices_empty(system.empty);
}

pub(super) fn refresh(win: &SettingsWindow, cfg: &config::SharedConfig) {
    // At most one enumeration per reused window. The completion reads the
    // latest config and translated label, not a pre-enumeration snapshot.
    if win.get_audio_devices_loading() {
        return;
    }
    win.set_audio_devices_loading(true);
    win.set_audio_devices_failed(false);
    let weak = win.as_weak();
    let cfg = cfg.clone();
    std::thread::spawn(move || {
        let devices = audio::list_devices();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(win) = weak.upgrade() else { return };
            match devices {
                Ok(devices) => show_devices(&win, &cfg, devices.inputs, devices.outputs),
                Err(_) => win.set_audio_devices_failed(true),
            }
            win.set_audio_devices_loading(false);
        });
    });
}

fn choose(win: &SettingsWindow, cfg: &config::SharedConfig, system: bool, index: i32, persist: impl FnOnce(&config::Config) -> anyhow::Result<()>) {
    if win.get_audio_devices_loading() || win.get_audio_devices_failed() {
        return;
    }
    let (items, missing) = if system {
        (win.get_system_devices(), win.get_system_device_missing())
    } else {
        (win.get_mic_devices(), win.get_mic_device_missing())
    };
    let names: Vec<String> = items.iter().map(|item| item.to_string()).collect();
    let previous = {
        let current = cfg.read();
        saved_index(&names, if system { current.system_audio_device.as_deref() } else { current.mic_device.as_deref() })
    };
    let chosen = selection(&names, index, missing);
    let restored_index = match chosen {
        Some(value) => match persist_selection(cfg, system, value, persist) {
            Ok(()) => {
                win.set_audio_save_state(1);
                // Remove the unavailable row only after saving the replacement.
                // Otherwise the old missing flag would disable the new mic too.
                if missing {
                    let mut available = names.clone();
                    available.pop();
                    if system {
                        win.set_system_devices(model(available));
                        win.set_system_device_missing(false);
                    } else {
                        win.set_mic_devices(model(available));
                        win.set_mic_device_missing(false);
                    }
                }
                if !system { win.set_mic_test_result(SharedString::default()); }
                index
            }
            Err(_) => {
                // Never forward the config error chain (it can contain paths).
                win.set_audio_save_state(2);
                previous
            }
        },
        None => previous,
    };
    if system {
        win.set_system_device_index(restored_index);
    } else {
        win.set_mic_device_index(restored_index);
    }
}

pub(super) fn wire(win: &SettingsWindow, cfg: &config::SharedConfig) {
    let weak = win.as_weak();
    let current = cfg.clone();
    win.on_audio_devices_refresh(move || {
        if let Some(win) = weak.upgrade() { refresh(&win, &current); }
    });
    let weak = win.as_weak();
    let current = cfg.clone();
    win.on_mic_device_index_selected(move |index| {
        if let Some(win) = weak.upgrade() { choose(&win, &current, false, index, config::save); }
    });
    let weak = win.as_weak();
    let current = cfg.clone();
    win.on_system_device_selected(move |index| {
        if let Some(win) = weak.upgrade() { choose(&win, &current, true, index, config::save); }
    });
    refresh(win, cfg);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn default_is_not_the_first_endpoint_or_a_translated_name() {
        let choices = DeviceChoices::new(vec!["Mic B".into(), "Mic A".into()], None, "Windows default");
        assert_eq!(choices.selected, 0);
        assert_eq!(selection(&choices.names, 0, choices.missing), Some(None));
        assert_eq!(selection(&choices.names, 1, false), Some(Some("Mic B".into())));
        let same_label = DeviceChoices::new(vec!["Windows default".into()], Some("Windows default"), "Windows default");
        assert_eq!(same_label.selected, 1);
        assert_eq!(selection(&same_label.names, 1, false), Some(Some("Windows default".into())));
    }

    #[test]
    fn missing_saved_device_is_preserved_but_not_selectable() {
        let choices = DeviceChoices::new(vec!["New headset".into()], Some("Old headset"), "Default");
        assert!(choices.missing);
        assert_eq!(choices.selected, 2);
        assert_eq!(choices.names[2], "Old headset");
        assert_eq!(selection(&choices.names, 2, true), None);
        assert_eq!(selection(&choices.names, 1, true), Some(Some("New headset".into())));
        assert_eq!(saved_index(&choices.names, Some("Old headset")), 2);
        assert_eq!(selection(&choices.names, -1, true), None);
        assert_eq!(selection(&choices.names, 99, true), None);
    }

    #[test]
    fn empty_list_can_clear_an_obsolete_binding_without_saving_a_placeholder() {
        let choices = DeviceChoices::new(vec![], Some("Gone"), "Default");
        assert!(choices.empty && choices.missing);
        assert_eq!(choices.selected, 1);
        assert_eq!(selection(&choices.names, 0, true), Some(None));
        assert_eq!(selection(&choices.names, 1, true), None);
        let blank = DeviceChoices::new(vec![], Some("   "), "Default");
        assert_eq!(blank.selected, 0);
        assert!(!blank.missing);
    }

    #[test]
    fn capture_mixes_remain_selectable_and_duplicates_do_not_shift_selection() {
        let devices = vec!["Headphones".into(), "A50 Stream Out".into(), "Headphones".into(), "".into()];
        let choices = DeviceChoices::new(devices, Some("A50 Stream Out"), "Default");
        assert_eq!(choices.names.len(), 3);
        assert_eq!(choices.selected, 2);
        assert!(!choices.missing);
    }

    #[test]
    fn window_state_tracks_selection_refresh_and_save_failure() {
        i_slint_backend_testing::init_no_event_loop();
        let win = SettingsWindow::new().unwrap();
        let cfg: config::SharedConfig = Default::default();
        cfg.write().system_audio_device = Some("Old headset".into());
        show_devices(&win, &cfg, vec!["A50 Stream Out".into()], vec!["New headset".into()]);
        assert!(win.get_system_device_missing());
        assert_eq!(win.get_system_device_index(), 3);
        choose(&win, &cfg, true, 1, |_| Ok(()));
        assert_eq!(cfg.read().system_audio_device.as_deref(), Some("New headset"));
        assert!(!win.get_system_device_missing());
        assert_eq!(win.get_system_device_index(), 1);
        assert_eq!(win.get_system_devices().row_count(), 3);
        assert_eq!(win.get_audio_save_state(), 1);
        // Slint changes its bound index before delivering the callback.
        win.set_system_device_index(0);
        choose(&win, &cfg, true, 0, |_| Err(anyhow::anyhow!("synthetic failure")));
        assert_eq!(win.get_system_device_index(), 1);
        assert_eq!(cfg.read().system_audio_device.as_deref(), Some("New headset"));
        assert_eq!(win.get_audio_save_state(), 2);
        choose(&win, &cfg, true, 0, |_| Ok(()));
        assert!(cfg.read().system_audio_device.is_none());
        assert_eq!(win.get_system_device_index(), 0);
        // Fresh enumeration must use current config, including changes made
        // while the worker was enumerating, rather than its starting snapshot.
        cfg.write().system_audio_device = Some("A50 Stream Out".into());
        show_devices(&win, &cfg, vec!["A50 Stream Out".into()], vec![]);
        assert_eq!(win.get_system_device_index(), 1);
        assert!(!win.get_system_devices_empty());
        assert!(!win.get_system_device_missing());
        win.set_audio_devices_loading(true);
        choose(&win, &cfg, true, 0, |_| panic!("must not save during enumeration"));
        win.set_audio_devices_loading(false);
        win.set_audio_devices_failed(true);
        choose(&win, &cfg, true, 0, |_| panic!("must not save after enumeration failure"));
        assert_eq!(cfg.read().system_audio_device.as_deref(), Some("A50 Stream Out"));
    }

    #[test]
    fn save_failure_keeps_config_and_unrelated_fields() {
        let mut initial = config::Config::default();
        initial.mic_device = Some("Mic".into());
        initial.system_audio_device = Some("Old".into());
        initial.meeting_context = "synthetic sentinel".into();
        let cfg: config::SharedConfig = Default::default();
        *cfg.write() = initial;
        let before = serde_json::to_value(&*cfg.read()).unwrap();
        let result = persist_selection(&cfg, true, None, |_| Err(anyhow::anyhow!("synthetic failure")));
        assert!(result.is_err());
        assert_eq!(serde_json::to_value(&*cfg.read()).unwrap(), before);
        persist_selection(&cfg, true, None, |candidate| {
            assert!(candidate.system_audio_device.is_none());
            assert_eq!(candidate.mic_device.as_deref(), Some("Mic"));
            Ok(())
        }).unwrap();
        let mut expected = before;
        expected["system_audio_device"] = serde_json::Value::Null;
        assert_eq!(serde_json::to_value(&*cfg.read()).unwrap(), expected);
    }
}
