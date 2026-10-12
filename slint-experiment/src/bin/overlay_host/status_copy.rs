use overlay_backend::config;
use slint::SharedString;
use crate::ui::OverlayBarWindow;

/// Short, human display name for a model id: drop a `.gguf`/`.bin` extension,
/// then take the first token (or the tier after "claude-"). Used by the bar's
/// active-stack readout. (#E10.2)
fn short_model_name(full: &str) -> String {
    let base = full.trim_end_matches(".gguf").trim_end_matches(".bin");
    let parts: Vec<&str> = base
        .split(['-', ':', '/', ' '])
        .filter(|s| !s.is_empty())
        .collect();
    match parts.first() {
        Some(&"claude") if parts.len() > 1 => parts[1].to_string(),
        Some(first) => (*first).to_string(),
        None => "—".to_string(),
    }
}

/// Build the bar's "active stack" label: which STT engine + which AI model are
/// live, prefixed with 🟢 (all-local), ☁ (all-cloud), or ◐ (mixed). (#E10.2)
pub(super) fn active_stack_label(c: &overlay_backend::config::Config) -> String {
    let (stt, stt_local): (String, bool) = match c.stt_provider.as_str() {
        // Show the platform accelerator so the bar reflects accelerated vs CPU.
        "gigaam" => (
            format!("GigaAM {}", gigaam_accelerator_name(c.stt_gigaam_gpu)),
            true,
        ),
        "whisper" => ("Whisper".to_string(), true),
        _ => ("Groq".to_string(), false),
    };
    let ai_local = matches!(c.ai_provider.as_str(), "local" | "mlx");
    let model_full = match c.ai_provider.as_str() {
        "local" => c.ai_local_model.as_str(),
        "mlx" => c.ai_mlx_model.as_str(),
        "openai" => c.openai_model.as_str(),
        "anthropic" => c.anthropic_model.as_str(),
        "codex" => c.codex_model.as_str(),
        _ => c.ai_model.as_str(),
    };
    // For a LOCAL model show the friendly "Gemma 12B" / "Gemma 26B-A4B" so the user
    // can tell the fallback vs primary model apart at a glance (the user asked to see
    // the selected model more explicitly); cloud models keep the short id.
    let model = if c.ai_provider == "mlx" {
        model_full.rsplit('/').next().unwrap_or(model_full).to_string()
    } else if ai_local {
        overlay_backend::local_ai::local_model_label(model_full)
    } else if c.ai_provider == "codex" {
        model_full.to_string()
    } else {
        short_model_name(model_full)
    };
    // ASCII tag + Latin-1 middle dot only — fancier glyphs (✕/✓/arrows) render
    // as missing-glyph boxes on the user's Slint+skia font fallback.
    let tag = if stt_local && ai_local {
        "local"
    } else if !stt_local && !ai_local {
        "cloud"
    } else {
        "mixed"
    };
    format!("{tag}: {stt} · {model}")
}

pub(super) fn ai_perf_label(
    perf: Option<overlay_backend::ai::RequestPerf>,
    load_ms: Option<u64>,
    loading: bool,
    is_ru: bool,
) -> String {
    if loading {
        return if is_ru {
            " · MLX загружается...".to_string()
        } else {
            " · MLX loading...".to_string()
        };
    }
    if let Some(perf) = perf {
        let mut parts = Vec::new();
        if let Some(ttft) = perf.ttft_ms {
            parts.push(if is_ru {
                format!("1-й {:.1}с", ttft as f64 / 1_000.0)
            } else {
                format!("first {:.1}s", ttft as f64 / 1_000.0)
            });
        }
        parts.push(if is_ru {
            format!("всего {:.1}с", perf.total_ms as f64 / 1_000.0)
        } else {
            format!("total {:.1}s", perf.total_ms as f64 / 1_000.0)
        });
        if let Some(rate) = perf.decode_tps {
            parts.push(if is_ru {
                format!("{rate:.0} дек/с")
            } else {
                format!("{rate:.0} dec/s")
            });
        }
        if let Some(rate) = perf.effective_tps {
            parts.push(if is_ru {
                format!("{rate:.1} общ/с")
            } else {
                format!("{rate:.1} e2e/s")
            });
        }
        return format!(" · {}", parts.join(" · "));
    }
    load_ms.map_or_else(String::new, |ms| {
        if is_ru {
            format!(" · загрузка {:.1}с", ms as f64 / 1_000.0)
        } else {
            format!(" · load {:.1}s", ms as f64 / 1_000.0)
        }
    })
}

pub(super) fn memory_size_label(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return String::new();
    };
    let mib = bytes as f64 / (1024.0 * 1024.0);
    if mib >= 1024.0 {
        format!("{:.1} GiB", mib / 1024.0)
    } else {
        format!("{mib:.0} MiB")
    }
}

pub(super) fn gigaam_accelerator_name(enabled: bool) -> &'static str {
    if !enabled {
        "CPU"
    } else if cfg!(target_os = "macos") {
        "Core ML"
    } else {
        "GPU(DirectML)"
    }
}

/// Title of a manually spawned "+ tile". The visible number is owned by the
/// tile UI ALONE — `tile.slint` prepends `#<sequence>` to `tile-title` — so the
/// title must carry NO `#N` of its own (the old `Вопрос по встрече #3` rendered
/// doubled: `#3  Вопрос по встрече #3`). Every other spawn path (F9, PTT,
/// vision, auto, read-only content tiles) already passes an unnumbered title.
pub(super) fn manual_tile_heading(has_transcript: bool, is_ru: bool) -> &'static str {
    if !is_ru && has_transcript {
        "Meeting question"
    } else if !is_ru {
        "Tile"
    } else if has_transcript {
        "Вопрос по встрече"
    } else {
        "Тайл"
    }
}

pub(super) fn summary_empty_copy(is_ru: bool) -> (&'static str, &'static str) {
    if is_ru {
        (
            "Сводка встречи",
            "Транскрипта пока нет. Нажмите Старт, поговорите — и Сводка соберёт итог встречи.",
        )
    } else {
        (
            "Meeting summary",
            "No transcript yet. Press Start and talk a bit — then Summary will assemble the meeting recap.",
        )
    }
}

#[cfg(target_os = "macos")]
pub(super) fn capture_stopped_copy(is_ru: bool) -> (&'static str, &'static str, &'static str) {
    if is_ru {
        (
            "захват остановлен",
            "Захват остановлен",
            "Аудиозахват остановился. Нажмите Старт, чтобы продолжить.",
        )
    } else {
        (
            "capture stopped",
            "Capture stopped",
            "Audio capture stopped. Press Start to continue.",
        )
    }
}

pub(super) fn mic_busy_status(is_ru: bool) -> &'static str {
    if is_ru {
        "микрофон занят"
    } else {
        "microphone busy"
    }
}

pub(super) fn manual_tile_placeholder(deep_locked: bool, has_transcript: bool, is_ru: bool) -> &'static str {
    if deep_locked {
        return overlay_backend::deep_lock::blocked_notice(is_ru);
    }
    match (has_transcript, is_ru) {
        (true, true) => "Спрашиваю AI…",
        (true, false) => "Asking AI…",
        (false, true) => "Транскрипт пока пуст. Нажмите старт на баре (или удерживайте «спросить») — AI ответит по последним репликам.",
        (false, false) => "The transcript is empty. Start a session from the bar (or hold ask) and AI will answer from the latest lines.",
    }
}

pub(super) fn manual_tile_not_configured(is_ru: bool) -> &'static str {
    if is_ru {
        "**AI не настроен.** Откройте Настройки → AI и выберите провайдера (локальный сервер или облачный мост)."
    } else {
        "**AI is not configured.** Open Settings → AI and choose a provider (local server or cloud bridge)."
    }
}

pub(super) fn manual_tile_failure(heading: &str, category: &str, is_ru: bool) -> String {
    if is_ru {
        format!("# {heading}\n\n**Не удалось получить ответ AI:** {category}\n\nПроверьте локальный AI-сервер или AI-мост (Настройки → AI).")
    } else {
        format!("# {heading}\n\n**AI could not return an answer:** {category}\n\nCheck the local AI server or AI bridge in Settings → AI.")
    }
}

/// Push the lock-chip UI state (suppress / deep / localized a11y) from the
/// LIVE config. Called on every chip transition AND on a live language switch
/// (the per-state description is Rust-built, so @tr does not refresh it).
pub(super) fn refresh_lock_chip(o: &OverlayBarWindow, cfg: &config::SharedConfig) {
    let snap = cfg.read();
    let managed = overlay_backend::deep_lock::cfg_is_managed_local(&snap);
    o.set_suppress_tiles(snap.suppress_tiles);
    // Cloud and external/Ollama keep the original two-state visual even when a
    // dormant managed-local deep lock is persisted for a later provider switch.
    o.set_deep_lock(managed && snap.deep_lock);
    o.set_lock_menu_managed(managed);
    o.set_lock_a11y(SharedString::from(overlay_backend::deep_lock::state_hint(
        snap.ui_is_ru(),
        managed,
        snap.suppress_tiles,
        snap.deep_lock,
    )));
}

#[cfg(test)]
mod tile_heading_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test asserts
    use super::{
        active_stack_label, ai_perf_label, manual_tile_failure, manual_tile_heading,
        manual_tile_not_configured, manual_tile_placeholder, memory_size_label, mic_busy_status,
        summary_empty_copy,
    };

    #[cfg(target_os = "macos")]
    use super::capture_stopped_copy;

    #[test]
    fn active_stack_uses_the_selected_direct_provider_model() {
        let mut cfg = overlay_backend::config::Config::defaults();
        cfg.stt_provider = "gigaam".into();
        cfg.ai_model = "claude-haiku-4-5".into();
        cfg.ai_provider = "codex".into();
        cfg.codex_model = "gpt-5.6-terra".into();
        let label = active_stack_label(&cfg);
        assert!(label.contains("gpt-5.6-terra"));
        assert!(!label.contains("haiku"));
    }

    /// Double-numbering guard: `tile.slint` prepends `#<sequence>`, so a title
    /// carrying its own number (or digit) renders doubled in the tile header.
    #[test]
    fn manual_tile_heading_carries_no_number() {
        for heading in [
            manual_tile_heading(true, true),
            manual_tile_heading(false, true),
            manual_tile_heading(true, false),
            manual_tile_heading(false, false),
        ] {
            assert!(!heading.contains('#'), "heading carries #: {heading:?}");
            assert!(
                !heading.chars().any(|c| c.is_ascii_digit()),
                "heading carries a digit: {heading:?}"
            );
        }
    }

    #[test]
    fn manual_tile_copy_follows_ui_language() {
        assert_eq!(manual_tile_heading(false, false), "Tile");
        assert!(manual_tile_placeholder(false, false, false).starts_with("The transcript"));
        assert!(manual_tile_not_configured(false).contains("not configured"));
        assert!(manual_tile_failure("Tile", "offline", false).contains("could not"));

        assert_eq!(manual_tile_heading(false, true), "Тайл");
        assert!(manual_tile_placeholder(false, false, true).starts_with("Транскрипт"));
        assert!(manual_tile_not_configured(true).contains("не настроен"));
        assert!(manual_tile_failure("Тайл", "offline", true).contains("Не удалось"));

        assert_eq!(
            manual_tile_placeholder(true, false, false),
            overlay_backend::deep_lock::blocked_notice(false)
        );
        assert_eq!(
            manual_tile_placeholder(true, false, true),
            overlay_backend::deep_lock::blocked_notice(true)
        );
    }

    #[test]
    fn summary_empty_notice_has_both_ui_languages() {
        assert_eq!(summary_empty_copy(false).0, "Meeting summary");
        assert!(summary_empty_copy(false).1.starts_with("No transcript"));
        assert_eq!(summary_empty_copy(true).0, "Сводка встречи");
        assert!(summary_empty_copy(true).1.starts_with("Транскрипта"));
    }

    #[test]
    fn mic_busy_status_follows_ui_language() {
        assert_eq!(mic_busy_status(false), "microphone busy");
        assert_eq!(mic_busy_status(true), "микрофон занят");
    }

    #[test]
    fn performance_and_memory_labels_are_honest_and_localized() {
        let perf = overlay_backend::ai::RequestPerf {
            decode_tps: Some(78.0),
            ttft_ms: Some(1_250),
            total_ms: 4_000,
            effective_tps: Some(5.0),
        };
        let english = ai_perf_label(Some(perf), None, false, false);
        assert!(english.contains("first ") && english.contains("total 4.0s"));
        assert!(english.contains("78 dec/s"));
        assert!(english.contains("5.0 e2e/s"));
        let russian = ai_perf_label(Some(perf), None, false, true);
        assert!(russian.contains("1-й 1.2с"));
        assert!(russian.contains("78 дек/с"));
        assert_eq!(memory_size_label(Some(512 * 1024 * 1024)), "512 MiB");
        assert_eq!(memory_size_label(None), "");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn capture_stopped_copy_is_generic_localized_and_requests_manual_start() {
        let english = capture_stopped_copy(false);
        assert_eq!(english.0, "capture stopped");
        assert!(english.2.contains("Press Start"));

        let russian = capture_stopped_copy(true);
        assert_eq!(russian.0, "захват остановлен");
        assert!(russian.2.contains("Нажмите Старт"));

        for copy in [
            english.0, english.1, english.2, russian.0, russian.1, russian.2,
        ] {
            assert!(!copy.contains("http") && !copy.contains("192.168."));
        }
    }
}
