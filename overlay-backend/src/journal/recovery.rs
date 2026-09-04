use super::time::now_unix_ms;
use super::writer::sessions_dir;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) const RECOVERY_LAST_LINES: usize = 8;
pub(crate) const RECOVERY_MAX_AGE_MS: u64 = 12 * 60 * 60 * 1000; // 12h
const RECOVERY_MAX_READ_BYTES: u64 = 16 * 1024 * 1024; // 16 MB

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnfinishedSession {
    pub session_id: String,
    pub path: PathBuf,
    pub started_unix_ms: u64,
    pub last_lines: Vec<String>,
    pub last_qa: Option<(String, String)>,
    pub summary: Option<String>,
}

#[must_use]
pub fn find_unfinished_session_in_default_dir() -> Option<UnfinishedSession> {
    let dir = sessions_dir().ok()?;
    find_unfinished_session(&dir)
}

#[must_use]
pub fn find_unfinished_session(journal_dir: &Path) -> Option<UnfinishedSession> {
    let newest = newest_jsonl(journal_dir)?;
    parse_unfinished(&newest)
}

pub(crate) fn newest_jsonl(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let Ok(e) = e else { continue };
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let Ok(mtime) = meta.modified() else { continue };
        match &best {
            Some((best_mtime, _)) if *best_mtime >= mtime => {}
            _ => best = Some((mtime, path)),
        }
    }
    best.map(|(_, p)| p)
}

pub(crate) fn parse_unfinished(path: &Path) -> Option<UnfinishedSession> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > RECOVERY_MAX_READ_BYTES {
            return None;
        }
    }
    let content = std::fs::read_to_string(path).ok()?;

    let mut started_unix_ms: Option<u64> = None;
    let mut has_start = false;
    let mut has_stop = false;
    let mut has_summary = false;
    let mut summary: Option<String> = None;
    let mut last_lines: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut pending_question: Option<String> = None;
    let mut last_qa: Option<(String, String)> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let kind = v
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        match kind {
            "session_start" => {
                has_start = true;
                if started_unix_ms.is_none() {
                    started_unix_ms = v.get("unix_ms").and_then(json_u64);
                }
            }
            "session_stop" => has_stop = true,
            "session_summary" => {
                has_summary = true;
                if let Some(s) = v
                    .get("summary")
                    .or_else(|| v.get("text"))
                    .and_then(serde_json::Value::as_str)
                {
                    if !s.trim().is_empty() {
                        summary = Some(s.trim().to_string());
                    }
                }
            }
            "transcript_line" => {
                let text = v
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                if !text.trim().is_empty() {
                    let src = v
                        .get("source")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let marker = match src {
                        "mic" => "mic: ",
                        "system" => "sys: ",
                        _ => "",
                    };
                    last_lines.push_back(format!("{marker}{}", text.trim()));
                    while last_lines.len() > RECOVERY_LAST_LINES {
                        last_lines.pop_front();
                    }
                }
            }
            "ai_request" => {
                if let Some(q) = v.get("user_prompt").and_then(serde_json::Value::as_str) {
                    if !q.trim().is_empty() {
                        pending_question = Some(q.trim().to_string());
                    }
                }
            }
            "ai_response" => {
                if let Some(ans) = v.get("text").and_then(serde_json::Value::as_str) {
                    if !ans.trim().is_empty() {
                        let q = pending_question.take().unwrap_or_default();
                        last_qa = Some((q, ans.trim().to_string()));
                    }
                }
            }
            _ => {}
        }
    }

    if !has_start || has_stop || has_summary {
        return None;
    }

    let started = started_unix_ms?;
    let now = now_unix_ms() as u64;
    if now.saturating_sub(started) > RECOVERY_MAX_AGE_MS {
        return None;
    }

    let session_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    Some(UnfinishedSession {
        session_id,
        path: path.to_path_buf(),
        started_unix_ms: started,
        last_lines: last_lines.into_iter().collect(),
        last_qa,
        summary,
    })
}

fn json_u64(v: &serde_json::Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    v.as_f64().and_then(|f| {
        if f.is_finite() && f >= 0.0 {
            Some(f as u64)
        } else {
            None
        }
    })
}
