//! Shared application state for the multi-window overlay-host.
//!
//! Single `Arc<Mutex<AppState>>` shared across overlay, tile, settings,
//! and replay windows (per migration plan Phase 1). Each window's
//! callbacks borrow the mutex briefly to mutate, then notify Slint
//! windows via setters from the UI thread.
//!
//! For Day 2 (skeleton) this struct only carries a few counters to
//! prove the cross-window state-sharing pipeline. Phases 2-5 will
//! add: tile registry, current session, user config, language,
//! palette state, etc.

use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct AppState {
    /// Monotonic count of tile spawns over the session. Used by the
    /// overlay bar to display "spawned N tiles" footer.
    pub tiles_spawned: u32,
    /// True when the overlay should stay on top of all other windows.
    /// Wired to overlay bar's "always on top" toggle.
    pub always_on_top: bool,
    /// True when the overlay should be hidden from screen capture.
    /// Wired to settings stealth toggle (WDA_EXCLUDEFROMCAPTURE).
    pub stealth: bool,
    /// True while the mic chip is in active mode. Drives the 3-second
    /// WASAPI probe via `audio::record_mic_blocking` in overlay_host.rs.
    /// Phase B2 continuous-capture wiring will keep this flag with the
    /// same semantics (active = capture pipeline running).
    pub mic_active: bool,
    /// True while the sys chip is in active mode. Drives the 3-second
    /// WASAPI loopback probe via `audio::record_sys_blocking`.
    pub sys_active: bool,
    /// True when the session timer is running.
    pub timer_active: bool,
    /// Elapsed session seconds (formatted to MM:SS by overlay bar).
    pub session_secs: u64,
    /// Cumulative session cost in USD.
    pub cost_usd: f64,
    /// True while a record_mic_blocking probe is running. Set on
    /// click-ON, cleared in the post-result invoke_from_event_loop
    /// closure. Re-entry guard so rapid double-click doesn't spawn
    /// concurrent WASAPI captures fighting for the same device.
    /// Caught by review-agent 2026-05-27.
    pub mic_probe_in_flight: bool,
    /// Same as mic_probe_in_flight but for the sys chip's loopback
    /// probe (record_sys_blocking).
    pub sys_probe_in_flight: bool,
    /// Live local-AI server child processes (llama-server, whisper-server)
    /// launched by the in-app installer. Tracked so they can be killed on
    /// app quit instead of being orphaned.
    pub local_ai_servers: Vec<std::process::Child>,
    /// Set true by the installer's "Cancel" button to abort an in-progress
    /// local-AI install; the worker thread + the curl poll loop check it.
    pub local_ai_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Convenience alias used by all window-spawning callbacks.
pub type SharedState = Arc<Mutex<AppState>>;

#[must_use]
pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState {
        always_on_top: true, // overlay defaults to topmost
        ..Default::default()
    }))
}

/// Cycle AI model among the full IDs accepted by both `overlay_backend::
/// ai::pricing_per_million` and the user's Claude bridge. IDs match what
/// the React/Tauri v0.1.1 app writes into `config.json`, so the two stacks
/// stay in sync.
///
/// Short names ("sonnet"/"haiku"/"opus") get mapped to the canonical
/// current-generation ID before cycling, so legacy configs still work.
/// Unknown IDs fall back to haiku (cheap default).
pub fn next_model(current: &str) -> &'static str {
    match current {
        "claude-haiku-4-5" | "haiku" => "claude-sonnet-4-6",
        "claude-sonnet-4-5" | "claude-sonnet-4-6" | "sonnet" => "claude-opus-4-7",
        "claude-opus-4-7" | "opus" => "claude-haiku-4-5",
        _ => "claude-haiku-4-5",
    }
}

/// Format session seconds as MM:SS (or H:MM:SS for >1 hour).
#[must_use]
pub fn format_timer(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Plain-Rust shape produced by the palette adapter before wrapping
/// in the Slint `PaletteResult` struct. Lifted out of `overlay_host`
/// so the per-row preview/fallback logic gets unit-test coverage
/// without needing a Slint runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteRow {
    pub key: String,
    pub title: String,
    pub preview: String,
    pub source: &'static str,
}

/// Convert a single `overlay_backend::kb::KBEntry` into the row data
/// the F4 palette renders. Preview is the first sentence (or first
/// 160 chars, whichever is shorter) of the body, with empty fallback.
/// Title falls back to the key if the heading is blank.
#[must_use]
pub fn palette_row_from(entry: &overlay_backend::kb::KBEntry) -> PaletteRow {
    let preview = entry
        .body
        .split_terminator(['.', '\n'])
        .next()
        .unwrap_or("")
        .chars()
        .take(160)
        .collect::<String>();
    let title = if entry.heading.is_empty() {
        entry.key.clone()
    } else {
        entry.heading.clone()
    };
    PaletteRow {
        key: entry.key.clone(),
        title,
        preview,
        source: entry.source,
    }
}

/// Map an AI-error-chain string to a privacy-safe category for tile
/// display. Pure function — extracted from overlay_host.rs so unit
/// tests can pin the classifier table without spinning up the UI.
/// Strips out URLs / IPs / bearer tokens that would otherwise land
/// in user-shared screenshots. Order of checks matters when keywords
/// overlap (e.g. "timeout" before "404" since reqwest timeout chain
/// can contain both).
#[must_use]
pub fn classify_ai_error(msg: &str) -> &'static str {
    let lower = msg.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        "AI bridge timed out"
    } else if lower.contains("connection refused") || lower.contains("connection error") {
        "AI bridge unreachable (connection refused)"
    } else if lower.contains("401") || lower.contains("403") || lower.contains("unauthorized") {
        "AI bridge rejected request (auth failure)"
    } else if lower.contains("404") || lower.contains("not found") {
        "AI bridge endpoint not found (URL or model wrong)"
    } else if lower.contains("429") || lower.contains("rate") {
        "AI bridge rate-limited"
    } else if lower.contains("500") || lower.contains("502") || lower.contains("503") {
        "AI bridge returned server error"
    } else {
        "AI bridge call failed (see overlay-host stderr for diagnostic)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strengthened invariant: every next_model output must be
    /// recognized by `overlay_backend::ai::pricing_per_million` —
    /// NOT just present in a local hardcoded list. The earlier
    /// version (a local `canonical` array) would have passed if
    /// both the array AND next_model were edited together to wrong
    /// IDs. This version asks the real pricing fn whether the
    /// output is a known model. If pricing_per_million falls through
    /// to its safe-default arm (3.0, 15.0), that's an unknown ID
    /// and the bridge will likely reject the API call. Caught by
    /// hallucination audit 2026-05-27.
    #[test]
    fn next_model_outputs_are_canonical_ids() {
        use overlay_backend::ai::pricing_per_million;
        // Safe-default arm output — any (3.0, 15.0) match is the
        // "unknown model" fallback, NOT a real recognized ID.
        const SAFE_DEFAULT: (f64, f64) = (3.0, 15.0);

        // Cycle from each of the 3 canonical full IDs 4 times each —
        // must always land on a recognized ID per pricing.
        let starts = ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-7"];
        for start in starts {
            // Verify the start ID itself is recognized — paranoia
            // since it's the test setup.
            let p0 = pricing_per_million(start);
            // sonnet-4-6 happens to share pricing (3.0, 15.0) with the
            // safe default. Allow that explicit case; reject everything
            // else hitting that exact arm.
            assert!(
                p0 != SAFE_DEFAULT || start == "claude-sonnet-4-6" || start == "claude-sonnet-4-5",
                "start ID {start:?} hits safe-default pricing — bad test setup"
            );
            let mut cur = start;
            for _ in 0..4 {
                cur = next_model(cur);
                let p = pricing_per_million(cur);
                assert!(
                    p != SAFE_DEFAULT
                        || cur == "claude-sonnet-4-6"
                        || cur == "claude-sonnet-4-5",
                    "next_model({start} → ...) produced {cur:?} which pricing_per_million doesn't recognize \
                     (hits safe-default arm) — bridge will reject the API call"
                );
            }
        }

        // Legacy short-name aliases also map to recognized IDs.
        for alias in ["haiku", "sonnet", "opus"] {
            let out = next_model(alias);
            let p = pricing_per_million(out);
            assert!(
                p != SAFE_DEFAULT || out == "claude-sonnet-4-6" || out == "claude-sonnet-4-5",
                "next_model({alias:?}) → {out:?} pricing unrecognized"
            );
        }

        // Unknown input falls back to cheap default (haiku-4-5).
        assert_eq!(next_model("garbage"), "claude-haiku-4-5");
    }

    /// Golden test for the palette row preview logic. Pins the
    /// 160-char cap, first-sentence-only behavior, and empty-heading
    /// fallback to key — the three regressions most likely to slip
    /// through future refactors. Review-agent recommended this as
    /// the highest-value test to add for the current Slint code path
    /// 2026-05-27.
    #[test]
    fn palette_row_preview_slicing() {
        use overlay_backend::kb::KBEntry;
        // 1. Body with multiple sentences — keeps only first.
        let e = KBEntry::new(
            "k8s".into(),
            "kubernetes — k8s".into(),
            "Container orchestration platform. Manages deployment. Heals failures.".into(),
            "glossary",
        );
        let r = palette_row_from(&e);
        assert_eq!(r.key, "k8s");
        assert_eq!(r.title, "kubernetes — k8s");
        assert_eq!(r.preview, "Container orchestration platform");
        assert_eq!(r.source, "glossary");

        // 2. Body longer than 160 chars in a single sentence → trimmed.
        let long_body = "a".repeat(300);
        let e2 = KBEntry::new(
            "long".into(),
            "Long entry".into(),
            long_body.clone(),
            "glossary",
        );
        let r2 = palette_row_from(&e2);
        assert_eq!(r2.preview.chars().count(), 160);
        assert!(long_body.starts_with(&r2.preview));

        // 3. Empty heading → falls back to key.
        let e3 = KBEntry::new(
            "naked".into(),
            String::new(),
            "tiny body".into(),
            "snippets",
        );
        let r3 = palette_row_from(&e3);
        assert_eq!(r3.title, "naked");
        assert_eq!(r3.preview, "tiny body");

        // 4. Newline-only body → empty preview.
        let e4 = KBEntry::new(
            "nl".into(),
            "Newline-only".into(),
            "\n\n\n".into(),
            "glossary",
        );
        let r4 = palette_row_from(&e4);
        assert_eq!(r4.preview, "");
    }

    /// Table-driven check that classify_ai_error never leaks the raw
    /// URL / IP / token from a reqwest error string. Each entry is
    /// (error-substring, expected-category, must-NOT-appear-in-output).
    #[test]
    fn classify_ai_error_table() {
        let cases: &[(&str, &str)] = &[
            (
                "error sending request for url (http://192.168.0.142:18902/v1/chat/completions): operation timed out",
                "AI bridge timed out",
            ),
            (
                "tcp connect: Connection refused (os error 10061)",
                "AI bridge unreachable (connection refused)",
            ),
            (
                "HTTP 401 Unauthorized from http://192.168.0.142:18902",
                "AI bridge rejected request (auth failure)",
            ),
            (
                "HTTP 404 Not Found (POST /v1/chat/completions)",
                "AI bridge endpoint not found (URL or model wrong)",
            ),
            (
                "HTTP 429 Too Many Requests (rate-limited 30s)",
                "AI bridge rate-limited",
            ),
            (
                "HTTP 502 Bad Gateway upstream",
                "AI bridge returned server error",
            ),
            (
                "some weird new failure mode we never heard of",
                "AI bridge call failed (see overlay-host stderr for diagnostic)",
            ),
        ];
        for (input, expected_category) in cases {
            let got = classify_ai_error(input);
            assert_eq!(
                got, *expected_category,
                "classify_ai_error({input:?}) returned {got:?}, expected {expected_category:?}"
            );
            // Privacy invariant: output must never contain the raw URL,
            // IP, port, or bearer-token-like substring from the input.
            assert!(
                !got.contains("192.168.") && !got.contains("http://") && !got.contains("https://"),
                "classify output {got:?} leaks URL/IP from input {input:?}"
            );
        }
    }
}
