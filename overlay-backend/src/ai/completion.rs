use super::control::{
    acquire_exclusive_ai, apply_local_no_think, apply_managed_gemma_sampler, apply_prompt_cache,
    http_client, resolve_managed_mlx_endpoint, transport_failure_kind, AI_SEMAPHORE, PROMPT_CACHE,
};
use super::pricing::TokenUsage;
use super::provider;
use super::stream::codex_failure_message;
use super::tps::{begin_request, record_complete_tps};
use super::types::{AiEndpoint, AiProtocol, ChatMessage};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

pub async fn complete_with_usage(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<(String, TokenUsage)> {
    complete_with_usage_inner(
        AiProtocol::OpenAiCompatible,
        base_url,
        bearer,
        model,
        messages,
        max_tokens,
        false,
    )
    .await
}

pub async fn complete_with_usage_endpoint(
    endpoint: &AiEndpoint,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<(String, TokenUsage)> {
    if endpoint.protocol == AiProtocol::CodexSubscription {
        let model = endpoint.model.clone();
        let reasoning_effort = endpoint.reasoning_effort.clone();
        return tokio::task::spawn_blocking(move || {
            let mut text = String::new();
            let usage = crate::codex_subscription::run_turn(
                &model,
                reasoning_effort.as_deref(),
                &messages,
                |event| {
                    if let crate::codex_subscription::TurnEvent::Delta { text: delta } = event {
                        text.push_str(&delta);
                    }
                    true
                },
            )
            .map_err(|failure| anyhow!(codex_failure_message(failure)))?;
            Ok((text, usage))
        })
        .await
        .map_err(|_| anyhow!("Codex answer unavailable"))?;
    }
    let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(endpoint.clone()).await?;
    complete_with_usage_inner(
        endpoint.protocol,
        &endpoint.base_url,
        &endpoint.bearer,
        &endpoint.model,
        messages,
        max_tokens,
        false,
    )
    .await
}

pub async fn complete(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<String> {
    complete_with_usage_inner(
        AiProtocol::OpenAiCompatible,
        base_url,
        bearer,
        model,
        messages,
        max_tokens,
        true,
    )
    .await
    .map(|(text, _)| text)
}

pub async fn complete_endpoint(
    endpoint: &AiEndpoint,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<String> {
    if endpoint.protocol == AiProtocol::CodexSubscription {
        return complete_with_usage_endpoint(endpoint, messages, max_tokens)
            .await
            .map(|(text, _)| text);
    }
    let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(endpoint.clone()).await?;
    complete_with_usage_inner(
        endpoint.protocol,
        &endpoint.base_url,
        &endpoint.bearer,
        &endpoint.model,
        messages,
        max_tokens,
        true,
    )
    .await
    .map(|(text, _)| text)
}

pub async fn complete_exclusive(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<String> {
    let _permit = acquire_exclusive_ai().await?;
    complete(base_url, bearer, model, messages, max_tokens).await
}

pub(crate) async fn complete_with_usage_inner(
    protocol: AiProtocol,
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    force_no_think: bool,
) -> Result<(String, TokenUsage)> {
    let max_retries = 3;
    let mut backoff = std::time::Duration::from_secs(1);
    let mut last_err = anyhow!("completion failed");

    for attempt in 0..=max_retries {
        let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(AiEndpoint {
            protocol,
            base_url: base_url.to_string(),
            bearer: bearer.to_string(),
            model: model.to_string(),
            reasoning_effort: None,
            is_local: false,
        })
        .await?;
        if attempt > 0 {
            log::warn!(
                "AI complete retry {}/{} after {:?}",
                attempt,
                max_retries,
                backoff
            );
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(std::time::Duration::from_secs(8));
        }
        let _permit = AI_SEMAPHORE.acquire().await.ok();
        match complete_once(
            endpoint.protocol,
            &endpoint.base_url,
            &endpoint.bearer,
            &endpoint.model,
            &messages,
            max_tokens,
            force_no_think,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) => {
                if is_permanent_ai_error(&e) {
                    return Err(e);
                }
                last_err = e;
            }
        }
    }
    Err(last_err)
}

pub(crate) fn is_permanent_ai_error(e: &anyhow::Error) -> bool {
    let s = format!("{e:#}");
    if s.contains("HTTP 429") {
        return false;
    }
    s.contains("HTTP 4") || s.contains("invalid json") || s.contains("parse models json")
}

pub(crate) async fn complete_once(
    protocol: AiProtocol,
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
    force_no_think: bool,
) -> Result<(String, TokenUsage)> {
    let request_started_at = std::time::Instant::now();
    let request_id = begin_request();
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), base_url) {
        return Err(anyhow!(crate::deep_lock::BLOCKED_ERROR));
    }
    let client = http_client();
    let url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        provider::endpoint_path(protocol)
    );
    let prompt_cache = PROMPT_CACHE.load(std::sync::atomic::Ordering::Relaxed)
        && protocol.supports_prompt_cache_control();
    let mut body =
        provider::request_body(protocol, model, messages, max_tokens, false, prompt_cache)?;
    if protocol == AiProtocol::OpenAiCompatible {
        apply_prompt_cache(&mut body);
        apply_local_no_think(&mut body, force_no_think);
        apply_managed_gemma_sampler(&mut body, base_url, model);
    }

    log::info!(
        "AI complete → provider (protocol={}, model={}, msgs={})",
        protocol.label(),
        model,
        messages.len()
    );

    let request = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(180));
    let resp = match provider::authorize(request, protocol, bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(error) => {
            log::warn!(
                "AI complete transport error ({})",
                transport_failure_kind(&error)
            );
            return Err(anyhow!("AI connection error"));
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "{}",
            crate::http_log::http_error_line("AI complete", status.as_u16(), body.len())
        );
        return Err(anyhow!("HTTP {status}"));
    }
    let v: Value = resp.json().await.context("invalid json in AI response")?;
    let parsed = provider::parse_completion(protocol, &v)?;
    let content = parsed.text.unwrap_or_default();
    let mut usage = TokenUsage {
        input: parsed.input_tokens.unwrap_or(0),
        output: parsed.output_tokens.unwrap_or(0),
        finish_reason: parsed.finish_reason.unwrap_or_else(|| "stop".into()),
        ..TokenUsage::default()
    };
    let throughput = record_complete_tps(
        request_id,
        request_started_at,
        parsed.server_tps,
        parsed.output_tokens,
    );
    usage.tok_per_sec = throughput.unwrap_or(0.0);
    Ok((content, usage))
}

pub async fn count_chat_tokens(base_url: &str, model: &str, content: &str) -> Result<u64> {
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), base_url) {
        return Err(anyhow!(crate::deep_lock::BLOCKED_ERROR));
    }
    let client = http_client();
    let url = format!("{}/tokenize", base_url.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(10))
        .json(&serde_json::json!({
            "model": model,
            "content": content,
        }))
        .send()
        .await
        .context("send tokenize request")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "{}",
            crate::http_log::http_error_line("tokenize", status.as_u16(), body.len())
        );
        return Err(anyhow!("HTTP {status}"));
    }
    let value: Value = resp.json().await.context("parse tokenize response")?;
    if let Some(tokens) = value.get("tokens").and_then(Value::as_array) {
        return Ok(tokens.len() as u64);
    }
    value
        .get("input_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("local prompt token count missing"))
}
