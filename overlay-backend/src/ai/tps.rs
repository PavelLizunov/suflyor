//! Process-wide AI throughput and request-latency telemetry.

use std::time::{Duration, Instant};

/// Rolling decode tokens/sec across recent AI requests.
static TPS_EWMA: std::sync::Mutex<f64> = std::sync::Mutex::new(0.0);
static REQUEST_STATE: std::sync::Mutex<RequestState> = std::sync::Mutex::new(RequestState {
    generation: 0,
    latest: None,
});
const REQUEST_PERF_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RequestPerf {
    pub decode_tps: Option<f64>,
    pub ttft_ms: Option<u64>,
    pub total_ms: u64,
    pub effective_tps: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct TimedRequestPerf {
    value: RequestPerf,
    recorded_at: Instant,
}

struct RequestState {
    generation: u64,
    latest: Option<TimedRequestPerf>,
}

/// Fold one request's decode tok/s into the rolling average.
pub fn record_tps(value: f64) {
    if !(value.is_finite() && value > 0.0) {
        return;
    }
    let mut average = match TPS_EWMA.lock() {
        Ok(average) => average,
        Err(poisoned) => poisoned.into_inner(),
    };
    *average = if *average <= 0.0 {
        value
    } else {
        0.3 * value + 0.7 * *average
    };
}

/// Current rolling decode tok/s, or `None` before the first completion.
pub fn avg_tps() -> Option<f64> {
    let average = match TPS_EWMA.lock() {
        Ok(average) => average,
        Err(poisoned) => poisoned.into_inner(),
    };
    (*average > 0.0).then_some(*average)
}

/// Mark a newer streaming request active and hide the previous completion.
pub(super) fn begin_request() -> u64 {
    let mut state = match REQUEST_STATE.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.generation = state.generation.wrapping_add(1);
    state.latest = None;
    state.generation
}

/// Last completed request metrics while they are still current.
#[must_use]
pub fn latest_request_perf() -> Option<RequestPerf> {
    let state = match REQUEST_STATE.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state
        .latest
        .as_ref()
        .filter(|request| request.recorded_at.elapsed() <= REQUEST_PERF_TTL)
        .map(|request| request.value)
}

/// Clear stale model-specific telemetry on an MLX start, swap, or stop.
pub fn clear_request_perf() {
    let mut state = match REQUEST_STATE.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.generation = state.generation.wrapping_add(1);
    state.latest = None;
    match TPS_EWMA.lock() {
        Ok(mut average) => *average = 0.0,
        Err(poisoned) => *poisoned.into_inner() = 0.0,
    }
}

/// Select one comparable streamed-generation TPS value. The process EWMA keeps
/// its historical chunk-rate fallback for providers without usage metadata.
fn stream_tps(
    server_tps: Option<f64>,
    completion_tokens: Option<u64>,
    delta_count: u32,
    elapsed_secs: f64,
) -> Option<f64> {
    server_tps
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .or_else(|| {
            completion_tokens
                .filter(|count| *count > 0 && elapsed_secs > 0.0)
                .map(|count| count as f64 / elapsed_secs)
        })
        .or_else(|| {
            (delta_count > 0 && elapsed_secs > 0.0).then_some(delta_count as f64 / elapsed_secs)
        })
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn request_perf(
    request_started_at: Instant,
    first_delta_at: Option<Instant>,
    terminal_at: Instant,
    server_tps: Option<f64>,
    completion_tokens: Option<u64>,
) -> RequestPerf {
    let generation_secs = first_delta_at
        .map(|first| terminal_at.saturating_duration_since(first).as_secs_f64())
        .unwrap_or(0.0);
    let total = terminal_at.saturating_duration_since(request_started_at);
    let total_secs = total.as_secs_f64();
    RequestPerf {
        // Never expose the SSE-chunk fallback as tokens/sec in the detailed UI.
        decode_tps: stream_tps(server_tps, completion_tokens, 0, generation_secs),
        ttft_ms: first_delta_at
            .map(|first| millis(first.saturating_duration_since(request_started_at))),
        total_ms: millis(total),
        effective_tps: completion_tokens
            .filter(|count| *count > 0 && total_secs > 0.0)
            .map(|count| count as f64 / total_secs),
    }
}

fn format_opt_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

fn format_opt_f64(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite() && *v > 0.0)
        .map_or_else(|| "-".to_string(), |v| format!("{v:.1}"))
}

fn format_stream_tps_metrics(perf: &RequestPerf, completion_tokens: Option<u64>) -> String {
    let ttft = format_opt_u64(perf.ttft_ms);
    let total = perf.total_ms;
    let decode = format_opt_f64(perf.decode_tps);
    let effective = format_opt_f64(perf.effective_tps);
    let tokens = format_opt_u64(completion_tokens);

    format!(
        "stream tps: ttft_ms={ttft} total_ms={total} decode_tps={decode} effective_tps={effective} completion_tokens={tokens}"
    )
}

pub(super) fn record_stream_tps(
    request_id: u64,
    request_started_at: Instant,
    delta_count: u32,
    first_delta_at: Option<Instant>,
    server_tps: Option<f64>,
    completion_tokens: Option<u64>,
) {
    let terminal_at = Instant::now();
    let generation_secs = first_delta_at
        .map(|started| terminal_at.saturating_duration_since(started).as_secs_f64())
        .unwrap_or(0.0);
    let value = request_perf(
        request_started_at,
        first_delta_at,
        terminal_at,
        server_tps,
        completion_tokens,
    );
    let mut state = match REQUEST_STATE.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    if state.generation != request_id {
        return;
    }
    let metrics = format_stream_tps_metrics(&value, completion_tokens);
    if let Some(rate) = stream_tps(server_tps, completion_tokens, delta_count, generation_secs) {
        record_tps(rate);
    }
    state.latest = Some(TimedRequestPerf {
        value,
        recorded_at: terminal_at,
    });
    drop(state);
    log::info!("{metrics}");
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn streamed_tps_prefers_server_metric_over_chunks_and_usage() {
        assert_eq!(stream_tps(Some(42.5), Some(80), 2, 4.0), Some(42.5));
        assert_eq!(stream_tps(None, Some(80), 2, 4.0), Some(20.0));
        assert_eq!(stream_tps(None, None, 8, 4.0), Some(2.0));
        assert_eq!(stream_tps(None, None, 0, 0.0), None);
        assert_eq!(stream_tps(Some(f64::NAN), Some(12), 1, 3.0), Some(4.0));
    }

    #[test]
    fn request_perf_separates_ttft_decode_and_end_to_end() {
        let started = Instant::now();
        let first = started + Duration::from_secs(2);
        let done = started + Duration::from_secs(4);
        let perf = request_perf(started, Some(first), done, Some(80.0), Some(20));
        assert_eq!(perf.ttft_ms, Some(2_000));
        assert_eq!(perf.total_ms, 4_000);
        assert_eq!(perf.decode_tps, Some(80.0));
        assert_eq!(perf.effective_tps, Some(5.0));
    }

    #[test]
    fn newer_request_suppresses_older_or_failed_telemetry() {
        clear_request_perf();
        let older = begin_request();
        let newer = begin_request();
        record_stream_tps(older, Instant::now(), 1, None, Some(1.0), Some(1));
        assert_eq!(latest_request_perf(), None);
        record_stream_tps(newer, Instant::now(), 1, None, Some(1.0), Some(1));
        assert!(latest_request_perf().is_some());
    }

    #[test]
    fn detailed_decode_never_calls_chunk_rate_tokens() {
        let started = Instant::now();
        let perf = request_perf(
            started,
            Some(started + Duration::from_secs(1)),
            started + Duration::from_secs(2),
            None,
            None,
        );
        assert_eq!(perf.decode_tps, None);
        assert_eq!(perf.effective_tps, None);
    }

    #[test]
    fn format_stream_tps_metrics_formats_numeric_and_dash_placeholders() {
        let perf = RequestPerf {
            ttft_ms: Some(150),
            total_ms: 1200,
            decode_tps: Some(42.5),
            effective_tps: Some(33.3),
        };
        let line = format_stream_tps_metrics(&perf, Some(40));
        assert_eq!(
            line,
            "stream tps: ttft_ms=150 total_ms=1200 decode_tps=42.5 effective_tps=33.3 completion_tokens=40"
        );

        let perf_none = RequestPerf {
            ttft_ms: None,
            total_ms: 500,
            decode_tps: None,
            effective_tps: None,
        };
        let line_none = format_stream_tps_metrics(&perf_none, None);
        assert_eq!(
            line_none,
            "stream tps: ttft_ms=- total_ms=500 decode_tps=- effective_tps=- completion_tokens=-"
        );
    }
}
