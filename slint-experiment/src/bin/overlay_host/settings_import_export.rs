//! Server-settings import / export Settings wiring (P1 of
//! `docs/overlay-host-gaps-and-next-checks.md` — splitting the
//! `settings_controller.rs` god-function by domain, the same way Phase 2's
//! `diagnostics.rs` and Wave 1-3's `settings_vision.rs` / `settings_stt.rs` /
//! `settings_ai.rs` were extracted).
//!
//! This module owns the SERVER-settings export/import wiring previously inlined
//! in `open_settings`: the server-only export (`on_export_server_settings_clicked`),
//! the two-step server-only import preview (`on_import_server_settings_clicked`),
//! the Apply (`on_apply_server_settings_clicked`), and the Cancel
//! (`on_cancel_server_settings_clicked`). The blocks moved here VERBATIM — same
//! captures (`cfg.clone()` + `win.as_weak()` per closure, plus the shared
//! `pending_server_import` cell threaded in as `pending`), same bodies,
//! byte-for-byte identical behavior. `open_settings` now only CALLS
//! `wire_import_export(&win, cfg, &pending_server_import)` where the four server
//! import/export blocks sat.
//!
//! NOT moved (a different — profile — domain, left in `open_settings`): the
//! full-PROFILE export/import (`on_export_profile_clicked`,
//! `on_import_profile_clicked`), the multi-profile management
//! (`on_profile_*`), and `refresh_profiles`.
//!
//! Also moved here: `apply_server_preview` (the REDACTED preview composer) —
//! its only caller is the `on_import_server_settings_clicked` closure moved with
//! it, so it lives here as a `pub(crate)` item. `msg_refresh_after_import` is
//! NOT moved: it is also called by `on_import_profile_clicked` (a PROFILE
//! closure that STAYS in `settings_controller.rs`), so it stays at the crate
//! root and the two server closures here reach it through `super::`.
//!
//! SECURITY (unchanged by this mechanical move): the import preview shows key
//! PRESENCE only and `mask_host`s the URLs (`apply_server_preview` never carries
//! a secret value); the result tile keeps its GENERIC messages so no `base_url`
//! / LAN IP leaks into a screen-shared Settings window. The server export
//! intentionally includes creds (a PC->PC transfer), exactly as before.
use super::{msg_refresh_after_import, ComponentHandle, Rc, RefCell, SettingsWindow, SharedString};

/// Wire the server-settings import/export Settings callbacks onto the Settings
/// window. Moved VERBATIM out of `open_settings` (P1 domain split) — same
/// captures, same behavior. Needs `win` (for the closures + their `as_weak()`),
/// `cfg` (cloned per closure), and `pending` — the `pending_server_import` cell
/// created in `open_settings` and shared by the import-preview / Apply / Cancel
/// closures (import stashes the parsed config, Apply takes it, Cancel drops it).
/// None of these blocks touch `state` / `overlay_weak` / `slint_rt` /
/// `rt_handle`, so no further params are threaded through.
pub(crate) fn wire_import_export(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
    pending: &Rc<RefCell<Option<overlay_backend::config::Config>>>,
) {
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
                        Err(e) => {
                            // Generic + log: the error chain can carry a path /
                            // internals into this screen-shared field (audit Q8).
                            eprintln!("[overlay-host] server export failed: {e:#}");
                            "[err] export failed (see log)".to_string()
                        }
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
        let pending = pending.clone();
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
                    eprintln!("[overlay-host] server import preview failed: {e:#}");
                    w.set_profile_io_result(SharedString::from(
                        "[err] could not read file (see log)",
                    ));
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
        let pending = pending.clone();
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
                Err(e) => {
                    eprintln!("[overlay-host] server settings apply/save failed: {e:#}");
                    "[err] apply failed (see log)".to_string()
                }
            };
            w.set_profile_io_result(SharedString::from(msg));
        });
    }

    // P1.7 — CANCEL the preview: drop the stashed config + hide the diff.
    {
        let weak = win.as_weak();
        let pending = pending.clone();
        win.on_cancel_server_settings_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            *pending.borrow_mut() = None;
            w.set_server_preview_ready(false);
            w.set_profile_io_result(SharedString::from("import cancelled"));
        });
    }
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
