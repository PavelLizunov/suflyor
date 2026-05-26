//! Pure-Rust data layer for the Replay viewer pilot.
//!
//! Day 2 of Phase 0 — this module replicates `journal::sessions_dir`,
//! `list_sessions`, and `load_session` from `src-tauri/src/lib.rs`
//! without pulling in any Tauri code. The migration plan asks for a
//! shared `replay_backend` module callable from both the Tauri command
//! handlers AND the Slint pilot; pulling overlay_mvp_lib into the
//! pilot crate would drag Tauri + WebView2 along, which defeats the
//! pilot's goal of standing up Slint independently.
//!
//! Phase 1 (post-pilot) is expected to extract this into a tiny shared
//! crate (e.g. `crates/journal-core/`) that both src-tauri AND the new
//! `src-rs/` UI controller depend on. For now, the duplication is
//! contained to ~80 lines and the canonical implementation in src-tauri
//! stays the source of truth — if its signature drifts, this module
//! follows.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// `%APPDATA%\overlay-mvp\sessions\` on Windows. Mirrors
/// `journal::sessions_dir()` in src-tauri.
pub fn sessions_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir")?;
    Ok(base.join("overlay-mvp").join("sessions"))
}

/// One row in the session combobox.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub modified_unix: u64,
}

/// List all `.jsonl` session journals, newest first.
pub fn list_sessions() -> Result<Vec<SessionInfo>> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let read = std::fs::read_dir(&dir).context("read sessions dir")?;
    let mut out: Vec<SessionInfo> = read
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let size_bytes = meta.len();
            let modified_unix = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            Some(SessionInfo {
                path: path.to_string_lossy().to_string(),
                filename,
                size_bytes,
                modified_unix,
            })
        })
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.modified_unix));
    Ok(out)
}

/// Read a JSONL session journal, parse each line as a Value.
/// Rejects files >10 MB and paths outside the sessions dir.
pub fn load_session(path: &Path) -> Result<Vec<serde_json::Value>> {
    const MAX_BYTES: u64 = 10 * 1024 * 1024;

    let sessions = sessions_dir()?
        .canonicalize()
        .context("canonicalize sessions dir")?;
    let canonical = path
        .canonicalize()
        .context("canonicalize session path")?;
    if !canonical.starts_with(&sessions) {
        anyhow::bail!("path is outside sessions dir");
    }

    let meta = std::fs::metadata(&canonical).context("stat session file")?;
    if meta.len() > MAX_BYTES {
        anyhow::bail!(
            "session file too large ({} bytes, max {MAX_BYTES})",
            meta.len()
        );
    }

    let content = std::fs::read_to_string(&canonical).context("read session file")?;
    let mut events = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(v) => events.push(v),
            Err(e) => eprintln!("[slint-replay] skip malformed line: {e}"),
        }
    }
    Ok(events)
}

// ===== Presentation helpers =====
//
// Mirrors the per-kind formatting from src/Replay.tsx ReplayRow().
// Kept here (data layer) rather than main.rs so future iterations
// can share via a presentation crate; today both sides would just
// import these from `replay_backend`.

/// Strip whitespace + collapse runs of spaces + truncate to N chars
/// with an ellipsis. Mirrors React's `preview()`.
pub fn preview(s: &str, n: usize) -> String {
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > n {
        let head: String = collapsed.chars().take(n).collect();
        format!("{head}…")
    } else {
        collapsed
    }
}

/// HH:MM:SS in UTC. The React version uses local time via `new Date()`;
/// for the pilot we stay in UTC to avoid a chrono/time dep. Will swap
/// to local-tz in Phase 1 with the `time` crate (smaller than chrono).
pub fn fmt_clock(unix_ms: Option<u64>) -> String {
    match unix_ms {
        Some(ms) => {
            let secs = ms / 1000;
            let h = (secs / 3600) % 24;
            let m = (secs / 60) % 60;
            let s = secs % 60;
            format!("{h:02}:{m:02}:{s:02}")
        }
        None => "--:--:--".to_string(),
    }
}

/// Helpers for serde_json::Value field extraction. Mirrors React's
/// `asStr` / `asNum` / `asBool`.
fn ev_str(ev: &serde_json::Value, key: &str) -> String {
    ev.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn ev_u64(ev: &serde_json::Value, key: &str) -> Option<u64> {
    ev.get(key).and_then(serde_json::Value::as_u64)
}

fn ev_f64(ev: &serde_json::Value, key: &str) -> Option<f64> {
    ev.get(key).and_then(serde_json::Value::as_f64)
}

fn ev_bool(ev: &serde_json::Value, key: &str) -> bool {
    ev.get(key)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Cost in USD from a journal event. Reads `cost_microcents` (newer
/// format) or falls back to `cost_usd` (legacy). Mirrors React's
/// `eventCost()`.
fn event_cost_usd(ev: &serde_json::Value) -> f64 {
    if let Some(micro) = ev_u64(ev, "cost_microcents") {
        return (micro as f64) / 100_000_000.0;
    }
    ev_f64(ev, "cost_usd").unwrap_or(0.0)
}

/// Per-kind row label + body for the timeline. Mirrors the giant
/// switch in `ReplayRow()` in src/Replay.tsx — keeps the kinds we
/// care about and falls back to JSON dump for unknown kinds.
pub fn render_event(ev: &serde_json::Value) -> (String, String) {
    let kind = ev_str(ev, "kind");
    match kind.as_str() {
        "session_start" => {
            let model = ev_str(ev, "ai_model");
            let prep = ev_str(ev, "prep_model");
            let ctx = ev_u64(ev, "meeting_context_chars").unwrap_or(0);
            let lang = ev_str(ev, "response_language");
            let mut body = format!("model={model} ctx_chars={ctx}");
            if !prep.is_empty() {
                body.push_str(&format!(" prep={prep}"));
            }
            if !lang.is_empty() {
                body.push_str(&format!(" lang={lang}"));
            }
            ("SESSION START".to_string(), body)
        }
        "session_stop" => ("SESSION STOP".to_string(), String::new()),
        "session_summary" => {
            let dur_min = ev_u64(ev, "duration_ms").unwrap_or(0) as f64 / 60_000.0;
            let lines = ev_u64(ev, "transcript_lines").unwrap_or(0);
            let mic = ev_u64(ev, "transcript_mic").unwrap_or(0);
            let sys = ev_u64(ev, "transcript_system").unwrap_or(0);
            let trig = ev_u64(ev, "detector_triggered").unwrap_or(0);
            let skip = ev_u64(ev, "detector_skipped").unwrap_or(0);
            let reqs = ev_u64(ev, "ai_requests_total").unwrap_or(0);
            let tiles = ev_u64(ev, "tiles_spawned").unwrap_or(0);
            let errs = ev_u64(ev, "ai_errors").unwrap_or(0);
            let rl = ev_u64(ev, "rate_limited").unwrap_or(0);
            let cost = ev_u64(ev, "total_cost_microcents").unwrap_or(0) as f64 / 100_000_000.0;
            let mut tail = format!("{reqs} AI · {tiles} tiles");
            if rl > 0 {
                tail.push_str(&format!(" · {rl} rate-limited"));
            }
            if errs > 0 {
                tail.push_str(&format!(" · {errs} errors"));
            }
            (
                "SUMMARY".to_string(),
                format!(
                    "{dur_min:.1} min · {lines} lines ({mic}🎤 · {sys}🗣) · detector: {trig}/{} · {tail} · ${cost:.4}",
                    trig + skip
                ),
            )
        }
        "transcript_line" => {
            let src = ev_str(ev, "source");
            let icon = if src == "mic" { "🎤" } else { "🗣" };
            // React renders transcript_line text FULL (no preview cap) so the
            // user can read complete utterances without scrolling into the
            // event JSON. Pilot does the same. Whitespace already normalized
            // by the journal writer.
            (format!("{icon} {src}"), ev_str(ev, "text"))
        }
        "detector_decision" => {
            let triggered = ev_bool(ev, "triggered");
            let text = ev_str(ev, "text");
            let trig_kind = ev_str(ev, "trigger_kind");
            let label = if triggered { "DETECT ✓" } else { "detect" };
            let reason = if triggered {
                format!("→ {}", if trig_kind.is_empty() { "trigger" } else { &trig_kind })
            } else {
                "no trigger".to_string()
            };
            (label.to_string(), format!("{} {}", preview(&text, 200), reason))
        }
        "ai_request" => {
            let purpose = ev_str(ev, "purpose");
            let model = ev_str(ev, "model");
            let prompt = {
                let full = ev_str(ev, "user_prompt");
                if full.is_empty() {
                    ev_str(ev, "user_prompt_preview")
                } else {
                    full
                }
            };
            let tokens_est = ev_u64(ev, "input_tokens_est");
            let mut head = model.clone();
            if let Some(t) = tokens_est {
                head.push_str(&format!(" · ~{t} in-tok"));
            }
            if ev_bool(ev, "attached_screenshot") {
                head.push_str(" · 📎");
            }
            (
                format!("AI REQ · {purpose}"),
                format!("{head} · {}", preview(&prompt, 240)),
            )
        }
        "ai_response" => {
            let purpose = ev_str(ev, "purpose");
            let latency = ev_u64(ev, "latency_ms");
            let finish = ev_str(ev, "finish_reason");
            let text = ev_str(ev, "text");
            let cost = event_cost_usd(ev);
            let mut head = String::new();
            if let Some(l) = latency {
                head.push_str(&format!("{l} ms"));
            }
            if !finish.is_empty() {
                if !head.is_empty() {
                    head.push_str(" · ");
                }
                head.push_str(&format!("finish={finish}"));
            }
            if cost > 0.0 {
                if !head.is_empty() {
                    head.push_str(" · ");
                }
                head.push_str(&format!("${cost:.4}"));
            }
            (
                format!("AI RESP · {purpose}"),
                format!("{head} · {}", preview(&text, 400)),
            )
        }
        "tile_spawn" => {
            let q = ev_str(ev, "question");
            let a = ev_str(ev, "answer");
            let is_translated = q.starts_with("🇷🇺") || q.starts_with("🇬🇧");
            let label = if is_translated { "TILE · 🌐" } else { "TILE" };
            (
                label.to_string(),
                format!("{} · {}", preview(&q, 80), preview(&a, 100)),
            )
        }
        "rate_limited" => {
            let what = ev_str(ev, "what");
            let text = ev_str(ev, "text");
            (
                format!("RATE LIMITED · {what}"),
                preview(&text, 240),
            )
        }
        "error" => {
            // React renders error.message in FULL (no preview) so stack
            // traces and long error text remain visible. Pilot matches.
            let module = ev_str(ev, "module");
            let message = ev_str(ev, "message");
            (format!("ERROR · {module}"), message)
        }
        other => {
            let body = serde_json::to_string(ev).unwrap_or_else(|_| "{}".to_string());
            (
                if other.is_empty() { "unknown".to_string() } else { other.to_string() },
                preview(&body, 200),
            )
        }
    }
}

/// Cumulative cost across all `ai_response` events in USD.
pub fn total_cost_usd(events: &[serde_json::Value]) -> (f64, u64) {
    let mut sum = 0.0;
    let mut count = 0;
    for e in events {
        if ev_str(e, "kind") == "ai_response" {
            sum += event_cost_usd(e);
            count += 1;
        }
    }
    (sum, count)
}
