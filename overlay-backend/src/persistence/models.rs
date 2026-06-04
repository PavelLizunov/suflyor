//! Plain row structs for the session catalog. No SQL / no rusqlite types leak
//! out of `persistence` through these — callers (UI, orchestration) see only
//! these owned values.

/// One indexed session — a projection of a JSONL journal file's lifecycle
/// events (`session_start` / `session_stop` / `session_summary`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// JSONL file stem (e.g. `2026-06-04_10-00-00_ab12`) — stable id.
    pub id: String,
    /// Absolute path to the source `.jsonl` (so the raw events stay reachable).
    pub journal_path: String,
    /// unix_ms of `session_start`, if the file had one.
    pub started_at_ms: Option<i64>,
    /// unix_ms of `session_stop`; `None` when the session crashed / is active.
    pub finished_at_ms: Option<i64>,
    /// `completed` | `crashed` | `active`.
    pub status: String,
    pub ai_model: Option<String>,
    pub transcript_lines: i64,
    pub ai_turns_count: i64,
    pub total_cost_microcents: i64,
    /// When this row was last (re)indexed (unix_ms).
    pub indexed_at_ms: i64,
}

/// One transcript line (mic or system audio) within a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utterance {
    pub session_id: String,
    pub unix_ms: i64,
    /// `mic` | `system`.
    pub source: String,
    pub text: String,
}

/// One AI question→answer turn within a session (an `ai_request` paired with
/// its `ai_response`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiTurn {
    pub session_id: String,
    pub unix_ms: i64,
    pub purpose: String,
    pub model: String,
    pub question: String,
    pub answer: String,
    pub latency_ms: Option<i64>,
    pub attached_screenshot: bool,
}

/// One full-text search hit — a matching utterance or AI question/answer, the
/// session it belongs to, and its BM25 rank (LOWER = more relevant, SQLite's
/// convention).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub session_id: String,
    /// `utterance` | `question` | `answer`.
    pub kind: String,
    pub unix_ms: i64,
    pub body: String,
    pub rank: f64,
}
