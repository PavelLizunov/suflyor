//! First-run guided setup wizard (Phase 4 of the `overlay_host.rs`
//! modularization — see `docs/overlay-host-modularization-plan.md` §5.4).
//!
//! This module owns the 7-step setup wizard: `open_wizard` (created on demand
//! like `open_text_ask` — centred + WDA-stealthed via
//! `present_window_stealth_aware`), `wire_wizard_steps` (the per-step nav +
//! live AI/STT/mic/sys checks, the "Install local AI" / "Open diagnostics"
//! shortcuts, and the in-window stealth toggle routed through the shared
//! `WindowRegistry`), and the wizard-only `refill_wizard_summary` (the step-7
//! summary refill — secret-free, never the `ReadinessItem.detail`).
//!
//! The per-step live checks REUSE the backend test fns directly
//! (`ai::test_connection` via the resolver, `stt::test_connection_backend`,
//! `audio::record_mic_blocking` / `play_tone_and_capture`) on throwaway threads;
//! they do NOT pull in the Diagnostics-tab internals. `WindowRegistry` +
//! `apply_scheme_wizard` + `global_scheme` / `global_stealth` /
//! `set_global_stealth` are `pub(crate)` in `window_lifecycle.rs` and reach this
//! module through the parent glob, as do the shared `try_acquire_mic` /
//! `release_mic` mic-guard pair (which stays in `overlay_host.rs` — it is used
//! by a dozen non-wizard sites too).
//!
//! Callers that STAY in `overlay_host.rs` — `main`'s 2200 ms Timer that calls
//! `open_wizard` on first run, and `open_settings`' "Run setup wizard" button —
//! resolve through the `use wizard::*;` re-export at crate root.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    apply_scheme_wizard, config, focus_window, global_scheme, global_stealth, grab_hwnd,
    present_window_stealth_aware, release_mic, set_global_stealth, set_skip_taskbar, set_stealth,
    try_acquire_mic, ComponentHandle, OverlayBarWindow, Rc, RefCell, SettingsWindow, SharedString,
    WindowRegistry, WizardWindow,
};

/// Refill the step-7 summary rows. Renders ONLY secret-free values: the live
/// check detail a step already painted ([ok]/[err], secret-free, same as the
/// Diagnostics tab) or a status WORD from `readiness().configured`. NEVER the
/// `ReadinessItem.detail` — for cloud AI it embeds the bridge LAN IP
/// (`ai_base_url`), which must not reach this screen-shareable summary panel
/// (security invariant). Called whenever step 7 is reached — via Next OR Skip —
/// so a fully-skipped run still shows a populated summary.
pub(crate) fn refill_wizard_summary(w: &WizardWindow, cfg: &overlay_backend::config::SharedConfig) {
    let r = cfg.read().readiness();
    let pick = |live: SharedString, configured: bool| -> SharedString {
        if !live.is_empty() {
            live
        } else if configured {
            SharedString::from("configured")
        } else {
            SharedString::from("—")
        }
    };
    w.set_summary_ai(pick(w.get_ai_detail(), r.ai.configured));
    w.set_summary_stt(pick(w.get_stt_detail(), r.stt.configured));
    w.set_summary_mic(pick(w.get_mic_detail(), r.mic.configured));
    w.set_summary_sys(pick(w.get_sys_detail(), r.sys.configured));
}

/// Wire all 7 wizard steps. Each per-step check REUSES the Diagnostics backend
/// fns (see `on_diagnostics_check_all_clicked`) on a throwaway thread, reporting
/// a level int (-1 checking · 0 ok · 4 needs-attention) + a secret-free detail.
/// An error NEVER blocks Next. Every closure is panic-free (no unwrap/expect).
#[allow(clippy::too_many_arguments)]
pub(crate) fn wire_wizard_steps(
    win: &WizardWindow,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    // Phase 1 (§5.1) — the wizard's in-window stealth toggle now re-stealths
    // EVERY open window (tiles / palette / text_ask / Settings / 🆘 help /
    // recover-offer) through `registry.apply_stealth`, replacing the previous
    // per-window clones + loop (palette_ref / text_ask_ref / help_ref /
    // recover_offer_ref / tiles are all reached via the registry now).
    registry: &WindowRegistry,
) {
    // nav-next: advance + auto-run the NEW step's check; refill summary at step 7.
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_nav_next(move || {
            let Some(w) = weak.upgrade() else { return };
            let n = (w.get_step() + 1).min(6);
            w.set_step(n);
            match n {
                1 => w.invoke_ai_test_clicked(),
                2 => w.invoke_stt_test_clicked(),
                3 => w.invoke_mic_test_clicked(),
                4 => w.invoke_sys_test_clicked(),
                6 => refill_wizard_summary(&w, &cfg_c),
                _ => {}
            }
        });
    }
    {
        let weak = win.as_weak();
        win.on_nav_back(move || {
            if let Some(w) = weak.upgrade() {
                w.set_step((w.get_step() - 1).max(0));
            }
        });
    }
    {
        let weak = win.as_weak();
        let cfg_c = cfg.clone();
        win.on_nav_skip(move || {
            if let Some(w) = weak.upgrade() {
                w.set_step((w.get_step() + 1).min(6));
                // Refill the summary if Skip lands on step 7 too — otherwise a
                // fully-skipped run shows blank rows (caught in live testing).
                if w.get_step() == 6 {
                    refill_wizard_summary(&w, &cfg_c);
                }
            }
        });
    }

    // Step 1: mode → write provider fields + save (this CREATES config.json).
    {
        let cfg_c = cfg.clone();
        win.on_mode_selected(move |m| {
            let mut c = cfg_c.write();
            match m {
                0 => {
                    c.ai_provider = "cloud".into();
                    c.stt_provider = "cloud".into();
                    c.vision_provider = "cloud".into();
                }
                1 => {
                    c.ai_provider = "local".into();
                    // Prefer GigaAM when it's configured (its model dir is set) —
                    // it's the stronger local STT and avoids defaulting to a
                    // whisper-server the user may not be running. Same "configured"
                    // test the diagnostics readiness() uses. Fall back to whisper.
                    c.stt_provider = if c.stt_gigaam_dir.trim().is_empty() {
                        "whisper".into()
                    } else {
                        "gigaam".into()
                    };
                    c.vision_provider = "local".into();
                }
                _ => {} // Mixed: leave provider fields as-is.
            }
            if let Err(e) = overlay_backend::config::save(&c) {
                eprintln!("[overlay-host] wizard mode save failed: {e:#}");
            }
        });
    }

    // Step 2: AI — REUSE ai::test_connection via the RESOLVER (security: never
    // the raw ai_base_url — ai_endpoint(false) picks local vs cloud).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_ai_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_ai_level(-1);
            w.set_ai_detail(SharedString::from(""));
            let (base, bearer, model) = {
                let c = cfg_c.read();
                let e = c.ai_endpoint(false);
                (e.base_url, e.bearer, e.model)
            };
            let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::ai::test_connection(base, bearer, model))
                        {
                            Ok(s) => (0, format!("[ok] {s}")),
                            Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                        }
                    }
                    Err(e) => (4, format!("[err] runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = wr.upgrade() {
                        w.set_ai_level(lvl);
                        w.set_ai_detail(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Step 3: STT — REUSE stt::test_connection_backend.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_stt_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_stt_level(-1);
            w.set_stt_detail(SharedString::from(""));
            let backend = cfg_c.read().stt_backend();
            let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => {
                        match rt.block_on(overlay_backend::stt::test_connection_backend(&backend)) {
                            Ok(s) => (0, format!("[ok] {s}")),
                            Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                        }
                    }
                    Err(e) => (4, format!("[err] runtime: {e}")),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = wr.upgrade() {
                        w.set_stt_level(lvl);
                        w.set_stt_detail(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Step 4: mic — REUSE record_mic_blocking + rms_dbfs WITH the single-mic
    // guard (so a wizard probe during an active session reports busy, not a
    // device fight). 3s blocking record on a throwaway thread.
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_mic_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_mic_level(-1);
            w.set_mic_detail(SharedString::from("recording 3s…"));
            let dev = cfg_c.read().mic_device.clone();
            let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) = if !try_acquire_mic() {
                    (
                        4,
                        "[!] mic busy — close PTT / dictation and retry".to_string(),
                    )
                } else {
                    let r = overlay_backend::audio::record_mic_blocking(3000, dev);
                    release_mic();
                    match r {
                        Ok(s) if s.is_empty() => (4, "[!] no audio captured".to_string()),
                        Ok(s) => {
                            let d = overlay_backend::audio::rms_dbfs(&s);
                            if d >= -45.0 {
                                (0, format!("[ok] heard you ({d:.0} dBFS)"))
                            } else {
                                (0, format!("[ok] capture works · quiet ({d:.0} dBFS)"))
                            }
                        }
                        Err(e) => (4, format!("[err] {e}").chars().take(80).collect()),
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = wr.upgrade() {
                        w.set_mic_level(lvl);
                        w.set_mic_detail(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Step 5: system audio — REUSE play_tone_and_capture (the diag sys-phase
    // self-test: play a tone through the default output, hear it on loopback).
    {
        let cfg_c = cfg.clone();
        let weak = win.as_weak();
        win.on_sys_test_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_sys_level(-1);
            w.set_sys_detail(SharedString::from(""));
            let dev = cfg_c.read().system_audio_device.clone();
            let wr = w.as_weak();
            std::thread::spawn(move || {
                let (lvl, msg): (i32, String) =
                    match overlay_backend::audio::play_tone_and_capture(dev) {
                        Ok(s) => {
                            let d = overlay_backend::audio::rms_dbfs(&s);
                            if d > -60.0 {
                                (
                                    0,
                                    format!("[ok] loopback heard the test tone ({d:.0} dBFS)"),
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
                    if let Some(w) = wr.upgrade() {
                        w.set_sys_level(lvl);
                        w.set_sys_detail(SharedString::from(msg));
                    }
                });
            });
        });
    }

    // Steps 2/6: "Install local AI" / "Open diagnostics" — dismiss the wizard
    // first (it's always-on-top, so otherwise the re-shown Settings opens BEHIND
    // it and looks like nothing happened), then open Settings on the right tab.
    {
        let weak = win.as_weak();
        let ow = overlay_weak.clone();
        let set = settings_ref.clone();
        win.on_install_local_clicked(move || {
            if let Some(w) = weak.upgrade() {
                w.invoke_cancelled();
            }
            if let Some(o) = ow.upgrade() {
                o.invoke_open_settings_clicked();
            }
            // 🧠 AI bridge tab (index 11) — the local-AI installer lives there.
            if let Some(sw) = set.borrow().as_ref() {
                sw.set_active_tab(11);
            }
        });
    }
    {
        let weak = win.as_weak();
        let ow = overlay_weak.clone();
        let set = settings_ref.clone();
        win.on_open_diagnostics(move || {
            if let Some(w) = weak.upgrade() {
                w.invoke_cancelled();
            }
            if let Some(o) = ow.upgrade() {
                o.invoke_open_settings_clicked();
            }
            // 🩺 Diagnostics tab (index 13) + auto-run the readiness check.
            if let Some(sw) = set.borrow().as_ref() {
                sw.set_active_tab(13);
                sw.invoke_diagnostics_check_all_clicked();
            }
        });
    }

    // Step 6: stealth — the SAME global stealth path as on_stealth_toggle_clicked,
    // routed through the shared `WindowRegistry` so the wizard toggle covers every
    // open window (incl. the wizard itself, which is in the registry). The bar is
    // driven inline (it also drops its taskbar button under stealth).
    {
        let ow = overlay_weak.clone();
        let registry_c = registry.clone();
        let cfg_c = cfg.clone();
        let state_c = state.clone();
        win.on_stealth_toggled(move |on| {
            {
                let mut st = match state_c.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.stealth = on;
            }
            set_global_stealth(on);
            {
                let mut c = cfg_c.write();
                c.stealth_enabled = on;
                let _ = config::save(&c);
            }
            if let Some(o) = ow.upgrade() {
                o.set_stealth_active(on);
                if let Ok(h) = grab_hwnd(o.window()) {
                    let _ = set_stealth(h, on);
                    let _ = set_skip_taskbar(h, on);
                }
            }
            // All other open windows (incl. this wizard) via the single path.
            registry_c.apply_stealth(on);
        });
    }
}

/// V0.8.4 — first-run guided setup. Created on demand like `open_text_ask`:
/// centred + WDA-stealthed via `present_window_stealth_aware`, keyboard-focused.
/// Re-opening while it's up just re-focuses the existing window.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_wizard(
    slot_ref: &Rc<RefCell<Option<WizardWindow>>>,
    cfg: &overlay_backend::config::SharedConfig,
    state: &slint_replay::app_state::SharedState,
    overlay_weak: &slint::Weak<OverlayBarWindow>,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    // Phase 1 (§5.1) — forwarded to wire_wizard_steps so the wizard's stealth
    // toggle re-stealths every open window through the shared registry (tiles /
    // palette / text_ask / 🆘 help / recover-offer are all reached via it).
    registry: &WindowRegistry,
) {
    {
        let slot = slot_ref.borrow();
        if let Some(existing) = slot.as_ref() {
            let _ = existing.show();
            if let Ok(hwnd) = grab_hwnd(existing.window()) {
                focus_window(hwnd);
            }
            return;
        }
    }
    let win = match WizardWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] WizardWindow::new failed: {e}");
            return;
        }
    };
    apply_scheme_wizard(&win, global_scheme());
    win.set_stealth_on(global_stealth());
    wire_wizard_steps(&win, cfg, state, overlay_weak, settings_ref, registry);
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        win.on_finished(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    {
        let weak = win.as_weak();
        let slot = slot_ref.clone();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
            *slot.borrow_mut() = None;
        });
    }
    present_window_stealth_aware(&win, |hwnd| {
        // Keep these transient overlay windows out of the taskbar + Alt-Tab,
        // like the bar/tiles — otherwise under stealth they leak an existence
        // entry while open (content is WDA-hidden, but the window button isn't).
        let _ = slint_replay::win32::set_skip_taskbar(hwnd, true);
        // OS-level rounded corners (opaque frameless window) — same as archive.
        slint_replay::win32::set_round_corners(hwnd);
        focus_window(hwnd);
    });
    *slot_ref.borrow_mut() = Some(win);
}
