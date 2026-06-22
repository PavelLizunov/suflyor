//! v0.22.0 — auto-name the active session with the LOCAL light model.
//!
//! Once a session has accumulated enough transcript, fire ONE cheap, no-think
//! completion on the LOCAL model to produce a short human title, then stash it
//! in [`crate::runtime_state::SlintRuntime::session_name`]. The bar's tick timer
//! polls that field and shows it next to the running clock — a live indicator of
//! WHICH session is in progress. Strictly best-effort + non-blocking:
//!
//! - Fires ONLY when the resolved AI endpoint is LOCAL (`is_local`) — it never
//!   spends cloud money on a title; a cloud-only user just stays unnamed.
//! - Runs on a detached tokio task and goes through the shared `AI_SEMAPHORE`
//!   (inside [`overlay_backend::ai::complete`]), so a live F9 answer keeps
//!   priority — the namer can never starve the user's real question.
//! - Fires exactly once per session (the `session_name_requested` latch); every
//!   later call is a cheap no-op under one lock.
//! - The result is dropped if the session changed while the model was thinking
//!   (generation guard), so a slow title can't land on the NEXT session.
//! - On any error (local down, empty reply) the session simply stays unnamed.

use crate::runtime_state::{lock, SharedSlintRuntime};
use overlay_backend::ai::{self, ChatMessage, MessageContent};
use overlay_backend::config::{AiEndpoint, SharedConfig};

/// Transcript lines required before we ask for a title — enough context for a
/// meaningful name without burning a call on the first stray word.
pub const NAME_TRIGGER_LINES: usize = 6;

/// Hard cap on the accepted title (chars) — defends the single-row bar against
/// a model that ignores "short". The bar elides beyond its width anyway; this
/// keeps the STORED title (→ archive later) from being a runaway sentence.
const NAME_MAX_CHARS: usize = 40;

/// Token budget for the title completion — a 3–5 word name fits easily.
const NAME_MAX_TOKENS: u32 = 32;

/// Re-gen throttle (v0.22.0): the name is refreshed at most once per
/// [`REGEN_INTERVAL_MS`], only after [`REGEN_MIN_NEW_LINES`] of new transcript,
/// and only when the triggering line follows a lull of ≥ [`REGEN_LULL_MS`] — so
/// the refresh lands in a quiet gap, not mid-speech. The shared `AI_SEMAPHORE`
/// keeps it behind live answers regardless.
const REGEN_INTERVAL_MS: u128 = 240_000; // 4 min
const REGEN_MIN_NEW_LINES: usize = 25;
const REGEN_LULL_MS: u128 = 6_000; // 6 s

/// What the namer should do on a given line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NamerAction {
    /// First name for the session (≥ `NAME_TRIGGER_LINES` lines, never named).
    First,
    /// Refresh an existing name (grown enough + interval elapsed + in a lull).
    Regen,
    /// Nothing to do this call.
    Skip,
}

/// Inputs to the (pure, testable) namer decision.
#[derive(Clone, Copy)]
struct Gate {
    inflight: bool,
    requested: bool,
    has_name: bool,
    len: usize,
    at_len: usize,
    now_ms: u128,
    at_ms: u128,
    gap_ms: u128,
}

/// First-shot vs throttled re-gen vs nothing — pure, see tests.
fn decide(g: &Gate) -> NamerAction {
    if g.inflight {
        return NamerAction::Skip; // one namer task at a time
    }
    if !g.requested {
        return if g.len >= NAME_TRIGGER_LINES {
            NamerAction::First
        } else {
            NamerAction::Skip
        };
    }
    // First shot already used; refresh only on real growth, after the interval,
    // and when this line follows a lull (so it lands in a quiet gap).
    if g.has_name
        && g.now_ms.saturating_sub(g.at_ms) >= REGEN_INTERVAL_MS
        && g.len.saturating_sub(g.at_len) >= REGEN_MIN_NEW_LINES
        && g.gap_ms >= REGEN_LULL_MS
    {
        NamerAction::Regen
    } else {
        NamerAction::Skip
    }
}

/// Drive the background session-namer for one transcript line at `now_ms`
/// (unix-ms). Fires the FIRST name once enough transcript lands, then a
/// throttled RE-GEN as the conversation grows (see [`decide`]). Best-effort,
/// non-blocking, LOCAL-only; the caller holds NO lock and runs inside a tokio
/// runtime (the transcript-forwarder task).
pub fn maybe_spawn_namer(rt: &SharedSlintRuntime, cfg: &SharedConfig, now_ms: u128) {
    let (action, gen, lines): (NamerAction, u64, Vec<String>) = {
        let mut s = lock(rt);
        let gap = now_ms.saturating_sub(s.last_transcript_ms);
        s.last_transcript_ms = now_ms;
        let action = decide(&Gate {
            inflight: s.session_name_inflight,
            requested: s.session_name_requested,
            has_name: s.session_name.is_some(),
            len: s.full_transcript.len(),
            at_len: s.session_name_at_len,
            now_ms,
            at_ms: s.session_name_at_ms,
            gap_ms: gap,
        });
        if action == NamerAction::Skip {
            (action, 0, Vec::new())
        } else {
            s.session_name_requested = true;
            s.session_name_inflight = true;
            // Arm the throttle on the ATTEMPT (not on success): a FAILED local
            // re-gen — or the cloud bail below — then waits a fresh interval
            // instead of re-attempting on every lull-following line.
            s.session_name_at_ms = now_ms;
            s.session_name_at_len = s.full_transcript.len();
            let lines = s.full_transcript.iter().map(|l| l.text.clone()).collect();
            (action, s.session_gen, lines)
        }
    };
    if action == NamerAction::Skip {
        return;
    }
    // Resolve the endpoint OUTSIDE the rt lock. Only the free LOCAL model names.
    let ep = cfg.read().ai_endpoint(true);
    if !ep.is_local {
        // Cloud / no local: release the guard. `requested` is latched + the
        // throttle is already armed, so there's no per-line churn — naming is
        // local-only by design.
        lock(rt).session_name_inflight = false;
        return;
    }
    let rt = rt.clone();
    let regen = action == NamerAction::Regen;
    tokio::spawn(async move {
        let result = generate_name(&ep, &lines).await;
        // Capture (session_id, name) to persist AFTER the lock — the sidecar
        // write is small file I/O and must not run under the rt mutex.
        let persist = {
            let mut s = lock(&rt);
            // Generation guard: discard a title that outlived its session, AND let
            // only the OWNING generation release its own in-flight latch — a late
            // task from a prior session must not clear the CURRENT session's latch
            // (that would let a second namer spawn for the live session). (The
            // throttle `at_ms`/`at_len` were already advanced at claim time, so a
            // failed reply leaves the prior name and re-arms the interval.)
            if s.session_gen != gen {
                None
            } else {
                s.session_name_inflight = false;
                match result {
                    Some(name) => {
                        eprintln!(
                            "[session-namer] {}: {name}",
                            if regen { "re-named" } else { "auto-named" }
                        );
                        s.session_name = Some(name.clone());
                        s.current_session_id.clone().map(|sid| (sid, name))
                    }
                    // First-shot with no reply stays unnamed (the latch blocks a
                    // retry); a failed re-gen silently keeps the prior name.
                    None if !regen => {
                        eprintln!("[session-namer] skipped (no local reply)");
                        None
                    }
                    None => None,
                }
            }
        };
        // Persist to the archive sidecar (empty session id = ephemeral → no-op).
        if let Some((sid, name)) = persist {
            overlay_backend::session_names::set(&sid, &name, now_ms);
        }
    });
}

/// One no-think local completion → a cleaned short title, or `None` on failure.
/// `pub` so the archive's ↻-regen can re-title a saved session from its stored
/// transcript (same prompt + cleanup as the live auto-namer).
pub async fn generate_name(ep: &AiEndpoint, lines: &[String]) -> Option<String> {
    let transcript = lines.join("\n");
    let system = "Придумай очень короткое название, НЕ больше 4 слов, для разговора по \
его началу. Ответь на языке разговора. Верни ТОЛЬКО название — без кавычек, без \
пояснений, без точки в конце.";
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: MessageContent::Text(system.to_string()),
        },
        ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(transcript),
        },
    ];
    let raw = ai::complete(
        &ep.base_url,
        &ep.bearer,
        &ep.model,
        messages,
        NAME_MAX_TOKENS,
    )
    .await
    .ok()?;
    let name = clean_name(&raw);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// First line, stripped of trailing sentence punctuation THEN wrapping quotes,
/// then capped. Order matters: `«Title».` → `Title` (strip the period before
/// the closing quote, else the quote is stranded).
fn clean_name(raw: &str) -> String {
    let first = raw.lines().next().unwrap_or("").trim();
    let no_punct = first.trim_end_matches(['.', '…', '!', '?']);
    let unquoted = no_punct.trim_matches(['"', '«', '»', '\'', '`']).trim();
    unquoted.chars().take(NAME_MAX_CHARS).collect()
}

/// Truncate an auto-name to a bar-safe width for the overlay's single-row bar:
/// at most `BAR_LABEL_CHARS` characters, with a trailing ellipsis when cut. The
/// bar's `overflow: elide` is a second guard, but capping in Rust keeps the
/// property value itself bounded regardless of layout quirks. The FULL name
/// stays in `SlintRuntime.session_name` (the archive shows it untruncated).
pub fn bar_label(name: &str) -> String {
    const BAR_LABEL_CHARS: usize = 36;
    if name.chars().count() <= BAR_LABEL_CHARS {
        return name.to_string();
    }
    let mut s: String = name.chars().take(BAR_LABEL_CHARS - 1).collect();
    s.push('…');
    s
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests assert directly; runtime code stays strict"
)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_wrapping_quotes_and_trailing_period() {
        assert_eq!(
            clean_name("«Собеседование по Rust»."),
            "Собеседование по Rust"
        );
        assert_eq!(clean_name("\"Hello world\""), "Hello world");
        assert_eq!(clean_name("Интеграция платежей!"), "Интеграция платежей");
    }

    #[test]
    fn clean_takes_first_line_only() {
        assert_eq!(clean_name("Title here\nrambling after"), "Title here");
    }

    #[test]
    fn clean_caps_length() {
        let long = "ц".repeat(200);
        assert!(clean_name(&long).chars().count() <= NAME_MAX_CHARS);
    }

    #[test]
    fn clean_blank_is_empty() {
        assert_eq!(clean_name(""), "");
        assert_eq!(clean_name("   "), "");
    }

    #[test]
    fn clean_keeps_interior_dots() {
        // A trailing-only strip must not touch a dot inside the title.
        assert_eq!(clean_name("Обзор v1.2 релиза"), "Обзор v1.2 релиза");
    }

    #[test]
    fn bar_label_passes_short_names_unchanged() {
        assert_eq!(
            bar_label("Путь к высокому доходу"),
            "Путь к высокому доходу"
        );
        assert_eq!(bar_label(""), "");
    }

    #[test]
    fn bar_label_caps_long_names_with_ellipsis() {
        let long = "Очень длинное название сессии про деньги, квартиру и доход";
        let out = bar_label(long);
        assert!(out.chars().count() <= 36);
        assert!(out.ends_with('…'));
    }

    fn gate(requested: bool, has_name: bool, len: usize) -> Gate {
        Gate {
            inflight: false,
            requested,
            has_name,
            len,
            at_len: 0,
            now_ms: 0,
            at_ms: 0,
            gap_ms: 0,
        }
    }

    #[test]
    fn decide_first_only_after_trigger_lines() {
        assert_eq!(
            decide(&gate(false, false, NAME_TRIGGER_LINES - 1)),
            NamerAction::Skip
        );
        assert_eq!(
            decide(&gate(false, false, NAME_TRIGGER_LINES)),
            NamerAction::First
        );
    }

    #[test]
    fn decide_inflight_blocks_everything() {
        let mut g = gate(false, false, 100);
        g.inflight = true;
        assert_eq!(decide(&g), NamerAction::Skip);
    }

    #[test]
    fn decide_regen_needs_name_interval_growth_and_lull() {
        // All conditions satisfied → Regen.
        let mut base = gate(true, true, REGEN_MIN_NEW_LINES + 1);
        base.now_ms = REGEN_INTERVAL_MS;
        base.gap_ms = REGEN_LULL_MS;
        assert_eq!(decide(&base), NamerAction::Regen);

        // No name yet (first shot failed) → never re-gen.
        let mut no_name = base;
        no_name.has_name = false;
        assert_eq!(decide(&no_name), NamerAction::Skip);

        // Interval not elapsed → Skip.
        let mut early = base;
        early.now_ms = REGEN_INTERVAL_MS - 1;
        assert_eq!(decide(&early), NamerAction::Skip);

        // Not enough new lines → Skip.
        let mut small = base;
        small.len = REGEN_MIN_NEW_LINES - 1;
        assert_eq!(decide(&small), NamerAction::Skip);

        // Mid-speech (no lull) → Skip.
        let mut busy = base;
        busy.gap_ms = REGEN_LULL_MS - 1;
        assert_eq!(decide(&busy), NamerAction::Skip);
    }
}
