//! Host-side route boundary for the one-resident-model macOS MLX sidecar.

use super::{
    markdown, AskRoute, MarkdownBlock, ModelRc, MonitorHint, RuntimeEvents, SharedString, TileKind,
    TileSpec, TileWindow, VecModel,
};
use std::sync::Arc;

fn lifecycle_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

fn selected_mlx_model(route: AskRoute, config: &overlay_backend::config::Config) -> Option<String> {
    match route {
        AskRoute::Text if config.ai_provider == "mlx" => Some(config.ai_mlx_model.clone()),
        AskRoute::Vision if config.vision_provider == "mlx" => {
            Some(config.vision_mlx_model.clone())
        }
        AskRoute::Vision
            if config.vision_provider == "same"
                && config.ai_provider == "mlx"
                && overlay_backend::mlx_install::catalog_model(&config.ai_mlx_model)
                    .is_some_and(|model| model.supports_images) =>
        {
            Some(config.ai_mlx_model.clone())
        }
        AskRoute::Text | AskRoute::Vision | AskRoute::Cloud => None,
    }
}

pub(crate) fn route_needs_mlx(route: AskRoute, config: &overlay_backend::config::Config) -> bool {
    selected_mlx_model(route, config).is_some()
}

/// Serialize every ordinary Settings/request activation through the same
/// one-resident-model boundary. The backend generation fence remains the
/// final fail-safe, but callers should not manufacture avoidable superseded
/// starts by bypassing this host lock.
pub(crate) fn activate_mlx_model(model: &str) -> Result<(), ()> {
    if overlay_backend::deep_lock::deep_lock_active() {
        return Err(());
    }
    if overlay_backend::mlx_runtime::active_endpoint_for_model(model).is_some() {
        return Ok(());
    }
    let _guard = lifecycle_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if overlay_backend::mlx_runtime::active_endpoint_for_model(model).is_none() {
        overlay_backend::ai::clear_request_perf();
        overlay_backend::mlx_runtime::start(model).map_err(|_| ())?;
    }
    overlay_backend::mlx_runtime::active_endpoint_for_model(model)
        .is_some()
        .then_some(())
        .ok_or(())
}

pub(crate) fn stop_mlx_model() {
    let _guard = lifecycle_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    overlay_backend::mlx_runtime::stop();
    overlay_backend::ai::clear_request_perf();
}

/// Stop a superseded prewarm only while that exact model is still resident.
#[cfg(target_os = "macos")]
pub(crate) fn stop_mlx_model_if_active(model: &str) {
    let _guard = lifecycle_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if overlay_backend::mlx_runtime::active_endpoint_for_model(model).is_some()
        && overlay_backend::mlx_runtime::stop_if_idle()
    {
        overlay_backend::ai::clear_request_perf();
    }
}

/// Resolve a request endpoint. Call only from an existing worker: MLX startup
/// is deliberately blocking and may switch the single resident sidecar model.
pub(crate) fn resolve_route_endpoint(
    route: AskRoute,
    config: &overlay_backend::config::SharedConfig,
) -> Result<overlay_backend::config::AiEndpoint, ()> {
    let selected = {
        let config = config.read();
        selected_mlx_model(route, &config)
    };
    if let Some(model) = selected.as_deref() {
        activate_mlx_model(model)?;
    }
    let config = config.read();
    if selected_mlx_model(route, &config) != selected {
        return Err(());
    }
    let endpoint = route.endpoint(&config);
    if selected.is_some()
        && (endpoint.base_url.is_empty()
            || endpoint.bearer.is_empty()
            || endpoint.model != selected.as_deref().unwrap_or_default())
    {
        return Err(());
    }
    Ok(endpoint)
}

/// Start the selected text model as part of an explicit Deep Lock -> unlocked
/// transition. Holding the same host lock as normal asks prevents a request
/// start from slipping through while the backend guard is temporarily lowered.
#[cfg(target_os = "macos")]
pub(crate) fn start_mlx_for_unlock(
    config: &overlay_backend::config::SharedConfig,
) -> Result<(), ()> {
    let _guard = lifecycle_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let model = {
        let config = config.read();
        if !config.deep_lock {
            return Err(());
        }
        selected_mlx_model(AskRoute::Text, &config).ok_or(())?
    };
    overlay_backend::deep_lock::set_deep_lock_active(false);
    overlay_backend::ai::clear_request_perf();
    let started = overlay_backend::mlx_runtime::start(&model).is_ok();
    overlay_backend::deep_lock::set_deep_lock_active(true);
    if !started || overlay_backend::mlx_runtime::active_endpoint_for_model(&model).is_none() || {
        let config = config.read();
        !config.deep_lock
            || selected_mlx_model(AskRoute::Text, &config).as_deref() != Some(model.as_str())
    } {
        overlay_backend::mlx_runtime::stop();
        return Err(());
    }
    Ok(())
}

pub(crate) fn show_mlx_runtime_error(weak: slint::Weak<TileWindow>, is_ru: bool) {
    let message = if is_ru {
        "Не удалось запустить локальный ИИ. Проверьте установленную модель в Настройках."
    } else {
        "Couldn't start local AI. Check the installed model in Settings."
    };
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(tile) = weak.upgrade() {
            tile.set_followup_busy(false);
            tile.set_source_label(SharedString::from(if is_ru {
                "ai · ошибка"
            } else {
                "ai · error"
            }));
            tile.set_blocks(ModelRc::new(VecModel::from(vec![MarkdownBlock {
                kind: markdown::kind::PARAGRAPH,
                text: SharedString::from(message),
                display_text: SharedString::from(message),
                lang: SharedString::from(""),
                marked: false,
            }])));
        }
    });
}

pub(crate) fn spawn_mlx_runtime_error(
    events: &Arc<dyn RuntimeEvents>,
    config: &overlay_backend::config::SharedConfig,
) {
    let (is_ru, monitor, stealth) = {
        let config = config.read();
        (
            config.ui_is_ru(),
            config.tile_monitor_name.clone(),
            config.stealth_enabled,
        )
    };
    let monitor = match monitor {
        Some(name) if !name.trim().is_empty() => MonitorHint::Named(name),
        _ => MonitorHint::Auto,
    };
    let _ = events.spawn_tile_full(
        TileSpec {
            question: if is_ru {
                "Локальный ИИ".into()
            } else {
                "Local AI".into()
            },
            answer: if is_ru {
                "Не удалось запустить локальный ИИ. Проверьте установленную модель в Настройках."
                    .into()
            } else {
                "Couldn't start local AI. Check the installed model in Settings.".into()
            },
            source: "ai_error".into(),
            is_translation: false,
            highlights: vec!["AI error".into()],
            summary_session: None,
        },
        monitor,
        stealth,
        TileKind::Error,
    );
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn selection_covers_text_vision_same_and_cloud_without_fallback() {
        let mut config = overlay_backend::config::Config::defaults();
        assert!(!route_needs_mlx(AskRoute::Text, &config));
        assert!(!route_needs_mlx(AskRoute::Vision, &config));
        assert!(!route_needs_mlx(AskRoute::Cloud, &config));

        config.ai_provider = "mlx".into();
        assert_eq!(
            selected_mlx_model(AskRoute::Text, &config).as_deref(),
            Some(overlay_backend::mlx_install::DEFAULT_TEXT_MODEL)
        );
        assert_eq!(selected_mlx_model(AskRoute::Vision, &config), None);

        config.vision_provider = "mlx".into();
        assert_eq!(
            selected_mlx_model(AskRoute::Vision, &config).as_deref(),
            Some(overlay_backend::mlx_install::DEFAULT_VISION_MODEL)
        );
        assert_eq!(selected_mlx_model(AskRoute::Cloud, &config), None);

        config.vision_provider = "same".into();
        config.ai_mlx_model = overlay_backend::mlx_install::DEFAULT_VISION_MODEL.into();
        assert_eq!(
            selected_mlx_model(AskRoute::Vision, &config).as_deref(),
            Some(overlay_backend::mlx_install::DEFAULT_VISION_MODEL)
        );
    }
}
