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
    populate_token_status, ComponentHandle, ModelRc, SettingsWindow, SharedString, VecModel,
};

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

/// Wire the AI-tab Settings callbacks onto the Settings window — both the cloud
/// bridge and the local server. Moved VERBATIM out of `open_settings` (P1 domain
/// split) — same captures, same behavior. Needs only `win` (for the closures +
/// the tests' / refreshes' `as_weak()`) and `cfg` (cloned per closure); none of
/// the AI blocks touch `state` / `overlay_weak` / `slint_rt` / `rt_handle` (the
/// bar's active-stack readout is refreshed by the Settings close handler, which
/// stays in `open_settings`), so no extra params are threaded through.
pub(crate) fn wire_ai_settings(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) {
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
        win.on_ai_provider_changed(move |idx| {
            let provider = if idx == 1 { "local" } else { "cloud" };
            let mut c = cfg_c.write();
            c.ai_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_provider save failed: {e:#}");
                return;
            }
            overlay_backend::ai::set_local_no_think(provider == "local" && !c.ai_local_thinking);
            drop(c);
            diag!("ai_provider -> {provider}");
            // #E10.1 — switching to Local auto-populates the model dropdown.
            if provider == "local" {
                fetch_models(weak.clone(), cfg_c.clone(), ModelTarget::Local);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.ai_local_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_base_url save failed: {e:#}");
                return;
            }
            drop(c);
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
        win.on_ai_local_model_selected(move |model| {
            let m = model.trim().to_string();
            if m.is_empty() {
                return;
            }
            let mut c = cfg_c.write();
            c.ai_local_model = m.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ai_local_model save failed: {e:#}");
                return;
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
        win.on_ai_local_vision_changed(move |on| {
            let mut c = cfg_c.write();
            c.ai_local_vision = on;
            let _ = overlay_backend::config::save(&c);
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
            let (base_url, bearer, model) = {
                let c = cfg_c.read();
                (
                    c.ai_local_base_url.clone(),
                    c.ai_local_bearer.clone(),
                    c.ai_local_model.clone(),
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
            let (base_url, bearer, model) = {
                let c = cfg_c.read();
                (
                    c.ai_base_url.clone(),
                    c.ai_bearer.clone(),
                    c.ai_model.clone(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(
                        base_url, bearer, model,
                    )) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
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
