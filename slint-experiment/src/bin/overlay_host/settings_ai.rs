//! AI Settings tab: cloud bridge + local-server provider config (P1 of
//! `docs/overlay-host-gaps-and-next-checks.md` — splitting the
//! `settings_controller.rs` god-function by domain, the same way Phase 2's
//! `diagnostics.rs` and Wave 1/2's `settings_vision.rs` / `settings_stt.rs`
//! were extracted).
//!
//! This module owns the AI wiring previously inlined in `open_settings`: the
//! cloud-bridge token saves (`on_ai_bearer_save`, `on_groq_api_key_save`), the
//! cloud base-url / model saves + dropdown refresh (`on_ai_base_url_save`,
//! `on_ai_model_selected`, `on_ai_models_refresh`), the prompt-cache toggle
//! (`on_ai_prompt_cache_changed`), the local provider switch + local-field
//! saves + dropdown refresh + feature toggles (`on_ai_provider_changed`,
//! `on_ai_local_base_url_save`, `on_ai_local_bearer_save`,
//! `on_ai_local_model_selected`, `on_ai_local_models_refresh`,
//! `on_ai_local_vision_changed`, `on_ai_local_thinking_changed`), and the two
//! live connection tests (`on_ai_local_test_clicked`,
//! `on_ai_bridge_test_clicked`). The blocks moved here VERBATIM — same captures
//! (`cfg.clone()`, plus `win.as_weak()` for the tests / refreshes), same bodies,
//! byte-for-byte identical behavior. `open_settings` now only CALLS
//! `wire_ai_settings(&win, cfg)` where the main AI cluster was.
//!
//! Also moved here: `ModelTarget` + `fetch_models` (the `{base_url}/models`
//! dropdown populate), which only the AI refresh closures + the on-open
//! `populate_token_status` seed call use; both are `pub(crate)`, so
//! `populate_token_status` (which STAYS in `settings_controller.rs` — it is
//! also called OUTSIDE the AI closures, on every Settings open, to seed the
//! token-status display) reaches them through the crate-root glob.
//!
//! NOT moved (different domain / separate later waves, left in
//! `open_settings`): the install / updater closures
//! (`on_install_local_ai_clicked`, `on_cancel_local_ai_clicked`,
//! `on_check_updates_clicked`, `on_install_update_clicked`), and the STT /
//! Vision blocks (already extracted to their own modules).
//!
//! SECURITY (unchanged by this mechanical move): the AI bridge / local
//! test-result tiles keep their GENERIC messages (`[ok] …` / `[err] …` capped
//! at 90 chars for the bridge, `[--] …` for the local test) so no `base_url` /
//! LAN IP leaks into a screen-shared Settings window. `ai_base_url` saves log
//! presence (char count) only, never the value.
//!
//! NOTE: this extraction imports the parent crate-root via `use super::*;`
//! (reaching `SettingsWindow` / `SharedString` / `ModelRc` / `VecModel` / the
//! `diag!` macro / `populate_token_status` / the `overlay_backend` config + ai
//! helpers). That is intentional for the move; imports narrow in a later pass.
use super::{
    active_stack_label, populate_token_status, refresh_lock_chip, ComponentHandle, ModelRc,
    OverlayBarWindow, SettingsWindow, SharedString, VecModel,
};
use slint::Model;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonically invalidates older worker results for the reused Settings
/// window. Hardware discovery for a 26B note can outlast a later model choice.
static LOCAL_MODEL_RESOURCE_WARNING_GENERATION: AtomicU64 = AtomicU64::new(0);
static CODEX_LOGIN_UI_GENERATION: AtomicU64 = AtomicU64::new(0);
static CODEX_SNAPSHOT_UI_GENERATION: AtomicU64 = AtomicU64::new(0);
const PREFERRED_CODEX_MODEL: &str = "gpt-5.6-luna";

fn codex_model_label(model: &overlay_backend::codex_subscription::CodexModel) -> SharedString {
    SharedString::from(model.display_name.clone())
}

fn preferred_codex_model_index(
    models: &[overlay_backend::codex_subscription::CodexModel],
    saved: &str,
    image_only: bool,
) -> Option<usize> {
    let allowed = |model: &overlay_backend::codex_subscription::CodexModel| {
        !image_only || model.input_modalities.iter().any(|value| value == "image")
    };
    if !saved.is_empty() {
        return models
            .iter()
            .position(|model| allowed(model) && model.id == saved);
    }
    models
        .iter()
        .position(|model| allowed(model) && model.id == PREFERRED_CODEX_MODEL)
        .or_else(|| {
            models
                .iter()
                .position(|model| allowed(model) && model.is_default)
        })
        .or_else(|| models.iter().position(allowed))
}

fn reasoning_label(effort: &str, is_ru: bool) -> String {
    let title = effort.chars().next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + &effort[first.len_utf8()..]
    });
    match (effort, is_ru) {
        ("none" | "minimal", true) => format!("{title} (без reasoning, быстрее всего)"),
        ("none" | "minimal", false) => format!("{title} (no reasoning, fastest)"),
        ("low", true) => format!("{title} (самый быстрый доступный)"),
        ("low", false) => format!("{title} (fastest available)"),
        _ => title,
    }
}

fn catalog_is_authoritative(state: &overlay_backend::codex_subscription::AccountState) -> bool {
    matches!(
        state,
        overlay_backend::codex_subscription::AccountState::SignedIn { .. }
    )
}

fn reasoning_normalization_notice(
    saved: &str,
    normalized: &str,
    is_ru: bool,
) -> Option<&'static str> {
    if saved.is_empty() || saved == normalized {
        None
    } else if is_ru {
        Some(
            "Сохранённый уровень рассуждений больше не поддерживается; выбран режим по умолчанию модели.",
        )
    } else {
        Some(
            "The saved reasoning effort is no longer supported; the model default is now selected.",
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexCopyResult {
    Empty,
    Copied,
    Failed,
}

fn copy_codex_user_code<E>(
    code: &str,
    write: impl FnOnce(&str) -> Result<(), E>,
) -> CodexCopyResult {
    if code.is_empty() {
        return CodexCopyResult::Empty;
    }
    if write(code).is_ok() {
        CodexCopyResult::Copied
    } else {
        CodexCopyResult::Failed
    }
}

fn codex_copy_status(result: CodexCopyResult, is_ru: bool) -> &'static str {
    match (result, is_ru) {
        (CodexCopyResult::Empty, _) => "",
        (CodexCopyResult::Copied, true) => "[ok] Код скопирован",
        (CodexCopyResult::Copied, false) => "[ok] Code copied",
        (CodexCopyResult::Failed, true) => "[err] Не удалось скопировать",
        (CodexCopyResult::Failed, false) => "[err] Copy failed",
    }
}

pub(crate) fn invalidate_codex_login_ui() -> u64 {
    overlay_backend::codex_subscription::cancel_pending_login();
    CODEX_SNAPSHOT_UI_GENERATION.fetch_add(1, Ordering::SeqCst);
    CODEX_LOGIN_UI_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

fn codex_ui_is_current(generation: u64) -> bool {
    CODEX_LOGIN_UI_GENERATION.load(Ordering::SeqCst) == generation
}

pub(crate) fn invalidate_codex_snapshot_ui() -> u64 {
    CODEX_SNAPSHOT_UI_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

fn codex_snapshot_ui_is_current(generation: u64) -> bool {
    CODEX_SNAPSHOT_UI_GENERATION.load(Ordering::SeqCst) == generation
}

fn codex_account_label(
    state: &overlay_backend::codex_subscription::AccountState,
    is_ru: bool,
) -> String {
    use overlay_backend::codex_subscription::AccountState;
    match state {
        AccountState::NotInstalled if is_ru => {
            "[--] Codex app-server не найден — установите официальный Codex".into()
        }
        AccountState::NotInstalled => {
            "[--] Codex app-server not found — install official Codex".into()
        }
        AccountState::SignedOut if is_ru => "[--] выход выполнен".into(),
        AccountState::SignedOut => "[--] signed out".into(),
        AccountState::SignInRequired if is_ru => "[--] требуется вход".into(),
        AccountState::SignInRequired => "[--] sign-in required".into(),
        AccountState::SignedIn { plan } if is_ru => plan.as_ref().map_or_else(
            || "[ok] вход через ChatGPT выполнен".into(),
            |plan| format!("[ok] вход через ChatGPT выполнен ({plan})"),
        ),
        AccountState::SignedIn { plan } => plan.as_ref().map_or_else(
            || "[ok] signed in with ChatGPT".into(),
            |plan| format!("[ok] signed in with ChatGPT ({plan})"),
        ),
        AccountState::Error if is_ru => "[err] не удалось проверить аккаунт Codex".into(),
        AccountState::Error => "[err] could not query the Codex account".into(),
    }
}

pub(crate) fn refresh_codex_account_status(
    weak: slint::Weak<SettingsWindow>,
    cfg: overlay_backend::config::SharedConfig,
) {
    let generation = invalidate_codex_snapshot_ui();
    if let Some(window) = weak.upgrade() {
        window.set_codex_models_busy(true);
    }
    std::thread::spawn(move || {
        let (
            is_ru,
            saved,
            saved_effort,
            saved_vision,
            saved_ai_provider,
            saved_vision_provider,
            baseline_same_available,
        ) = {
            let c = cfg.read();
            (
                c.ui_is_ru(),
                c.codex_model.clone(),
                c.codex_reasoning_effort.clone(),
                c.codex_vision_model.clone(),
                c.ai_provider.clone(),
                c.vision_provider.clone(),
                c.same_text_model_accepts_images_declared(),
            )
        };
        let snapshot = overlay_backend::codex_subscription::provider_snapshot();
        let authoritative_catalog = catalog_is_authoritative(&snapshot.account);
        let label = codex_account_label(&snapshot.account, is_ru);
        let ids: Vec<String> = snapshot
            .models
            .iter()
            .map(|model| model.id.clone())
            .collect();
        let labels: Vec<SharedString> = snapshot.models.iter().map(codex_model_label).collect();
        let selected = preferred_codex_model_index(&snapshot.models, &saved, false);
        let selected_id = selected.and_then(|index| ids.get(index)).cloned();
        let selected_model = selected.and_then(|index| snapshot.models.get(index));
        let selected_text_accepts_images = selected_model
            .is_some_and(|model| model.input_modalities.iter().any(|value| value == "image"));
        let same_available = if saved_ai_provider == "codex" {
            selected_text_accepts_images
        } else {
            baseline_same_available
        };
        let vision_models: Vec<_> = snapshot
            .models
            .iter()
            .filter(|model| model.input_modalities.iter().any(|value| value == "image"))
            .collect();
        let vision_selected = preferred_codex_model_index(&snapshot.models, &saved_vision, true)
            .and_then(|selected| {
                let id = &snapshot.models[selected].id;
                vision_models.iter().position(|model| model.id == *id)
            });
        let vision_selected_id = vision_selected.map(|index| vision_models[index].id.clone());
        let vision_ids: Vec<SharedString> = vision_models
            .iter()
            .map(|model| SharedString::from(model.id.clone()))
            .collect();
        let vision_labels: Vec<SharedString> = vision_models
            .iter()
            .map(|model| codex_model_label(model))
            .collect();
        let mut reasoning_ids = vec![String::new()];
        if let Some(model) = selected_model {
            reasoning_ids.extend(model.reasoning_efforts.iter().cloned());
        }
        let default_effort = selected_model
            .and_then(|model| model.default_reasoning_effort.as_deref())
            .unwrap_or(if is_ru {
                "по настройке модели"
            } else {
                "model default"
            });
        let mut reasoning_labels = vec![if is_ru {
            format!("По умолчанию ({default_effort})")
        } else {
            format!("Default ({default_effort})")
        }];
        reasoning_labels.extend(
            reasoning_ids
                .iter()
                .skip(1)
                .map(|effort| reasoning_label(effort, is_ru)),
        );
        let reasoning_selected = reasoning_ids
            .iter()
            .position(|effort| effort == &saved_effort)
            .unwrap_or(0);
        let normalized_effort = reasoning_ids
            .get(reasoning_selected)
            .cloned()
            .unwrap_or_default();
        let saved_missing = !saved.is_empty() && selected_id.is_none();
        let vision_saved_missing = !saved_vision.is_empty() && vision_selected_id.is_none();
        if let Some(id) = selected_id.clone() {
            if (saved.is_empty() || normalized_effort != saved_effort)
                && codex_snapshot_ui_is_current(generation)
            {
                let mut c = cfg.write();
                if codex_snapshot_ui_is_current(generation)
                    && c.codex_model == saved
                    && c.codex_reasoning_effort == saved_effort
                {
                    c.codex_model = id;
                    c.codex_reasoning_effort = normalized_effort.clone();
                    if overlay_backend::config::save(&c).is_err() {
                        eprintln!("[overlay-host] Codex model save failed");
                    }
                }
            }
        }
        if saved_vision.is_empty() {
            if let Some(id) = vision_selected_id.clone() {
                if codex_snapshot_ui_is_current(generation) {
                    let mut c = cfg.write();
                    if codex_snapshot_ui_is_current(generation) && c.codex_vision_model.is_empty() {
                        c.codex_vision_model = id;
                        if overlay_backend::config::save(&c).is_err() {
                            eprintln!("[overlay-host] Codex vision model save failed");
                        }
                    }
                }
            }
        }
        let mut same_forced_off = false;
        if authoritative_catalog
            && saved_ai_provider == "codex"
            && saved_vision_provider == "same"
            && codex_snapshot_ui_is_current(generation)
        {
            let mut c = cfg.write();
            if codex_snapshot_ui_is_current(generation)
                && c.ai_provider == "codex"
                && c.vision_provider == "same"
            {
                if selected_text_accepts_images {
                    if let Some(id) = selected_id.as_ref() {
                        if c.codex_vision_model != *id {
                            c.codex_vision_model.clone_from(id);
                            if overlay_backend::config::save(&c).is_err() {
                                eprintln!("[overlay-host] Codex same-model vision save failed");
                            }
                        }
                    }
                } else {
                    c.vision_provider = "off".into();
                    c.codex_vision_model.clear();
                    same_forced_off = true;
                    if overlay_backend::config::save(&c).is_err() {
                        eprintln!("[overlay-host] invalid same-model vision repair failed");
                    }
                }
            }
        }
        let mut secondary_status = Vec::new();
        if let Some(notice) =
            reasoning_normalization_notice(&saved_effort, &normalized_effort, is_ru)
        {
            secondary_status.push(notice.to_string());
        }
        if saved_missing {
            secondary_status.push(if is_ru {
                "Сохранённая модель недоступна; выбор сохранён без замены.".to_string()
            } else {
                "The saved model is unavailable; the selection was preserved.".to_string()
            });
        }
        if vision_saved_missing {
            secondary_status.push(if is_ru {
                "Сохранённая модель зрения недоступна или не принимает изображения; выбор сохранён без замены."
                    .to_string()
            } else {
                "The saved vision model is unavailable or cannot accept images; the selection was preserved."
                    .to_string()
            });
        }
        if let Some(status) = snapshot.rate_limits {
            secondary_status.push(if is_ru {
                format!(
                    "Лимит аккаунта: {}",
                    status
                        .replace("% used", "% использовано")
                        .replace(" min", " мин")
                )
            } else {
                format!("Account limit: {status}")
            });
        }
        let model_ids: Vec<SharedString> = ids.into_iter().map(SharedString::from).collect();
        let reasoning_ids: Vec<SharedString> =
            reasoning_ids.into_iter().map(SharedString::from).collect();
        let reasoning_labels: Vec<SharedString> = reasoning_labels
            .into_iter()
            .map(SharedString::from)
            .collect();
        let _ = slint::invoke_from_event_loop(move || {
            if codex_snapshot_ui_is_current(generation) {
                if let Some(window) = weak.upgrade() {
                    window.set_codex_auth_status(SharedString::from(label));
                    window.set_codex_auth_busy(false);
                    window.set_codex_models_busy(false);
                    window.set_codex_model_ids(ModelRc::new(VecModel::from(model_ids)));
                    window.set_codex_model_labels(ModelRc::new(VecModel::from(labels)));
                    window.set_codex_model_index(selected.map_or(-1, |index| index as i32));
                    window.set_codex_vision_model_ids(ModelRc::new(VecModel::from(vision_ids)));
                    window
                        .set_codex_vision_model_labels(ModelRc::new(VecModel::from(vision_labels)));
                    window.set_codex_vision_model_index(
                        vision_selected.map_or(-1, |index| index as i32),
                    );
                    window.set_codex_reasoning_ids(ModelRc::new(VecModel::from(reasoning_ids)));
                    window
                        .set_codex_reasoning_labels(ModelRc::new(VecModel::from(reasoning_labels)));
                    window.set_codex_reasoning_index(reasoning_selected as i32);
                    window.set_vision_same_available(same_available);
                    if same_forced_off {
                        window.set_vision_provider_index(0);
                    }
                    window.set_codex_rate_status(SharedString::from(secondary_status.join("\n")));
                }
            }
        });
    });
}

/// Which model dropdown a fetch populates — the cloud bridge or the local server.
#[derive(Clone, Copy)]
pub(crate) enum ModelTarget {
    Cloud,
    Local,
}

/// Fetch a server's model list (`GET {base_url}/models`) off-thread and populate
/// the matching Settings dropdown (cloud bridge or local), pre-selecting the
/// saved model (kept in the list even if the server is down so it's never lost).
/// Reuses the test-button pattern — a throwaway current-thread runtime +
/// invoke_from_event_loop — because open_settings has no rt_handle. Reads cfg
/// inside the worker thread so it never contends with a config lock held on the
/// UI thread. No-op when the base URL is blank. (#E10.1)
pub(crate) fn fetch_models(
    weak: slint::Weak<SettingsWindow>,
    cfg: overlay_backend::config::SharedConfig,
    target: ModelTarget,
) {
    std::thread::spawn(move || {
        let (base_url, bearer, saved) = {
            let c = cfg.read();
            match target {
                ModelTarget::Cloud => (
                    c.ai_base_url.clone(),
                    c.ai_bearer.clone(),
                    c.ai_model.clone(),
                ),
                ModelTarget::Local => (
                    c.ai_local_base_url.clone(),
                    c.ai_local_bearer.clone(),
                    c.ai_local_model.clone(),
                ),
            }
        };
        if base_url.trim().is_empty() {
            return;
        }
        let models: Vec<String> = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .and_then(|rt| {
                rt.block_on(overlay_backend::ai::list_models(&base_url, &bearer))
                    .ok()
            })
            .unwrap_or_default();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let mut list = models;
            if !saved.is_empty() && !list.iter().any(|m| m == &saved) {
                list.insert(0, saved.clone());
            }
            let idx = list.iter().position(|m| m == &saved).unwrap_or(0) as i32;
            let shared: Vec<SharedString> = list.into_iter().map(SharedString::from).collect();
            let model = ModelRc::new(VecModel::from(shared));
            match target {
                ModelTarget::Cloud => {
                    w.set_ai_models(model);
                    w.set_ai_model_index(idx);
                }
                ModelTarget::Local => {
                    w.set_ai_local_models(model);
                    w.set_ai_local_model_index(idx);
                }
            }
        });
    });
}

/// Resolve the managed-model resource note outside Slint's event loop. Hardware
/// discovery can call WMI, which must never make opening Settings or changing a
/// model appear frozen.
pub(crate) fn refresh_local_model_resource_warning(
    win: &SettingsWindow,
    root: std::path::PathBuf,
    base_url: String,
    model: String,
) {
    let generation = LOCAL_MODEL_RESOURCE_WARNING_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    win.set_local_model_resource_warning(SharedString::from("Проверяю ресурсы модели..."));
    let weak = win.as_weak();
    std::thread::spawn(move || {
        let warning =
            overlay_backend::local_ai::local_model_resource_warning(&root, &base_url, &model);
        let _ = slint::invoke_from_event_loop(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if LOCAL_MODEL_RESOURCE_WARNING_GENERATION.load(Ordering::Relaxed) == generation {
                w.set_local_model_resource_warning(SharedString::from(warning));
            }
        });
    });
}

/// Wire the AI-tab Settings callbacks onto the Settings window — both the cloud
/// bridge and the local server. Moved VERBATIM out of `open_settings` (P1 domain
/// split) — same captures, same behavior. Needs only `win` (for the closures +
/// the tests' / refreshes' `as_weak()`) and `cfg` (cloned per closure); none of
/// the AI blocks touch `state` / `overlay_weak` / `slint_rt` / `rt_handle` (the
/// bar's active-stack readout is refreshed by the Settings close handler, which
/// stays in `open_settings`), so no extra params are threaded through.
pub(crate) fn wire_ai_settings(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
) {
    // Phase E6 — token + AI bridge config save wires.
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_ai_bearer_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("[overlay-host] ai_bearer save skipped: empty input");
                return;
            }
            {
                let mut c = cfg_c.write();
                c.ai_bearer = trimmed;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_bearer save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] ai_bearer saved to config.json");
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_openai_key_save(move |new_value| {
            if let Err(error) = overlay_backend::credentials::write(
                overlay_backend::credentials::SecretSlot::OpenAi,
                new_value.as_str(),
            ) {
                eprintln!("[overlay-host] OpenAI credential save failed: {error:#}");
                if let Some(w) = weak_for_refresh.upgrade() {
                    let is_ru = cfg_c.read().ui_language != "en";
                    w.set_openai_key_status(if is_ru {
                        "[ошибка] защищённое хранилище недоступно".into()
                    } else {
                        "[err] secure storage unavailable".into()
                    });
                }
                return;
            }
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_anthropic_key_save(move |new_value| {
            if let Err(error) = overlay_backend::credentials::write(
                overlay_backend::credentials::SecretSlot::Anthropic,
                new_value.as_str(),
            ) {
                eprintln!("[overlay-host] Anthropic credential save failed: {error:#}");
                if let Some(w) = weak_for_refresh.upgrade() {
                    let is_ru = cfg_c.read().ui_language != "en";
                    w.set_anthropic_key_status(if is_ru {
                        "[ошибка] защищённое хранилище недоступно".into()
                    } else {
                        "[err] secure storage unavailable".into()
                    });
                }
                return;
            }
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak_for_refresh = win.as_weak();
        win.on_groq_api_key_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("[overlay-host] groq_api_key save skipped: empty input");
                return;
            }
            {
                let mut c = cfg_c.write();
                c.groq_api_key = trimmed;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] groq_api_key save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] groq_api_key saved to config.json");
            if let Some(w) = weak_for_refresh.upgrade() {
                populate_token_status(&w, &cfg_c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_base_url_save(move |new_value| {
            let trimmed = new_value.trim().to_string();
            {
                let mut c = cfg_c.write();
                c.ai_base_url = trimmed.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_base_url save failed: {e:#}");
                    return;
                }
            }
            // Log presence only — ai_base_url often embeds the user's LAN
            // IP / proxy port (network-topology leak). See ai.rs no-log note.
            eprintln!("[overlay-host] ai_base_url saved ({} chars)", trimmed.len());
            // #E10.1 — re-query the cloud model list against the new URL.
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Cloud);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_openai_base_url_save(move |value| {
            let mut c = cfg_c.write();
            c.openai_base_url = value.trim().to_string();
            if let Err(error) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] OpenAI URL save failed: {error:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_openai_model_save(move |value| {
            let mut c = cfg_c.write();
            c.openai_model = value.trim().to_string();
            if let Err(error) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] OpenAI model save failed: {error:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_anthropic_base_url_save(move |value| {
            let mut c = cfg_c.write();
            c.anthropic_base_url = value.trim().to_string();
            if let Err(error) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] Anthropic URL save failed: {error:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_anthropic_model_save(move |value| {
            let mut c = cfg_c.write();
            c.anthropic_model = value.trim().to_string();
            if let Err(error) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] Anthropic model save failed: {error:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_model_selected(move |new_value| {
            let trimmed = new_value.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            {
                let mut c = cfg_c.write();
                c.ai_model = trimmed.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_model save failed: {e:#}");
                    return;
                }
            }
            eprintln!("[overlay-host] ai_model selected: {trimmed}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_models_refresh(move || {
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Cloud);
        });
    }
    {
        // E9 — experimental prompt-caching toggle (default off; persists +
        // applies live via the ai.rs static).
        let cfg_c = cfg.clone();
        win.on_ai_prompt_cache_changed(move |on| {
            {
                let mut c = cfg_c.write();
                c.ai_prompt_cache = on;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_prompt_cache save failed: {e:#}");
                    return;
                }
            }
            overlay_backend::ai::set_prompt_cache(on);
            diag!("ai_prompt_cache -> {on}");
        });
    }
    // E9 Phase 1 — local AI provider switch + local-field saves + test.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let overlay = overlay_weak.clone();
        win.on_ai_provider_changed(move |idx| {
            // Selecting the catalog entry only reveals it; activation stays
            // behind the explicit "Enable for text" action.
            if cfg!(target_os = "macos") && idx == 4 {
                return;
            }
            let provider = match idx {
                1 => "local",
                2 => "openai",
                3 => "anthropic",
                4 => "codex",
                _ => "cloud",
            };
            let (leaving_mlx, vision_uses_mlx) = {
                let current = cfg_c.read();
                (
                    current.ai_provider == "mlx" && provider != "mlx",
                    current.vision_provider == "mlx",
                )
            };
            if leaving_mlx
                && !vision_uses_mlx
                && !overlay_backend::mlx_runtime::stop_if_idle()
            {
                if let Some(window) = weak.upgrade() {
                    window.set_ai_provider_index(4);
                }
                return;
            }
            let mut c = cfg_c.write();
            if provider == "local" && !cfg!(target_os = "macos") {
                overlay_backend::local_ai::select_local_provider(
                    &mut c,
                    &overlay_backend::local_ai::default_root(),
                );
            } else {
                c.ai_provider = provider.to_string();
            }
            if overlay_backend::deep_lock::cfg_is_managed_local(&c) && c.deep_lock {
                c.suppress_tiles = true;
            }
            if c.vision_provider == "same" && !c.same_text_model_accepts_images_declared() {
                c.vision_provider = "off".into();
            }
            let vision_state = (
                c.vision_provider.clone(),
                c.same_text_model_accepts_images_declared(),
            );
            let local_state = (provider == "local").then(|| {
                let root = overlay_backend::local_ai::default_root();
                (
                    c.ai_local_base_url.clone(),
                    c.ai_local_quality,
                    c.ai_local_model.clone(),
                    c.ai_local_vision,
                    c.vision_provider.clone(),
                    overlay_backend::local_ai::local_vision_available(&c, &root),
                )
            });
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_provider save failed: {e:#}");
                return;
            }
            let codex_needed =
                !cfg!(target_os = "macos") && (provider == "codex" || c.vision_provider == "codex");
            overlay_backend::ai::set_local_no_think(provider == "local" && !c.ai_local_thinking);
            drop(c);
            if let Some(o) = overlay.upgrade() {
                refresh_lock_chip(&o, &cfg_c);
            }
            if let Some(window) = weak.upgrade() {
                window.set_vision_provider_index(
                    super::settings_vision::vision_provider_index_from_id(&vision_state.0),
                );
                window.set_vision_same_available(vision_state.1);
            }
            diag!("ai_provider -> {provider}");
            if codex_needed {
                refresh_codex_account_status(weak.clone(), cfg_c.clone());
            } else {
                invalidate_codex_login_ui();
                if let Some(window) = weak.upgrade() {
                    window.set_codex_auth_busy(false);
                    window.set_codex_models_busy(false);
                    window.set_codex_login_url(SharedString::default());
                    window.set_codex_user_code(SharedString::default());
                    window.set_codex_copy_status(SharedString::default());
                }
            }
            // #E10.1 — switching to Local auto-populates the model dropdown.
            if let Some((
                base_url,
                quality,
                model,
                local_vision,
                vision_provider,
                vision_available,
            )) = local_state
            {
                if let Some(w) = weak.upgrade() {
                    w.set_ai_local_base_url_input(SharedString::from(base_url.clone()));
                    w.set_ai_local_quality(quality);
                    w.set_ai_local_model_profile_index(
                        overlay_backend::local_ai::ManagedModel::from_config(&model, quality)
                            .index(),
                    );
                    w.set_ai_local_models(ModelRc::new(VecModel::from(vec![SharedString::from(
                        model.clone(),
                    )])));
                    w.set_ai_local_model_index(0);
                    w.set_ai_local_vision(local_vision);
                    w.set_vision_same_available(local_vision);
                    w.set_ai_local_vision_available(vision_available);
                    w.set_vision_provider_index(
                        super::settings_vision::vision_provider_index_from_id(&vision_provider),
                    );
                    refresh_local_model_resource_warning(
                        &w,
                        overlay_backend::local_ai::default_root(),
                        base_url,
                        model,
                    );
                }
                fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
            }
        });
    }

    // RC14 — ChatGPT subscription sign-in is delegated to the official Codex
    // app-server. Suflyor receives only account state and device-code display
    // fields; it never receives or stores OAuth tokens.
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_connect_clicked(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let is_ru = cfg_c.read().ui_is_ru();
            let generation = invalidate_codex_login_ui();
            let attempt = overlay_backend::codex_subscription::begin_device_login();
            window.set_codex_auth_busy(true);
            window.set_codex_login_url(SharedString::default());
            window.set_codex_user_code(SharedString::default());
            window.set_codex_copy_status(SharedString::default());
            window.set_codex_auth_status(SharedString::from(if is_ru {
                "Запуск официального входа Codex..."
            } else {
                "Starting official Codex sign-in..."
            }));
            let worker_weak = window.as_weak();
            let worker_cfg = cfg_c.clone();
            std::thread::spawn(move || {
                overlay_backend::codex_subscription::device_login(attempt, move |event| {
                    use overlay_backend::codex_subscription::LoginEvent;
                    let event_weak = worker_weak.clone();
                    let event_cfg = worker_cfg.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let Some(window) = event_weak.upgrade() else {
                            return;
                        };
                        if !codex_ui_is_current(generation) {
                            return;
                        }
                        let is_ru = event_cfg.read().ui_is_ru();
                        match event {
                            LoginEvent::AwaitingUser {
                                verification_url,
                                user_code,
                            } => {
                                window.set_codex_login_url(SharedString::from(verification_url));
                                window.set_codex_user_code(SharedString::from(user_code));
                                window.set_codex_copy_status(SharedString::default());
                                window.set_codex_auth_status(SharedString::from(if is_ru {
                                    "Откройте страницу входа и введите показанный код"
                                } else {
                                    "Open the sign-in page and enter the shown code"
                                }));
                            }
                            LoginEvent::SignedIn { plan } => {
                                let state =
                                    overlay_backend::codex_subscription::AccountState::SignedIn {
                                        plan,
                                    };
                                window.set_codex_auth_busy(false);
                                window.set_codex_login_url(SharedString::default());
                                window.set_codex_user_code(SharedString::default());
                                window.set_codex_copy_status(SharedString::default());
                                window.set_codex_auth_status(SharedString::from(
                                    codex_account_label(&state, is_ru),
                                ));
                                refresh_codex_account_status(window.as_weak(), event_cfg);
                            }
                            LoginEvent::SignInRequired => {
                                window.set_codex_auth_busy(false);
                                window.set_codex_login_url(SharedString::default());
                                window.set_codex_user_code(SharedString::default());
                                window.set_codex_copy_status(SharedString::default());
                                window.set_codex_auth_status(SharedString::from(if is_ru {
                                    "[--] вход не завершён — повторите подключение"
                                } else {
                                    "[--] sign-in not completed — connect again"
                                }));
                            }
                            LoginEvent::Error => {
                                window.set_codex_auth_busy(false);
                                window.set_codex_login_url(SharedString::default());
                                window.set_codex_user_code(SharedString::default());
                                window.set_codex_copy_status(SharedString::default());
                                window.set_codex_auth_status(SharedString::from(if is_ru {
                                    "[err] вход Codex недоступен"
                                } else {
                                    "[err] Codex sign-in unavailable"
                                }));
                            }
                        }
                    });
                });
            });
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        let overlay = overlay_weak.clone();
        win.on_codex_model_selected(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if index < 0 {
                return;
            }
            let Some(model) = window.get_codex_model_ids().row_data(index as usize) else {
                return;
            };
            let model = model.to_string();
            let vision_ids = window.get_codex_vision_model_ids();
            let accepts_images = (0..vision_ids.row_count()).any(|row| {
                vision_ids
                    .row_data(row)
                    .is_some_and(|candidate| candidate.as_str() == model.as_str())
            });
            invalidate_codex_snapshot_ui();
            let mut c = cfg_c.write();
            c.codex_model.clone_from(&model);
            c.codex_reasoning_effort.clear();
            let same_forced_off = if c.vision_provider == "same" {
                if accepts_images {
                    c.codex_vision_model.clone_from(&model);
                    false
                } else {
                    c.vision_provider = "off".into();
                    c.codex_vision_model.clear();
                    true
                }
            } else {
                false
            };
            let active_stack = active_stack_label(&c);
            if overlay_backend::config::save(&c).is_err() {
                eprintln!("[overlay-host] Codex model save failed");
            }
            drop(c);
            window.set_vision_same_available(accepts_images);
            if same_forced_off {
                window.set_vision_provider_index(0);
            }
            if let Some(bar) = overlay.upgrade() {
                bar.set_active_stack(SharedString::from(active_stack));
            }
            refresh_codex_account_status(window.as_weak(), cfg_c.clone());
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_reasoning_selected(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if index < 0 {
                return;
            }
            let Some(effort) = window.get_codex_reasoning_ids().row_data(index as usize) else {
                return;
            };
            let mut c = cfg_c.write();
            c.codex_reasoning_effort = effort.to_string();
            if overlay_backend::config::save(&c).is_err() {
                eprintln!("[overlay-host] Codex reasoning save failed");
            }
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_vision_model_selected(move |index| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if index < 0 {
                return;
            }
            let Some(model) = window.get_codex_vision_model_ids().row_data(index as usize) else {
                return;
            };
            let mut c = cfg_c.write();
            c.codex_vision_model = model.to_string();
            if overlay_backend::config::save(&c).is_err() {
                eprintln!("[overlay-host] Codex vision model save failed");
            }
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_models_refresh(move || {
            refresh_codex_account_status(weak.clone(), cfg_c.clone());
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_disconnect_clicked(move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let is_ru = {
                let mut c = cfg_c.write();
                if c.ai_provider == "codex" && c.vision_provider == "same" {
                    c.vision_provider = "off".into();
                    window.set_vision_provider_index(0);
                    if overlay_backend::config::save(&c).is_err() {
                        eprintln!("[overlay-host] Codex disconnect vision repair failed");
                    }
                }
                c.ui_is_ru()
            };
            window.set_vision_same_available(false);
            let generation = invalidate_codex_login_ui();
            window.set_codex_auth_busy(true);
            window.set_codex_models_busy(false);
            window.set_codex_login_url(SharedString::default());
            window.set_codex_user_code(SharedString::default());
            window.set_codex_copy_status(SharedString::default());
            let worker_weak = window.as_weak();
            std::thread::spawn(move || {
                let state = overlay_backend::codex_subscription::disconnect();
                let label = codex_account_label(&state, is_ru);
                let _ =
                    slint::invoke_from_event_loop(move || {
                        if codex_ui_is_current(generation) {
                            if let Some(window) = worker_weak.upgrade() {
                                window.set_codex_auth_busy(false);
                                window.set_codex_auth_status(SharedString::from(label));
                                window.set_codex_model_ids(ModelRc::new(VecModel::from(Vec::<
                                    SharedString,
                                >::new(
                                ))));
                                window.set_codex_model_labels(ModelRc::new(VecModel::from(Vec::<
                                    SharedString,
                                >::new(
                                ))));
                                window.set_codex_model_index(-1);
                                window.set_codex_reasoning_ids(ModelRc::new(VecModel::from(
                                    Vec::<SharedString>::new(),
                                )));
                                window.set_codex_reasoning_labels(ModelRc::new(VecModel::from(
                                    Vec::<SharedString>::new(),
                                )));
                                window.set_codex_reasoning_index(-1);
                                window.set_codex_vision_model_ids(ModelRc::new(VecModel::from(
                                    Vec::<SharedString>::new(),
                                )));
                                window.set_codex_vision_model_labels(ModelRc::new(VecModel::from(
                                    Vec::<SharedString>::new(),
                                )));
                                window.set_codex_vision_model_index(-1);
                                window.set_codex_rate_status(SharedString::default());
                            }
                        }
                    });
            });
        });
    }
    win.on_codex_open_signin_clicked(move |url| {
        if !overlay_backend::codex_subscription::allowed_signin_url(url.as_str()) {
            return;
        }
        if let Err(error) = std::process::Command::new("explorer.exe")
            .arg(url.as_str())
            .spawn()
        {
            eprintln!("[overlay-host] Codex sign-in page launch failed: {error}");
        }
    });
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_codex_copy_code_clicked(move |code| {
            let Some(window) = weak.upgrade() else {
                return;
            };
            if code != window.get_codex_user_code() || code.is_empty() {
                window.set_codex_copy_status(SharedString::default());
                return;
            }
            let result = copy_codex_user_code(code.as_str(), |value| {
                slint_replay::native::clipboard::set_text(value).map_err(|_| ())
            });
            if result == CodexCopyResult::Failed {
                eprintln!("[overlay-host] Codex code copy failed");
            }
            window.set_codex_copy_status(SharedString::from(codex_copy_status(
                result,
                cfg_c.read().ui_is_ru(),
            )));
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let overlay = overlay_weak.clone();
        win.on_ai_local_base_url_save(move |v| {
            let base_url = v.trim().to_string();
            let root = overlay_backend::local_ai::default_root();
            let (
                managed,
                quality,
                model,
                saved_base_url,
                local_vision,
                vision_provider,
                vision_available,
            ) = {
                let mut c = cfg_c.write();
                c.ai_local_base_url = base_url.clone();
                let managed = overlay_backend::local_ai::is_managed_llama_endpoint(&base_url);
                if managed {
                    overlay_backend::local_ai::repair_managed_model_state(&mut c, &root);
                    if c.ai_provider == "local" && c.deep_lock {
                        c.suppress_tiles = true;
                    }
                }
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_local_base_url save failed: {e:#}");
                    return;
                }
                (
                    managed,
                    c.ai_local_quality,
                    c.ai_local_model.clone(),
                    c.ai_local_base_url.clone(),
                    c.ai_local_vision,
                    c.vision_provider.clone(),
                    overlay_backend::local_ai::local_vision_available(&c, &root),
                )
            };
            if let Some(o) = overlay.upgrade() {
                refresh_lock_chip(&o, &cfg_c);
            }
            if let Some(w) = weak.upgrade() {
                w.set_managed_local_server(managed);
                w.set_ai_local_quality(quality);
                w.set_ai_local_model_profile_index(
                    overlay_backend::local_ai::ManagedModel::from_config(&model, quality).index(),
                );
                w.set_ai_local_base_url_input(SharedString::from(saved_base_url.clone()));
                w.set_ai_local_vision(local_vision);
                w.set_vision_same_available(local_vision);
                w.set_ai_local_vision_available(vision_available);
                w.set_vision_provider_index(super::settings_vision::vision_provider_index_from_id(
                    &vision_provider,
                ));
                refresh_local_model_resource_warning(&w, root, saved_base_url, model);
            }
            // #E10.1 — re-query models against the new URL.
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.ai_local_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_model_selected(move |model| {
            let m = model.trim().to_string();
            if m.is_empty() {
                return;
            }
            let base_url = {
                let mut c = cfg_c.write();
                c.ai_local_model = m.clone();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_local_model save failed: {e:#}");
                    return;
                }
                c.ai_local_base_url.clone()
            };
            if let Some(w) = weak.upgrade() {
                refresh_local_model_resource_warning(
                    &w,
                    overlay_backend::local_ai::default_root(),
                    base_url,
                    m.clone(),
                );
            }
            diag!("ai_local_model selected: {m}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_models_refresh(move || {
            fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_vision_changed(move |on| {
            let root = overlay_backend::local_ai::default_root();
            let (local_vision, vision_provider) = {
                let mut c = cfg_c.write();
                overlay_backend::local_ai::set_local_vision(&mut c, &root, on);
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ai_local_vision save failed: {e:#}");
                }
                (c.ai_local_vision, c.vision_provider.clone())
            };
            if let Some(w) = weak.upgrade() {
                w.set_ai_local_vision(local_vision);
                w.set_vision_same_available(local_vision);
                w.set_vision_provider_index(super::settings_vision::vision_provider_index_from_id(
                    &vision_provider,
                ));
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_ai_local_thinking_changed(move |on| {
            let mut c = cfg_c.write();
            c.ai_local_thinking = on;
            let _ = overlay_backend::config::save(&c);
            // Mirror the boot-time + provider-switch logic: no-think is the
            // INVERSE of "thinking" and only applies to the local provider.
            overlay_backend::ai::set_local_no_think(c.ai_provider == "local" && !on);
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_test_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_ai_local_test_result(SharedString::from("Проверка…"));
            let (base_url, bearer, model, ui_is_ru) = {
                let c = cfg_c.read();
                (
                    c.ai_local_base_url.clone(),
                    c.ai_local_bearer.clone(),
                    c.ai_local_model.clone(),
                    c.ui_is_ru(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::ai::test_connection(
                            base_url, bearer, model,
                        )) {
                            Ok(s) => format!("[ok] {s}"),
                            // A deep-locked managed server: the specific
                            // localized notice instead of a misleading "server
                            // not answering".
                            Err(e)
                                if overlay_backend::deep_lock::is_blocked_error(&e.to_string()) =>
                            {
                                format!(
                                    "[--] {}",
                                    overlay_backend::deep_lock::blocked_notice(ui_is_ru)
                                )
                            }
                            // UI-audit 2026-06-13: do NOT echo the raw error body
                            // into the panel. A starting llama-server returns the
                            // full `HTTP 503 — {"error":{"message":"Loading
                            // model"…}}` JSON, which (a) stretched the window and
                            // (b) read as a failure when the model is just still
                            // loading. Map the common cases to a short human line;
                            // a generic message otherwise (also avoids leaking a
                            // base_url/host from a transport error).
                            Err(e) => {
                                let es = e.to_string().to_lowercase();
                                if es.contains("503") || es.contains("loading") {
                                    "Сервер запускает модель — подождите ~10 с и повторите."
                                        .to_string()
                                } else {
                                    "[--] Локальный сервер не отвечает — проверьте, что он запущен."
                                        .to_string()
                                }
                            }
                        }
                    }
                    Err(e) => format!("[--] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_ai_local_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Phase E6 v27 — AI bridge connection test. Off-thread (local
    // current-thread tokio runtime) so the blocking HTTP round-trip
    // doesn't freeze the UI; result posted back via invoke_from_
    // event_loop. ASCII status prefixes (no ✓/✗ missing-glyphs).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_bridge_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_ai_bridge_test_result(SharedString::from("testing…"));
            let endpoint = cfg_c.read().ai_endpoint(false);
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::ai::test_connection_endpoint(endpoint)) {
                            Ok(s) => format!("[ok] {s}"),
                            Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                        }
                    }
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_ai_bridge_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;
    use std::cell::RefCell;

    fn model(
        id: &str,
        is_default: bool,
        image: bool,
    ) -> overlay_backend::codex_subscription::CodexModel {
        overlay_backend::codex_subscription::CodexModel {
            id: id.to_string(),
            display_name: id.to_string(),
            is_default,
            default_reasoning_effort: Some("medium".to_string()),
            reasoning_efforts: vec!["low".to_string(), "medium".to_string()],
            input_modalities: if image {
                vec!["text".to_string(), "image".to_string()]
            } else {
                vec!["text".to_string()]
            },
        }
    }

    #[test]
    fn fresh_codex_selection_prefers_luna_but_preserves_explicit_choice() {
        let models = vec![
            model("gpt-default", true, true),
            model(PREFERRED_CODEX_MODEL, false, true),
            model("gpt-explicit", false, false),
        ];
        assert_eq!(preferred_codex_model_index(&models, "", false), Some(1));
        assert_eq!(
            preferred_codex_model_index(&models, "gpt-explicit", false),
            Some(2)
        );
        assert_eq!(
            preferred_codex_model_index(&models, "retired-model", false),
            None
        );
    }

    #[test]
    fn codex_vision_selection_filters_text_only_models() {
        let models = vec![
            model("text-default", true, false),
            model(PREFERRED_CODEX_MODEL, false, false),
            model("vision-model", false, true),
        ];
        assert_eq!(preferred_codex_model_index(&models, "", true), Some(2));
        assert_eq!(
            preferred_codex_model_index(&models, PREFERRED_CODEX_MODEL, true),
            None
        );
    }

    #[test]
    fn reasoning_labels_do_not_invent_unsupported_off_value() {
        assert_eq!(reasoning_label("low", false), "Low (fastest available)");
        assert_eq!(
            reasoning_label("minimal", false),
            "Minimal (no reasoning, fastest)"
        );
        assert_eq!(reasoning_label("high", false), "High");
    }

    #[test]
    fn catalog_repairs_require_a_signed_in_account() {
        use overlay_backend::codex_subscription::AccountState;

        assert!(catalog_is_authoritative(&AccountState::SignedIn {
            plan: Some("pro".into())
        }));
        assert!(!catalog_is_authoritative(&AccountState::SignedOut));
        assert!(!catalog_is_authoritative(&AccountState::Error));
        assert!(!catalog_is_authoritative(&AccountState::NotInstalled));
    }

    #[test]
    fn effort_normalization_is_explained_in_the_current_language() {
        assert_eq!(
            reasoning_normalization_notice("high", "", false),
            Some(
                "The saved reasoning effort is no longer supported; the model default is now selected."
            )
        );
        assert!(reasoning_normalization_notice("low", "low", false).is_none());
        assert!(reasoning_normalization_notice("", "", true).is_none());
    }

    #[test]
    fn model_picker_label_contains_no_reasoning_metadata() {
        let model = model("gpt-clean", true, true);
        assert_eq!(codex_model_label(&model), "gpt-clean");
        assert!(!codex_model_label(&model).contains("reasoning"));
    }

    #[test]
    fn codex_copy_writes_exact_displayed_code_and_skips_blank() {
        let written = RefCell::new(Vec::new());
        let result = copy_codex_user_code("ABCD-1234", |value| {
            written.borrow_mut().push(value.to_string());
            Ok::<(), ()>(())
        });
        assert_eq!(result, CodexCopyResult::Copied);
        assert_eq!(written.into_inner(), ["ABCD-1234"]);

        let called = RefCell::new(false);
        assert_eq!(
            copy_codex_user_code("", |_| {
                *called.borrow_mut() = true;
                Ok::<(), ()>(())
            }),
            CodexCopyResult::Empty
        );
        assert!(!called.into_inner());
    }

    #[test]
    fn codex_copy_feedback_is_short_generic_and_localized() {
        assert_eq!(
            codex_copy_status(CodexCopyResult::Copied, false),
            "[ok] Code copied"
        );
        assert_eq!(
            codex_copy_status(CodexCopyResult::Copied, true),
            "[ok] Код скопирован"
        );
        assert_eq!(
            codex_copy_status(CodexCopyResult::Failed, false),
            "[err] Copy failed"
        );
        assert_eq!(
            codex_copy_status(CodexCopyResult::Failed, true),
            "[err] Не удалось скопировать"
        );
    }
}
