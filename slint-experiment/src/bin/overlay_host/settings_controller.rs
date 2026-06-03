// Settings controller: the Settings window + every Settings-tab handler
// (Phase 7b -- the FINAL phase of the `overlay_host.rs` modularization; see
// `docs/overlay-host-modularization-plan.md` section 5.8/5.9).
//
// This module owns the host-side Settings surface, moved here VERBATIM:
//
// - `open_settings` -- the entire Settings controller, including all its inline
//   closures: the stealth toggle, the colour-scheme switch, the tile-opacity
//   slider, and every tab handler (AI / STT / Vision / Audio / Profile /
//   Interface / Diagnostics / Hotkeys), the server import / export wiring, and
//   the Interface-tab "Run setup wizard" button (forwards to `open_wizard`);
// - the updater + local-AI UI closures Phase 6 deferred because they are inline
//   inside `open_settings`: `on_install_local_ai_clicked`,
//   `on_cancel_local_ai_clicked`, `on_check_updates_clicked`,
//   `on_install_update_clicked` (the backend installer / updater itself stays in
//   `overlay-backend`; only the Slint wiring lives here);
// - `ModelTarget` + `fetch_models` (the `{base_url}/models` dropdown populate);
// - the Settings helpers `msg_refresh_after_import`, `apply_server_preview`,
//   `refresh_profiles`, `populate_token_status`.
//
// What STAYS in `overlay_host.rs` (reached here through the glob below):
// - `fn main` (the gear-chip handler that calls `open_settings(...)` resolves
//   via the `use settings_controller::*;` re-export);
// - the other window openers `open_text_ask` / `open_help` / `open_palette`
//   (+ `results_index` / `kb_to_palette_results`) and `short_model_name` /
//   `active_stack_label` (the latter is `pub(crate)` and read by the bar +
//   `open_settings`'s close handler through `use super::*;`).
//
// SECURITY (unchanged by this mechanical move, per section 9):
// - the Diagnostics "Copy report" closure still calls `build_diag_report`
//   (diagnostics.rs) -- the report stays REDACTED (no bearer / API key /
//   transcript / profile / screenshot; LAN bridge IP masked);
// - the AI / STT / Vision test-result tiles keep their GENERIC messages -- no
//   `base_url` / LAN IP leaks into a screen-shared Settings window;
// - `apply_server_preview` shows key PRESENCE only and `mask_host`s the URLs.
//
// NOTE (section 7): this extraction imports the parent crate-root via
// `use super::*;` (the moved code reaches the Slint `SettingsWindow` /
// `OverlayBarWindow` types, the win32 helpers, the `WindowRegistry`,
// `present_window_stealth_aware` / `apply_scheme_settings` / `clamp_scheme` /
// `set_global_stealth`, `populate_diagnostics` / `build_diag_report`,
// `open_wizard`, `try_acquire_mic` / `release_mic`, and `active_stack_label`
// through it). That is intentional for the move; imports narrow in a later pass.
use super::*;

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

#[allow(clippy::too_many_arguments)]
pub(crate) fn open_settings(
    state: &slint_replay::app_state::SharedState,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    tiles_ref: &TileWindows,
    cfg: &overlay_backend::config::SharedConfig,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    // Phase 1 (§5.1) — the Settings-tab stealth toggle + colour-scheme switch
    // now reach every open window through this registry (text_ask / palette /
    // wizard / 🆘 help / recover-offer included), and the nested "Run setup
    // wizard" button forwards the same registry to `open_wizard`.
    registry: &WindowRegistry,
) {
    // Light up the bar's ⚙ chip while Settings is open (user: "значок
    // настроек не загорается когда настройки открыты"). Cleared in the
    // window's close handler below.
    if let Some(o) = overlay_weak.upgrade() {
        o.set_settings_open(true);
    }
    let mut settings_slot = settings_ref.borrow_mut();
    if let Some(existing) = settings_slot.as_ref() {
        // Refresh token status + profiles — config might have changed since last open.
        populate_token_status(existing, cfg);
        populate_diagnostics(existing, cfg);
        {
            let snap = cfg.read();
            refresh_profiles(existing, &snap);
        }
        let _ = existing.show();
        return;
    }
    let win = match SettingsWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] SettingsWindow::new failed: {e}");
            return;
        }
    };
    {
        let st = state.lock().ok();
        if let Some(st) = st {
            win.set_always_on_top_toggle(st.always_on_top);
            win.set_stealth_toggle(st.stealth);
        }
    }
    populate_token_status(&win, cfg);
    populate_diagnostics(&win, cfg);
    // Phase E8 — show the running version in the Updates tab.
    win.set_app_version(SharedString::from(env!("CARGO_PKG_VERSION")));
    // Phase E6 v29 / F — load the profile list + active context into the editor,
    // and seed the Coaching + Auto-tiles controls (previously dead/cosmetic).
    {
        let snap = cfg.read();
        refresh_profiles(&win, &snap);
        win.set_coaching_debrief(snap.post_meeting_debrief_enabled);
        win.set_auto_tiles_enabled(snap.auto_tiles_enabled);
        win.set_trigger_keywords_input(SharedString::from(snap.trigger_keywords.as_str()));
    }

    // Phase E6 v23 — populate the Audio tab's mic dropdown from real
    // WASAPI capture endpoints + select the saved device. User: "Audio
    // не подгружает реальные микрофоны".
    {
        // V0.8.4 — WASAPI device enumeration (cold COM + a per-endpoint
        // friendly-name RPC to the audio service) was ~30-300ms of SYNCHRONOUS
        // pre-show stall on the UI thread, which made the gear feel laggy. Show a
        // placeholder now and fill the dropdown when enumeration returns from a
        // worker thread (mirrors the mic-test / fetch_models off-thread pattern).
        win.set_mic_devices(ModelRc::new(VecModel::from(vec![SharedString::from(
            "(loading devices…)",
        )])));
        win.set_mic_device_index(0);
        let saved = cfg.read().mic_device.clone();
        let weak = win.as_weak();
        std::thread::spawn(move || {
            let devices = overlay_backend::audio::list_devices()
                .map(|d| d.inputs)
                .unwrap_or_default();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(w) = weak.upgrade() else { return };
                let model: Vec<SharedString> = if devices.is_empty() {
                    vec![SharedString::from("(no capture devices found)")]
                } else {
                    devices
                        .iter()
                        .map(|d| SharedString::from(d.as_str()))
                        .collect()
                };
                // Find the saved device's index (default 0 = system default).
                let sel = saved
                    .as_deref()
                    .and_then(|name| devices.iter().position(|d| d == name))
                    .unwrap_or(0);
                w.set_mic_devices(ModelRc::new(VecModel::from(model)));
                w.set_mic_device_index(sel as i32);
            });
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_mic_device_selected(move |name| {
            let mut c = cfg_c.write();
            c.mic_device = Some(name.to_string());
            let _ = overlay_backend::config::save(&c);
            eprintln!("[overlay-host] mic_device -> {name}");
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_mic_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_mic_test_result(SharedString::from("recording 3s…"));
            let device = cfg_c.read().mic_device.clone();
            let weak_for_result = w.as_weak();
            // Blocking WASAPI record off the UI thread; post result back.
            std::thread::spawn(move || {
                // M-1: take the single-mic guard so this test can't collide with
                // PTT / voice follow-up / dictation (a 2nd WASAPI open garbles
                // both). Report busy instead of recording.
                let msg = if !try_acquire_mic() {
                    "[!] mic busy — close PTT / dictation and retry".to_string()
                } else {
                    let result = overlay_backend::audio::record_mic_blocking(3000, device);
                    release_mic();
                    match result {
                        Ok(samples) if samples.is_empty() => "no audio captured".to_string(),
                        Ok(samples) => {
                            // RMS energy + a -45 dBFS speech threshold (silent room
                            // is < -55 dBFS). Shared helper with the diagnostics tab
                            // — User: "я могу ничего не говорить, но всё равно OK"
                            // was the old peak==0 check passing on any tiny noise.
                            let dbfs = overlay_backend::audio::rms_dbfs(&samples);
                            if dbfs < -45.0 {
                                format!(
                                    "[!] too quiet ({dbfs:.0} dBFS) — say something / check mic"
                                )
                            } else {
                                format!("[ok] heard you ({dbfs:.0} dBFS RMS)")
                            }
                        }
                        Err(e) => format!("error: {e}"),
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_for_result.upgrade() {
                        w.set_mic_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    let s2 = state.clone();
    let tiles_ref2 = tiles_ref.clone();
    win.on_always_on_top_changed(move |on| {
        if let Ok(mut st) = s2.lock() {
            st.always_on_top = on;
        }
        for t in tiles_ref2.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_always_on_top(hwnd, on);
            }
        }
    });

    let s3 = state.clone();
    let overlay_for_stealth = overlay_weak.clone();
    let cfg_st = cfg.clone();
    // Phase 1 (§5.1) — ONE registry clone replaces the per-window stealth loops
    // (tiles / this Settings window / text_ask / palette / 🆘 help / recover-offer
    // / wizard). Note: unlike the bar + wizard handlers, the Settings-tab toggle
    // does NOT drop the bar's taskbar button — that behaviour is preserved by
    // driving the bar inline here without `set_skip_taskbar`.
    let registry_stealth = registry.clone();
    win.on_stealth_changed(move |on| {
        if let Ok(mut st) = s3.lock() {
            st.stealth = on;
        }
        // #111 — global source-of-truth so later-created windows inherit it.
        set_global_stealth(on);
        // #E10.2 — persist so stealth survives a restart.
        {
            let mut c = cfg_st.write();
            c.stealth_enabled = on;
            let _ = config::save(&c);
        }
        // #111 — also flip the overlay bar itself (toggling stealth here
        // previously left it visible to capture). The bar stays inline (it is not
        // in the registry); the Settings window itself is covered by the registry.
        if let Some(o) = overlay_for_stealth.upgrade() {
            o.set_stealth_active(on);
            if let Ok(hwnd) = grab_hwnd(o.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
        // Every other open window (incl. this Settings window) via the one path.
        registry_stealth.apply_stealth(on);
    });

    // V0.8.4 — Settings → Interface "🪄 Run setup wizard" button. Re-opens the
    // guided first-run wizard on demand (it is also auto-shown on first launch).
    {
        // The wizard slot lives in the registry; forward the same registry so the
        // wizard's stealth toggle reaches every open window (Phase 1 §5.1).
        let wz = registry.wizard.clone();
        let cfg_w = cfg.clone();
        let st = settings_ref.clone();
        let state_w = state.clone();
        let ow = overlay_weak.clone();
        let registry_w = registry.clone();
        win.on_open_wizard_clicked(move || {
            open_wizard(&wz, &cfg_w, &state_w, &ow, &st, &registry_w);
        });
    }

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
    // ===== V4 — vision (screenshot) channel: provider switch + field saves + test =====
    {
        let cfg_c = cfg.clone();
        win.on_vision_provider_changed(move |idx| {
            let provider = match idx {
                0 => "off",
                1 => "same",
                3 => "local",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.vision_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_provider save failed: {e:#}");
                return;
            }
            diag!("vision_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_phonetics_changed(move |on| {
            let mut c = cfg_c.write();
            c.vision_phonetics = on;
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_phonetics save failed: {e:#}");
                return;
            }
            diag!("vision_phonetics -> {on}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_model save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_base_url_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_base_url = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_base_url save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_bearer_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_bearer = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_bearer save failed: {e:#}");
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_vision_local_model_save(move |v| {
            let mut c = cfg_c.write();
            c.vision_local_model = v.trim().to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] vision_local_model save failed: {e:#}");
            }
        });
    }
    {
        // Vision connection test — resolve the vision endpoint, reuse the AI
        // bridge tester. Off-thread so the HTTP round-trip can't freeze the UI.
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_vision_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_vision_test_result(SharedString::from("testing…"));
            let Some(ep) = cfg_c.read().vision_endpoint() else {
                w.set_vision_test_result(SharedString::from("[--] vision is off"));
                return;
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => match rt.block_on(overlay_backend::ai::test_connection(
                        ep.base_url,
                        ep.bearer,
                        ep.model,
                    )) {
                        Ok(s) => format!("[ok] {s}"),
                        Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                    },
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_vision_test_result(SharedString::from(msg));
                    }
                });
            });
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
        win.on_stt_gigaam_gpu_changed(move |on| {
            let mut c = cfg_c.write();
            c.stt_gigaam_gpu = on;
            let _ = overlay_backend::config::save(&c);
            // Apply immediately: update the global ORT accelerator + drop the
            // cached model so the next transcription reloads on the new backend.
            // (The live session pipeline reloads its own copy next session.)
            overlay_backend::stt::configure_gigaam_accelerator(on);
            overlay_backend::stt::reset_gigaam_cache();
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_local_test_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_ai_local_test_result(SharedString::from("testing…"));
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
                            Err(e) => format!("[--] {e}"),
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

    // E10.4 — one-click in-app local-AI installer. Runs the whole
    // download + launch pipeline on a worker thread, streams progress to
    // the panel, and on success stores the server handles (for kill-on-
    // quit), writes the local config (secrets preserved), and refreshes
    // the panel dropdowns + the bar's active-stack readout.
    {
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        let overlay_c = overlay_weak.clone();
        let weak = win.as_weak();
        win.on_install_local_ai_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            if w.get_local_ai_installing() {
                return; // re-entry guard
            }
            w.set_local_ai_installing(true);
            w.set_local_ai_progress(0.0);
            w.set_local_ai_gpu(SharedString::from(""));
            w.set_local_ai_status(SharedString::from("Подготовка…"));
            let cfg_t = cfg_c.clone();
            let state_t = state_c.clone();
            let overlay_t = overlay_c.clone();
            let weak_t = w.as_weak();
            // Shared cancel flag (lives in AppState so the Cancel button can
            // flip it); reset before each run.
            let cancel = {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel.clone()
            };
            cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            std::thread::spawn(move || {
                let on = {
                    let weak_p = weak_t.clone();
                    move |p: overlay_backend::local_ai::Progress| {
                        let weak_p = weak_p.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            let Some(w) = weak_p.upgrade() else { return };
                            match p {
                                overlay_backend::local_ai::Progress::Step(s) => {
                                    w.set_local_ai_status(SharedString::from(s));
                                }
                                overlay_backend::local_ai::Progress::Bytes {
                                    label,
                                    done,
                                    total,
                                } => {
                                    let frac = if total > 0 {
                                        (done as f64 / total as f64) as f32
                                    } else {
                                        0.0
                                    };
                                    w.set_local_ai_progress(frac);
                                    let mb = |b: u64| (b as f64) / 1_048_576.0;
                                    w.set_local_ai_status(SharedString::from(format!(
                                        "{label}: {:.0} / {:.0} MB",
                                        mb(done),
                                        mb(total)
                                    )));
                                }
                                overlay_backend::local_ai::Progress::Gpu(s) => {
                                    w.set_local_ai_on_gpu(s.starts_with("GPU"));
                                    w.set_local_ai_gpu(SharedString::from(s));
                                }
                            }
                        });
                    }
                };
                // Re-install hardening: stop any servers we previously launched
                // so a fresh `--mmproj` llama-server can bind :8080. Without this
                // a stale vision-less server keeps the port and the new one
                // silently fails to start (wait_ready still sees the old one and
                // reports success). Fresh installs have nothing to drain.
                {
                    let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                    for mut child in s.local_ai_servers.drain(..) {
                        let _ = child.kill();
                    }
                }
                let opts = overlay_backend::local_ai::InstallOptions::default();
                match overlay_backend::local_ai::install(&opts, &cancel, &on) {
                    Ok(res) => {
                        let model = res.ai_local_model.clone();
                        let gigaam_dir = res.stt_gigaam_dir.clone();
                        let on_gpu = res.on_gpu;
                        {
                            let mut c = cfg_t.write();
                            overlay_backend::local_ai::apply_result(&mut c, &res);
                            if let Err(e) = overlay_backend::config::save(&c) {
                                eprintln!("[overlay-host] local-ai config save failed: {e:#}");
                            }
                            overlay_backend::ai::set_local_no_think(!c.ai_local_thinking);
                        }
                        {
                            let mut s = state_t.lock().unwrap_or_else(|p| p.into_inner());
                            s.local_ai_servers.extend(res.servers);
                        }
                        let weak_done = weak_t.clone();
                        let overlay_done = overlay_t.clone();
                        let cfg_done = cfg_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            diag!("local-ai installed: model={} gpu={}", model, on_gpu);
                            if let Some(w) = weak_done.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_progress(1.0);
                                w.set_local_ai_status(SharedString::from(
                                    "Готово. Локальный AI настроен и запущен.",
                                ));
                                w.set_ai_provider_index(1);
                                w.set_ai_local_base_url_input(SharedString::from(
                                    overlay_backend::local_ai::LLAMA_BASE_URL,
                                ));
                                w.set_stt_provider_index(2);
                                w.set_stt_whisper_url_input(SharedString::from(
                                    overlay_backend::local_ai::WHISPER_BASE_URL,
                                ));
                                w.set_stt_gigaam_dir_input(SharedString::from(gigaam_dir));
                            }
                            if let Some(o) = overlay_done.upgrade() {
                                o.set_active_stack(SharedString::from(active_stack_label(
                                    &cfg_done.read(),
                                )));
                            }
                        });
                    }
                    Err(e) => {
                        let cancelled = e
                            .to_string()
                            .contains(overlay_backend::local_ai::CANCEL_SENTINEL);
                        let msg = if cancelled {
                            "Отменено.".to_string()
                        } else {
                            eprintln!("[overlay-host] local-ai install failed: {e:#}");
                            format!("Ошибка установки: {e}")
                        };
                        let weak_err = weak_t.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak_err.upgrade() {
                                w.set_local_ai_installing(false);
                                w.set_local_ai_status(SharedString::from(msg));
                            }
                        });
                    }
                }
            });
        });
    }

    // E10.4 — Cancel button: flip the shared cancel flag the install worker
    // thread + the curl poll loop watch.
    {
        let state_c = state.clone();
        let weak = win.as_weak();
        win.on_cancel_local_ai_clicked(move || {
            {
                let s = state_c.lock().unwrap_or_else(|p| p.into_inner());
                s.local_ai_cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(w) = weak.upgrade() {
                w.set_local_ai_status(SharedString::from("Отмена…"));
            }
        });
    }

    // Phase E6 v20 — tile opacity slider. Persists to config AND
    // applies to all currently-visible tiles via tiles_ref.
    {
        let cfg_c = cfg.clone();
        let tiles_c = tiles_ref.clone();
        win.on_tile_body_opacity_changed(move |new_value| {
            let clamped = new_value.clamp(0.5, 1.0);
            {
                let mut c = cfg_c.write();
                c.tile_body_opacity = clamped;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tile_body_opacity save failed: {e:#}");
                    return;
                }
            }
            // Phase E6 v36 — update the process-global so EVERY future
            // tile (F9 / F3 / KB-palette / auto-spawn) spawns at this
            // opacity, not just the ones currently on screen.
            set_global_tile_opacity(clamped);
            // Apply live to all currently-visible tiles.
            for tile in tiles_c.borrow().iter() {
                tile.set_body_opacity(clamped);
            }
            eprintln!("[overlay-host] tile_body_opacity -> {clamped:.2}");
        });
    }

    // Phase E6 v38 — interface-language switch. Selecting Русский/English
    // in the Interface tab switches the bundled translation LIVE (Slint
    // re-evaluates every @tr() binding) and persists ui_language so the
    // choice survives restart. Previously the dropdown was inert — it
    // showed "Русский" but never applied anything, so a stale .po made
    // the UI look English even though "ru" was nominally selected.
    {
        let cfg_lang = cfg.clone();
        win.on_language_selected(move |idx| {
            let lang = if idx == 1 { "en" } else { "ru" };
            match slint::select_bundled_translation(lang) {
                Ok(()) => eprintln!("[overlay-host] UI language -> {lang}"),
                Err(e) => eprintln!("[overlay-host] language {lang} not available: {e}"),
            }
            let mut c = cfg_lang.write();
            c.ui_language = lang.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] ui_language save failed: {e:#}");
            }
        });
    }

    // Colour-scheme switch. Selecting a scheme in the Interface tab recolours
    // EVERY window live (Theme is a per-window global, so we walk each one),
    // updates the process-global so future tiles/palette inherit it, and
    // persists color_scheme. Mirrors the tile-opacity handler's shape.
    {
        let cfg_scheme = cfg.clone();
        let overlay_scheme = overlay_weak.clone();
        // Phase 1 (§5.1) — re-skin every open window through the registry (the
        // bar stays inline). This now also reaches the palette / text_ask /
        // wizard / 🆘 help / recover-offer windows if open — same "no window
        // forgotten" guarantee as stealth; previously only tiles + Settings were
        // re-skinned live (the others kept their construction-time scheme).
        let registry_scheme = registry.clone();
        win.on_color_scheme_selected(move |idx| {
            let scheme = clamp_scheme(idx);
            // Persist first so a crash mid-repaint still survives the choice.
            {
                let mut c = cfg_scheme.write();
                c.color_scheme = scheme;
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] color_scheme save failed: {e:#}");
                    return;
                }
            }
            // Future windows (tiles, palette) read this at construction.
            set_global_scheme(scheme);
            // Re-skin all currently-live windows: bar inline, the rest via registry.
            if let Some(o) = overlay_scheme.upgrade() {
                apply_scheme_bar(&o, scheme);
            }
            registry_scheme.apply_scheme(scheme);
            eprintln!("[overlay-host] color_scheme -> {scheme}");
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

    // Phase E6 v27 — STT (Groq) connection test. Same off-thread
    // pattern; hits the Groq /models endpoint with the saved key.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_stt_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_stt_test_result(SharedString::from("testing…"));
            let backend = cfg_c.read().stt_backend();
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let msg = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::stt::test_connection_backend(&backend)) {
                            Ok(s) => format!("[ok] {s}"),
                            Err(e) => format!("[err] {e:#}").chars().take(90).collect(),
                        }
                    }
                    Err(e) => format!("[err] runtime: {e}"),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_stt_test_result(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // P1.1 — "Copy report": redacted diagnostics → clipboard with a brief
    // "copied" confirmation. build_diag_report masks the LAN bridge IP and
    // carries no bearer / API key / transcript / profile text.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_diagnostics_copy_report_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let report = build_diag_report(&cfg_c);
            match clipboard_win::set_clipboard_string(&report) {
                Ok(()) => {
                    w.set_diag_copied(true);
                    let wk = w.as_weak();
                    Timer::single_shot(Duration::from_millis(1800), move || {
                        if let Some(w) = wk.upgrade() {
                            w.set_diag_copied(false);
                        }
                    });
                }
                Err(e) => eprintln!("[overlay-host] diag report copy failed: {e}"),
            }
        });
    }

    // #131 — diagnostics "Проверить всё": live-ping the ACTIVE AI endpoint
    // (resolved via ai_endpoint — NOT the raw cloud fields) + the active STT
    // backend, in ONE off-thread runtime, and write both rows back. Mic / sys
    // / stealth rows stay config-readiness (their live checks live on Audio).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_diagnostics_check_all_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_diag_ai_level(-1);
            w.set_diag_ai_detail(SharedString::from(""));
            w.set_diag_stt_level(-1);
            w.set_diag_stt_detail(SharedString::from(""));
            w.set_diag_mic_level(-1);
            w.set_diag_sys_level(-1);
            let (ai_base, ai_bearer, ai_model, stt_backend, mic_device, sys_device) = {
                let c = cfg_c.read();
                let ep = c.ai_endpoint(false);
                (
                    ep.base_url,
                    ep.bearer,
                    ep.model,
                    c.stt_backend(),
                    c.mic_device.clone(),
                    c.system_audio_device.clone(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                // 1. AI + STT live pings (async, on a throwaway runtime).
                let (ai_level, ai_msg, stt_level, stt_msg): (i32, String, i32, String) =
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => {
                            let (al, am): (i32, String) = match rt.block_on(
                                overlay_backend::ai::test_connection(ai_base, ai_bearer, ai_model),
                            ) {
                                Ok(s) => (0, format!("[ok] {s}")),
                                Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                            };
                            let (sl, sm): (i32, String) = match rt.block_on(
                                overlay_backend::stt::test_connection_backend(&stt_backend),
                            ) {
                                Ok(s) => (0, format!("[ok] {s}")),
                                Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                            };
                            (al, am, sl, sm)
                        }
                        Err(e) => {
                            let m = format!("[err] runtime: {e}");
                            (4, m.clone(), 4, m)
                        }
                    };
                let weak_a = weak_res.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_a.upgrade() {
                        w.set_diag_ai_level(ai_level);
                        w.set_diag_ai_detail(SharedString::from(ai_msg));
                        w.set_diag_stt_level(stt_level);
                        w.set_diag_stt_detail(SharedString::from(stt_msg));
                    }
                });
                // 2. Microphone — record 3s. "Готов" if the capture path works
                // (device opens + samples flow); a quiet result is fine (you
                // just didn't speak) — only a device error fails.
                // M-1: guard the diagnostics mic probe with the single-mic lock
                // too, so "Проверить всё" during an active session reports busy
                // instead of fighting PTT/voice/dictation for the device.
                let (mic_level, mic_msg): (i32, String) = if !try_acquire_mic() {
                    (
                        4,
                        "[!] mic busy — close PTT / dictation and retry".to_string(),
                    )
                } else {
                    let r = overlay_backend::audio::record_mic_blocking(3000, mic_device);
                    release_mic();
                    match r {
                        Ok(s) if s.is_empty() => (4, "[!] no audio captured".to_string()),
                        Ok(s) => {
                            let dbfs = overlay_backend::audio::rms_dbfs(&s);
                            if dbfs >= -45.0 {
                                (0, format!("[ok] heard you ({dbfs:.0} dBFS)"))
                            } else {
                                (0, format!("[ok] capture works · quiet ({dbfs:.0} dBFS)"))
                            }
                        }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    }
                };
                let weak_m = weak_res.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_m.upgrade() {
                        w.set_diag_mic_level(mic_level);
                        w.set_diag_mic_detail(SharedString::from(mic_msg));
                    }
                });
                // 3. System audio — SELF-TEST: play a short test tone through the
                // default output while capturing the loopback. If the loopback
                // hears our own tone, the output→loopback path works — the user
                // doesn't have to play anything.
                let (sys_level, sys_msg): (i32, String) =
                    match overlay_backend::audio::play_tone_and_capture(sys_device) {
                        Ok(s) => {
                            let dbfs = overlay_backend::audio::rms_dbfs(&s);
                            if dbfs > -60.0 {
                                (
                                    0,
                                    format!("[ok] loopback heard the test tone ({dbfs:.0} dBFS)"),
                                )
                            } else {
                                (
                                    4,
                                    "[!] test tone not captured — output device ≠ loopback source?"
                                        .to_string(),
                                )
                            }
                        }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_res.upgrade() {
                        w.set_diag_sys_level(sys_level);
                        w.set_diag_sys_detail(SharedString::from(sys_msg));
                    }
                });
            });
        });
    }

    // Phase E10 — STT provider selector + local-engine fields.
    {
        let cfg_c = cfg.clone();
        win.on_stt_provider_changed(move |idx| {
            let provider = match idx {
                1 => "gigaam",
                2 => "whisper",
                _ => "cloud",
            };
            let mut c = cfg_c.write();
            c.stt_provider = provider.to_string();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_provider save failed: {e:#}");
                return;
            }
            diag!("stt_provider -> {provider}");
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_gigaam_dir_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_gigaam_dir = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_gigaam_dir save failed: {e:#}");
                return;
            }
            diag!("stt_gigaam_dir saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_url_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_url = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_url save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_url saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_bearer_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_bearer = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_bearer save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_bearer saved ({} chars)", trimmed.len());
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_stt_whisper_model_save(move |v| {
            let trimmed = v.trim().to_string();
            let mut c = cfg_c.write();
            c.stt_whisper_model = trimmed.clone();
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] stt_whisper_model save failed: {e:#}");
                return;
            }
            diag!("stt_whisper_model saved ({} chars)", trimmed.len());
        });
    }

    // P1.7 — config parsed from a picked server-settings file, awaiting the
    // user's explicit Apply (set by the import-preview handler, taken by Apply,
    // cleared by Cancel). Kept out of the live config until confirmed.
    let pending_server_import: Rc<RefCell<Option<overlay_backend::config::Config>>> =
        Rc::new(RefCell::new(None));

    // Phase E6 v28 — full-profile export (incl. keys). Native save
    // dialog via rfd; writes the whole config.json to the chosen path.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_export_profile_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Export overlay-mvp settings (contains API keys)")
                .set_file_name("suflyor-settings.json")
                .add_filter("JSON", &["json"])
                .save_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "export cancelled".to_string(),
                Some(path) => match overlay_backend::config::export_to(&path, &snapshot) {
                    Ok(()) => format!("[ok] exported to {}", path.display()),
                    Err(e) => format!("[err] {e:#}"),
                },
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // Phase E6 v28 — full-profile import. Native open dialog; loads +
    // persists the config, then re-syncs the token-status display.
    // Live re-apply of every field would need a broader refresh, so
    // we tell the user to restart for full effect.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_import_profile_clicked(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Import overlay-mvp settings")
                .add_filter("JSON", &["json"])
                .pick_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "import cancelled".to_string(),
                Some(path) => match overlay_backend::config::import_from(&path) {
                    Ok(imported) => {
                        // Push the freshly-loaded values into the shared
                        // config so the running session sees them, then
                        // refresh the token-status display.
                        *cfg_c.write() = imported;
                        msg_refresh_after_import(&w, &cfg_c)
                    }
                    Err(e) => format!("[err] {e:#}"),
                },
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — server-ONLY EXPORT. Native save dialog; writes ONLY the AI/STT
    // server fields (incl. creds — intentional for a PC->PC transfer) and none
    // of the machine-local fields (profiles/devices/snippets/context).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_export_server_settings_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Export server settings (AI/STT only — contains API keys)")
                .set_file_name("suflyor-server-settings.json")
                .add_filter("JSON", &["json"])
                .save_file();
            let Some(w) = weak.upgrade() else { return };
            let msg = match picked {
                None => "export cancelled".to_string(),
                Some(path) => {
                    match overlay_backend::config::export_server_settings_to(&path, &snapshot) {
                        Ok(()) => format!("[ok] server settings exported to {}", path.display()),
                        Err(e) => format!("[err] {e:#}"),
                    }
                }
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — server-ONLY settings import, NOW two-step: pick a file -> show a
    // REDACTED preview (provider/url/model old->new + key presence as set/—;
    // never a secret value) and stash the parsed config; the user then clicks
    // Apply (below) to actually merge. The machine-local GigaAM model path is
    // kept from THIS PC on apply (apply_server_settings).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_import_server_settings_clicked(move || {
            let snapshot = cfg_c.read().clone();
            let picked = rfd::FileDialog::new()
                .set_title("Import server settings (AI/STT only) from a backup")
                .add_filter("JSON", &["json"])
                .pick_file();
            let Some(w) = weak.upgrade() else { return };
            let Some(path) = picked else {
                w.set_profile_io_result(SharedString::from("import cancelled"));
                return;
            };
            // Read + parse + build the redacted preview. The parse error stays
            // value-free (parse_config_bytes inside). No save happens yet.
            match overlay_backend::config::preview_server_settings_from(&path, &snapshot) {
                Ok((preview, imported)) => {
                    apply_server_preview(&w, &preview);
                    *pending.borrow_mut() = Some(imported);
                    w.set_server_preview_ready(true);
                    w.set_profile_io_result(SharedString::from(
                        "review the changes below, then Apply",
                    ));
                }
                Err(e) => {
                    *pending.borrow_mut() = None;
                    w.set_server_preview_ready(false);
                    w.set_profile_io_result(SharedString::from(format!("[err] {e:#}")));
                }
            }
        });
    }

    // P1.7 — APPLY the previewed server settings. Merges the stashed config's
    // server fields onto the current one (EXCLUDING the machine-local GigaAM
    // dir), persists, applies live, and refreshes the token-status display.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_apply_server_settings_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let Some(imported) = pending.borrow_mut().take() else {
                w.set_server_preview_ready(false);
                w.set_profile_io_result(SharedString::from("nothing to apply"));
                return;
            };
            let merged = {
                let current = cfg_c.read().clone();
                overlay_backend::config::apply_server_settings(&current, imported)
            };
            w.set_server_preview_ready(false);
            let msg = match overlay_backend::config::save(&merged) {
                Ok(()) => {
                    // Apply to the running session + refresh token-status.
                    *cfg_c.write() = merged;
                    let _ = msg_refresh_after_import(&w, &cfg_c);
                    "[ok] server settings applied (AI/STT providers, URLs, models, keys). Local profiles, devices, UI and snippets kept; the local GigaAM model path was kept from this PC. Restart for full effect.".to_string()
                }
                Err(e) => format!("[err] {e:#}"),
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — CANCEL the preview: drop the stashed config + hide the diff.
    {
        let weak = win.as_weak();
        let pending = pending_server_import.clone();
        win.on_cancel_server_settings_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            *pending.borrow_mut() = None;
            w.set_server_preview_ready(false);
            w.set_profile_io_result(SharedString::from("import cancelled"));
        });
    }

    // Phase E6 v29 — meeting-context (Profile) save. Writes to
    // cfg.meeting_context + persists; new AI calls read it from cfg
    // so it applies immediately (no restart needed for this field).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_meeting_context_save(move |text| {
            {
                let mut c = cfg_c.write();
                // Phase F — also mirror into the active profile so the picker
                // and the live context never drift.
                c.save_active_context(&text);
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] meeting_context save failed: {e:#}");
                    if let Some(w) = weak.upgrade() {
                        w.set_meeting_context_result(SharedString::from("[err] save failed"));
                    }
                    return;
                }
            }
            let chars = text.chars().count();
            eprintln!("[overlay-host] meeting_context saved ({chars} chars)");
            if let Some(w) = weak.upgrade() {
                w.set_meeting_context_result(SharedString::from(format!(
                    "[ok] saved ({chars} chars)"
                )));
            }
        });
    }
    // Phase F — multi-profile picker handlers. Each mutates cfg, persists, and
    // refreshes the picker + editor from cfg so the UI mirrors config exactly.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_selected(move |idx| {
            if idx < 0 {
                return;
            }
            let mut c = cfg_c.write();
            c.select_profile(idx as usize);
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_add(move |name| {
            let mut c = cfg_c.write();
            let ok = c.add_profile(name.as_str()).is_some();
            if ok {
                let _ = overlay_backend::config::save(&c);
            }
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if ok {
                    "[ok] профиль добавлен"
                } else {
                    "[--] пустое или занятое имя"
                }));
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_rename(move |name| {
            let mut c = cfg_c.write();
            let ok = c.rename_active_profile(name.as_str());
            if ok {
                let _ = overlay_backend::config::save(&c);
            }
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if ok {
                    "[ok] переименовано"
                } else {
                    "[--] пустое или занятое имя"
                }));
            }
        });
    }
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_profile_delete(move || {
            let mut c = cfg_c.write();
            c.delete_active_profile();
            let _ = overlay_backend::config::save(&c);
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from("[ok] профиль удалён"));
            }
        });
    }
    // Phase F — Coaching + Auto-tiles toggles (were dead). Each persists; the
    // detector + session-stop logic read these from cfg at runtime, so changes
    // apply without a restart.
    {
        let cfg_c = cfg.clone();
        win.on_coaching_debrief_changed(move |on| {
            let mut c = cfg_c.write();
            c.post_meeting_debrief_enabled = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_auto_tiles_enabled_changed(move |on| {
            let mut c = cfg_c.write();
            c.auto_tiles_enabled = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        let cfg_c = cfg.clone();
        win.on_trigger_keywords_save(move |text| {
            // Clamp: these keywords prepend to EVERY STT prompt, so a huge paste
            // would balloon every transcription. Trim + cap (cf. kb::search's
            // 200-char DoS guard).
            let clamped: String = text.trim().chars().take(400).collect();
            let mut c = cfg_c.write();
            c.trigger_keywords = clamped;
            let _ = overlay_backend::config::save(&c);
        });
    }

    // Phase E6 v43 — "Structure via AI": one-shot ai::complete that turns
    // the free-form / dictated context into a clean interview profile, then
    // replaces the editor field (user reviews + Saves). Off-thread (tokio)
    // so the UI doesn't block; result posted back via invoke_from_event_loop.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_context_process_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let current = w.get_meeting_context_input().to_string();
            if current.trim().is_empty() {
                w.set_meeting_context_result(SharedString::from(
                    "[--] пусто — нечего обрабатывать",
                ));
                return;
            }
            let (base_url, bearer, model, is_local) = {
                let c = cfg_c.read();
                // Structuring uses the smarter "prep" model.
                let ep = c.ai_endpoint(true);
                (ep.base_url, ep.bearer, ep.model, ep.is_local)
            };
            if base_url.is_empty() || model.is_empty() || (!is_local && bearer.is_empty()) {
                w.set_meeting_context_result(SharedString::from(
                    "[--] AI мост не настроен (вкладка AI мост)",
                ));
                return;
            }
            w.set_context_processing(true);
            w.set_meeting_context_result(SharedString::from("обработка через AI…"));
            let weak2 = w.as_weak();
            // Off-thread with a local current-thread runtime (reqwest is
            // async-only); same pattern as the AI-bridge / STT test buttons.
            std::thread::spawn(move || {
                let messages = vec![
                    ai::ChatMessage {
                        role: "system".into(),
                        content: ai::MessageContent::Text(
                            "Преобразуй текст пользователя в чёткий профиль для интервью: \
                             роль, ключевые навыки, технологии, области фокуса. Кратко, по \
                             пунктам, на русском. Исправь ошибки распознавания речи. Без \
                             преамбулы — сразу профиль."
                                .into(),
                        ),
                    },
                    ai::ChatMessage {
                        role: "user".into(),
                        content: ai::MessageContent::Text(current),
                    },
                ];
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(ai::complete(&base_url, &bearer, &model, messages, 1024)),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak2.upgrade() else {
                        return;
                    };
                    w.set_context_processing(false);
                    match res {
                        Ok(text) if !text.trim().is_empty() => {
                            w.set_meeting_context_input(SharedString::from(
                                text.trim().to_string(),
                            ));
                            w.set_meeting_context_result(SharedString::from(
                                "[ok] обработано — проверь и нажми «Сохранить контекст»",
                            ));
                        }
                        Ok(_) => w.set_meeting_context_result(SharedString::from(
                            "[--] AI вернул пустой ответ",
                        )),
                        Err(e) => w.set_meeting_context_result(SharedString::from(format!(
                            "[--] ошибка AI: {e}"
                        ))),
                    }
                });
            });
        });
    }

    // Phase E6 v43 — voice dictation into the context field. Toggle:
    // click to start recording the mic, click again to stop. The record
    // thread (audio::record_source_until_stop) transcribes on a local
    // runtime then APPENDS the text to the editor (user reviews + Saves).
    // Reuses the PTT 30s watchdog so a forgotten "stop" can't leak a
    // thread. dictate_stop is owned by the handler closure.
    {
        let dictate_stop: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::new(RefCell::new(None));
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_context_dictate_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            // Toggle OFF: stop the in-flight recording.
            if let Some(stop) = dictate_stop.borrow_mut().take() {
                stop.store(true, Ordering::Release);
                w.set_context_dictating(false);
                w.set_meeting_context_result(SharedString::from("расшифровка…"));
                return;
            }
            // Toggle ON: start a new recording.
            let (
                mic_dev,
                stt_backend,
                stt_is_local,
                groq_key,
                stt_language,
                trigger_keywords,
                meeting_context,
            ) = {
                let c = cfg_c.read();
                (
                    c.mic_device.clone(),
                    c.stt_backend(),
                    c.stt_is_local(),
                    c.groq_api_key.clone(),
                    c.stt_language.clone(),
                    c.trigger_keywords.clone(),
                    c.meeting_context.clone(),
                )
            };
            if !stt_is_local && groq_key.is_empty() {
                w.set_meeting_context_result(SharedString::from(
                    "[--] ключ Groq не задан (вкладка STT)",
                ));
                return;
            }
            // M2 — single-mic guard (shared with PTT-mic + voice follow-up).
            if !try_acquire_mic() {
                w.set_meeting_context_result(SharedString::from("[--] микрофон занят"));
                return;
            }
            let stop = Arc::new(AtomicBool::new(false));
            *dictate_stop.borrow_mut() = Some(stop.clone());
            spawn_ptt_watchdog(stop.clone());
            w.set_context_dictating(true);
            w.set_meeting_context_result(SharedString::from("запись… (нажми «Остановить»)"));
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                let pcm =
                    audio::record_source_until_stop(audio::AudioSource::Mic, mic_dev, None, stop)
                        .unwrap_or_else(|e| {
                            eprintln!("[overlay-host] dictation record failed: {e:#}");
                            Vec::new()
                        });
                release_mic(); // M2 — free the mic before transcription
                let text = if pcm.len() < 4800 {
                    String::new()
                } else {
                    let whisper_prompt =
                        stt::build_whisper_prompt(&trigger_keywords, &meeting_context);
                    match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => rt
                            .block_on(stt::transcribe_once(
                                &stt_backend,
                                &pcm,
                                stt_language.as_deref(),
                                whisper_prompt.as_deref(),
                            ))
                            .unwrap_or_default(),
                        Err(_) => String::new(),
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_res.upgrade() else {
                        return;
                    };
                    w.set_context_dictating(false);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        w.set_meeting_context_result(SharedString::from(
                            "[--] ничего не распознано",
                        ));
                        return;
                    }
                    let cur = w.get_meeting_context_input().to_string();
                    let joined = if cur.trim().is_empty() {
                        trimmed.to_string()
                    } else {
                        format!("{cur} {trimmed}")
                    };
                    w.set_meeting_context_input(SharedString::from(joined));
                    w.set_meeting_context_result(SharedString::from(
                        "[ok] добавлено — проверь и нажми «Сохранить контекст»",
                    ));
                });
            });
        });
    }

    // Phase E6 v25 — frameless Settings drag (cursor-delta, same as
    // bar + tiles). The "Settings" sidebar header is the handle.
    {
        let weak = win.as_weak();
        win.on_drag_start_requested(move || {
            if let Some(w) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_begin(hwnd);
                }
            }
        });
        let weak_move = win.as_weak();
        win.on_drag_moved(move || {
            if let Some(w) = weak_move.upgrade() {
                if let Ok(hwnd) = grab_hwnd(w.window()) {
                    drag_update(hwnd);
                }
            }
        });
    }

    // Phase E8 — in-app auto-update (Updates tab). Network calls run on a
    // detached thread with a local current-thread tokio runtime (same
    // pattern as the AI/STT test buttons — open_settings has no rt_handle).
    {
        let weak = win.as_weak();
        win.on_check_updates_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            w.set_update_checking(true);
            w.set_update_available(false);
            w.set_update_status(SharedString::from("Проверка GitHub…"));
            diag!("update: checking GitHub for newer release");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::check_latest(env!(
                        "CARGO_PKG_VERSION"
                    ))),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak2.upgrade() else {
                        return;
                    };
                    w.set_update_checking(false);
                    match res {
                        Ok(info) if info.newer && !info.download_url.is_empty() => {
                            w.set_update_download_url(SharedString::from(info.download_url));
                            w.set_update_available(true);
                            w.set_update_status(SharedString::from(format!(
                                "Доступна версия {} — нажмите «Обновить сейчас»",
                                info.latest_version
                            )));
                        }
                        Ok(info) if info.newer => w.set_update_status(SharedString::from(format!(
                            "Есть версия {}, но в релизе нет установщика",
                            info.latest_version
                        ))),
                        Ok(info) => w.set_update_status(SharedString::from(format!(
                            "У вас последняя версия ({})",
                            info.latest_version
                        ))),
                        Err(e) => {
                            w.set_update_status(SharedString::from(format!("Ошибка проверки: {e}")))
                        }
                    }
                });
            });
        });
    }
    {
        let weak = win.as_weak();
        win.on_install_update_clicked(move || {
            let Some(w) = weak.upgrade() else {
                return;
            };
            let url = w.get_update_download_url().to_string();
            if url.is_empty() {
                return;
            }
            w.set_update_checking(true);
            w.set_update_status(SharedString::from("Скачивание установщика…"));
            diag!("update: downloading installer");
            let weak2 = w.as_weak();
            std::thread::spawn(move || {
                let res = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt.block_on(overlay_backend::update::download_installer(&url)),
                    Err(e) => Err(anyhow::anyhow!("runtime: {e}")),
                };
                match res {
                    Ok(path) => match overlay_backend::update::run_installer(&path) {
                        Ok(()) => {
                            // Installer launched — quit so it can overwrite the
                            // running binary (its first page is interactive, so
                            // the app is gone before it reaches the File step).
                            diag!("update: installer launched, quitting app");
                            let _ = slint::invoke_from_event_loop(|| {
                                let _ = slint::quit_event_loop();
                            });
                        }
                        Err(e) => {
                            // P0.3: the installer failed to spawn (blocked exe /
                            // deleted file). Do NOT quit — stay open + show why.
                            diag!("update: installer spawn FAILED: {e:#}");
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak2.upgrade() {
                                    w.set_update_checking(false);
                                    w.set_update_status(SharedString::from(
                                        "Не удалось запустить установщик — приложение оставлено открытым (см. лог)",
                                    ));
                                }
                            });
                        }
                    },
                    Err(e) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = weak2.upgrade() {
                                w.set_update_checking(false);
                                w.set_update_status(SharedString::from(format!(
                                    "Ошибка обновления: {e}"
                                )));
                            }
                        });
                    }
                }
            });
        });
    }

    let weak_close = win.as_weak();
    let settings_close = settings_ref.clone();
    let overlay_for_close = overlay_weak.clone();
    let cfg_for_close = cfg.clone();
    win.on_close_clicked(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *settings_close.borrow_mut() = None;
        // Un-light the bar's ⚙ chip + refresh the active-stack readout (the
        // user may have switched STT/AI provider while Settings was open).
        if let Some(o) = overlay_for_close.upgrade() {
            o.set_settings_open(false);
            o.set_active_stack(SharedString::from(active_stack_label(
                &cfg_for_close.read(),
            )));
        }
    });

    // Phase E6 v26 — apply DWM per-pixel alpha so the frameless window's rounded
    // corners composite over the desktop (otherwise the corners show black).
    // make_transparent_tile = WS_EX_TOOLWINDOW + DWM blur-behind, NO click-
    // through (settings needs clicks). Review M1 — route through the stealth-
    // aware presenter so Settings, like tiles, never flashes on a screen-share
    // before WDA is applied; the DWM call is the `decorate` step (always runs).
    present_window_stealth_aware(&win, |hwnd| {
        let _ = make_transparent_tile(hwnd);
    });
    *settings_slot = Some(win);
}

/// Phase E6 v28 — after a profile import, refresh the token-status +
/// mic-opacity display so the user sees the new values, and return a
/// confirmation string for the result line.
pub(crate) fn msg_refresh_after_import(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) -> String {
    populate_token_status(win, cfg);
    "[ok] imported — restart binary for full effect".to_string()
}

/// P1.7 — compose the REDACTED server-import preview into the Settings props.
/// Each line is data-only (provider/url/model old->new + key PRESENCE as
/// "set"/"—"), built the same way the diagnostics `detail` strings are (shown
/// raw, not @tr'd). It NEVER carries a secret value — `preview_server_settings`
/// only ever fills booleans for keys, asserted by the redaction guard test.
pub(crate) fn apply_server_preview(
    win: &SettingsWindow,
    p: &overlay_backend::config::ServerSettingsPreview,
) {
    // "value" or "—" for an empty string; "set"/"—" for a presence bool.
    let v = |s: &str| {
        let t = s.trim();
        if t.is_empty() {
            "—".to_string()
        } else {
            t.to_string()
        }
    };
    let key = |present: bool| if present { "set" } else { "—" };
    let line = |g: &overlay_backend::config::PreviewGroup| -> String {
        // Mask the host in the URL portion (copyable text) — keeps scheme/port/
        // path, blanks the private LAN IP. Provider + model are non-secret.
        let url_old = overlay_backend::config::mask_host(&g.base_url_old);
        let url_new = overlay_backend::config::mask_host(&g.base_url_new);
        format!(
            "{}: provider {} -> {} | url {} -> {} | model {} -> {} | key {} -> {}",
            g.label,
            v(&g.provider_old),
            v(&g.provider_new),
            v(&url_old),
            v(&url_new),
            v(&g.model_old),
            v(&g.model_new),
            key(g.key_present_old),
            key(g.key_present_new),
        )
    };
    win.set_server_preview_cloud(SharedString::from(line(&p.cloud_ai)));
    win.set_server_preview_local(SharedString::from(line(&p.local_ai)));
    win.set_server_preview_vision(SharedString::from(line(&p.vision)));
    win.set_server_preview_stt(SharedString::from(line(&p.stt)));
    // GigaAM local model path: kept from THIS PC on apply. Show the incoming
    // path (masked is unnecessary — a filesystem path is not a secret, but it
    // IS machine-local) only when one side carries it, to keep the line useful.
    let gig = if p.gigaam_dir_incoming.trim().is_empty() && p.gigaam_dir_current.trim().is_empty() {
        String::new()
    } else {
        format!(
            "local GigaAM model path kept from this PC ({}); the imported file's path ({}) is NOT applied",
            v(&p.gigaam_dir_current),
            v(&p.gigaam_dir_incoming),
        )
    };
    win.set_server_preview_gigaam(SharedString::from(gig));
}

/// Push the multi-profile state into the Settings UI: the profile-name list, the
/// active index, and the active profile's context into the editor. Called on open
/// and after every add/select/rename/delete so the picker never drifts from cfg.
pub(crate) fn refresh_profiles(win: &SettingsWindow, c: &overlay_backend::config::Config) {
    let names: Vec<SharedString> = c
        .context_profiles
        .iter()
        .map(|p| SharedString::from(p.name.as_str()))
        .collect();
    win.set_profile_names(ModelRc::new(VecModel::from(names)));
    // Default to the first profile (0) when profiles exist but none is marked
    // active (e.g. after deleting the active one): otherwise the ComboBox bound
    // to -1 shows blank AND Rename/Delete stay disabled though selectable
    // profiles exist (audit #28). -1 only when there are no profiles at all.
    win.set_active_profile_index(match c.active_profile_index() {
        Some(i) => i as i32,
        None if !c.context_profiles.is_empty() => 0,
        None => -1,
    });
    win.set_meeting_context_input(SharedString::from(c.meeting_context.as_str()));
}

/// Populate the Settings window's token-status display properties
/// from the current `cfg`. Phase E6 — gives the user a way to SEE
/// whether ai_bearer / groq_api_key are configured without leaking
/// the values themselves (shows length + first 3 chars as fingerprint).
pub(crate) fn populate_token_status(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    // Phase E6 v18 — ASCII status prefixes ("[ok]" / "[--]") instead of
    // Unicode ✓ / ❌ which Slint+skia rendered as missing-glyph boxes
    // on the user's font fallback. Same root cause as the Close button
    // fix in settings_panel.slint and the quit chip fix in cycle 15.
    let c = cfg.read();
    let ai_status = if c.ai_bearer.is_empty() {
        "[--] not set".to_string()
    } else {
        let len = c.ai_bearer.chars().count();
        // #134: do NOT echo the key's leading chars into the UI — Settings is
        // captured on screen-share unless stealth is on. Show presence only.
        format!("[ok] set ({len} chars)")
    };
    let groq_status = if c.groq_api_key.is_empty() {
        "[--] not set".to_string()
    } else {
        let len = c.groq_api_key.chars().count();
        format!("[ok] set ({len} chars)")
    };
    win.set_ai_bearer_status(SharedString::from(ai_status));
    win.set_groq_api_key_status(SharedString::from(groq_status));
    // Phase E6 v20 — load tile opacity from config so the slider
    // reflects the saved value on Settings re-open.
    win.set_tile_body_opacity(c.tile_body_opacity);
    win.set_ai_base_url_input(SharedString::from(c.ai_base_url.clone()));
    // V4 — vision section: provider index + non-secret fields (bearers stay blank
    // on screen; saving a blank field is a no-op the user controls).
    win.set_vision_provider_index(match c.vision_provider.as_str() {
        "off" => 0,
        "same" => 1,
        "local" => 3,
        _ => 2,
    });
    win.set_vision_base_url_input(SharedString::from(c.vision_base_url.clone()));
    win.set_vision_model_input(SharedString::from(c.vision_model.clone()));
    win.set_vision_local_base_url_input(SharedString::from(c.vision_local_base_url.clone()));
    win.set_vision_local_model_input(SharedString::from(c.vision_local_model.clone()));
    win.set_vision_test_result(SharedString::from(""));
    win.set_ai_prompt_cache(c.ai_prompt_cache);
    win.set_ai_provider_index(i32::from(c.ai_provider == "local"));
    win.set_ai_local_base_url_input(SharedString::from(c.ai_local_base_url.clone()));
    // #E10.1 — seed both model dropdowns (cloud bridge + local) with the saved
    // model so each shows immediately; the full lists are fetched from
    // {base_url}/models AFTER the read guard is released (see end of fn).
    let seed_one = |saved: &str| -> ModelRc<SharedString> {
        let v: Vec<SharedString> = if saved.is_empty() {
            vec![]
        } else {
            vec![SharedString::from(saved)]
        };
        ModelRc::new(VecModel::from(v))
    };
    win.set_ai_models(seed_one(&c.ai_model));
    win.set_ai_model_index(0);
    win.set_ai_local_models(seed_one(&c.ai_local_model));
    win.set_ai_local_model_index(0);
    win.set_ai_local_vision(c.ai_local_vision);
    win.set_vision_phonetics(c.vision_phonetics);
    win.set_ai_local_thinking(c.ai_local_thinking);
    // Phase E10 — STT provider selector + local-engine fields.
    win.set_stt_provider_index(match c.stt_provider.as_str() {
        "gigaam" => 1,
        "whisper" => 2,
        _ => 0,
    });
    win.set_stt_gigaam_dir_input(SharedString::from(c.stt_gigaam_dir.clone()));
    win.set_stt_gigaam_gpu(c.stt_gigaam_gpu);
    win.set_stt_whisper_url_input(SharedString::from(c.stt_whisper_url.clone()));
    win.set_stt_whisper_bearer_input(SharedString::from(c.stt_whisper_bearer.clone()));
    win.set_stt_whisper_model_input(SharedString::from(c.stt_whisper_model.clone()));
    // Phase E6 v38 — reflect the saved interface language in the
    // Interface-tab dropdown (0=Русский, 1=English).
    win.set_ui_language_index(if c.ui_language == "en" { 1 } else { 0 });
    // Reflect the saved colour scheme in the Interface-tab dropdown, and seed
    // this Settings window's own Theme global so it opens already skinned.
    win.set_color_scheme_index(clamp_scheme(c.color_scheme));
    apply_scheme_settings(win, c.color_scheme);

    // #E10.1 — release the config read guard, THEN fetch the model lists
    // off-thread (the worker also reads cfg, so we must not hold the guard
    // across the spawn). Cloud list always (the bridge field is always
    // shown); local only when it's the active provider.
    let is_local = c.ai_provider == "local";
    drop(c);
    fetch_models(win.as_weak(), cfg.clone(), ModelTarget::Cloud);
    if is_local {
        fetch_models(win.as_weak(), cfg.clone(), ModelTarget::Local);
    }
}
