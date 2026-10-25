use crate::ui::OverlayBarWindow;
use overlay_backend::config;
use slint::SharedString;

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

/// Build the bar's "active stack" label: which STT engine and which AI model
/// are live, e.g. `STT: Groq · AI: Gemma 12B`. (#E10.2)
pub(super) fn active_stack_label(c: &overlay_backend::config::Config) -> String {
    let (stt, _stt_local): (String, bool) = match c.stt_provider.as_str() {
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
        model_full
            .rsplit('/')
            .next()
            .unwrap_or(model_full)
            .to_string()
    } else if ai_local {
        overlay_backend::local_ai::local_model_label(model_full)
    } else if c.ai_provider == "codex" {
        model_full.to_string()
    } else {
        short_model_name(model_full)
    };
    // Latin-1 middle dot only — fancier glyphs (✕/✓/arrows) render as
    // missing-glyph boxes on the user's Slint+skia font fallback. Both labels
    // are the tab names in Settings, so the line reads the same in RU and EN.
    format!("STT: {stt} · AI: {model}")
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
    if !enabled || cfg!(target_os = "linux") {
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
            "Стенограммы пока нет. Нажмите «старт» на панели, поговорите, и сводка соберёт итог встречи.",
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

pub(super) fn manual_tile_placeholder(
    deep_locked: bool,
    has_transcript: bool,
    is_ru: bool,
) -> &'static str {
    if deep_locked {
        return overlay_backend::deep_lock::blocked_notice(is_ru);
    }
    match (has_transcript, is_ru) {
        (true, true) => "Спрашиваю AI…",
        (true, false) => "Asking AI…",
        (false, true) => "Стенограмма пока пуста. Нажмите «старт» на панели (или удерживайте «спросить»), и AI ответит по последним репликам.",
        (false, false) => "The transcript is empty. Start a session from the bar (or hold ask) and AI will answer from the latest lines.",
    }
}

pub(super) fn manual_tile_not_configured(is_ru: bool) -> &'static str {
    if is_ru {
        "**AI не настроен.** Откройте Настройки → AI-провайдеры и выберите провайдера (локальный сервер или облачный мост)."
    } else {
        "**AI is not configured.** Open Settings → AI providers and choose a provider (local server or cloud bridge)."
    }
}

pub(super) fn manual_tile_failure(heading: &str, category: &str, is_ru: bool) -> String {
    if is_ru {
        format!("# {heading}\n\n**Не удалось получить ответ AI:** {category}\n\nПроверьте локальный AI-сервер или AI-мост (Настройки → AI-провайдеры).")
    } else {
        format!("# {heading}\n\n**AI could not return an answer:** {category}\n\nCheck the local AI server or AI bridge in Settings → AI providers.")
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
        active_stack_label, ai_perf_label, gigaam_accelerator_name, manual_tile_failure,
        manual_tile_heading, manual_tile_not_configured, manual_tile_placeholder,
        memory_size_label, mic_busy_status, summary_empty_copy,
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
        assert!(manual_tile_not_configured(false).contains("Settings → AI providers"));
        assert!(manual_tile_failure("Tile", "offline", false).contains("Settings → AI providers"));
        assert!(manual_tile_failure("Tile", "offline", false).contains("could not"));

        assert_eq!(manual_tile_heading(false, true), "Тайл");
        assert!(manual_tile_placeholder(false, false, true).starts_with("Стенограмма"));
        assert!(manual_tile_not_configured(true).contains("не настроен"));
        assert!(manual_tile_not_configured(true).contains("Настройки → AI-провайдеры"));
        assert!(manual_tile_failure("Тайл", "offline", true).contains("Настройки → AI-провайдеры"));
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
    fn gigaam_accelerator_matches_the_native_platform() {
        assert_eq!(gigaam_accelerator_name(false), "CPU");
        let enabled = if cfg!(target_os = "linux") {
            "CPU"
        } else if cfg!(target_os = "macos") {
            "Core ML"
        } else {
            "GPU(DirectML)"
        };
        assert_eq!(gigaam_accelerator_name(true), enabled);
    }

    #[test]
    fn summary_empty_notice_has_both_ui_languages() {
        assert_eq!(summary_empty_copy(false).0, "Meeting summary");
        assert!(summary_empty_copy(false).1.starts_with("No transcript"));
        assert_eq!(summary_empty_copy(true).0, "Сводка встречи");
        assert!(summary_empty_copy(true).1.starts_with("Стенограммы"));
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

/// One plain line for a failed connection test. The full error chain can carry
/// URLs, paths and library jargon, so it goes to the log, not the screen.
pub(super) fn connection_failed(ru: bool, chain: &str) -> String {
    log::warn!("connection test failed: {chain}");
    let has = |needles: &[&str]| {
        let lower = chain.to_lowercase();
        needles.iter().any(|n| lower.contains(n))
    };
    let (ru_text, en_text) = if has(&["no groq api key", "no api key", "missing api key"]) {
        (
            "Нет ключа: добавьте его в Настройках",
            "No key yet: add it in Settings",
        )
    } else if has(&["401", "403", "unauthorized", "forbidden", "invalid api key"]) {
        ("Ключ не подошёл", "The key was rejected")
    } else if has(&["404", "not found"]) {
        (
            "По этому адресу ничего не найдено",
            "Nothing found at this address",
        )
    } else if has(&[
        "timed out",
        "timeout",
        "refused",
        "error sending request",
        "dns",
        "connect",
    ]) {
        ("Сервер не отвечает", "The server does not answer")
    } else {
        (
            "Не удалось подключиться: проверьте адрес, ключ и модель",
            "Could not connect: check the address, key and model",
        )
    };
    if ru { ru_text } else { en_text }.to_string()
}

/// The GigaAM action the STT tab offers under the folder field.
#[derive(Clone, Copy)]
pub(super) enum GigaamAction {
    /// «Download / Resume»: the managed model is missing or incomplete.
    Download,
    /// «Check files»: the managed model is installed.
    CheckFiles,
    /// No button applies: the model lives in the person's own folder.
    OwnFolder,
}

/// Result of the STT check for the local GigaAM model: what happened and the
/// next step the STT tab offers. Other failures read as connection failures.
pub(super) fn gigaam_check_failed(ru: bool, chain: &str, action: GigaamAction) -> String {
    let accelerator = if cfg!(target_os = "macos") {
        "Apple Core ML"
    } else if cfg!(target_os = "linux") {
        "ONNX Runtime"
    } else {
        "GPU (DirectML)"
    };
    let (button_ru, button_en) = match action {
        GigaamAction::CheckFiles => ("«Проверить файлы»", "Check files"),
        _ => ("«Загрузить / Продолжить»", "Download / Resume"),
    };
    if chain.contains(overlay_backend::stt::GIGAAM_NOT_DOWNLOADED) {
        if ru {
            format!("В этой папке нет модели GigaAM. Нажмите {button_ru} выше или укажите папку, где лежит model.int8.onnx.")
        } else {
            format!("This folder has no GigaAM model. Press {button_en} above or point to the folder with model.int8.onnx.")
        }
    } else if chain.contains(overlay_backend::stt::GIGAAM_LOAD_FAILED) {
        match (action, ru) {
            (GigaamAction::OwnFolder, true) => format!("Модель есть, но не запустилась. Выключите {accelerator} и проверьте снова; не помогло — укажите папку с рабочей моделью."),
            (GigaamAction::OwnFolder, false) => format!("The model is there but did not start. Turn off {accelerator} and check again; if that fails, point to a folder with a working model."),
            (_, true) => format!("Модель есть, но не запустилась. Выключите {accelerator} и проверьте снова; не помогло — нажмите {button_ru} выше."),
            (_, false) => format!("The model is there but did not start. Turn off {accelerator} and check again; if that fails, press {button_en} above."),
        }
    } else {
        connection_failed(ru, chain)
    }
}

/// A connection test that could not even start (no async runtime).
pub(super) fn check_not_started(ru: bool, err: &dyn std::fmt::Display) -> String {
    log::warn!("connection test could not start: {err}");
    if ru {
        "Проверка не запустилась"
    } else {
        "The check could not start"
    }
    .to_string()
}

#[cfg(test)]
mod connection_text_tests {
    use super::connection_failed;

    #[test]
    fn failures_read_as_plain_words() {
        assert_eq!(
            connection_failed(true, "no Groq API key set"),
            "Нет ключа: добавьте его в Настройках"
        );
        assert_eq!(
            connection_failed(false, "HTTP 401 Unauthorized"),
            "The key was rejected"
        );
        assert_eq!(
            connection_failed(true, "error sending request: connection refused"),
            "Сервер не отвечает"
        );
        assert_eq!(
            connection_failed(false, "weird"),
            "Could not connect: check the address, key and model"
        );
    }

    #[test]
    fn gigaam_failures_name_the_next_step() {
        use super::{gigaam_check_failed, GigaamAction};
        use overlay_backend::stt::{GIGAAM_LOAD_FAILED, GIGAAM_NOT_DOWNLOADED};
        let missing = gigaam_check_failed(true, GIGAAM_NOT_DOWNLOADED, GigaamAction::Download);
        assert!(missing.contains("«Загрузить / Продолжить»"), "{missing}");
        let broken = gigaam_check_failed(false, GIGAAM_LOAD_FAILED, GigaamAction::CheckFiles);
        assert!(broken.contains("Check files"), "{broken}");
        let partial = gigaam_check_failed(true, GIGAAM_LOAD_FAILED, GigaamAction::Download);
        assert!(partial.contains("«Загрузить / Продолжить»"), "{partial}");
        let own = gigaam_check_failed(true, GIGAAM_LOAD_FAILED, GigaamAction::OwnFolder);
        assert!(own.contains("укажите папку"), "{own}");
        assert_eq!(
            gigaam_check_failed(true, "weird", GigaamAction::Download),
            connection_failed(true, "weird")
        );
    }
}
