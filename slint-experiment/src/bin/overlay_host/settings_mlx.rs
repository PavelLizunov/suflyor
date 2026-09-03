// macOS-only Settings wiring for the two pinned, on-demand MLX models.

use super::{ComponentHandle, SettingsWindow, SharedString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Text(i32),
    Vision(i32),
}

impl Role {
    fn model(self) -> &'static str {
        match self {
            Self::Text(0) => overlay_backend::mlx_install::DEFAULT_TEXT_MODEL,
            Self::Text(_) => overlay_backend::mlx_install::GEMMA4_MODEL,
            Self::Vision(0) => overlay_backend::mlx_install::DEFAULT_VISION_MODEL,
            Self::Vision(_) => overlay_backend::mlx_install::GEMMA4_MODEL,
        }
    }

    fn total_size(self) -> u64 {
        match self {
            Self::Text(0) => 4_851_993_338,
            Self::Text(_) => 10_177_611_148,
            Self::Vision(0) => 1_749_079_691,
            Self::Vision(_) => 10_177_611_148,
        }
    }
}

fn update_state(window: &SettingsWindow, role: Role, installed: bool, failed: bool) {
    let active = overlay_backend::mlx_runtime::selected_model().as_deref() == Some(role.model());
    match role {
        Role::Text(_) => {
            window.set_mlx_text_installed(installed);
            window.set_mlx_text_active(active);
            window.set_mlx_text_failed(failed);
        }
        Role::Vision(_) => {
            window.set_mlx_vision_installed(installed);
            window.set_mlx_vision_active(active);
            window.set_mlx_vision_failed(failed);
        }
    }
}

fn update_active_state(window: &SettingsWindow) {
    let selected = overlay_backend::mlx_runtime::selected_model();
    let text_role = Role::Text(window.get_mlx_text_model_index());
    let vision_role = Role::Vision(window.get_mlx_vision_model_index());
    window.set_mlx_text_active(selected.as_deref() == Some(text_role.model()));
    window.set_mlx_vision_active(selected.as_deref() == Some(vision_role.model()));
}

fn set_busy(window: &SettingsWindow, role: Role, busy: bool) {
    match role {
        Role::Text(_) => window.set_mlx_text_busy(busy),
        Role::Vision(_) => window.set_mlx_vision_busy(busy),
    }
}

fn format_mebibytes(bytes: u64) -> String {
    format!("{:.0}", bytes as f64 / 1_048_576.0)
}

fn set_progress(window: &SettingsWindow, role: Role, done: u64, total: u64) {
    let done_label = SharedString::from(format_mebibytes(done));
    let total_label = SharedString::from(format_mebibytes(total));
    match role {
        Role::Text(_) => {
            window.set_mlx_text_progress(done as f32 / total.max(1) as f32);
            window.set_mlx_text_done(done_label);
            window.set_mlx_text_total(total_label);
        }
        Role::Vision(_) => {
            window.set_mlx_vision_progress(done as f32 / total.max(1) as f32);
            window.set_mlx_vision_done(done_label);
            window.set_mlx_vision_total(total_label);
        }
    }
}

fn install(role: Role, weak: slint::Weak<SettingsWindow>, cancel: Arc<AtomicBool>) {
    cancel.store(false, Ordering::Release);
    if let Some(window) = weak.upgrade() {
        match role {
            Role::Text(_) => window.set_mlx_text_checking(false),
            Role::Vision(_) => window.set_mlx_vision_checking(false),
        }
        set_busy(&window, role, true);
        update_state(&window, role, false, false);
    }
    std::thread::spawn(move || {
        let weak_progress = weak.clone();
        let result =
            overlay_backend::mlx_install::install(role.model(), &cancel, &move |done, total| {
                let weak = weak_progress.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        set_progress(&window, role, done, total);
                    }
                });
            });
        let cancelled = cancel.load(Ordering::Acquire);
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                set_busy(&window, role, false);
                update_state(&window, role, result.is_ok(), result.is_err() && !cancelled);
                match role {
                    Role::Text(_) => window.set_mlx_text_cancelled(cancelled),
                    Role::Vision(_) => window.set_mlx_vision_cancelled(cancelled),
                }
            }
        });
    });
}

fn enable(
    role: Role,
    weak: slint::Weak<SettingsWindow>,
    cfg: overlay_backend::config::SharedConfig,
) {
    if let Some(window) = weak.upgrade() {
        set_busy(&window, role, true);
        update_state(&window, role, true, false);
    }
    std::thread::spawn(move || {
        let result = super::super::activate_mlx_model(role.model())
            .map_err(|()| anyhow::anyhow!("MLX activation failed"))
            .and_then(|()| {
                let mut config = cfg.write();
                let previous = (
                    config.ai_provider.clone(),
                    config.ai_mlx_model.clone(),
                    config.vision_provider.clone(),
                    config.vision_mlx_model.clone(),
                );
                match role {
                    Role::Text(_) => {
                        config.ai_mlx_model = role.model().into();
                        config.ai_provider = "mlx".into();
                        if role.model() == overlay_backend::mlx_install::GEMMA4_MODEL {
                            config.vision_provider = "same".into();
                        } else if config.vision_provider == "same" {
                            config.vision_provider = "off".into();
                        }
                    }
                    Role::Vision(_) => {
                        config.vision_mlx_model = role.model().into();
                        config.vision_provider = "mlx".into();
                    }
                }
                if let Err(error) = overlay_backend::config::save(&config) {
                    config.ai_provider = previous.0;
                    config.ai_mlx_model = previous.1;
                    config.vision_provider = previous.2;
                    config.vision_mlx_model = previous.3;
                    drop(config);
                    let _ = overlay_backend::mlx_runtime::stop_if_idle();
                    return Err(error);
                }
                Ok(())
            });
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                set_busy(&window, role, false);
                update_state(&window, role, true, result.is_err());
                update_active_state(&window);
                if result.is_ok() {
                    match role {
                        Role::Text(_) => {
                            window.set_ai_provider_index(4);
                            if role.model() == overlay_backend::mlx_install::GEMMA4_MODEL {
                                window.set_vision_same_available(true);
                                window.set_vision_provider_index(1);
                            }
                        }
                        Role::Vision(_) => window.set_vision_provider_index(6),
                    }
                }
            }
        });
    });
}

fn refresh(role: Role, weak: slint::Weak<SettingsWindow>) {
    std::thread::spawn(move || {
        let installed = overlay_backend::mlx_install::installed_snapshot(role.model()).is_some();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                match role {
                    Role::Text(_) => window.set_mlx_text_checking(false),
                    Role::Vision(_) => window.set_mlx_vision_checking(false),
                }
                update_state(&window, role, installed, false);
            }
        });
    });
}

pub(super) fn wire(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) {
    if !cfg!(target_os = "macos") {
        return;
    }
    let (text_model, vision_model) = {
        let c = cfg.read();
        (c.ai_mlx_model.clone(), c.vision_mlx_model.clone())
    };
    let text_idx = if text_model == overlay_backend::mlx_install::GEMMA4_MODEL { 1 } else { 0 };
    let vision_idx = if vision_model == overlay_backend::mlx_install::GEMMA4_MODEL { 1 } else { 0 };
    win.set_mlx_text_model_index(text_idx);
    win.set_mlx_vision_model_index(vision_idx);

    let text_cancel = Arc::new(AtomicBool::new(false));
    let vision_cancel = Arc::new(AtomicBool::new(false));

    let weak = win.as_weak();
    let cancel = text_cancel.clone();
    win.on_mlx_text_download(move || {
        if let Some(window) = weak.upgrade() {
            let role = Role::Text(window.get_mlx_text_model_index());
            install(role, weak.clone(), cancel.clone());
        }
    });
    let cancel = text_cancel;
    win.on_mlx_text_cancel(move || cancel.store(true, Ordering::Release));
    let weak = win.as_weak();
    let cfg_text = cfg.clone();
    win.on_mlx_text_enable(move || {
        if let Some(window) = weak.upgrade() {
            let role = Role::Text(window.get_mlx_text_model_index());
            enable(role, weak.clone(), cfg_text.clone());
        }
    });

    let weak = win.as_weak();
    win.on_mlx_text_model_changed(move |idx| {
        if let Some(window) = weak.upgrade() {
            let role = Role::Text(idx);
            window.set_mlx_text_total(SharedString::from(format_mebibytes(role.total_size())));
            window.set_mlx_text_checking(true);
            refresh(role, weak.clone());
        }
    });

    let weak = win.as_weak();
    let cancel = vision_cancel.clone();
    win.on_mlx_vision_download(move || {
        if let Some(window) = weak.upgrade() {
            let role = Role::Vision(window.get_mlx_vision_model_index());
            install(role, weak.clone(), cancel.clone());
        }
    });
    let cancel = vision_cancel;
    win.on_mlx_vision_cancel(move || cancel.store(true, Ordering::Release));
    let weak = win.as_weak();
    let cfg_vision = cfg.clone();
    win.on_mlx_vision_enable(move || {
        if let Some(window) = weak.upgrade() {
            let role = Role::Vision(window.get_mlx_vision_model_index());
            enable(role, weak.clone(), cfg_vision.clone());
        }
    });

    let weak = win.as_weak();
    win.on_mlx_vision_model_changed(move |idx| {
        if let Some(window) = weak.upgrade() {
            let role = Role::Vision(idx);
            window.set_mlx_vision_total(SharedString::from(format_mebibytes(role.total_size())));
            window.set_mlx_vision_checking(true);
            refresh(role, weak.clone());
        }
    });
}

pub(super) fn populate(win: &SettingsWindow) {
    let text_role = Role::Text(win.get_mlx_text_model_index());
    let vision_role = Role::Vision(win.get_mlx_vision_model_index());

    if !win.get_mlx_text_busy() {
        win.set_mlx_text_installed(false);
        win.set_mlx_text_active(false);
        win.set_mlx_text_failed(false);
        win.set_mlx_text_cancelled(false);
        win.set_mlx_text_progress(0.0);
        win.set_mlx_text_done(SharedString::from("0"));
        let total = if text_role.total_size() == 4_851_993_338 {
            4_851_993_338
        } else {
            10_177_611_148
        };
        win.set_mlx_text_total(SharedString::from(format_mebibytes(total)));
    }
    if !win.get_mlx_vision_busy() {
        win.set_mlx_vision_installed(false);
        win.set_mlx_vision_active(false);
        win.set_mlx_vision_failed(false);
        win.set_mlx_vision_cancelled(false);
        win.set_mlx_vision_progress(0.0);
        win.set_mlx_vision_done(SharedString::from("0"));
        let total = if vision_role.total_size() == 1_749_079_691 {
            1_749_079_691
        } else {
            10_177_611_148
        };
        win.set_mlx_vision_total(SharedString::from(format_mebibytes(total)));
    }
    if cfg!(target_os = "macos") && !win.get_mlx_text_busy() {
        win.set_mlx_text_checking(true);
        refresh(text_role, win.as_weak());
    }
    if cfg!(target_os = "macos") && !win.get_mlx_vision_busy() {
        win.set_mlx_vision_checking(true);
        refresh(vision_role, win.as_weak());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::format_mebibytes;

    #[test]
    fn progress_bytes_match_the_windows_megabyte_display() {
        assert_eq!(format_mebibytes(0), "0");
        assert_eq!(format_mebibytes(4_851_993_338), "4627");
        assert_eq!(format_mebibytes(1_749_079_691), "1668");
    }
}
