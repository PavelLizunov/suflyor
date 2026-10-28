//! Explicit synthetic speed sample for an already loaded local model.
//! Engine timings, not HTTP wall time, supply prefill/decode throughput.

use super::{control, provider, AiEndpoint, AiProtocol, ChatMessage, MessageContent};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Serialize, Deserialize)]
pub struct ModelSpeedSample {
    pub measured_at_ms: u64,
    pub model: String,
    pub sample_size: u32,
    pub input_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub prefill_tps: Option<f64>,
    pub decode_tps: Option<f64>,
    pub first_token_ms: Option<f64>,
    pub total_ms: f64,
}

/// One request, at most 128 output tokens and three minutes including admission.
/// The caller's cancellation flag and a newly activated deep lock drop the HTTP
/// request. No engine is started, no model downloaded, and no transcript is sent.
pub async fn measure_model_speed(
    endpoint: AiEndpoint,
    sample_size: u32,
    cancel: &AtomicBool,
) -> Result<ModelSpeedSample> {
    if !endpoint.is_local || endpoint.protocol != AiProtocol::OpenAiCompatible {
        bail!("speed measurement requires a local OpenAI-compatible model");
    }
    if endpoint.base_url.trim().is_empty() || endpoint.model.trim().is_empty() {
        bail!("select and load a model first");
    }
    let blocked = || crate::deep_lock::endpoint_blocked(
        crate::deep_lock::deep_lock_active(), &endpoint.base_url,
    );
    if blocked() { bail!(crate::deep_lock::BLOCKED_ERROR); }
    let stopped = async {
        loop {
            if cancel.load(Ordering::Acquire) { bail!("speed measurement cancelled"); }
            if blocked() { bail!(crate::deep_lock::BLOCKED_ERROR); }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    let work = async {
        // Match the live/prep switch order: drain AI requests before lifecycle.
        let _permit = super::acquire_exclusive_ai().await?;
        let _lifecycle = crate::local_ai::lifecycle_lock().acquire_owned().await
            .context("local AI lifecycle unavailable")?;
        let size = sample_size.min(2);
        let mut nonce = [0u8; 8];
        getrandom::getrandom(&mut nonce).map_err(|_| anyhow::anyhow!("sample ID unavailable"))?;
        // Change the very first user tokens between runs so a repeated sample
        // cannot silently turn a cached prefix into a claimed prefill speed.
        let mut input = format!("Sample {:016x}.\n", u64::from_le_bytes(nonce));
        for row in 0..[24, 96, 384][size as usize] {
            input.push_str(&format!(
                "Record {row}: schedule budget storage deployment metrics routing capacity.\n"
            ));
        }
        input.push_str("Write a numbered explanation of these topics. Keep writing until the output limit.");
        let messages = vec![ChatMessage { role: "user".into(), content: MessageContent::Text(input) }];
        let mut body = provider::request_body(endpoint.protocol, &endpoint.model, &messages, 128, true, false)?;
        control::apply_local_no_think(&mut body, true);
        body["stream_options"] = json!({"include_usage": true});
        body["cache_prompt"] = json!(false);
        body["strata_checkpoint"] = json!(false);
        body["reasoning_budget_tokens"] = json!(0);
        let _done = super::local_server::request_started(&endpoint.base_url, &messages);
        let start = Instant::now();
        let request = control::http_client().post(format!(
            "{}/chat/completions", endpoint.base_url.trim_end_matches('/')
        )).timeout(Duration::from_secs(170));
        let mut response = provider::authorize(request, endpoint.protocol, &endpoint.bearer)
            .json(&body).send().await.map_err(|_| anyhow::anyhow!("model did not answer"))?;
        if !response.status().is_success() {
            bail!("speed measurement HTTP {}", response.status().as_u16());
        }
        let mut sample = ModelSpeedSample {
            measured_at_ms: crate::journal::now_unix_ms() as u64,
            model: endpoint.model.rsplit(['/', '\\']).next().unwrap_or("model").chars().take(160).collect(),
            sample_size: size, input_tokens: None, cached_tokens: None, output_tokens: None,
            prefill_tps: None, decode_tps: None, first_token_ms: None, total_ms: 0.0,
        };
        let mut bytes = Vec::new();
        let mut received = 0usize;
        let mut finished = false;
        let mut done = false;
        while let Some(chunk) = response.chunk().await.map_err(|_| anyhow::anyhow!("speed response interrupted"))? {
            received += chunk.len();
            if received > 2 * 1024 * 1024 { bail!("speed response too large"); }
            bytes.extend_from_slice(&chunk);
            while let Some(end) = bytes.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = bytes.drain(..=end).collect();
                let line = std::str::from_utf8(&line).context("invalid speed response")?.trim();
                let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
                if data == "[DONE]" { done = true; break; }
                let event: Value = serde_json::from_str(data).context("invalid speed response")?;
                if event.get("error").is_some() { bail!("model rejected the speed measurement"); }
                if let Some(model) = event.get("model").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                    sample.model = model.rsplit(['/', '\\']).next().unwrap_or("model").chars().take(160).collect();
                }
                let choice = event.pointer("/choices/0");
                if choice.and_then(|c| c.get("finish_reason")).and_then(Value::as_str)
                    .is_some_and(|reason| matches!(reason, "stop" | "length")) { finished = true; }
                let delta = choice.and_then(|c| c.get("delta"));
                if sample.first_token_ms.is_none() && ["content", "reasoning_content"].iter().any(|name| {
                    delta.and_then(|d| d.get(name)).and_then(Value::as_str).is_some_and(|s| !s.is_empty())
                }) { sample.first_token_ms = Some(start.elapsed().as_secs_f64() * 1000.0); }
                if let Some(n) = event.pointer("/usage/prompt_tokens").and_then(Value::as_u64) { sample.input_tokens = Some(n); }
                if let Some(n) = event.pointer("/usage/completion_tokens").and_then(Value::as_u64) { sample.output_tokens = Some(n); }
                if let Some(n) = event.pointer("/usage/prompt_tokens_details/cached_tokens").and_then(Value::as_u64) { sample.cached_tokens = Some(n); }
                if let Some(t) = event.get("timings").filter(|t| t.is_object()) {
                    sample.cached_tokens = t.get("cache_n").and_then(Value::as_u64).or(sample.cached_tokens);
                    // Only freshly read tokens count; usage.prompt_tokens may include cache hits.
                    sample.prefill_tps = if t.get("prompt_n").and_then(Value::as_u64) == Some(0) {
                        None
                    } else {
                        positive(t.get("prompt_per_second")).or_else(|| rate(t, "prompt_n", "prompt_ms"))
                    };
                    sample.decode_tps = positive(t.get("predicted_per_second"))
                        .or_else(|| rate(t, "predicted_n", "predicted_ms"));
                    sample.input_tokens = sample.input_tokens.or_else(|| {
                        t.get("prompt_n").and_then(Value::as_u64)
                            .zip(sample.cached_tokens).map(|(read, cached)| read.saturating_add(cached))
                    });
                    sample.output_tokens = sample.output_tokens.or_else(|| t.get("predicted_n").and_then(Value::as_u64));
                }
            }
            if done { break; }
        }
        if !done || !finished { bail!("speed measurement did not finish"); }
        sample.total_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(sample)
    };
    tokio::select! {
        biased;
        result = stopped => result,
        result = tokio::time::timeout(Duration::from_secs(180), work) => {
            result.context("speed measurement timed out")?
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SavedModelSpeed {
    key: String,
    pub context_preset: String,
    pub engine_version: Option<String>,
    pub sample: ModelSpeedSample,
}

fn scope(cfg: &crate::config::Config) -> (String, Option<String>) {
    use sha2::{Digest, Sha256};
    let root = crate::local_ai::default_root();
    let engine = if crate::local_ai::is_managed_strata_endpoint(&cfg.ai_local_base_url) {
        crate::local_ai::strata_engine_version(&root).map(|v| format!("Strata {v}"))
    } else if crate::local_ai::is_managed_llama_endpoint(&cfg.ai_local_base_url) {
        crate::local_ai::installed_engine_build(&root).map(|v| format!("llama.cpp b{v}"))
    } else { None };
    // Credentials are never stored, and the endpoint/custom file path is hashed.
    let key = json!([cfg.ai_provider, cfg.ai_local_base_url, cfg.ai_local_model,
        cfg.ai_local_context, cfg.ai_local_custom_gguf, engine]).to_string();
    (crate::download::hex(&Sha256::digest(key.as_bytes())), engine)
}

pub fn load_last_sample(cfg: &crate::config::Config) -> Option<SavedModelSpeed> {
    let path = crate::paths::data_root()?.join("model-speed.json");
    if std::fs::metadata(&path).ok()?.len() > 16 * 1024 { return None; }
    let saved: SavedModelSpeed = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (cfg.ai_provider == "local" && saved.key == scope(cfg).0).then_some(saved)
}

pub fn save_last_sample(cfg: &crate::config::Config, sample: ModelSpeedSample) -> Result<()> {
    use std::io::Write;
    let root = crate::paths::data_root().context("configuration directory unavailable")?;
    std::fs::create_dir_all(&root)?;
    let (key, engine_version) = scope(cfg);
    let saved = SavedModelSpeed { key, context_preset: cfg.ai_local_context.clone(), engine_version, sample };
    let mut file = tempfile::NamedTempFile::new_in(&root)?;
    file.write_all(&serde_json::to_vec(&saved)?)?;
    file.persist(root.join("model-speed.json"))?;
    Ok(())
}

fn positive(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0)
}

fn rate(timings: &Value, count: &str, ms: &str) -> Option<f64> {
    let tokens = positive(timings.get(count))?;
    let elapsed = positive(timings.get(ms))?;
    let rate = tokens / elapsed * 1000.0;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}
