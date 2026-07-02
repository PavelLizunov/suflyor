//! Diagnostics tab: readiness population + the REDACTED clipboard report
//! (Phase 2 of the `overlay_host.rs` modularization — see
//! `docs/overlay-host-modularization-plan.md` §5.2).
//!
//! This module owns the config-only readiness snapshot pushed into the
//! Settings "Diagnostics" tab (`populate_diagnostics`) and the SECURITY-CRITICAL
//! `build_diag_report` clipboard builder with its host/IP redaction helpers
//! (`redact_ipv4`, `redact_urls`, `is_ipv4`). The redaction strips the LAN
//! bridge IP / any base_url host (IPv4, IPv6 AND DNS) so a copied report is safe
//! to paste into a support thread and NEVER carries a bearer / API key /
//! transcript / profile / screenshot (§9 — Secrets).
//!
//! The `Check all` live-ping wiring and the `Copy report` button wiring live
//! inside the Settings-tab closures in `overlay_host.rs` (they move in Phase 7);
//! those closures call `build_diag_report()` / `populate_diagnostics()` through
//! the `use diagnostics::*;` re-export. The shared `hotkey_diag_row` (hotkey
//! state, Phase 3) and `active_stack_label` (also used by the bar) deliberately
//! stay in `overlay_host.rs` and are reached via the parent glob below.
//!
//! NOTE (§7): the parent crate-root symbols this module references are imported
//! explicitly below.
use super::{
    active_stack_label, hotkey_diag_row, try_acquire_mic, ComponentHandle, Duration,
    SettingsWindow, SharedString, Timer,
};

/// #131 — push the config-only readiness snapshot + the active-stack summary
/// into the diagnostics tab. Live AI/STT pings are layered on by the
/// `on_diagnostics_check_all_clicked` handler.
pub(crate) fn populate_diagnostics(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
    // Warm the GPU-name cache off-thread now (the Diagnostics tab is opening), so
    // the later "Copy report" click reads it without ever blocking the event loop.
    prime_gpu_cache();
    // Clear any stale "Собрать логи" path from a previous open (reused window).
    win.set_diag_logs_path(SharedString::from(""));
    let c = cfg.read();
    let r = c.readiness();
    win.set_diag_summary(SharedString::from(active_stack_label(&c)));
    win.set_diag_ai_level(if r.ai.configured { 0 } else { 2 });
    win.set_diag_ai_detail(SharedString::from(r.ai.detail));
    win.set_diag_stt_level(if r.stt.configured { 0 } else { 2 });
    win.set_diag_stt_detail(SharedString::from(r.stt.detail));
    // mic/sys: neutral ("—") until "Check all" records a live sample (#133) —
    // a configured device is NOT proof it actually hears.
    win.set_diag_mic_level(3);
    win.set_diag_mic_detail(SharedString::from(r.mic.detail));
    win.set_diag_sys_level(3);
    win.set_diag_sys_detail(SharedString::from(r.sys.detail));
    // P1.1 — Vision (F8): 0=ready (configured), 3=neutral "off" (intentional).
    win.set_diag_vision_level(if r.vision.configured { 0 } else { 3 });
    win.set_diag_vision_detail(SharedString::from(r.vision.detail));
    // P1.2 — global-hotkey registration outcome (per-key conflict surfacing).
    let (hk_level, hk_registered, hk_failed) = hotkey_diag_row();
    win.set_diag_hotkeys_level(hk_level);
    win.set_diag_hotkeys_detail(SharedString::from(hk_registered));
    win.set_diag_hotkeys_failed(SharedString::from(hk_failed));
    win.set_diag_stealth_on(r.stealth_on);
}

/// True if `s` is exactly a dotted IPv4 literal (four 0–255 octets). Used to mask
/// the LAN bridge address out of the copyable diagnostics report.
pub(crate) fn is_ipv4(s: &str) -> bool {
    let mut parts = 0;
    for p in s.split('.') {
        parts += 1;
        if parts > 4 || p.is_empty() || p.len() > 3 || p.parse::<u8>().is_err() {
            return false;
        }
    }
    parts == 4
}

/// Replace any IPv4 literal (e.g. the private bridge 192.168.x.y) with "<ip>" so
/// a copied diagnostics report is safe to paste into a support thread. Ports and
/// paths are kept ("http://192.168.0.142:18902/v1" → "http://<ip>:18902/v1").
/// UTF-8 safe (operates on chars); only ASCII digit/dot runs are inspected.
pub(crate) fn redact_ipv4(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            run.push(ch);
        } else {
            if is_ipv4(&run) {
                out.push_str("<ip>");
            } else {
                out.push_str(&run);
            }
            run.clear();
            out.push(ch);
        }
    }
    if is_ipv4(&run) {
        out.push_str("<ip>");
    } else {
        out.push_str(&run);
    }
    out
}

/// Mask the HOST of every `http(s)://…` URL in `s`, keeping scheme/port/path,
/// via the same `mask_host` the Settings preview uses. Unlike `redact_ipv4`
/// this also masks a DNS host (Tailscale / mDNS / FQDN) or an IPv6 literal —
/// the readiness() detail strings embed the raw RESOLVED base_url, and the
/// copied report is advertised "safe to paste into a support thread", so a
/// `http://bridge.tailnet.ts.net:18902/v1` must not land verbatim. A URL token
/// runs from `http` until the first ASCII whitespace (or end). UTF-8 safe.
pub(crate) fn redact_urls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    // Take the EARLIEST of either scheme each iteration (not http-then-https):
    // a report with `https://b … http://a` must mask `https://b` first, else
    // the text before the (later) http match would echo it verbatim.
    while let Some(pos) = [rest.find("http://"), rest.find("https://")]
        .into_iter()
        .flatten()
        .min()
    {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        // URL ends at the first whitespace — base_url has no spaces.
        let end = tail.find(char::is_whitespace).unwrap_or(tail.len());
        out.push_str(&overlay_backend::config::mask_host(&tail[..end]));
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Replace the current user's home directory with the `%USERPROFILE%` placeholder
/// so the copied report never leaks the OS username via a model/cache path
/// (`C:\Users\alice\suflyor-local-ai\…` → `%USERPROFILE%\suflyor-local-ai\…`).
/// The report is advertised "safe to paste into a support thread", but the host /
/// IP redaction never touched LOCAL FILE PATHS — so the STT model dir used to
/// carry the username verbatim. Reads `USERPROFILE`; delegates to the pure,
/// case-insensitive `redact_home_all_forms` seam (unit-testable without env).
pub(crate) fn redact_user_home(s: &str) -> String {
    match std::env::var("USERPROFILE") {
        Ok(home) => redact_home_all_forms(s, &home),
        Err(_) => s.to_string(),
    }
}

/// Mask the home dir in EVERY separator form that shows up in the log, not only
/// our own single-backslash paths: third-party libs (transcribe_rs / GigaAM) log
/// model paths via Rust `{:?}` Debug, which DOUBLES the backslashes
/// (`C:\\Users\\alice\\…`), and some std/cargo lines use forward slashes. A
/// single-form pass misses the `\\`-escaped form and leaks the OS username —
/// caught live (2026-06-27): 28 unmasked hits in a "Собрать логи" export.
/// Conscious residual: an 8.3 short name (`X3D_MU~1`) isn't derived from
/// USERPROFILE and would slip all three forms — not seen in live logs and a
/// low-identity mangled token, so left uncovered rather than risk over-masking.
fn redact_home_all_forms(s: &str, home: &str) -> String {
    let r = redact_home_in(s, home);
    let r = redact_home_in(&r, &home.replace('\\', "\\\\"));
    redact_home_in(&r, &home.replace('\\', "/"))
}

/// Pure core of `redact_user_home`: replace every ASCII-case-INSENSITIVE
/// occurrence of `home` with `%USERPROFILE%`. Windows paths are case-insensitive
/// and the STT model dir is a USER-PICKED folder (not derived from `USERPROFILE`),
/// so its casing can legitimately differ from the env var — a case-sensitive match
/// would silently miss and leak the username. `to_ascii_lowercase` preserves byte
/// length AND char boundaries (ASCII A–Z↔a–z is one byte; non-ASCII bytes are
/// untouched), and a match starts on an ASCII byte (always a boundary), so every
/// index taken from the lowercased haystack is a valid slice index into the
/// original `s`. No-op on an implausibly short `home` (len < 4) so a stray root
/// can't blanket the report.
fn redact_home_in(s: &str, home: &str) -> String {
    if home.len() < 4 {
        return s.to_string();
    }
    let hay = s.to_ascii_lowercase();
    let needle = home.to_ascii_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    while let Some(rel) = hay[last..].find(&needle) {
        let start = last + rel;
        out.push_str(&s[last..start]);
        out.push_str("%USERPROFILE%");
        last = start + needle.len();
    }
    out.push_str(&s[last..]);
    out
}

/// Cached GPU adapter name(s) — computed once, OFF the event loop (see
/// `prime_gpu_cache`). `build_diag_report` only ever READS this, so the "Copy
/// report" click can never block on a slow / hung WMI query.
static GPU_CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Kick off the (potentially slow, WMI-blocking) GPU query ONCE on a background
/// thread, idempotently. `populate_diagnostics` calls this when the Diagnostics
/// tab is shown, so the value is cached well before the user reaches "Copy
/// report" — keeping `build_diag_report` (which runs on the Slint event loop)
/// non-blocking even if the WMI service is hung. The result is static for the
/// session (the GPU does not change), so caching it once is correct.
pub(crate) fn prime_gpu_cache() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        std::thread::spawn(|| {
            let _ = GPU_CACHE.set(gpu_name());
        });
    });
}

/// Best-effort GPU adapter name(s) for the diagnostics report — helps localise the
/// "Память" render bug (DWM / GPU-driver specific). Queries WMI via PowerShell with
/// NO console window; ANY failure → "unknown" (never breaks the report). The result
/// is a plain device string (no IP / secret / user input), so the report's redaction
/// pass leaves it intact. Runs ONLY on the background thread spawned by
/// `prime_gpu_cache` — never on the event loop.
fn gpu_name() -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name",
        ])
        .output();
    let Ok(out) = out else {
        return "unknown".to_string();
    };
    if !out.status.success() {
        return "unknown".to_string();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if names.is_empty() {
        "unknown".to_string()
    } else {
        names.join(", ")
    }
}

/// P1.1 — build a REDACTED plain-text diagnostics report for the clipboard.
/// Carries subsystem status + NEUTRAL details only: never a bearer / API key /
/// transcript / profile text / screenshot, and the host of any base_url is
/// masked (IPv4, IPv6 AND DNS). Safe to paste into a support thread.
pub(crate) fn build_diag_report(cfg: &overlay_backend::config::SharedConfig) -> String {
    let c = cfg.read();
    let r = c.readiness();
    let st = |configured: bool| {
        if configured {
            "ready"
        } else {
            "not configured"
        }
    };
    let dev = |d: &str| {
        if d.is_empty() {
            "default".to_string()
        } else {
            d.to_string()
        }
    };
    let (hk_level, hk_registered, hk_failed) = hotkey_diag_row();
    let hk_line = if hk_level == 0 {
        format!("ok ({hk_registered})")
    } else if !hk_failed.is_empty() {
        format!("CONFLICT: {hk_failed} (ok: {hk_registered})")
    } else {
        "unavailable".to_string()
    };
    let report = format!(
        "suflyor diagnostics (v{})\n\
         AI: {} — {}\n\
         STT: {} — {}\n\
         Vision: {} — {}\n\
         Microphone: {}\n\
         System audio: {}\n\
         Hotkeys: {}\n\
         Stealth: {}\n\
         System: {} {} · GPU: {}\n",
        env!("CARGO_PKG_VERSION"),
        st(r.ai.configured),
        r.ai.detail,
        st(r.stt.configured),
        r.stt.detail,
        if r.vision.configured { "ready" } else { "off" },
        r.vision.detail,
        dev(&r.mic.detail),
        dev(&r.sys.detail),
        hk_line,
        if r.stealth_on { "on" } else { "off" },
        std::env::consts::OS,
        std::env::consts::ARCH,
        GPU_CACHE.get().map(String::as_str).unwrap_or("unknown"),
    );
    // Mask the user-home (→ %USERPROFILE%) so a local model path can't leak the
    // OS username; then the host of any base_url (IPv4 / IPv6 / DNS) keeping
    // scheme/port/path; redact_ipv4 is a backstop for any bare IPv4 that wasn't
    // part of a URL. redact_ipv4 alone matched ONLY dotted-IPv4, so a DNS / IPv6
    // bridge host used to leak verbatim into the copied report.
    redact_ipv4(&redact_urls(&redact_user_home(&report)))
}

/// P0 — the Diagnostics tab owns its two button callbacks. Settings only WIRES
/// this (see `settings_controller.rs`). Moved VERBATIM from `open_settings`:
/// the redacted "Copy report" clipboard write and the "Проверить всё" live
/// AI+STT ping / 3 s mic sample (single-mic guarded) / system-audio self-test.
pub(crate) fn wire_diagnostics(win: &SettingsWindow, cfg: &overlay_backend::config::SharedConfig) {
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

    // DB maintenance: back up + NON-DESTRUCTIVE repair (integrity_check → checkpoint
    // / reindex / FTS-rebuild / vacuum → re-check) on a WORKER THREAD (VACUUM etc.
    // block — keep them off the event loop). Never deletes user rows; the backup
    // path is reported so the tester can find it.
    {
        let weak = win.as_weak();
        win.on_db_repair_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_db_repair_status(SharedString::from("Проверка базы…"));
            let done = w.as_weak();
            std::thread::spawn(move || {
                let msg =
                    match overlay_backend::persistence::maintenance::diagnose_and_repair_default() {
                        Ok(h) => {
                            let backup = h.backup_path.unwrap_or_else(|| "—".to_string());
                            if h.healthy {
                                format!("[ok] База в порядке. Резервная копия: {backup}")
                            } else {
                                format!(
                                    "[!] Остались проблемы ({}). Данные не удалялись; резервная копия: {backup}",
                                    h.issues.len()
                                )
                            }
                        }
                        Err(e) => format!("[err] Не удалось проверить базу: {e}"),
                    };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = done.upgrade() {
                        w.set_db_repair_status(SharedString::from(msg));
                        // Refresh the Память tab lists so a clear is reflected
                        // IMMEDIATELY — the deleted rows were lingering in the UI
                        // until a Settings reopen (bounded 100-row re-read).
                        super::settings_memory::reload_memory(&w);
                    }
                });
            });
        });
    }

    // DB clears (destructive, but the Slint side requires a 2-tap confirm and the
    // backend backs up the DB FIRST — bailing if the backup fails). Worker thread
    // so a big DELETE never blocks the event loop. Reuses db-repair-status for the
    // result line. Only the memory tables are touched (whitelisted in the backend);
    // sessions/archive are never affected.
    {
        let weak = win.as_weak();
        win.on_db_clear_queue_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_db_repair_status(SharedString::from("Очистка очереди…"));
            let done = w.as_weak();
            std::thread::spawn(move || {
                let msg =
                    match overlay_backend::persistence::maintenance::clear_memory_candidates_default() {
                        Ok(r) => format!(
                            "[ok] Очередь предложений очищена: удалено {}. Резервная копия: {}",
                            r.cleared, r.backup_path
                        ),
                        Err(e) => format!("[err] Очистка отменена: {e}"),
                    };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = done.upgrade() {
                        w.set_db_repair_status(SharedString::from(msg));
                        // Refresh the Память tab lists so a clear is reflected
                        // IMMEDIATELY — the deleted rows were lingering in the UI
                        // until a Settings reopen (bounded 100-row re-read).
                        super::settings_memory::reload_memory(&w);
                    }
                });
            });
        });
    }
    {
        let weak = win.as_weak();
        win.on_db_clear_memory_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            w.set_db_repair_status(SharedString::from("Очистка памяти…"));
            let done = w.as_weak();
            std::thread::spawn(move || {
                let msg =
                    match overlay_backend::persistence::maintenance::clear_memory_items_default() {
                        Ok(r) => format!(
                            "[ok] Одобренная память очищена: удалено {}. Резервная копия: {}",
                            r.cleared, r.backup_path
                        ),
                        Err(e) => format!("[err] Очистка отменена: {e}"),
                    };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = done.upgrade() {
                        w.set_db_repair_status(SharedString::from(msg));
                        // Refresh the Память tab lists so a clear is reflected
                        // IMMEDIATELY — the deleted rows were lingering in the UI
                        // until a Settings reopen (bounded 100-row re-read).
                        super::settings_memory::reload_memory(&w);
                    }
                });
            });
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
            w.set_diag_vision_level(-1);
            w.set_diag_vision_detail(SharedString::from(""));
            let (ai_base, ai_bearer, ai_model, stt_backend, mic_device, sys_device, vision_ep) = {
                let c = cfg_c.read();
                let ep = c.ai_endpoint(false);
                (
                    ep.base_url,
                    ep.bearer,
                    ep.model,
                    c.stt_backend(),
                    c.mic_device.clone(),
                    c.system_audio_device.clone(),
                    c.vision_endpoint(),
                )
            };
            let weak_res = w.as_weak();
            std::thread::spawn(move || {
                // 1. AI + STT live pings (async, on a throwaway runtime).
                let (ai_level, ai_msg, stt_level, stt_msg, vis_level, vis_msg): (
                    i32,
                    String,
                    i32,
                    String,
                    i32,
                    String,
                ) = match tokio::runtime::Builder::new_current_thread()
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
                        let (sl, sm): (i32, String) = match rt
                            .block_on(overlay_backend::stt::test_connection_backend(&stt_backend))
                        {
                            Ok(s) => (0, format!("[ok] {s}")),
                            Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                        };
                        // P2 — Vision live-check: send a SYNTHETIC image (never the
                        // user's screen) to the resolved vision endpoint, so a
                        // "ready" result means the IMAGE path works — not just text
                        // reachability (the old check only pinged text). Vision off
                        // → neutral "off" (3), not an error.
                        let (vl, vm): (i32, String) = match vision_ep {
                            None => (3, "off".to_string()),
                            Some(ep) => match rt.block_on(overlay_backend::vision::test_connection(
                                ep.base_url,
                                ep.bearer,
                                ep.model,
                            )) {
                                Ok(s) => (0, format!("[ok] {s}")),
                                Err(e) => (4, format!("[err] {e:#}").chars().take(80).collect()),
                            },
                        };
                        (al, am, sl, sm, vl, vm)
                    }
                    Err(e) => {
                        let m = format!("[err] runtime: {e}");
                        (4, m.clone(), 4, m.clone(), 4, m)
                    }
                };
                let weak_a = weak_res.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_a.upgrade() {
                        w.set_diag_ai_level(ai_level);
                        w.set_diag_ai_detail(SharedString::from(ai_msg));
                        w.set_diag_stt_level(stt_level);
                        w.set_diag_stt_detail(SharedString::from(stt_msg));
                        w.set_diag_vision_level(vis_level);
                        w.set_diag_vision_detail(SharedString::from(vis_msg));
                    }
                });
                // 2. Microphone — record 3s. "Готов" if the capture path works
                // (device opens + samples flow); a quiet result is fine (you
                // just didn't speak) — only a device error fails.
                // M-1: guard the diagnostics mic probe with the single-mic lock
                // too, so "Проверить всё" during an active session reports busy
                // instead of fighting PTT/voice/dictation for the device.
                let mic_guard = try_acquire_mic();
                let (mic_level, mic_msg): (i32, String) = if mic_guard.is_none() {
                    (
                        4,
                        "[!] mic busy — close PTT / dictation and retry".to_string(),
                    )
                } else {
                    let r = overlay_backend::audio::record_mic_blocking(3000, mic_device);
                    drop(mic_guard); // release before processing (RAII: also on panic)
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

    // F — "Собрать логи": write a REDACTED copy of overlay-host.log (username →
    // %USERPROFILE%, hosts/IPs masked) and reveal it in Explorer, so a tester can
    // attach the log without hunting for the file. Off-thread (the log can be
    // large); the redaction reuses the same passes as "Copy report".
    {
        let weak = win.as_weak();
        win.on_diagnostics_collect_logs_clicked(move || {
            let Some(w) = weak.upgrade() else { return };
            let weak_done = w.as_weak();
            std::thread::spawn(move || {
                let msg = match collect_redacted_log() {
                    Ok(out) => {
                        reveal_in_explorer(&out);
                        // Show the FULL path so the tester can find/copy the file
                        // even if the Explorer reveal misses (AppData is hidden +
                        // cluttered, which tripped the v0.24.0 tester).
                        // Mask the username in the shown path (Settings can be
                        // screen-shared); %USERPROFILE% still resolves if the
                        // tester pastes it into Explorer's address bar.
                        format!(
                            "Сохранён лог: {}",
                            redact_user_home(&out.display().to_string())
                        )
                    }
                    Err(e) => {
                        eprintln!("[overlay-host] collect logs failed: {e}");
                        "Не удалось собрать лог — подробности в overlay-host.log".to_string()
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = weak_done.upgrade() {
                        w.set_diag_logs_path(SharedString::from(msg));
                    }
                });
            });
        });
    }
}

/// F — read overlay-host.log, REDACT it (username → %USERPROFILE%, hosts/IPs
/// masked — the same passes "Copy report" uses), and write a support-safe copy
/// to the DESKTOP, returning that path. Saving to the Desktop (not the hidden,
/// cluttered %APPDATA% data dir) is deliberate: the tester must see the file
/// immediately and never hunt for it (live feedback, 2026-06-27). Falls back to
/// the data dir only if the Desktop known-folder can't be resolved. Log sites
/// print presence flags / char-counts (never key values), but the log DOES embed
/// the OS username in file paths, so this masks it before the tester shares it.
fn collect_redacted_log() -> std::io::Result<std::path::PathBuf> {
    let root = overlay_backend::paths::data_root().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no data root for the log")
    })?;
    let raw = std::fs::read_to_string(root.join("overlay-host.log"))?;
    let redacted = redact_user_home(&redact_ipv4(&redact_urls(&raw)));
    let dir = dirs::desktop_dir().unwrap_or(root);
    let out = dir.join("suflyor-log.txt");
    std::fs::write(&out, redacted)?;
    Ok(out)
}

/// Open Explorer with `path` selected so the tester can grab/attach it directly.
/// `raw_arg` keeps the explicit quotes around the path so `/select` works even
/// when the profile path contains a space (e.g. a username with a space).
fn reveal_in_explorer(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    // No creation_flags here: explorer.exe is a GUI app; CREATE_NO_WINDOW is for
    // console processes and can interfere with how it opens/selects the folder.
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}

#[cfg(test)]
mod tests {
    //! Locks the "Copy report" redaction contract (its security boundary, §9 —
    //! Secrets): the LAN bridge IP / any base_url host (IPv4, IPv6, DNS) must be
    //! masked, while ports / paths / non-IP tokens survive so the report stays
    //! useful and safe to paste into a support thread. Pure: no bridge, no UI,
    //! no network.
    use super::*;

    // P1.1 — lock the "Copy report" redaction contract (its security boundary):
    // the LAN bridge IP must be masked, while ports / paths / non-IP tokens
    // survive so the report stays useful and safe to paste into a support thread.
    #[test]
    fn redact_ipv4_masks_lan_ip_keeps_port_and_path() {
        assert_eq!(
            redact_ipv4("http://192.168.0.142:18902/v1"),
            "http://<ip>:18902/v1"
        );
        assert_eq!(redact_ipv4("local 127.0.0.1 ok"), "local <ip> ok");
        // No IPv4 → untouched (model id, app version).
        assert_eq!(redact_ipv4("claude-sonnet-4-6"), "claude-sonnet-4-6");
        assert_eq!(
            redact_ipv4("suflyor diagnostics (v1.16.1)"),
            "suflyor diagnostics (v1.16.1)"
        );
    }

    // The copied report is "safe to paste into a support thread", but the host/IP
    // redaction never masked LOCAL FILE PATHS — so the STT model dir (under the
    // user home) leaked the OS username verbatim. Lock that the user-home →
    // %USERPROFILE% masking holds and is path-safe.
    #[test]
    fn redact_user_home_masks_profile_dir_keeps_rest() {
        // Canonical: home is a prefix of the STT model dir.
        assert_eq!(
            redact_home_in(
                "STT: ready — gigaam · C:\\Users\\alice\\suflyor-local-ai\\gigaam-v3",
                "C:\\Users\\alice"
            ),
            "STT: ready — gigaam · %USERPROFILE%\\suflyor-local-ai\\gigaam-v3"
        );
        // Case-INSENSITIVE: a user-picked dir can differ in casing from USERPROFILE
        // (drive letter or any segment) — must still be masked, not leaked.
        assert_eq!(
            redact_home_in("models at c:\\users\\Alice\\m", "C:\\Users\\alice"),
            "models at %USERPROFILE%\\m"
        );
        // EVERY occurrence is masked (a future line could embed the home twice).
        assert_eq!(
            redact_home_in("C:\\Users\\bob\\a and C:\\Users\\bob\\b", "C:\\Users\\bob"),
            "%USERPROFILE%\\a and %USERPROFILE%\\b"
        );
        // A bare / implausibly short home must NOT blanket the whole report.
        assert_eq!(redact_home_in("C:\\x is fine", "C:\\"), "C:\\x is fine");
        // No home occurrence → untouched (model id, version, device name).
        assert_eq!(
            redact_home_in("GPU: NVIDIA GeForce RTX 5060 Ti", "C:\\Users\\bob"),
            "GPU: NVIDIA GeForce RTX 5060 Ti"
        );
    }

    #[test]
    fn redact_user_home_masks_double_backslash_and_forward_slash_forms() {
        // transcribe_rs / GigaAM log model paths via {:?} Debug, which DOUBLES the
        // backslashes; the home (from USERPROFILE) is single-backslash. The export
        // must still mask it — this is the live F leak (28 hits) pinned as a test.
        let home = r"C:\Users\alice";
        let dbl = redact_home_all_forms(
            r#"Loading GigaAM model from "C:\\Users\\alice\\suflyor-local-ai\\m.onnx""#,
            home,
        );
        assert!(!dbl.to_ascii_lowercase().contains("alice"), "leaked: {dbl}");
        assert!(dbl.contains("%USERPROFILE%"));
        // Forward-slash paths (std / cargo) are masked too.
        assert_eq!(
            redact_home_all_forms("cache at C:/Users/alice/.cargo", home),
            "cache at %USERPROFILE%/.cargo"
        );
        // Single-backslash still works (no regression).
        assert_eq!(
            redact_home_all_forms("at C:\\Users\\alice\\x", home),
            "at %USERPROFILE%\\x"
        );
    }

    #[test]
    fn redact_urls_masks_dns_ipv6_and_ipv4_hosts_keeping_scheme_port_path() {
        // FIX #7 — a DNS bridge host (Tailscale / mDNS / FQDN) must be masked,
        // not echoed verbatim, while scheme + port + path are kept.
        assert_eq!(
            redact_urls("AI: ready — local llama @ http://bridge.tailnet.ts.net:18902/v1"),
            "AI: ready — local llama @ http://***:18902/v1"
        );
        // IPv6 literal host is masked too (redact_ipv4 never matched these).
        assert_eq!(
            redact_urls("STT: http://[2001:db8::1]:9000/v1 ok"),
            "STT: http://***:9000/v1 ok"
        );
        // P0-2: bracketed IPv6 WITHOUT a port must be fully masked (no ':abcd]' leak).
        assert_eq!(
            redact_urls("vision @ http://[fd00::abcd]/v1"),
            "vision @ http://***/v1"
        );
        // Plain IPv4 in a URL — host blanked, port/path kept.
        assert_eq!(
            redact_urls("http://192.168.0.142:18902/v1"),
            "http://***:18902/v1"
        );
        // https + a DNS host appearing BEFORE an http URL: the earliest match
        // must be masked first so the leading text can't echo it verbatim.
        assert_eq!(
            redact_urls("a https://api.example.com/v1 b http://10.0.0.5:1234/v1 c"),
            "a https://***/v1 b http://***:1234/v1 c"
        );
        // No URL → untouched.
        assert_eq!(redact_urls("Hotkeys: ok (F9, F4)"), "Hotkeys: ok (F9, F4)");
    }

    #[test]
    fn redact_urls_masks_dns_host_in_a_report_shaped_string() {
        // FIX #7 — `build_diag_report` composes its lines from readiness()
        // details that embed the raw resolved base_url, then applies this
        // redact_urls pass before the clipboard copy. Asserting the pass on a
        // string shaped like that report keeps the test hermetic (the real
        // `build_diag_report` reads a SharedConfig, which loads live secrets).
        let report = "suflyor diagnostics (v1.16.1)\n\
             AI: ready — local · http://bridge.tailnet.ts.net:18902/v1 · my-local-gemma\n\
             STT: ready — groq cloud\n\
             Hotkeys: ok (F9, F4)\n";
        let masked = redact_urls(report);
        assert!(
            !masked.contains("bridge.tailnet.ts.net"),
            "FIX #7: DNS base_url host leaked into copied report:\n{masked}"
        );
        assert!(
            masked.contains("http://***:18902/v1"),
            "masked URL should keep scheme/port/path:\n{masked}"
        );
        // Non-URL lines are untouched.
        assert!(masked.contains("suflyor diagnostics (v1.16.1)"));
        assert!(masked.contains("STT: ready — groq cloud"));
    }

    #[test]
    fn is_ipv4_accepts_valid_rejects_ports_and_versions() {
        assert!(is_ipv4("10.0.0.1"));
        assert!(is_ipv4("255.255.255.255"));
        assert!(!is_ipv4("18902")); // a bare port
        assert!(!is_ipv4("1.16.1")); // 3 octets (a version)
        assert!(!is_ipv4("1.2.3.4.5")); // 5 octets
        assert!(!is_ipv4("256.1.1.1")); // octet > 255
        assert!(!is_ipv4("")); // empty
    }
}
