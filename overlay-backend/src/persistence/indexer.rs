//! Idempotent JSONL → SQLite indexer. Reads the append-only journals (the source
//! of truth) and projects them into the catalog. Re-running is safe: each session
//! is replaced wholesale by id (the file stem), so re-indexing never duplicates.
//!
//! Parsing is done over `serde_json::Value` (not the borrowed `JournalEvent`
//! enum) so an OLD journal with missing/extra fields, or a corrupt line, never
//! breaks indexing — unknown kinds + unparseable lines are skipped.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::Path;

use super::models::{AiTurn, Session, Utterance};
use super::sqlite_store::Store;

/// Outcome of an [`index_all`] sweep.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IndexStats {
    pub scanned: usize,
    pub indexed: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Index ONE journal file into the catalog (replacing any prior rows for it).
/// Returns the indexed [`Session`].
pub fn index_journal_file(store: &mut Store, path: &Path) -> Result<Session> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    if id.is_empty() {
        bail!("journal path has no usable file stem: {}", path.display());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("read journal {}", path.display()))?;

    let mut started_at_ms = None;
    let mut finished_at_ms = None;
    let mut ai_model = None;
    let mut utterances: Vec<Utterance> = Vec::new();
    let mut ai_turns: Vec<AiTurn> = Vec::new();
    let mut cost_sum: i64 = 0;
    // An ai_request awaiting its ai_response, so a turn carries both the
    // question and the answer.
    let mut pending: Option<AiTurn> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue; // skip a corrupt line, keep indexing the rest
        };
        match v.get("kind").and_then(Value::as_str).unwrap_or_default() {
            "session_start" => {
                started_at_ms = num(&v, "unix_ms");
                let m = text(&v, "ai_model");
                if !m.is_empty() {
                    ai_model = Some(m);
                }
            }
            "session_stop" => finished_at_ms = num(&v, "unix_ms"),
            "transcript_line" => utterances.push(Utterance {
                session_id: id.clone(),
                unix_ms: num(&v, "unix_ms").unwrap_or(0),
                source: text(&v, "source"),
                text: text(&v, "text"),
            }),
            "ai_request" => {
                // A new request before the prior one got a response → flush the
                // prior as an answer-less (errored / incomplete) turn.
                if let Some(p) = pending.take() {
                    ai_turns.push(p);
                }
                pending = Some(AiTurn {
                    session_id: id.clone(),
                    unix_ms: num(&v, "unix_ms").unwrap_or(0),
                    purpose: text(&v, "purpose"),
                    model: text(&v, "model"),
                    question: text(&v, "user_prompt"),
                    answer: String::new(),
                    latency_ms: None,
                    attached_screenshot: flag(&v, "attached_screenshot"),
                });
            }
            "ai_response" => {
                cost_sum = cost_sum.saturating_add(num(&v, "cost_microcents").unwrap_or(0));
                let answer = text(&v, "text");
                let latency = num(&v, "latency_ms");
                match pending.take() {
                    Some(mut p) => {
                        p.answer = answer;
                        p.latency_ms = latency;
                        if p.model.is_empty() {
                            p.model = text(&v, "model");
                        }
                        ai_turns.push(p);
                    }
                    None => ai_turns.push(AiTurn {
                        session_id: id.clone(),
                        unix_ms: num(&v, "unix_ms").unwrap_or(0),
                        purpose: text(&v, "purpose"),
                        model: text(&v, "model"),
                        question: String::new(),
                        answer,
                        latency_ms: latency,
                        attached_screenshot: false,
                    }),
                }
            }
            _ => {}
        }
    }
    if let Some(p) = pending.take() {
        ai_turns.push(p);
    }

    let session = Session {
        id,
        journal_path: path.display().to_string(),
        started_at_ms,
        finished_at_ms,
        // A file with no session_stop is a crashed / killed session (its file is
        // still immutable — the LIVE session is skipped by index_all).
        status: if finished_at_ms.is_some() {
            "completed".to_string()
        } else {
            "crashed".to_string()
        },
        ai_model,
        transcript_lines: utterances.len() as i64,
        ai_turns_count: ai_turns.len() as i64,
        total_cost_microcents: cost_sum,
        indexed_at_ms: now_ms(),
    };
    store.replace_session(&session, &utterances, &ai_turns)?;
    Ok(session)
}

/// Index every `*.jsonl` under `sessions_dir` that isn't already cataloged.
/// `skip_active` (the live session's id / file stem) is never indexed — its
/// file is still being written. Already-indexed (immutable) journals are
/// skipped. A single file failing is logged + counted, never aborts the sweep.
pub fn index_all(
    store: &mut Store,
    sessions_dir: &Path,
    skip_active: Option<&str>,
) -> Result<IndexStats> {
    let mut stats = IndexStats::default();
    let already = store.indexed_session_ids()?;
    let Ok(entries) = std::fs::read_dir(sessions_dir) else {
        return Ok(stats); // no sessions dir yet → nothing to index
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        stats.scanned += 1;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if stem.is_empty() || Some(stem.as_str()) == skip_active || already.contains(&stem) {
            stats.skipped += 1;
            continue;
        }
        match index_journal_file(store, &path) {
            Ok(_) => stats.indexed += 1,
            Err(e) => {
                log::warn!("catalog: index {} failed: {e:#}", path.display());
                stats.failed += 1;
            }
        }
    }
    Ok(stats)
}

fn num(v: &Value, key: &str) -> Option<i64> {
    v.get(key)
        .and_then(|x| x.as_i64().or_else(|| x.as_u64().map(|n| n as i64)))
}

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn flag(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use std::io::Write;

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        path
    }

    #[test]
    fn indexes_a_completed_session_pairing_qa() {
        let mut store = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = write_jsonl(
            dir.path(),
            "2026-06-04_10-00-00_ab12.jsonl",
            &[
                r#"{"kind":"session_start","unix_ms":1000,"ai_model":"gemma-4-E4B","prep_model":"x","response_language":"ru"}"#,
                r#"{"kind":"transcript_line","unix_ms":1100,"source":"mic","text":"what is a hash map"}"#,
                r#"{"kind":"transcript_line","unix_ms":1200,"source":"system","text":"interviewer"}"#,
                r#"{"kind":"ai_request","unix_ms":1300,"purpose":"live_ask","model":"gemma","system_prompt":"sys","user_prompt":"what is a hash map","attached_screenshot":false}"#,
                r#"{"kind":"ai_response","unix_ms":1500,"purpose":"live_ask","model":"gemma","latency_ms":200,"finish_reason":"stop","text":"a key-value structure","cost_microcents":0}"#,
                r#"{"kind":"session_stop","unix_ms":2000}"#,
            ],
        );
        let s = index_journal_file(&mut store, &path).unwrap();
        assert_eq!(s.id, "2026-06-04_10-00-00_ab12");
        assert_eq!(s.status, "completed");
        assert_eq!(s.started_at_ms, Some(1000));
        assert_eq!(s.finished_at_ms, Some(2000));
        assert_eq!(s.ai_model.as_deref(), Some("gemma-4-E4B"));
        assert_eq!(s.transcript_lines, 2);
        assert_eq!(s.ai_turns_count, 1);

        let turns = store.session_ai_turns(&s.id).unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].question, "what is a hash map");
        assert_eq!(turns[0].answer, "a key-value structure");
        assert_eq!(turns[0].latency_ms, Some(200));
    }

    #[test]
    fn missing_session_stop_is_crashed() {
        let mut store = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = write_jsonl(
            dir.path(),
            "crashed_sess.jsonl",
            &[
                r#"{"kind":"session_start","unix_ms":10,"ai_model":"m","prep_model":"p","response_language":"ru"}"#,
            ],
        );
        let s = index_journal_file(&mut store, &path).unwrap();
        assert_eq!(s.status, "crashed");
        assert!(s.finished_at_ms.is_none());
    }

    #[test]
    fn corrupt_line_is_skipped_not_fatal() {
        let mut store = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = write_jsonl(
            dir.path(),
            "sess.jsonl",
            &[
                "not json at all",
                r#"{"kind":"transcript_line","unix_ms":1,"source":"mic","text":"hi"}"#,
                r#"{"kind":"session_stop","unix_ms":2}"#,
            ],
        );
        let s = index_journal_file(&mut store, &path).unwrap();
        assert_eq!(s.transcript_lines, 1);
        assert_eq!(s.status, "completed");
    }

    #[test]
    fn index_all_skips_already_indexed_and_active() {
        let mut store = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        write_jsonl(
            dir.path(),
            "a.jsonl",
            &[r#"{"kind":"session_stop","unix_ms":1}"#],
        );
        write_jsonl(
            dir.path(),
            "b.jsonl",
            &[r#"{"kind":"session_stop","unix_ms":1}"#],
        );
        write_jsonl(
            dir.path(),
            "live.jsonl",
            &[r#"{"kind":"session_start","unix_ms":1}"#],
        );

        // First sweep skips the live one, indexes a + b.
        let first = index_all(&mut store, dir.path(), Some("live")).unwrap();
        assert_eq!(first.scanned, 3);
        assert_eq!(first.indexed, 2);
        assert_eq!(first.skipped, 1);

        // Second sweep: a + b already indexed, live still skipped → nothing new.
        let second = index_all(&mut store, dir.path(), Some("live")).unwrap();
        assert_eq!(second.indexed, 0);
        assert_eq!(second.skipped, 3);
        assert_eq!(store.list_sessions().unwrap().len(), 2);
    }

    #[test]
    fn index_all_on_missing_dir_is_empty_not_error() {
        let mut store = Store::open_in_memory().unwrap();
        let stats = index_all(&mut store, Path::new("C:/no/such/dir/here"), None).unwrap();
        assert_eq!(stats, IndexStats::default());
    }
}
