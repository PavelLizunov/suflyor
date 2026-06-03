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
//! NOTE (§7): this mechanical move imports the parent crate-root via
//! `use super::*;`. That is intentional for the extraction; the imports get
//! narrowed in a later pass.
use super::*;

/// #131 — push the config-only readiness snapshot + the active-stack summary
/// into the diagnostics tab. Live AI/STT pings are layered on by the
/// `on_diagnostics_check_all_clicked` handler.
pub(crate) fn populate_diagnostics(
    win: &SettingsWindow,
    cfg: &overlay_backend::config::SharedConfig,
) {
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
         Stealth: {}\n",
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
    );
    // Mask the host of any base_url FIRST (IPv4 / IPv6 / DNS), keeping
    // scheme/port/path; then redact_ipv4 as a backstop for any bare IPv4 that
    // wasn't part of a URL. redact_ipv4 alone matched ONLY dotted-IPv4, so a
    // DNS / IPv6 bridge host used to leak verbatim into the copied report.
    redact_ipv4(&redact_urls(&report))
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
