//! macOS Settings window wiring for the production overlay host.
//!
//! Reuses the shared `SettingsWindow` controller logic so all 16 tabs
//! (AI bridge, STT provider, profiles, retention, language, diagnostics)
//! function identically on macOS.

use std::cell::RefCell;

use slint::ComponentHandle;
use slint::ModelRc;
use slint::SharedString;
use slint::VecModel;

use crate::ui;

pub(super) struct MacSettingsSlice {
    win: RefCell<Option<ui::SettingsWindow>>,
    cfg: overlay_backend::config::SharedConfig,
}

impl MacSettingsSlice {
    /// Create the settings slice wrapping the shared config.
    pub(super) fn new(cfg: overlay_backend::config::SharedConfig) -> Self {
        Self {
            win: RefCell::new(None),
            cfg,
        }
    }

    /// Open or raise the full Slint Settings window.
    pub(super) fn open_settings(&self, bar_weak: slint::Weak<ui::OverlayBarWindow>) {
        if let Some(bar) = bar_weak.upgrade() {
            bar.set_settings_open(true);
        }

        let mut slot = self.win.borrow_mut();
        if let Some(win) = slot.as_ref() {
            self.populate(win);
            let _ = win.show();
            let _ = slint_replay::native::window::raise_key_front(win.window());
            return;
        }

        let win = match ui::SettingsWindow::new() {
            Ok(w) => w,
            Err(error) => {
                slint_replay::logging::line(&format!(
                    "[macos] SettingsWindow::new failed: {error}"
                ));
                return;
            }
        };

        self.populate(&win);
        self.wire_callbacks(&win, bar_weak);

        let _ = win.show();
        if let Err(error) = slint_replay::native::window::configure_floating(win.window()) {
            slint_replay::logging::line(&format!(
                "[macos] settings configure_floating failed: {error}"
            ));
        }
        let _ = slint_replay::native::window::raise_key_front(win.window());

        *slot = Some(win);
    }

    fn populate(&self, win: &ui::SettingsWindow) {
        win.global::<ui::Platform>().set_is_macos(true);
        let c = self.cfg.read();

        // App version in Updates tab
        win.set_app_version(SharedString::from(env!("CARGO_PKG_VERSION")));

        // AI Provider: 0 = cloud, 1 = local, 4 = codex
        let ai_idx = match c.ai_provider.as_str() {
            "local" => 1,
            "codex" => 4,
            _ => 0,
        };
        win.set_ai_provider_index(ai_idx);

        // Mask token for privacy: show generic status string, never raw key
        let key_set = !c.groq_api_key.trim().is_empty();
        win.set_groq_api_key_status(SharedString::from(if key_set {
            "[ok] set"
        } else {
            "[--] not set"
        }));

        // STT Provider: 0 = Groq, 1 = GigaAM, 2 = Local Whisper/UAP
        win.set_stt_provider_index(if c.stt_is_local() { 2 } else { 0 });

        // Interface language: 0 = Russian, 1 = English
        win.set_ui_language_index(if c.ui_language == "en" { 1 } else { 0 });

        // STT Language: None -> 0 (auto), ru -> 1, en -> 2
        let stt_lang_idx = match c.stt_language.as_deref() {
            Some("ru") => 1,
            Some("en") => 2,
            _ => 0,
        };
        win.set_stt_language_index(stt_lang_idx);

        win.set_stt_whisper_url_input(SharedString::from(c.stt_whisper_url.as_str()));
        win.set_trigger_keywords_input(SharedString::from(c.trigger_keywords.as_str()));

        // Toggles
        win.set_record_audio(c.record_audio_enabled);
        win.set_auto_tiles_enabled(c.auto_tiles_enabled);
        win.set_suppress_tiles(c.suppress_tiles);
        win.set_coaching_live_tiles(c.live_coaching_tiles_enabled);
        win.set_coaching_debrief(c.post_meeting_debrief_enabled);

        // Profiles
        self.refresh_profiles_ui(win, &c);

        // Codex auth status refresh if configured
        if c.ai_provider == "codex" || c.vision_provider == "codex" {
            let is_ru = c.ui_is_ru();
            win.set_codex_auth_busy(true);
            let weak = win.as_weak();
            std::thread::spawn(move || {
                let snapshot = overlay_backend::codex_subscription::provider_snapshot();
                let status_text = match snapshot.account {
                    overlay_backend::codex_subscription::AccountState::SignedIn { plan } => {
                        let plan_str = plan.as_deref().unwrap_or("pro");
                        if is_ru {
                            format!("[ok] Подключено к Codex ({plan_str})")
                        } else {
                            format!("[ok] Signed in with Codex ({plan_str})")
                        }
                    }
                    overlay_backend::codex_subscription::AccountState::SignInRequired => {
                        if is_ru {
                            "[--] Необходим вход в Codex".to_string()
                        } else {
                            "[--] Codex sign-in required".to_string()
                        }
                    }
                    _ => {
                        if is_ru {
                            "[--] Не подключено".to_string()
                        } else {
                            "[--] Not connected".to_string()
                        }
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak.upgrade() {
                        w.set_codex_auth_status(SharedString::from(status_text));
                        w.set_codex_auth_busy(false);
                    }
                });
            });
        }
    }

    fn refresh_profiles_ui(&self, win: &ui::SettingsWindow, c: &overlay_backend::config::Config) {
        let names: Vec<SharedString> = c
            .context_profiles
            .iter()
            .map(|p| SharedString::from(p.name.as_str()))
            .collect();
        win.set_profile_names(ModelRc::new(VecModel::from(names)));
        win.set_active_profile_index(match c.active_profile_index() {
            Some(i) => i as i32,
            None if !c.context_profiles.is_empty() => 0,
            None => -1,
        });
        win.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
    }

    fn wire_callbacks(
        &self,
        win: &ui::SettingsWindow,
        bar_weak: slint::Weak<ui::OverlayBarWindow>,
    ) {
        let weak_win = win.as_weak();
        let bar_weak_close = bar_weak;

        win.on_close_clicked(move || {
            if let Some(win) = weak_win.upgrade() {
                let _ = win.hide();
            }
            if let Some(bar) = bar_weak_close.upgrade() {
                bar.set_settings_open(false);
            }
        });

        // AppKit native window drag via header
        let weak_drag = win.as_weak();
        win.on_drag_start_requested(move || {
            let Some(win) = weak_drag.upgrade() else {
                return;
            };
            if let Err(error) = slint_replay::native::window::begin_drag(win.window()) {
                slint_replay::logging::line(&format!("[macos] settings drag failed: {error}"));
            }
        });

        // Interface language change
        let cfg_lang = self.cfg.clone();
        win.on_language_selected(move |idx| {
            let mut c = cfg_lang.write();
            c.ui_language = if idx == 1 {
                "en".to_string()
            } else {
                "ru".to_string()
            };
            let _ = overlay_backend::config::save(&c);
            if let Err(e) = slint::select_bundled_translation(&c.ui_language) {
                slint_replay::logging::line(&format!("[macos] translation select error: {e}"));
            }
            slint_replay::logging::line(&format!(
                "[macos] ui_language changed to {}",
                c.ui_language
            ));
        });

        // Save & Persist config changes
        let cfg_ai = self.cfg.clone();
        win.on_ai_provider_changed(move |idx| {
            let mut c = cfg_ai.write();
            c.ai_provider = if idx == 1 {
                "local".to_string()
            } else if idx == 4 {
                "codex".to_string()
            } else {
                "groq".to_string()
            };
            let _ = overlay_backend::config::save(&c);
            slint_replay::logging::line(&format!(
                "[macos] ai_provider changed to {}",
                c.ai_provider
            ));
        });

        let cfg_stt = self.cfg.clone();
        win.on_stt_provider_changed(move |idx| {
            let mut c = cfg_stt.write();
            c.stt_provider = if idx == 2 {
                "uap".to_string()
            } else if idx == 1 {
                "gigaam".to_string()
            } else {
                "groq".to_string()
            };
            let _ = overlay_backend::config::save(&c);
            slint_replay::logging::line(&format!(
                "[macos] stt_provider changed to {}",
                c.stt_provider
            ));
        });

        let cfg_stt_lang = self.cfg.clone();
        win.on_stt_language_changed(move |idx| {
            let mut c = cfg_stt_lang.write();
            c.stt_language = match idx {
                1 => Some("ru".to_string()),
                2 => Some("en".to_string()),
                _ => None,
            };
            let _ = overlay_backend::config::save(&c);
            slint_replay::logging::line("[macos] stt_language updated and saved");
        });

        let cfg_kw = self.cfg.clone();
        win.on_trigger_keywords_save(move |text| {
            let clamped: String = text.trim().chars().take(400).collect();
            let mut c = cfg_kw.write();
            c.trigger_keywords = clamped;
            let _ = overlay_backend::config::save(&c);
            slint_replay::logging::line("[macos] trigger keywords updated and saved");
        });

        let cfg_wurl = self.cfg.clone();
        win.on_stt_whisper_url_save(move |text| {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let mut c = cfg_wurl.write();
                c.stt_whisper_url = trimmed.to_string();
                let _ = overlay_backend::config::save(&c);
                slint_replay::logging::line("[macos] stt_whisper_url updated and saved");
            }
        });

        // Meeting context & profiles
        let cfg_ctx = self.cfg.clone();
        let weak_ctx = win.as_weak();
        win.on_meeting_context_save(move |text| {
            let mut c = cfg_ctx.write();
            c.save_active_context(&text);
            if let Err(e) = overlay_backend::config::save(&c) {
                slint_replay::logging::line(&format!("[macos] meeting context save failed: {e}"));
                if let Some(w) = weak_ctx.upgrade() {
                    w.set_meeting_context_result(SharedString::from("[err] save failed"));
                }
            } else if let Some(w) = weak_ctx.upgrade() {
                let count = text.chars().count();
                w.set_meeting_context_result(SharedString::from(format!(
                    "[ok] saved ({count} chars)"
                )));
            }
        });

        let cfg_prof_sel = self.cfg.clone();
        let weak_prof_sel = win.as_weak();
        win.on_profile_selected(move |idx| {
            if idx < 0 {
                return;
            }
            let mut c = cfg_prof_sel.write();
            c.select_profile(idx as usize);
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak_prof_sel.upgrade() {
                let names: Vec<SharedString> = c
                    .context_profiles
                    .iter()
                    .map(|p| SharedString::from(p.name.as_str()))
                    .collect();
                w.set_profile_names(ModelRc::new(VecModel::from(names)));
                w.set_active_profile_index(match c.active_profile_index() {
                    Some(i) => i as i32,
                    None if !c.context_profiles.is_empty() => 0,
                    None => -1,
                });
                w.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
            }
        });

        let cfg_prof_add = self.cfg.clone();
        let weak_prof_add = win.as_weak();
        win.on_profile_add(move |name| {
            let mut c = cfg_prof_add.write();
            let added = c.add_profile(name.as_str()).is_some();
            if added {
                let _ = overlay_backend::config::save(&c);
            }
            if let Some(w) = weak_prof_add.upgrade() {
                let names: Vec<SharedString> = c
                    .context_profiles
                    .iter()
                    .map(|p| SharedString::from(p.name.as_str()))
                    .collect();
                w.set_profile_names(ModelRc::new(VecModel::from(names)));
                w.set_active_profile_index(match c.active_profile_index() {
                    Some(i) => i as i32,
                    None if !c.context_profiles.is_empty() => 0,
                    None => -1,
                });
                w.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
            }
        });

        let cfg_prof_del = self.cfg.clone();
        let weak_prof_del = win.as_weak();
        win.on_profile_delete(move || {
            let mut c = cfg_prof_del.write();
            c.delete_active_profile();
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak_prof_del.upgrade() {
                let names: Vec<SharedString> = c
                    .context_profiles
                    .iter()
                    .map(|p| SharedString::from(p.name.as_str()))
                    .collect();
                w.set_profile_names(ModelRc::new(VecModel::from(names)));
                w.set_active_profile_index(match c.active_profile_index() {
                    Some(i) => i as i32,
                    None if !c.context_profiles.is_empty() => 0,
                    None => -1,
                });
                w.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
            }
        });
    }
}
