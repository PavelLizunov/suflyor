// Settings controller: the Settings window + every Settings-tab handler
// (Phase 7b -- the FINAL phase of the `overlay_host.rs` modularization; see
// `docs/overlay-host-modularization-plan.md` section 5.8/5.9).
//
// This module owns the host-side Settings surface, moved here VERBATIM:
//
// - `open_settings` -- the Settings controller, including its remaining inline
//   closures: the stealth toggle, the colour-scheme switch, the tile-opacity
//   slider, the Audio (mic) + Profile + Interface + Hotkeys tab handlers, the
//   full-PROFILE export / import, and the Interface-tab "Run setup wizard"
//   button (forwards to `open_wizard`). The per-domain tab clusters now live in
//   sibling modules and `open_settings` only CALLs each one: `wire_ai_settings`
//   (settings_ai.rs), `wire_stt_settings` (settings_stt.rs), `wire_vision_settings`
//   (settings_vision.rs), `wire_import_export` (settings_import_export.rs — the
//   server-only import/export), `wire_updates` (settings_updates.rs), and
//   `wire_local_ai` (settings_local_ai.rs);
// - the Settings helpers `msg_refresh_after_import` (also called by the PROFILE
//   import closure that stays here), `refresh_profiles`, `populate_token_status`
//   (the latter reaches `ModelTarget` + `fetch_models` — now in `settings_ai.rs`
//   — through the crate-root glob). `apply_server_preview` moved to
//   `settings_import_export.rs` with its only caller.
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
// - `apply_server_preview` (now in settings_import_export.rs) shows key PRESENCE
//   only and `mask_host`s the URLs.
//
// NOTE (section 7): this extraction imports the parent crate-root via
// `use super::*;` (the moved code reaches the Slint `SettingsWindow` /
// `OverlayBarWindow` types, the win32 helpers, the `WindowRegistry`,
// `present_window_stealth_aware` / `apply_scheme_settings` / `clamp_scheme` /
// `set_global_stealth`, `populate_diagnostics` / `build_diag_report`,
// `open_wizard`, `try_acquire_mic` / `release_mic`, and `active_stack_label`
// through it). That is intentional for the move; imports narrow in a later pass.
use super::{
    active_stack_label, ai, apply_scheme_bar, apply_scheme_settings, audio, clamp_scheme, config,
    drag_begin, drag_update, fetch_models, grab_hwnd, make_transparent_tile, open_wizard,
    parse_tile_monitor_pin, populate_diagnostics, present_window_stealth_aware,
    preset_for_tts_rate, set_always_on_top, set_global_scheme, set_global_stealth,
    set_global_tile_monitor, set_global_tile_opacity, set_stealth, spawn_ptt_watchdog, stt,
    try_acquire_mic, wire_ai_settings, wire_diagnostics, wire_import_export, wire_local_ai,
    wire_memory, wire_stt_settings, wire_updates, wire_vision_settings, wire_voice_settings, Arc,
    AtomicBool, ComponentHandle, ComponentRow, ModelRc, ModelTarget, Ordering, OverlayBarWindow,
    Rc, RefCell, SettingsWindow, SharedString, TileWindows, VecModel, WindowRegistry,
};

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
            populate_component_rows(existing, &snap);
        }
        let _ = existing.show();
        // The F8 / capture-chip flow hides EVERY app window via Win32 SW_HIDE
        // (hide_own_windows) so the overlay isn't in the screenshot. That bypasses
        // Slint, so the line above can no-op (Slint still thinks Settings is shown)
        // and the window would stay invisible with the gear stuck lit — the user's
        // "настройки активны, но окно пропало". Force it Win32-visible + to the
        // front so the gear ALWAYS recovers it.
        if let Ok(hwnd) = grab_hwnd(existing.window()) {
            slint_replay::win32::reveal_window(hwnd);
        }
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
        win.set_record_audio(snap.record_audio_enabled);
        win.set_auto_tiles_enabled(snap.auto_tiles_enabled);
        win.set_suppress_tiles(snap.suppress_tiles);
        win.set_trigger_keywords_input(SharedString::from(snap.trigger_keywords.as_str()));
        // v0.15.0 — seed the storage/retention controls. Days wins the display
        // when both bounds are set (hand-edited config); saving from the UI
        // then keeps exactly one bound active.
        let (mode, value) = if snap.record_retention_days > 0 {
            (2, snap.record_retention_days.to_string())
        } else if snap.record_retention_sessions > 0 {
            (1, snap.record_retention_sessions.to_string())
        } else {
            (0, "10".to_string()) // placeholder N for when the user switches mode
        };
        win.set_retention_mode(mode);
        win.set_retention_value(SharedString::from(value));
        win.set_journal_keep_value(SharedString::from(
            snap.journal_retention_sessions.to_string(),
        ));
        win.set_journal_mb_value(SharedString::from(snap.journal_max_total_mb.to_string()));

        // Onboarding «Компоненты» — readiness rows, refreshed on EVERY open
        // (see populate_component_rows; also called on the reused-window path).
        populate_component_rows(&win, &snap);
    }

    // Onboarding «Компоненты» — inline install for the LIGHT, single-call
    // installers (voices / OCR). The heavy ones (engine / model / STT) keep their
    // dedicated panels with progress + cancel, so they have no inline button.
    // Wired ONCE on the fresh window (the callback persists across reopens).
    {
        let weak = win.as_weak();
        let cfg_inst = cfg.clone();
        win.on_component_install(move |idx| {
            let Some(w) = weak.upgrade() else {
                return;
            };
            if w.get_component_busy_index() != -1 {
                return; // one inline install at a time
            }
            w.set_component_busy_index(idx);
            w.set_component_busy_phase(1); // preparing
            w.set_component_busy_label(SharedString::from(""));
            let weak_done = w.as_weak();
            let cfg_t = cfg_inst.clone();
            std::thread::spawn(move || {
                // Row order from components::status(): 0=engine 1=model 2=stt
                // 3=voices 4=ocr. Only 3/4 are inline-installable here.
                let result: std::result::Result<(), String> = match idx {
                    3 => {
                        let cancel = std::sync::atomic::AtomicBool::new(false);
                        let weak_cb = weak_done.clone();
                        let on = move |p: overlay_backend::tts_install::VoiceProgress| {
                            use overlay_backend::tts_install::VoiceProgress;
                            let (phase, label): (i32, String) = match p {
                                VoiceProgress::Downloading(l) => (2, l),
                                VoiceProgress::Verifying(l) => (3, l),
                                VoiceProgress::Unpacking(l) => (4, l),
                                VoiceProgress::AlreadyInstalled(l) => (5, l),
                                VoiceProgress::AllInstalled => (6, String::new()),
                                VoiceProgress::PartiallyInstalled(f) => (7, f),
                                VoiceProgress::PackFailed(l) => (9, l),
                            };
                            let weak_in = weak_cb.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_in.upgrade() {
                                    w.set_component_busy_phase(phase);
                                    w.set_component_busy_label(SharedString::from(label));
                                }
                            });
                        };
                        overlay_backend::tts_install::install_voices(&cancel, &on)
                            .map_err(|e| format!("{e:#}"))
                    }
                    4 => {
                        let weak_cb = weak_done.clone();
                        let on = move |p: overlay_backend::ocr_install::OcrProgress| {
                            use overlay_backend::ocr_install::OcrProgress;
                            let phase: i32 = match p {
                                OcrProgress::Downloading => 10,
                                OcrProgress::Verifying => 11,
                                OcrProgress::Unpacking => 12,
                                OcrProgress::AlreadyInstalled => 13,
                                OcrProgress::Installed => 14,
                            };
                            let weak_in = weak_cb.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = weak_in.upgrade() {
                                    w.set_component_busy_phase(phase);
                                }
                            });
                        };
                        overlay_backend::ocr_install::install(&on).map_err(|e| format!("{e:#}"))
                    }
                    _ => Ok(()),
                };
                if let Err(e) = &result {
                    // Log detail locally; the UI line stays generic (screen-shareable).
                    diag!("[overlay-host] component install (idx={idx}) failed: {e}");
                }
                let failed = result.is_err();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(w) = weak_done.upgrade() else {
                        return;
                    };
                    if failed {
                        // Keep the row marked busy so the generic message shows; the
                        // next Settings open clears it (populate resets busy-index).
                        w.set_component_busy_phase(8); // generic failure
                    } else {
                        // Refresh from live state — the just-installed row goes green.
                        let snap = cfg_t.read();
                        populate_component_rows(&w, &snap);
                    }
                });
            });
        });
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
                let mic_guard = try_acquire_mic();
                let msg = if mic_guard.is_none() {
                    "[!] mic busy — close PTT / dictation and retry".to_string()
                } else {
                    let result = overlay_backend::audio::record_mic_blocking(3000, device);
                    drop(mic_guard); // release before processing (RAII: also on panic)
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

    // ===== AI: cloud bridge + local server (provider switch, token/url/model
    // saves, dropdown refresh, prompt-cache toggle, bridge + local tests) =====
    // Extracted to settings_ai.rs (P1 domain split) — wired verbatim there
    // (`ModelTarget` + `fetch_models` moved with it). The install/updater
    // closures below stay in open_settings (separate later wave).
    wire_ai_settings(&win, cfg);

    // ===== V4 — vision (screenshot) channel: provider switch + field saves + test =====
    // Extracted to settings_vision.rs (P1 domain split) — wired verbatim there.
    wire_vision_settings(&win, cfg);

    // ===== Read-aloud (Озвучка): voice chooser + speed preset + test =====
    wire_voice_settings(&win, cfg);

    // ===== Local AI: one-click in-app installer (download pipeline + Cancel) =====
    // Extracted to settings_local_ai.rs (P1 domain split) — wired verbatim there.
    // SECURITY: download -> verify -> spawn stays in overlay_backend::local_ai;
    // this only CALLs install(); the sequence is byte-for-byte unchanged.
    wire_local_ai(&win, cfg, state, overlay_weak);

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

    // Tile-placement monitor picker. Index 0 = Auto (pick_monitor); index i>0 =
    // the (i-1)-th live monitor. Persists its top-left signature (survives a
    // monitor reorder) and updates the process-global so every future tile
    // honours the choice immediately.
    {
        let cfg_m = cfg.clone();
        win.on_tile_monitor_changed(move |idx| {
            let pin = if idx <= 0 {
                None
            } else {
                slint_replay::win32::enum_monitors()
                    .get((idx - 1) as usize)
                    .map(|m| (m.left, m.top))
            };
            // Apply to the runtime FIRST so new tiles honour the choice even if
            // persisting config later fails — the dropdown never lies about where
            // tiles will spawn. Saving config.json is best-effort.
            set_global_tile_monitor(pin);
            {
                let mut c = cfg_m.write();
                c.tile_monitor_name = pin.map(|(l, t)| format!("{l},{t}"));
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] tile_monitor save failed: {e:#}");
                }
            }
            eprintln!("[overlay-host] tile_monitor -> {pin:?}");
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
        let win_lang = win.as_weak();
        win.on_language_selected(move |idx| {
            let lang = if idx == 1 { "en" } else { "ru" };
            match slint::select_bundled_translation(lang) {
                Ok(()) => eprintln!("[overlay-host] UI language -> {lang}"),
                Err(e) => eprintln!("[overlay-host] language {lang} not available: {e}"),
            }
            {
                let mut c = cfg_lang.write();
                c.ui_language = lang.to_string();
                if let Err(e) = overlay_backend::config::save(&c) {
                    eprintln!("[overlay-host] ui_language save failed: {e:#}");
                }
            }
            // The @tr() bindings re-evaluate automatically; the Rust-built monitor
            // dropdown labels do not — rebuild them in the new language.
            if let Some(w) = win_lang.upgrade() {
                populate_tile_monitors(&w, &cfg_lang.read());
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

    // P0 — Diagnostics tab owns its callbacks (diagnostics.rs); Settings only wires.
    wire_diagnostics(&win, cfg);

    // ===== STT (speech-to-text): provider switch + GigaAM GPU + field saves + test =====
    // Extracted to settings_stt.rs (P1 domain split) — wired verbatim there.
    // (`on_stt_gigaam_gpu_changed` moved out of the AI-local block region above
    // into wire_stt_settings; the mic device/test callbacks stay in open_settings.)
    wire_stt_settings(&win, cfg);

    // ===== 💭 Memory (Phase 3b.3): curated personal memory review =====
    // Pending candidates (approve/reject) + approved items (delete) over the
    // SQLite memory tables, + Extract (heuristic over recent sessions).
    // Extracted to settings_memory.rs.
    wire_memory(&win);

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
                .set_title("Export suflyor settings (contains API keys)")
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
                .set_title("Import suflyor settings")
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

    // ===== Server settings import/export (server-only export, two-step import
    // preview, Apply, Cancel) =====
    // Extracted to settings_import_export.rs (P1 domain split) — wired verbatim
    // there (`apply_server_preview` moved with it; the shared
    // `pending_server_import` cell is threaded in). The PROFILE export/import
    // (above) + `refresh_profiles` + `msg_refresh_after_import` STAY here.
    wire_import_export(&win, cfg, &pending_server_import);

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
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] profile select save failed: {e:#}");
            }
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
            let added = c.add_profile(name.as_str()).is_some();
            let saved = if added {
                overlay_backend::config::save(&c)
            } else {
                Ok(())
            };
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if !added {
                    "[--] пустое или занятое имя"
                } else if let Err(e) = &saved {
                    eprintln!("[overlay-host] profile add save failed: {e:#}");
                    "[err] не удалось сохранить — проверьте диск"
                } else {
                    "[ok] профиль добавлен"
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
            let saved = if ok {
                overlay_backend::config::save(&c)
            } else {
                Ok(())
            };
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(if !ok {
                    "[--] пустое или занятое имя"
                } else if let Err(e) = &saved {
                    eprintln!("[overlay-host] profile rename save failed: {e:#}");
                    "[err] не удалось сохранить — проверьте диск"
                } else {
                    "[ok] переименовано"
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
            // Confirm-without-verify was the v0.18 "Готово when not done" class:
            // report success ONLY if the save actually persisted, else the profile
            // reappears on next launch with a green status (audit Q3).
            let saved = overlay_backend::config::save(&c);
            if let Some(w) = weak.upgrade() {
                refresh_profiles(&w, &c);
                w.set_meeting_context_result(SharedString::from(match &saved {
                    Ok(()) => "[ok] профиль удалён",
                    Err(e) => {
                        eprintln!("[overlay-host] profile delete save failed: {e:#}");
                        "[err] не удалось сохранить — проверьте диск"
                    }
                }));
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
    // v0.13.0 — raw-audio recording toggle + "open recordings folder". The tee
    // in slint_session reads record_audio_enabled at the NEXT session start, so
    // a change applies to the next session (an in-flight recording keeps going).
    {
        let cfg_c = cfg.clone();
        win.on_record_audio_changed(move |on| {
            let mut c = cfg_c.write();
            c.record_audio_enabled = on;
            let _ = overlay_backend::config::save(&c);
        });
    }
    // v0.15.0 — recordings retention policy. The prune runs at the NEXT session
    // start (recorder::start), so a change applies from the next call on.
    // Invalid / partial numeric input is ignored — the config keeps its last
    // valid value (the field re-seeds to canonical on the next Settings open).
    {
        let cfg_c = cfg.clone();
        win.on_retention_changed(move |mode, value| {
            let n: Option<u32> = value.trim().parse().ok();
            let mut c = cfg_c.write();
            match mode {
                0 => {
                    c.record_retention_sessions = 0;
                    c.record_retention_days = 0;
                }
                1 => match n.filter(|n| *n > 0) {
                    Some(n) => {
                        c.record_retention_sessions = n;
                        c.record_retention_days = 0;
                    }
                    None => return,
                },
                2 => match n.filter(|n| *n > 0) {
                    Some(n) => {
                        c.record_retention_days = n;
                        c.record_retention_sessions = 0;
                    }
                    None => return,
                },
                _ => return,
            }
            let _ = overlay_backend::config::save(&c);
        });
    }
    // v0.15.0 — journal (archive transcripts) retention: count + MB cap, both
    // 0 = unlimited. Applied at the next session start (journal open prune).
    {
        let cfg_c = cfg.clone();
        win.on_journal_retention_changed(move |keep, mb| {
            let (Ok(keep), Ok(mb)) = (keep.trim().parse::<u32>(), mb.trim().parse::<u32>()) else {
                return; // partial / non-numeric input — keep the last valid value
            };
            let mut c = cfg_c.write();
            c.journal_retention_sessions = keep;
            c.journal_max_total_mb = mb;
            let _ = overlay_backend::config::save(&c);
        });
    }
    {
        win.on_open_recordings_clicked(move || {
            match overlay_backend::recorder::recordings_dir() {
                Ok(dir) => {
                    // Create it first so the folder opens even before the first
                    // recording exists. Explorer launch is fire-and-forget.
                    let _ = std::fs::create_dir_all(&dir);
                    if let Err(e) = std::process::Command::new("explorer").arg(&dir).spawn() {
                        eprintln!("[overlay-host] open recordings folder failed: {e}");
                    }
                }
                Err(e) => eprintln!("[overlay-host] cannot resolve recordings dir: {e:#}"),
            }
        });
    }
    {
        // The config.json hint path is clickable → open the data folder
        // (%APPDATA%\suflyor) in Explorer. Mirrors the recordings opener;
        // create_dir_all so it works before the first save. NOTE: Explorer is a
        // normal window (not stealth-excluded) — fine for config-time use.
        win.on_open_data_folder_clicked(move || match overlay_backend::paths::data_root() {
            Some(dir) => {
                let _ = std::fs::create_dir_all(&dir);
                if let Err(e) = std::process::Command::new("explorer").arg(&dir).spawn() {
                    eprintln!("[overlay-host] open data folder failed: {e}");
                }
            }
            None => eprintln!("[overlay-host] cannot resolve data folder"),
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
        win.on_suppress_tiles_changed(move |on| {
            let mut c = cfg_c.write();
            c.suppress_tiles = on;
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
            let Some(mic_guard) = try_acquire_mic() else {
                w.set_meeting_context_result(SharedString::from("[--] микрофон занят"));
                return;
            };
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
                drop(mic_guard); // M2 — free the mic before transcription (RAII: also released on a record-thread panic)
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

    // ===== Updates (GitHub check + download-then-run installer) =====
    // Extracted to settings_updates.rs (P1 domain split) — wired verbatim there.
    // SECURITY: download -> verify -> spawn stays in overlay_backend::update;
    // this only CALLs it; the sequence is byte-for-byte unchanged.
    wire_updates(&win);

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

/// Push the onboarding «Компоненты» readiness rows (engine / model / STT /
/// voices / OCR) into the Settings UI, built from the backend's single source of
/// truth so the hub can never disagree with the real install state. Called on
/// EVERY Settings open — both the fresh-window seed AND the reused-window
/// refresh path — so the status reflects reality after the user installs a
/// component (mirrors populate_token_status / refresh_profiles). RU names + a
/// where-to-install hint are dynamic content, like the archive labels.
pub(crate) fn populate_component_rows(
    win: &SettingsWindow,
    snap: &overlay_backend::config::Config,
) {
    use overlay_backend::components::{status, ComponentKind};
    // Clear any stuck inline-install busy state so a reopened window never shows
    // a stale "Устанавливается…"/error from a previous open.
    win.set_component_busy_index(-1);
    let rows: Vec<ComponentRow> = status(snap)
        .into_iter()
        .map(|c| {
            // Light, single-call installers wired inline in the hub.
            let installable = matches!(c.kind, ComponentKind::Voices | ComponentKind::Ocr);
            // «Открыть» jump target for the heavy components (their full installer
            // with progress + cancel lives in a dedicated panel); -1 = inline.
            let jump_tab: i32 = match c.kind {
                ComponentKind::Engine | ComponentKind::LocalModel => 11, // AI мост
                ComponentKind::Stt => 12,                                // STT
                ComponentKind::Voices | ComponentKind::Ocr => -1,
            };
            let (name, hint) = match c.kind {
                ComponentKind::Engine => (
                    "Движок (llama.cpp)",
                    "Настройки → AI → AI мост → «Обновить движок»",
                ),
                ComponentKind::LocalModel => (
                    "Локальная модель ИИ",
                    "Настройки → AI → AI мост → «Установить локальный ИИ»",
                ),
                ComponentKind::Stt => ("Распознавание речи (STT)", "Настройки → AI → STT"),
                ComponentKind::Voices => {
                    ("Голоса (TTS)", "Настройки → AI → TTS → «Установить голоса»")
                }
                ComponentKind::Ocr => (
                    "Распознавание текста (OCR)",
                    "Настройки → AI → AI мост → Vision → «Установить OCR»",
                ),
            };
            ComponentRow {
                name: SharedString::from(name),
                detail: SharedString::from(c.detail.as_str()),
                installed: c.installed,
                hint: SharedString::from(hint),
                installable,
                jump_tab,
            }
        })
        .collect();
    win.set_component_rows(ModelRc::new(VecModel::from(rows)));
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

/// Build the tile-placement monitor dropdown from the LIVE displays. Item 0 =
/// Auto; the saved pin (`cfg.tile_monitor_name`) preselects its monitor when it
/// still matches a current display. Labels are language-dependent
/// (`cfg.ui_language`) — call this on Settings open AND on a live language
/// switch. The `.slint` card hides itself when there is only one display.
pub(crate) fn populate_tile_monitors(win: &SettingsWindow, c: &overlay_backend::config::Config) {
    let monitors = slint_replay::win32::enum_monitors();
    let ru = c.ui_language == "ru";
    let mon_word = if ru { "Монитор" } else { "Monitor" };
    let primary_tag = if ru {
        " · основной"
    } else {
        " · primary"
    };
    let mut labels: Vec<SharedString> = vec![SharedString::from(if ru {
        "Авто (рекомендуется)"
    } else {
        "Auto (recommended)"
    })];
    let pin = c
        .tile_monitor_name
        .as_deref()
        .and_then(parse_tile_monitor_pin);
    let mut sel: i32 = 0;
    for (i, m) in monitors.iter().enumerate() {
        let tag = if m.is_primary { primary_tag } else { "" };
        labels.push(SharedString::from(format!(
            "{} {} · {}×{}{}",
            mon_word,
            i + 1,
            m.width(),
            m.height(),
            tag
        )));
        if pin == Some((m.left, m.top)) {
            sel = (i + 1) as i32;
        }
    }
    win.set_tile_monitors(ModelRc::new(VecModel::from(labels)));
    win.set_tile_monitor_index(sel);
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
    // Tile-placement monitor dropdown — rebuilt here AND on a live language
    // switch (Rust-built labels don't auto-refresh like @tr bindings do).
    populate_tile_monitors(win, &c);
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
    // OCR engine (Tesseract) install state — re-checked each open (the data dir
    // is user-writable + the installer button writes it). Transient install
    // status reset so a stale "Готово…" can't survive a reopen.
    win.set_ocr_installed(overlay_backend::ocr_install::is_installed());
    win.set_ocr_installing(false);
    win.set_ocr_install_phase(0);
    // UI-audit 2026-06-13 (CRITICAL): the Settings window is REUSED, not
    // recreated, so every transient one-shot status string survived from the
    // previous open and described a STALE action (the user caught a lingering
    // "Готово: умная модель (12B)…" after a model switch). `populate_token_status`
    // runs on every open incl. reopen, so clear them ALL here — only
    // `vision_test_result` was reset before. Persistent state (key presence,
    // dropdowns, toggles, ai_local_quality) is reseeded below and is NOT touched.
    let blank = || SharedString::from("");
    win.set_quality_status(blank());
    win.set_local_ai_status(blank());
    win.set_local_ai_gpu(blank());
    win.set_local_ai_on_gpu(false);
    win.set_ai_local_test_result(blank());
    win.set_ai_bridge_test_result(blank());
    win.set_stt_test_result(blank());
    win.set_mic_test_result(blank());
    win.set_meeting_context_result(blank());
    win.set_profile_io_result(blank());
    win.set_server_preview_ready(false);
    win.set_update_status(blank());
    // Q2 — clear the updater's transient state too, else the "⬇ Update now"
    // button + its stale download URL survive a reopen while the status text was
    // already cleared above (reused-window state-truth gap). The user just
    // re-checks; nothing persists a half-finished check across opens.
    win.set_update_available(false);
    win.set_update_download_url(blank());
    win.set_update_checking(false);
    win.set_memory_status(blank());
    // «Компоненты» tab — the per-row install progress (busy index + status text)
    // is transient (set by the install worker). Reset BOTH so a reopened Settings
    // can't show a stale "Подготовка…" on a row whose install already finished
    // (the project's #1 reused-window stale-status class — caught by
    // settings_reset_guard).
    win.set_component_busy_phase(0);
    win.set_component_busy_label(blank());
    win.set_component_busy_index(-1);
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
    win.set_vision_test_practice(c.vision_test_practice);
    // Read-aloud (Озвучка): build the installed-voice dropdown + reflect the
    // saved voice/speed. The neural voices live in `%APPDATA%\suflyor\tts`;
    // `tts::voices()` scans them (empty until the user installs one → the panel
    // shows a "no voices" hint and disables the Test button).
    {
        let voices = overlay_backend::tts::voices();
        let names: Vec<SharedString> = voices
            .iter()
            .map(|v| SharedString::from(v.name.as_str()))
            .collect();
        // Show the voice the ENGINE actually resolves to, not blindly voices[0]:
        // for an empty/uninstalled `tts_voice`, `pick_voice_id` mirrors the
        // sidecar's preference (Irina → any Piper → any RU → first), so the
        // dropdown label matches the voice that Test / read-aloud will play.
        let vidx = overlay_backend::tts::pick_voice_id(&voices, &c.tts_voice)
            .and_then(|id| voices.iter().position(|v| v.id == id))
            .unwrap_or(0) as i32;
        win.set_tts_available(!voices.is_empty());
        win.set_tts_voice_names(ModelRc::new(VecModel::from(names)));
        win.set_tts_voice_index(vidx);
        win.set_tts_rate_index(preset_for_tts_rate(c.tts_rate));
        // Reset the transient install state on (re)open — the Settings window is
        // reused, so a leftover "Готово…" / progress string must not survive.
        win.set_tts_installing(false);
        win.set_tts_install_phase(0);
        win.set_tts_install_label(SharedString::from(""));
    }
    win.set_ai_local_thinking(c.ai_local_thinking);
    // v0.18.0 — local model size choice + whether the 12B is downloaded yet.
    win.set_ai_local_quality(c.ai_local_quality);
    {
        let root = overlay_backend::local_ai::default_root();
        win.set_quality_model_present(overlay_backend::local_ai::quality_model_present(&root));
        // v0.18.2 — 12B vision projector state + engine build (drives the
        // download-projector button / "update engine first" hint / Installed: bNNNN).
        win.set_quality_vision_present(overlay_backend::local_ai::quality_vision_present(&root));
        win.set_quality_vision_supported(overlay_backend::local_ai::quality_vision_supported(
            &root,
        ));
        win.set_engine_build(SharedString::from(
            overlay_backend::local_ai::installed_engine_build(&root)
                .map(|b| format!("b{b}"))
                .unwrap_or_default(),
        ));
        win.set_vision12b_status(blank());
        win.set_engine_update_status(blank());
    }
    // Phase E10 — STT provider selector + local-engine fields.
    win.set_stt_provider_index(match c.stt_provider.as_str() {
        "gigaam" => 1,
        "whisper" => 2,
        _ => 0,
    });
    // Recognition language (stt_language): None=auto → 0, "ru" → 1, "en" → 2.
    win.set_stt_language_index(match c.stt_language.as_deref() {
        Some("ru") => 1,
        Some("en") => 2,
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
