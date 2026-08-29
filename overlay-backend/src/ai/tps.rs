//! Process-wide AI decode-throughput telemetry.

/// Rolling tokens/sec across recent AI requests (streaming + non-streaming both
/// feed it). EWMA so the bar can show an at-a-glance "is generation slower than
/// usual?" trend without a chart.
static TPS_EWMA: std::sync::Mutex<f64> = std::sync::Mutex::new(0.0);

/// Fold one request's tok/s into the rolling average. Ignores non-positive /
/// non-finite values (failed or zero-length generations).
pub fn record_tps(value: f64) {
    if !(value.is_finite() && value > 0.0) {
        return;
    }
    let mut average = match TPS_EWMA.lock() {
        Ok(average) => average,
        Err(poisoned) => poisoned.into_inner(),
    };
    // Alpha 0.3 is responsive to a real slowdown without being jumpy per token.
    *average = if *average <= 0.0 {
        value
    } else {
        0.3 * value + 0.7 * *average
    };
}

/// Current rolling tok/s, or `None` if no request has completed yet this run.
pub fn avg_tps() -> Option<f64> {
    let average = match TPS_EWMA.lock() {
        Ok(average) => average,
        Err(poisoned) => poisoned.into_inner(),
    };
    (*average > 0.0).then_some(*average)
}

/// Select one comparable streamed-generation TPS value. Prefer the inference
/// engine's decode metric, then exact completion-token usage over the observed
/// generation window. Counting SSE deltas is a last resort because providers
/// may batch multiple tokens into one chunk.
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
            (delta_count > 0 && elapsed_secs > 0.0)
                .then_some(delta_count as f64 / elapsed_secs)
        })
}

pub(super) fn record_stream_tps(
    delta_count: u32,
    first_delta_at: Option<std::time::Instant>,
    server_tps: Option<f64>,
    completion_tokens: Option<u64>,
) {
    let elapsed_secs = first_delta_at.map_or(0.0, |started| started.elapsed().as_secs_f64());
    if let Some(rate) = stream_tps(server_tps, completion_tokens, delta_count, elapsed_secs) {
        record_tps(rate);
    }
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
}
