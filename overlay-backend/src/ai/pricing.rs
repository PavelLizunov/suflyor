/// USD price per 1M tokens for each model. Re-verify on each model launch.
pub fn pricing_per_million(model: &str) -> (f64, f64) {
    // (input, output)
    match model {
        // Official OpenAI pricing, verified 2026-08-09:
        // https://developers.openai.com/api/docs/models/gpt-5.2
        "gpt-5.2" | "gpt-5.2-chat-latest" => (1.75, 14.0),
        "gpt-5.2-pro" => (21.0, 168.0),
        "claude-haiku-4-5" => (1.0, 5.0),
        "claude-sonnet-4-5" | "claude-sonnet-4-6" => (3.0, 15.0),
        // Opus 4.6/4.7/4.8 are all $5/$25 — the old (15,75) over-billed 3×.
        "claude-opus-4-5" | "claude-opus-4-6" | "claude-opus-4-7" | "claude-opus-4-8" => {
            (5.0, 25.0)
        }
        "claude-fable-5" | "claude-mythos-5" => (10.0, 50.0),
        _ => (3.0, 15.0), // safe default for an unknown model
    }
}

/// The single canonical money-conversion rule: 1 USD = 100_000_000
/// microcents (1 microcent = 10⁻⁸ USD). Internal accounting uses
/// microcents (u64) to avoid f64 drift over long sessions; display
/// paths convert with [`microcents_to_usd`].
pub const MICROCENTS_PER_USD: f64 = 100_000_000.0;

/// USD float view of a microcents amount — the display conversion shared
/// by every UI path. Internal accounting stays in microcents
/// ([`cost_microcents`]) to avoid f64 precision loss over long sessions.
#[must_use]
pub fn microcents_to_usd(microcents: u64) -> f64 {
    (microcents as f64) / MICROCENTS_PER_USD
}

/// Cost in microcents (see [`MICROCENTS_PER_USD`]). Use this for
/// internal accumulation to avoid f64 precision loss over long sessions.
pub fn cost_microcents(model: &str, input_tokens: u64, output_tokens: u64) -> u64 {
    let (p_in_per_m, p_out_per_m) = pricing_per_million(model);
    // microcents per token = price_per_million_usd × MICROCENTS_PER_USD / 1_000_000 = price × 100
    let micro_in = (p_in_per_m * 100.0) as u64; // microcents per input token
    let micro_out = (p_out_per_m * 100.0) as u64;
    input_tokens
        .saturating_mul(micro_in)
        .saturating_add(output_tokens.saturating_mul(micro_out))
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    /// Generation throughput in tokens/second for THIS request — llama's own
    /// `timings.predicted_per_second` when present, else completion_tokens over
    /// the wall-clock request time. 0.0 if unknown. Feeds the per-tile label.
    pub tok_per_sec: f64,
    /// The provider's own `choices[0].finish_reason` — "stop" (natural end),
    /// "length" (truncated by max_tokens), etc. Falls back to "stop" when the
    /// provider omits/empties it, so journaling sites can carry it verbatim
    /// (audit D4: non-streaming sites previously hardcoded "stop" and lost
    /// real truncation signals).
    pub finish_reason: String,
}
