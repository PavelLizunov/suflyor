use serde::Serialize;

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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recovered_from_session_id: Option<&'a str>,
    },
    SessionStop {
        unix_ms: u128,
    },
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
        audio_ms: u64,
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
        system_prompt: &'a str,
        user_prompt: &'a str,
        attached_screenshot: bool,
        input_tokens_est: u64,
    },
    AiResponse {
        unix_ms: u128,
        purpose: &'a str,
        model: &'a str,
        latency_ms: u64,
        finish_reason: &'a str,
        text: &'a str,
        output_tokens_est: u64,
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
