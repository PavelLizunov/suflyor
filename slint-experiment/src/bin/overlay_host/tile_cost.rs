//! Pure cost-cap + transcript-selection helpers carved out of `tile_ask.rs`
//! (the `tile_ask` split — see `docs/overlay-host-modular-structure-current.md`,
//! "P1: разрезать tile_ask.rs"). No UI, no network: the per-session cost-budget
//! math (`cost_cap_reason`), the non-blocking "over cap" warning emit on the
//! continuation paths (`warn_if_over_cost_cap`), and the recent-labeled
//! transcript selector (`select_recent_labeled`) the ask entrypoints feed to the
//! AI. Reached from `tile_ask.rs` through the `use tile_cost::*;` re-export.
//!
//! NOTE (§7): only the crate-root symbols actually used are imported below.
use super::{Arc, RuntimeEvents, SharedSlintRuntime};

/// Re-exported so the existing binary ask paths keep their crate-root import.
pub(crate) use slint_replay::runtime_state::cost_cap_reason;

/// Local helper: last `max` transcript lines labeled with speaker
/// tags `[ПОЛЬЗОВАТЕЛЬ]` / `[СОБЕСЕДНИК]`. Mirrors src-tauri's
/// `select_recent_lines_labeled` — kept local to avoid promoting a
/// tiny helper to overlay-backend.
pub(crate) fn select_recent_labeled(
    transcript: &std::collections::VecDeque<overlay_backend::audio::TranscriptLine>,
    max: usize,
) -> Vec<String> {
    let n = transcript.len();
    let start = n.saturating_sub(max);
    transcript
        .iter()
        .skip(start)
        .map(|l| {
            let src = match l.source {
                overlay_backend::audio::AudioSource::System => "[СОБЕСЕДНИК]",
                overlay_backend::audio::AudioSource::Mic => "[ПОЛЬЗОВАТЕЛЬ]",
            };
            format!("{src} {}", l.text)
        })
        .collect()
}

/// v0.8.2 (MAJOR-2) — sticky-cloud cost-cap warning. After a 🧠 / Shift+F9
/// escalation a tile's `live` route stays `Cloud`, so EVERY subsequent text
/// follow-up + 🔄 regenerate + 🎤 voice follow-up is now a BILLABLE cloud call.
/// `fire_f9_ask` already emits `cost:cap-hit` when over budget, but the
/// continuation paths did not — so the per-session cap was silently ignored
/// mid-conversation (the regression sticky-cloud introduced). This emits the
/// SAME non-blocking warning (warn only — never block a continuation the user
/// is mid-thread on). No-op for local ($0) calls, when no cap is set, or before
/// any spend has accrued.
pub(crate) fn warn_if_over_cost_cap(
    events: &Arc<dyn RuntimeEvents>,
    cfg: &overlay_backend::config::SharedConfig,
    slint_rt: &SharedSlintRuntime,
    is_local: bool,
    source: &str,
) {
    if is_local {
        return;
    }
    let cap_usd = cfg.read().max_session_cost_usd;
    let current_micro = slint_replay::runtime_state::lock(slint_rt).session_cost_microcents;
    if let Some(reason) = cost_cap_reason(cap_usd, current_micro) {
        events.emit(
            "cost:cap-hit",
            serde_json::json!({
                "reason": reason,
                "source": source,
                "blocking": false,
            }),
        );
    }
}
