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
    /// ms from the RECORDING start (the line's audio offset) — drives the player
    /// seek + the on-screen/copy timecodes. `None` for sessions indexed before the
    /// `audio_ms` migration (old journals didn't store it) → the caller falls back
    /// to the prev-line wall-clock approximation. (F1.)
    pub audio_ms: Option<i64>,
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

/// A suggested memory fragment mined from a session, awaiting the user's
/// approve / reject / edit (Phase 3b — curated personal memory). Only an
/// APPROVED candidate becomes a [`MemoryItem`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCandidate {
    pub id: i64,
    pub profile_id: String,
    /// The session it was mined from (`None` for a manually-added candidate).
    pub source_session_id: Option<String>,
    /// `experience` | `preference` | `answer` | `weak_topic` | `note`.
    pub kind: String,
    pub text: String,
    /// Why it was suggested — shown to the user at review time.
    pub reason: String,
    /// `pending` | `approved` | `rejected`.
    pub status: String,
    pub created_at_ms: i64,
}

/// A user-APPROVED memory item — the ONLY memory `context_builder` may mix into
/// a new AI request. User-owned + durable: survives a catalog re-index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItem {
    pub id: i64,
    pub profile_id: String,
    /// `experience` | `preference` | `answer` | `weak_topic` | `note`.
    pub kind: String,
    pub text: String,
    pub source_session_id: Option<String>,
    pub approved_at_ms: i64,
    /// `None` = active; `Some(ms)` = archived (soft-deleted — stops feeding
    /// context but stays on record).
    pub archived_at_ms: Option<i64>,
    /// `none` | `pending` | `done` (Phase 4 embeddings).
    pub embedding_status: String,
}

/// Fields for inserting a new [`MemoryCandidate`]. The store assigns `id`,
/// defaults `status` to `pending`, and stamps `created_at_ms` from the caller.
#[derive(Debug, Clone)]
pub struct NewMemoryCandidate {
    pub profile_id: String,
    pub source_session_id: Option<String>,
    pub kind: String,
    pub text: String,
    pub reason: String,
}

/// Fields for inserting a new [`MemoryItem`] directly (a manually-added note, or
/// the item minted when a candidate is approved). The store assigns `id`, stamps
/// `approved_at_ms` from the caller, and defaults `embedding_status` to `none`.
#[derive(Debug, Clone)]
pub struct NewMemoryItem {
    pub profile_id: String,
    pub kind: String,
    pub text: String,
    pub source_session_id: Option<String>,
}
