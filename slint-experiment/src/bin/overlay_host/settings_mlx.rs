// macOS-only Settings wiring for the two pinned, on-demand MLX models.

use super::{ComponentHandle, SettingsWindow, SharedString};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone, Copy)]
enum Role {
    Text,
    Vision,
}

impl Role {
    fn model(self) -> &'static str {
        match self {
            Self::Text => overlay_backend::mlx_install::DEFAULT_TEXT_MODEL,
            Self::Vision => overlay_backend::mlx_install::DEFAULT_VISION_MODEL,
        }
    }
}

fn update_state(window: &SettingsWindow, role: Role, installed: bool, failed: bool) {
    let active = overlay_backend::mlx_runtime::selected_model().as_deref() == Some(role.model());
    match role {
        Role::Text => {
            window.set_mlx_text_installed(installed);
            window.set_mlx_text_active(active);
            window.set_mlx_text_failed(failed);
        }
        Role::Vision => {
            window.set_mlx_vision_installed(installed);
            window.set_mlx_vision_active(active);
            window.set_mlx_vision_failed(failed);
        }
    }
}

fn update_active_state(window: &SettingsWindow) {
    let selected = overlay_backend::mlx_runtime::selected_model();
    window.set_mlx_text_active(selected.as_deref() == Some(Role::Text.model()));
    window.set_mlx_vision_active(selected.as_deref() == Some(Role::Vision.model()));
}

fn set_busy(window: &SettingsWindow, role: Role, busy: bool) {
    match role {
        Role::Text => window.set_mlx_text_busy(busy),
        Role::Vision => window.set_mlx_vision_busy(busy),
    }
}

fn set_progress(window: &SettingsWindow, role: Role, done: u64, total: u64) {
    match role {
        Role::Text => {
            window.set_mlx_text_progress(done as f32 / total.max(1) as f32);
            window.set_mlx_text_done(SharedString::from(done.to_string()));
            window.set_mlx_text_total(SharedString::from(total.to_string()));
        }
        Role::Vision => {
            window.set_mlx_vision_progress(done as f32 / total.max(1) as f32);
            window.set_mlx_vision_done(SharedString::from(done.to_string()));
            window.set_mlx_vision_total(SharedString::from(total.to_string()));
        }
    }
}

fn install(role: Role, weak: slint::Weak<SettingsWindow>, cancel: Arc<AtomicBool>) {
    cancel.store(false, Ordering::Release);
    if let Some(window) = weak.upgrade() {
        match role {
            Role::Text => window.set_mlx_text_checking(false),
            Role::Vision => window.set_mlx_vision_checking(false),
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
                    Role::Text => window.set_mlx_text_cancelled(cancelled),
                    Role::Vision => window.set_mlx_vision_cancelled(cancelled),
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
                    Role::Text => {
                        config.ai_mlx_model = role.model().into();
                        config.ai_provider = "mlx".into();
                        if config.vision_provider == "same" {
                            config.vision_provider = "off".into();
                        }
                    }
                    Role::Vision => {
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
                        Role::Text => window.set_ai_provider_index(4),
                        Role::Vision => window.set_vision_provider_index(6),
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
                    Role::Text => window.set_mlx_text_checking(false),
                    Role::Vision => window.set_mlx_vision_checking(false),
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
    let text_cancel = Arc::new(AtomicBool::new(false));
    let vision_cancel = Arc::new(AtomicBool::new(false));

    let weak = win.as_weak();
    let cancel = text_cancel.clone();
    win.on_mlx_text_download(move || install(Role::Text, weak.clone(), cancel.clone()));
    let cancel = text_cancel;
    win.on_mlx_text_cancel(move || cancel.store(true, Ordering::Release));
    let weak = win.as_weak();
    let cfg_text = cfg.clone();
    win.on_mlx_text_enable(move || enable(Role::Text, weak.clone(), cfg_text.clone()));

    let weak = win.as_weak();
    let cancel = vision_cancel.clone();
    win.on_mlx_vision_download(move || install(Role::Vision, weak.clone(), cancel.clone()));
    let cancel = vision_cancel;
    win.on_mlx_vision_cancel(move || cancel.store(true, Ordering::Release));
    let weak = win.as_weak();
    let cfg_vision = cfg.clone();
    win.on_mlx_vision_enable(move || enable(Role::Vision, weak.clone(), cfg_vision.clone()));
}

pub(super) fn populate(win: &SettingsWindow) {
    if !win.get_mlx_text_busy() {
        win.set_mlx_text_installed(false);
        win.set_mlx_text_active(false);
        win.set_mlx_text_failed(false);
        win.set_mlx_text_cancelled(false);
        win.set_mlx_text_progress(0.0);
        win.set_mlx_text_done(SharedString::from("0"));
        win.set_mlx_text_total(SharedString::from("4851993338"));
    }
    if !win.get_mlx_vision_busy() {
        win.set_mlx_vision_installed(false);
        win.set_mlx_vision_active(false);
        win.set_mlx_vision_failed(false);
        win.set_mlx_vision_cancelled(false);
        win.set_mlx_vision_progress(0.0);
        win.set_mlx_vision_done(SharedString::from("0"));
        win.set_mlx_vision_total(SharedString::from("1749079691"));
    }
    if cfg!(target_os = "macos") && !win.get_mlx_text_busy() {
        win.set_mlx_text_checking(true);
        refresh(Role::Text, win.as_weak());
    }
    if cfg!(target_os = "macos") && !win.get_mlx_vision_busy() {
        win.set_mlx_vision_checking(true);
        refresh(Role::Vision, win.as_weak());
    }
}
