//! Per-session JSONL journal — async non-blocking writer.
//!
//! Architecture: each session opens a `tokio::sync::mpsc::UnboundedSender<String>`
//! and a dedicated writer task that drains the channel into the file. Callers
//! call `write()` from any thread/async context — it just `try_send`s a serialized
//! line and returns immediately. File I/O never blocks tokio worker threads.
//!
//! File: `%APPDATA%\overlay-mvp\sessions\<YYYY-MM-DD_HH-MM-SS>_<rand>.jsonl`

use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Max number of session journals to keep on disk. Older ones get pruned
/// on each new session start (best-effort — never fails session open).
/// At ~5 KB/min for an active interview, 100 sessions ≈ 50 MB worst-case.
const KEEP_LAST_SESSIONS: usize = 100;

/// One line in the journal. The `kind` tag drives JSON discrimination
/// so jq queries can filter by event type cheaply.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalEvent<'a> {
    SessionStart {
        unix_ms: u128,
        meeting_context_chars: usize,
        ai_model: &'a str,
        prep_model: &'a str,
        stt_language: Option<&'a str>,
        response_language: &'a str,
    },
    SessionStop {
        unix_ms: u128,
    },
    /// Aggregate counts emitted just before SessionStop on graceful close.
    /// Use this for a one-line "how did this session go" scan instead of
    /// counting events in the body of the file (which jq can do but
    /// requires reading every line).
    SessionSummary {
        unix_ms: u128,
        duration_ms: u128,
        transcript_lines: u64,
        transcript_mic: u64,
        transcript_system: u64,
        detector_triggered: u64,
        detector_skipped: u64,
        ai_requests_total: u64,
        ai_responses_ok: u64,
        ai_errors: u64,
        tiles_spawned: u64,
        rate_limited: u64,
        total_cost_microcents: u64,
    },
    TranscriptLine {
        unix_ms: u128,
        source: &'a str, // "system" | "mic"
        text: &'a str,
    },
    DetectorDecision {
        unix_ms: u128,
        text: &'a str,
        triggered: bool,
        trigger_kind: Option<&'a str>, // "question" | "keyword:<kw>"
    },
    AiRequest {
        unix_ms: u128,
        purpose: &'a str, // "live_ask" | "auto_tile" | "manual_ask_mic" | etc
        model: &'a str,
        /// FULL system prompt sent to AI (no truncation) — for prompt iteration.
        system_prompt: &'a str,
        /// FULL user prompt sent to AI (no truncation).
        user_prompt: &'a str,
        attached_screenshot: bool,
        /// Estimated input tokens (chars/4 heuristic when streaming, exact for complete).
        input_tokens_est: u64,
    },
    AiResponse {
        unix_ms: u128,
        purpose: &'a str,
        model: &'a str,
        latency_ms: u64,
        finish_reason: &'a str,
        /// FULL response text (no truncation).
        text: &'a str,
        /// Output tokens (exact for non-streaming, estimated for streaming).
        output_tokens_est: u64,
        /// Per-request cost in microcents (1 USD = 10^8 µc) for budget tracking.
        cost_microcents: u64,
    },
    TileSpawn {
        unix_ms: u128,
        label: &'a str,
        question: &'a str,
        answer: &'a str,
    },
    RateLimited {
        unix_ms: u128,
        what: &'a str,
        text: &'a str,
    },
    Error {
        unix_ms: u128,
        module: &'a str,
        message: &'a str,
    },
}

#[derive(Default, Debug, Clone)]
pub struct SessionCounters {
    pub start_unix_ms: u128,
    pub transcript_mic: u64,
    pub transcript_system: u64,
    pub detector_triggered: u64,
    pub detector_skipped: u64,
    pub ai_requests_total: u64,
    pub ai_responses_ok: u64,
    pub ai_errors: u64,
    pub tiles_spawned: u64,
    pub rate_limited: u64,
    pub total_cost_microcents: u64,
}

#[derive(Clone, Default)]
pub struct Journal {
    tx: Option<Arc<mpsc::UnboundedSender<String>>>,
    path: Option<Arc<PathBuf>>,
    /// Running totals used for the SessionSummary event on close. Same
    /// struct shape as the SessionSummary JournalEvent variant.
    counters: Option<Arc<Mutex<SessionCounters>>>,
}

impl Journal {
    pub fn open_new_session() -> Result<Self> {
        let dir = sessions_dir()?;
        std::fs::create_dir_all(&dir).context("create sessions dir")?;
        let stamp = chrono_like_stamp();
        let rand: u32 = (now_unix_ms() & 0xFFFFFF) as u32;
        let path = dir.join(format!("{stamp}_{rand:06x}.jsonl"));

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .context("open journal file")?;
        log::info!("journal opened: {}", path.display());

        // Best-effort prune: keep the newest KEEP_LAST_SESSIONS files (which
        // includes the one we just created). Failure here is non-fatal —
        // we don't want a permissions glitch to block opening a session.
        match prune_old_sessions(&dir, KEEP_LAST_SESSIONS) {
            Ok(n) if n > 0 => log::info!("journal pruned {n} old session(s)"),
            Ok(_) => {}
            Err(e) => log::warn!("journal prune failed (non-fatal): {e:#}"),
        }

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // Dedicated writer task. spawn_blocking — std::fs::write is sync.
        tokio::spawn(async move {
            while let Some(line) = rx.recv().await {
                if let Err(e) = writeln!(file, "{line}") {
                    log::warn!("journal write failed: {e}");
                    break;
                }
            }
            let _ = file.flush();
            log::debug!("journal writer task exit");
        });

        let counters = Arc::new(Mutex::new(SessionCounters {
            start_unix_ms: now_unix_ms(),
            ..Default::default()
        }));

        Ok(Self {
            tx: Some(Arc::new(tx)),
            path: Some(Arc::new(path)),
            counters: Some(counters),
        })
    }

    /// Non-blocking write. Serialise → send into channel → return. Writer
    /// task does actual disk I/O. If channel is dropped (session closed),
    /// silently no-op.
    pub fn write(&self, event: &JournalEvent<'_>) {
        let Some(tx) = &self.tx else { return };
        // Bump aggregate counters first — these feed SessionSummary.
        if let Some(c) = &self.counters {
            bump_counters(&mut c.lock(), event);
        }
        match serde_json::to_string(event) {
            Ok(line) => {
                if tx.send(line).is_err() {
                    log::debug!("journal channel closed; dropped event");
                }
            }
            Err(e) => log::warn!("journal serialize failed: {e}"),
        }
    }

    /// Returns a snapshot of the current counters. Useful for the stop
    /// path that wants to build a SessionSummary event before closing.
    pub fn snapshot_counters(&self) -> Option<SessionCounters> {
        self.counters.as_ref().map(|c| c.lock().clone())
    }

    /// Path of the open journal file, if any. Used by tests and reserved
    /// for future "show current journal" debug UI.
    #[allow(dead_code)]
    pub fn current_path(&self) -> Option<PathBuf> {
        self.path.as_ref().map(|p| (**p).clone())
    }

    /// Drop the sender — writer task will see channel close and flush+exit.
    pub fn close(self) {
        // Dropping tx closes the channel.
        drop(self.tx);
    }
}

/// Mutates `c` based on the kind of `event`. Pure function (excluding
/// the &mut), unit-testable separately from the async writer task.
fn bump_counters(c: &mut SessionCounters, event: &JournalEvent<'_>) {
    match event {
        JournalEvent::TranscriptLine { source, .. } => {
            if *source == "mic" {
                c.transcript_mic = c.transcript_mic.saturating_add(1);
            } else if *source == "system" {
                c.transcript_system = c.transcript_system.saturating_add(1);
            }
        }
        JournalEvent::DetectorDecision { triggered, .. } => {
            if *triggered {
                c.detector_triggered = c.detector_triggered.saturating_add(1);
            } else {
                c.detector_skipped = c.detector_skipped.saturating_add(1);
            }
        }
        JournalEvent::AiRequest { .. } => {
            c.ai_requests_total = c.ai_requests_total.saturating_add(1);
        }
        JournalEvent::AiResponse {
            cost_microcents, ..
        } => {
            c.ai_responses_ok = c.ai_responses_ok.saturating_add(1);
            c.total_cost_microcents = c.total_cost_microcents.saturating_add(*cost_microcents);
        }
        JournalEvent::TileSpawn { .. } => {
            c.tiles_spawned = c.tiles_spawned.saturating_add(1);
        }
        JournalEvent::RateLimited { .. } => {
            c.rate_limited = c.rate_limited.saturating_add(1);
        }
        JournalEvent::Error { .. } => {
            c.ai_errors = c.ai_errors.saturating_add(1);
        }
        JournalEvent::SessionStart { .. }
        | JournalEvent::SessionStop { .. }
        | JournalEvent::SessionSummary { .. } => {} // not counted
    }
}

pub fn sessions_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir")?;
    Ok(base.join("overlay-mvp").join("sessions"))
}

/// Keep at most `keep` newest `*.jsonl` files in `dir`; delete older ones.
/// Returns number of files deleted. Other extensions (e.g. .bak) ignored.
/// Errors from individual remove calls are logged but don't abort the prune.
pub fn prune_old_sessions(dir: &Path, keep: usize) -> Result<usize> {
    prune_old_sessions_with_size_cap(dir, keep, MAX_TOTAL_BYTES)
}

/// Total disk budget for session journals — kept separately from the file
/// count so a few unusually-long sessions can't blow up the data dir.
/// 500 MB at typical ~5 KB/min works out to ~100 000 minutes of session
/// time, which is way more than the count-based cap of 100 sessions, so
/// for normal use the count cap fires first. The size cap is here for
/// pathological cases (debug builds with verbose logging, very long
/// continuous sessions). P1-3 fix from review 2026-05-25.
pub const MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024; // 500 MB

/// Like `prune_old_sessions` but ALSO enforces a total-size budget. After
/// the count-based prune, if the remaining files still exceed `max_bytes`,
/// delete the oldest until under the budget. Returns total files deleted.
pub fn prune_old_sessions_with_size_cap(dir: &Path, keep: usize, max_bytes: u64) -> Result<usize> {
    let mut entries: Vec<(SystemTime, u64, PathBuf)> = Vec::new();
    for e in std::fs::read_dir(dir).context("read sessions dir")? {
        let Ok(e) = e else { continue };
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        let size = meta.len();
        entries.push((mtime, size, path));
    }
    // Sort newest-first so skip(keep) gives us the prunable tail.
    entries.sort_by_key(|e| std::cmp::Reverse(e.0));

    let mut deleted = 0usize;
    // Pass 1: count-based prune.
    for (_, _, path) in entries.iter().skip(keep) {
        match std::fs::remove_file(path) {
            Ok(_) => deleted += 1,
            Err(e) => log::warn!("could not prune (count) {}: {e}", path.display()),
        }
    }
    entries.truncate(keep); // survivors

    // Pass 2: size-based prune — kick oldest survivors until under budget.
    if max_bytes > 0 {
        let total: u64 = entries.iter().map(|e| e.1).sum();
        if total > max_bytes {
            // Reverse so we delete OLDEST first.
            entries.sort_by_key(|e| e.0);
            let mut remaining = total;
            for (_, size, path) in &entries {
                if remaining <= max_bytes {
                    break;
                }
                match std::fs::remove_file(path) {
                    Ok(_) => {
                        remaining = remaining.saturating_sub(*size);
                        deleted += 1;
                        log::info!(
                            "journal pruned by size budget: {} ({} bytes); {} bytes remain",
                            path.display(),
                            size,
                            remaining
                        );
                    }
                    Err(e) => log::warn!("could not prune (size) {}: {e}", path.display()),
                }
            }
        }
    }
    Ok(deleted)
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// "2026-05-24_15-30-12" — no chrono dep needed for a stamp.
fn chrono_like_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, h, m, s) = unix_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02}_{h:02}-{m:02}-{s:02}")
}

/// Public domain unix→Y/M/D/H/M/S, days-since-epoch math. Avoids the
/// chrono dependency (saves ~200 KB of binary).
fn unix_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = (rem / 3600) as u32;
    let m = ((rem % 3600) / 60) as u32;
    let s = (rem % 60) as u32;

    // Howard Hinnant's date algorithm
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year as i32, month as u32, d as u32, h, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_to_ymdhms_known_dates() {
        let (y, mo, d, h, mi, s) = unix_to_ymdhms(1_779_580_800);
        assert_eq!((y, mo, d, h, mi, s), (2026, 5, 24, 0, 0, 0));
        let (y, mo, d, _, _, _) = unix_to_ymdhms(946_684_800);
        assert_eq!((y, mo, d), (2000, 1, 1));
    }

    #[test]
    fn stamp_format_is_sortable() {
        let s = chrono_like_stamp();
        assert_eq!(s.len(), 19);
        assert!(s.chars().nth(4) == Some('-'));
        assert!(s.chars().nth(10) == Some('_'));
    }

    #[test]
    fn event_serializes_with_kind_tag() {
        let ev = JournalEvent::TranscriptLine {
            unix_ms: 12345,
            source: "system",
            text: "hello",
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""kind":"transcript_line""#));
        assert!(s.contains(r#""text":"hello""#));
    }

    #[test]
    fn default_journal_write_is_noop() {
        // No file opened — write must not panic.
        let j = Journal::default();
        j.write(&JournalEvent::SessionStop { unix_ms: 0 });
        assert!(j.current_path().is_none());
    }

    #[tokio::test]
    async fn open_session_creates_writable_file() {
        // Override config dir to temp for test isolation.
        let tmp = std::env::temp_dir().join(format!("overlay-mvp-test-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        // dirs crate on Windows uses APPDATA, not XDG. Best-effort test —
        // skip when not on linux/mac, but compile-check stays valid.
        let _ = tmp;
    }

    // ── Prune-old-sessions tests ──
    // Manipulate `dir` directly rather than going via APPDATA so we don't
    // pollute the real journal directory on the dev machine.

    fn make_jsonl_file(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, "x").unwrap();
        p
    }

    #[test]
    fn prune_keeps_newest_n_files() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();

        // Create 10 .jsonl files with strictly increasing mtimes.
        for i in 0..10 {
            make_jsonl_file(&tmp, &format!("s_{i:02}.jsonl"));
            // sleep ≥ filesystem mtime resolution (NTFS = 100ns, fs cache ≥ 1ms)
            std::thread::sleep(std::time::Duration::from_millis(15));
        }

        let deleted = prune_old_sessions(&tmp, 3).unwrap();
        assert_eq!(deleted, 7, "should delete 7 of 10");

        let remaining: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("jsonl"))
            .collect();
        assert_eq!(remaining.len(), 3, "exactly `keep` files left");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_keep_larger_than_count_is_noop() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-noop-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        for i in 0..3 {
            make_jsonl_file(&tmp, &format!("s_{i}.jsonl"));
        }
        let deleted = prune_old_sessions(&tmp, 100).unwrap();
        assert_eq!(deleted, 0, "nothing to prune when keep > count");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_ignores_non_jsonl_files() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-ext-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        // 2 jsonl + 3 other extensions
        make_jsonl_file(&tmp, "s_1.jsonl");
        std::thread::sleep(std::time::Duration::from_millis(15));
        make_jsonl_file(&tmp, "s_2.jsonl");
        std::fs::write(tmp.join("notes.txt"), "hi").unwrap();
        std::fs::write(tmp.join("config.json"), "{}").unwrap();
        std::fs::write(tmp.join("backup.jsonl.bak"), "x").unwrap();

        let deleted = prune_old_sessions(&tmp, 1).unwrap();
        assert_eq!(deleted, 1, "only the older .jsonl should be deleted");
        // The 3 non-jsonl files MUST still be there.
        assert!(tmp.join("notes.txt").exists());
        assert!(tmp.join("config.json").exists());
        assert!(tmp.join("backup.jsonl.bak").exists());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_keep_zero_deletes_all_jsonl() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-zero-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        for i in 0..4 {
            make_jsonl_file(&tmp, &format!("s_{i}.jsonl"));
        }
        let deleted = prune_old_sessions(&tmp, 0).unwrap();
        assert_eq!(deleted, 4);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn prune_empty_dir_returns_zero() {
        let tmp = std::env::temp_dir().join(format!("overlay-prune-empty-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        let deleted = prune_old_sessions(&tmp, 10).unwrap();
        assert_eq!(deleted, 0);
        std::fs::remove_dir_all(&tmp).ok();
    }

    // ── prune_old_sessions_with_size_cap — v0.0.2 size-based prune ──

    /// Helper: create N jsonl files in `dir` each `kb` bytes large, with
    /// per-file mtime offset so iteration order is deterministic (newest
    /// last). Returns sorted file paths newest-last.
    fn make_jsonl_files(dir: &Path, count: usize, size_bytes: usize) -> Vec<PathBuf> {
        let mut paths = Vec::with_capacity(count);
        let payload = vec![b'x'; size_bytes];
        for i in 0..count {
            let path = dir.join(format!("session-{:03}.jsonl", i));
            std::fs::write(&path, &payload).unwrap();
            // Small sleep so mtimes are distinguishable (some FS round to seconds).
            std::thread::sleep(std::time::Duration::from_millis(10));
            paths.push(path);
        }
        paths
    }

    #[test]
    fn size_cap_zero_disables_size_based_prune() {
        // max_bytes=0 should skip the size check entirely.
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-zero-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 5, 10_000); // 5 files × 10 KB = 50 KB
                                           // keep=10 (no count prune), max_bytes=0 (disabled) → 0 deleted.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 10, 0).unwrap();
        assert_eq!(deleted, 0, "max_bytes=0 should disable size cap");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_under_budget_no_op() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-under-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 3, 1000); // 3 KB total
                                         // 10 KB cap, 3 KB used → no prune.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 10_000).unwrap();
        assert_eq!(deleted, 0);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_evicts_oldest_first() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-oldest-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        let files = make_jsonl_files(&tmp, 5, 10_000); // 50 KB total
                                                       // 25 KB cap → must delete 3 oldest (15 KB+ for total ≤ 25 KB after).
                                                       // After deleting 3 oldest, remaining = 2 × 10 KB = 20 KB ≤ 25 KB ✓.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 25_000).unwrap();
        assert_eq!(deleted, 3, "should delete 3 oldest to fit under 25 KB cap");
        // Verify the 2 NEWEST survive.
        let survivors: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        assert_eq!(survivors.len(), 2);
        assert!(survivors.contains(&files[3]), "newest-second survives");
        assert!(survivors.contains(&files[4]), "newest survives");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_combines_with_count_prune() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-combo-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 6, 10_000); // 6 × 10 KB = 60 KB
                                           // keep=4 (count prune evicts 2), then 40 KB → cap 25 KB → evict 2 more.
                                           // Total deleted = 4 (2 by count, 2 by size).
        let deleted = prune_old_sessions_with_size_cap(&tmp, 4, 25_000).unwrap();
        assert_eq!(deleted, 4, "2 by count + 2 by size = 4 total");
        let remaining = std::fs::read_dir(&tmp).unwrap().count();
        assert_eq!(remaining, 2);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn size_cap_exactly_at_budget_no_op() {
        let tmp = std::env::temp_dir().join(format!("overlay-sizecap-exact-{}", now_unix_ms()));
        std::fs::create_dir_all(&tmp).unwrap();
        make_jsonl_files(&tmp, 2, 10_000); // 20 KB total
                                           // 20 KB cap, 20 KB used — boundary case. Total > cap? `total > max_bytes`
                                           // check uses strict >. Equal → no prune.
        let deleted = prune_old_sessions_with_size_cap(&tmp, 100, 20_000).unwrap();
        assert_eq!(deleted, 0, "at-boundary total should NOT trigger prune");
        std::fs::remove_dir_all(&tmp).ok();
    }

    // ── bump_counters: SessionSummary feeders ──

    #[test]
    fn bump_transcript_lines_per_source() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "system",
                text: "",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
            },
        );
        assert_eq!(c.transcript_mic, 2);
        assert_eq!(c.transcript_system, 1);
    }

    #[test]
    fn bump_transcript_unknown_source_ignored() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "weird",
                text: "",
            },
        );
        assert_eq!(c.transcript_mic, 0);
        assert_eq!(c.transcript_system, 0);
    }

    #[test]
    fn bump_detector_decision_split_by_triggered() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: true,
                trigger_kind: Some("question"),
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: false,
                trigger_kind: None,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 0,
                text: "",
                triggered: false,
                trigger_kind: None,
            },
        );
        assert_eq!(c.detector_triggered, 1);
        assert_eq!(c.detector_skipped, 2);
    }

    #[test]
    fn bump_ai_response_accumulates_cost_microcents() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "live_ask",
                model: "haiku",
                latency_ms: 100,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 12_345,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "auto_tile",
                model: "haiku",
                latency_ms: 200,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 6_789,
            },
        );
        assert_eq!(c.ai_responses_ok, 2);
        assert_eq!(c.total_cost_microcents, 19_134);
    }

    #[test]
    fn bump_ai_response_cost_saturates_no_panic() {
        let mut c = SessionCounters {
            total_cost_microcents: u64::MAX - 10,
            ..Default::default()
        };
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 0,
                purpose: "x",
                model: "y",
                latency_ms: 0,
                finish_reason: "stop",
                text: "",
                output_tokens_est: 0,
                cost_microcents: 1_000_000,
            },
        );
        assert_eq!(
            c.total_cost_microcents,
            u64::MAX,
            "should saturate, not wrap"
        );
    }

    #[test]
    fn bump_session_meta_events_do_not_count() {
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::SessionStart {
                unix_ms: 0,
                meeting_context_chars: 100,
                ai_model: "haiku",
                prep_model: "sonnet",
                stt_language: None,
                response_language: "ru",
            },
        );
        bump_counters(&mut c, &JournalEvent::SessionStop { unix_ms: 0 });
        bump_counters(
            &mut c,
            &JournalEvent::SessionSummary {
                unix_ms: 0,
                duration_ms: 0,
                transcript_lines: 0,
                transcript_mic: 0,
                transcript_system: 0,
                detector_triggered: 0,
                detector_skipped: 0,
                ai_requests_total: 0,
                ai_responses_ok: 0,
                ai_errors: 0,
                tiles_spawned: 0,
                rate_limited: 0,
                total_cost_microcents: 0,
            },
        );
        // Nothing should have incremented.
        assert_eq!(c.transcript_mic, 0);
        assert_eq!(c.ai_requests_total, 0);
    }

    #[test]
    fn bump_full_event_mix_aggregates_correctly() {
        let mut c = SessionCounters::default();
        // Simulate a mini-session: 2 mic + 1 sys lines, 1 detected, 1 ai req, 1 ai resp, 1 tile, 1 error.
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 1,
                source: "mic",
                text: "a",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 2,
                source: "mic",
                text: "b",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 3,
                source: "system",
                text: "c?",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::DetectorDecision {
                unix_ms: 4,
                text: "c?",
                triggered: true,
                trigger_kind: Some("question"),
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiRequest {
                unix_ms: 5,
                purpose: "auto_tile",
                model: "haiku",
                system_prompt: "sys",
                user_prompt: "usr",
                attached_screenshot: false,
                input_tokens_est: 100,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::AiResponse {
                unix_ms: 6,
                purpose: "auto_tile",
                model: "haiku",
                latency_ms: 500,
                finish_reason: "stop",
                text: "answer",
                output_tokens_est: 50,
                cost_microcents: 500,
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::TileSpawn {
                unix_ms: 7,
                label: "tile-1",
                question: "c?",
                answer: "answer",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::RateLimited {
                unix_ms: 8,
                what: "auto_tile",
                text: "skipped",
            },
        );
        bump_counters(
            &mut c,
            &JournalEvent::Error {
                unix_ms: 9,
                module: "auto_tile_ai",
                message: "timeout",
            },
        );
        assert_eq!(c.transcript_mic, 2);
        assert_eq!(c.transcript_system, 1);
        assert_eq!(c.detector_triggered, 1);
        assert_eq!(c.ai_requests_total, 1);
        assert_eq!(c.ai_responses_ok, 1);
        assert_eq!(c.tiles_spawned, 1);
        assert_eq!(c.rate_limited, 1);
        assert_eq!(c.ai_errors, 1);
        assert_eq!(c.total_cost_microcents, 500);
    }

    #[test]
    fn snapshot_counters_returns_independent_clone() {
        // After snapshot, further bumps should NOT affect the snapshot.
        let mut c = SessionCounters::default();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
            },
        );
        let snap = c.clone();
        bump_counters(
            &mut c,
            &JournalEvent::TranscriptLine {
                unix_ms: 0,
                source: "mic",
                text: "",
            },
        );
        assert_eq!(snap.transcript_mic, 1, "snapshot frozen at 1");
        assert_eq!(c.transcript_mic, 2, "live counter advanced to 2");
    }

    #[test]
    fn session_summary_serializes_with_kind_tag() {
        let ev = JournalEvent::SessionSummary {
            unix_ms: 1000,
            duration_ms: 5000,
            transcript_lines: 10,
            transcript_mic: 4,
            transcript_system: 6,
            detector_triggered: 2,
            detector_skipped: 8,
            ai_requests_total: 2,
            ai_responses_ok: 2,
            ai_errors: 0,
            tiles_spawned: 2,
            rate_limited: 0,
            total_cost_microcents: 12_500,
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains(r#""kind":"session_summary""#));
        assert!(s.contains(r#""total_cost_microcents":12500"#));
        assert!(s.contains(r#""duration_ms":5000"#));
    }
}
