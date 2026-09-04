use super::control::{
    acquire_exclusive_ai, apply_local_no_think, apply_managed_gemma_sampler, apply_prompt_cache,
    http_client, resolve_managed_mlx_endpoint, transport_failure_kind, AI_SEMAPHORE, PROMPT_CACHE,
};
use super::pricing::TokenUsage;
use super::provider;
use super::stream::codex_failure_message;
use super::tps::record_tps;
use super::types::{AiEndpoint, AiProtocol, ChatMessage};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

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
    const MAX_ATTEMPTS: usize = 3;
    let mut last_err = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(AiEndpoint {
            protocol,
            base_url: base_url.to_string(),
            bearer: bearer.to_string(),
            model: model.to_string(),
            reasoning_effort: None,
            is_local: false,
        })
        .await?;

        let _permit = AI_SEMAPHORE.acquire().await.ok();
        match complete_once(
            endpoint.protocol,
            &endpoint.base_url,
            &endpoint.bearer,
            &endpoint.model,
            messages.clone(),
            max_tokens,
            force_no_think,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) => {
                let msg = format!("{e:#}");
                if is_permanent_ai_error(&msg) {
                    log::warn!("AI complete permanent failure (no retry): {msg}");
                    return Err(e);
                }
                if attempt == MAX_ATTEMPTS {
                    log::warn!("AI complete final attempt {} failed: {msg}", attempt);
                    last_err = Some(e);
                    break;
                }
                let delay_ms = 1000u64 * (1u64 << (attempt - 1)); // 1s, 2s, 4s
                log::warn!(
                    "AI complete attempt {}/{} failed: {msg} — retrying in {}ms",
                    attempt,
                    MAX_ATTEMPTS,
                    delay_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("AI complete failed without specific error")))
}

pub fn is_permanent_ai_error(msg: &str) -> bool {
    if let Some(rest) = msg.split("HTTP ").nth(1) {
        let code: u16 = rest
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        return (400..500).contains(&code) && code != 429;
    }
    false
}

pub(crate) async fn complete_once(
    protocol: AiProtocol,
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    force_no_think: bool,
) -> Result<(String, TokenUsage)> {
    let t0 = std::time::Instant::now();
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
        provider::request_body(protocol, model, &messages, max_tokens, false, prompt_cache)?;
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
        anyhow::bail!("HTTP {status}");
    }
    let v: Value = resp.json().await.context("parse json")?;
    let (text, usage) = provider::parse_completion(protocol, &v, t0.elapsed().as_secs_f64());
    record_tps(usage.tok_per_sec);
    Ok((text, usage))
}

pub async fn count_chat_tokens(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<u64> {
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), base_url) {
        return Err(anyhow!(crate::deep_lock::BLOCKED_ERROR));
    }
    let url = format!(
        "{}/chat/completions/input_tokens",
        base_url.trim_end_matches('/')
    );
    let mut body = json!({
        "model": model,
        "messages": messages,
    });
    apply_prompt_cache(&mut body);
    apply_local_no_think(&mut body, true);
    apply_managed_gemma_sampler(&mut body, base_url, model);
    let response = http_client()
        .post(url)
        .timeout(std::time::Duration::from_secs(30))
        .bearer_auth(bearer)
        .json(&body)
        .send()
        .await
        .context("count local prompt tokens")?;
    if !response.status().is_success() {
        anyhow::bail!("local prompt token count failed");
    }
    response
        .json::<Value>()
        .await
        .context("parse local prompt token count")?
        .get("input_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("local prompt token count missing"))
}
