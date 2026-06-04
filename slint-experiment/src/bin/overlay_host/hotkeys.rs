//! Global-hotkey registration + the hotkey-registration diagnostics state
//! (Phase 3 of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.3).
//!
//! This module owns the one-time global-hotkey REGISTRATION ([`register_hotkeys`])
//! — it builds the process-wide `GlobalHotKeyManager`, registers every F-key
//! (F3 / F4 / F6 / F8 / Shift+F8 / F9 / Shift+F9 / F1), logs each
//! "Fx hotkey registered", and records the per-key outcome into [`HOTKEY_DIAG`]
//! so the Settings ▸ Diagnostics tab can surface a CONFLICTING key by name
//! instead of a blanket "hotkeys disabled" (audit P1.2). The captured outcome is
//! read back through [`hotkey_diag_row`], which `diagnostics.rs` reaches via the
//! parent glob (`use hotkeys::*;` re-exports it at crate root).
//!
//! What deliberately STAYS in `overlay_host.rs`: the hotkey EVENT-DISPATCH timer
//! (the `GlobalHotKeyEvent::receiver()` poll loop), because it captures a dozen
//! main-scoped closures + `Rc`-borrowed window/state slots and only needs to
//! compare the pressed `event.id` against the ids this module hands back. So
//! [`register_hotkeys`] returns a [`RegisteredHotkeys`] carrying the manager
//! (which `main` must keep ALIVE — dropping it unregisters every hotkey) plus
//! each key's `.id()` for the dispatch to match on. Registration order, the log
//! strings, and the `HOTKEY_DIAG` write are preserved byte-for-byte (MECHANICAL
//! move, §2 — no behaviour change).
//!
//! NOTE (§7): this module is self-contained on `global_hotkey` + std (it
//! references NO parent crate-root symbol via short name — everything is a full
//! path), so it carries NO `use super::…` import. It is still the re-export seam
//! (`use hotkeys::*;` in the parent lifts `hotkey_diag_row` to crate root for
//! `diagnostics.rs`).

/// P1.2 — outcome of registering the global hotkeys at startup, captured ONCE so
/// the Diagnostics tab can surface a per-key conflict (not just an eprintln!).
/// Set in the registration loop; read by [`hotkey_diag_row`].
pub(crate) struct HotkeyDiag {
    /// Space-separated keys that registered, e.g. "F1 F3 F4 F6 F8 F9 Shift+F9".
    registered: String,
    /// Comma-separated keys whose `register()` failed (conflict / already taken).
    /// Empty when every key registered.
    failed: String,
    /// True when the hotkey MANAGER itself couldn't be created (nothing tried).
    manager_missing: bool,
}

static HOTKEY_DIAG: std::sync::OnceLock<HotkeyDiag> = std::sync::OnceLock::new();

/// Render the captured hotkey-registration outcome (P1.2) into a (level, detail,
/// failed) triple for the Diagnostics tab. level 0=all registered, 4=a key
/// failed (conflict), 3=manager unavailable / not yet captured. `failed` is the
/// raw key list so the .slint composes the translated "conflict:" wording.
pub(crate) fn hotkey_diag_row() -> (i32, String, String) {
    match HOTKEY_DIAG.get() {
        None => (3, String::new(), String::new()),
        // Manager failed to init → level 4 (error) but EMPTY failed list, so the
        // UI/report render "unavailable", not a misleading "conflict: all".
        Some(d) if d.manager_missing => (4, String::new(), String::new()),
        Some(d) if d.failed.is_empty() => (0, d.registered.clone(), String::new()),
        Some(d) => (4, d.registered.clone(), d.failed.clone()),
    }
}

/// Result of [`register_hotkeys`]: the live `GlobalHotKeyManager` (which the
/// caller MUST keep alive — dropping it unregisters every hotkey) plus each
/// registered key's `.id()` so the dispatch timer in `main` can match a pressed
/// `GlobalHotKeyEvent` against the right action. `manager` is `None` when the
/// manager itself couldn't be created (hotkeys disabled); the ids are still
/// computed (they're just never delivered, so the dispatch arms stay dormant).
pub(crate) struct RegisteredHotkeys {
    /// Kept alive by `main`; `None` when the manager couldn't be created.
    pub manager: Option<global_hotkey::GlobalHotKeyManager>,
    pub f1_id: u32,
    pub f3_id: u32,
    pub f4_id: u32,
    pub f6_id: u32,
    pub f8_id: u32,
    pub sf8_id: u32,
    pub f9_id: u32,
    pub sf9_id: u32,
}

/// ===== Global hotkeys (Phase D2 + B3 extra) =====
///
/// global-hotkey 0.6 owns a single process-wide event receiver +
/// platform-specific manager. We register one hotkey per F-key,
/// then poll the receiver every 50 ms from a Slint Timer — fires
/// on UI thread so we can touch Rc-borrowed state without Send.
///
/// Registered keys (see Settings ▸ Hotkeys):
///   F3 — re-ask the last question     F4 — KB palette (toggle)
///   F6 — manual tile from transcript  F8 — screenshot → vision
///   F9 — ask the AI now
///
/// Returns the manager (kept alive by the caller) + each key's id for the
/// dispatch loop. Registration order, the per-key log lines, and the
/// `HOTKEY_DIAG` write are identical to the former inline block in `main`.
pub(crate) fn register_hotkeys() -> RegisteredHotkeys {
    let hotkey_manager = match global_hotkey::GlobalHotKeyManager::new() {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("[overlay-host] GlobalHotKeyManager init failed: {e}. Hotkeys disabled.");
            None
        }
    };
    let f3_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F3);
    let f4_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F4);
    // Phase E3 slice 3 — F6 manual spawn from last transcript line
    // (bypasses auto-detector). Matches src-tauri hotkey table.
    let f6_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F6);
    // Phase E3 slice 2 — F9 ask (live AI streaming via overlay-backend's
    // ask_stream_loop). Matches src-tauri/React-side semantic where F9
    // is the "ask AI with full transcript context" hotkey.
    let f9_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F9);
    // V0.8.0 (Поток D) — Shift+F9 = one-shot escalate the ask to the smart cloud
    // model (deeper reasoning) without flipping the persistent provider.
    let sf9_hotkey = global_hotkey::hotkey::HotKey::new(
        Some(global_hotkey::hotkey::Modifiers::SHIFT),
        global_hotkey::hotkey::Code::F9,
    );
    // V2 — F8 screenshot → vision (captures the monitor under the cursor and
    // streams a vision model's reading into a tile, via the SEPARATE vision
    // endpoint so text can stay local).
    let f8_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F8);
    // Feature #3 — Shift+F8 = the SAME region capture but in TRANSLATE mode
    // (output only the translation, no screen description). For games/subtitles.
    let sf8_hotkey = global_hotkey::hotkey::HotKey::new(
        Some(global_hotkey::hotkey::Modifiers::SHIFT),
        global_hotkey::hotkey::Code::F8,
    );
    // V0.8.4 — F1 opens the 🆘 help (toggle, like F4).
    let f1_hotkey = global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F1);
    let f3_id = f3_hotkey.id();
    let f4_id = f4_hotkey.id();
    let f6_id = f6_hotkey.id();
    let f9_id = f9_hotkey.id();
    let sf9_id = sf9_hotkey.id();
    let f8_id = f8_hotkey.id();
    let sf8_id = sf8_hotkey.id();
    let f1_id = f1_hotkey.id();
    {
        // P1.2 — capture per-key registration so the Diagnostics tab can name a
        // conflicting key instead of a blanket "hotkeys disabled" (audit P1.2).
        let mut registered: Vec<&str> = Vec::new();
        let mut failed: Vec<&str> = Vec::new();
        if let Some(m) = hotkey_manager.as_ref() {
            for (label, hk) in [
                ("F3", f3_hotkey),
                ("F4", f4_hotkey),
                ("F6", f6_hotkey),
                ("F8", f8_hotkey),
                ("Shift+F8", sf8_hotkey),
                ("F9", f9_hotkey),
                ("Shift+F9", sf9_hotkey),
                ("F1", f1_hotkey),
            ] {
                match m.register(hk) {
                    Ok(()) => {
                        eprintln!("[overlay-host] {label} hotkey registered");
                        registered.push(label);
                    }
                    Err(e) => {
                        eprintln!("[overlay-host] {label} register failed: {e}");
                        failed.push(label);
                    }
                }
            }
        }
        let _ = HOTKEY_DIAG.set(HotkeyDiag {
            registered: registered.join(" "),
            failed: failed.join(", "),
            manager_missing: hotkey_manager.is_none(),
        });
    }

    RegisteredHotkeys {
        manager: hotkey_manager,
        f1_id,
        f3_id,
        f4_id,
        f6_id,
        f8_id,
        sf8_id,
        f9_id,
        sf9_id,
    }
}
