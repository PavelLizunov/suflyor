use super::control::{
    apply_local_no_think, apply_managed_gemma_sampler, apply_prompt_cache, http_client,
    resolve_managed_mlx_endpoint, transport_failure_kind, AI_SEMAPHORE, PROMPT_CACHE,
};
use super::provider;
use super::tps::{begin_request, record_stream_tps};
use super::types::{AiEndpoint, AiEvent, AiProtocol, ChatMessage};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use tokio::sync::mpsc;

pub fn stream_chat(
    base_url: String,
    bearer: String,
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> mpsc::Receiver<AiEvent> {
    stream_chat_endpoint(
        AiEndpoint {
            protocol: AiProtocol::OpenAiCompatible,
            base_url,
            bearer,
            model,
            reasoning_effort: None,
            is_local: false,
        },
        messages,
        max_tokens,
    )
}

pub fn stream_chat_endpoint(
    endpoint: AiEndpoint,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> mpsc::Receiver<AiEvent> {
    let request_started_at = std::time::Instant::now();
    let request_id = begin_request();
    let (tx, rx) = mpsc::channel::<AiEvent>(64);

    tokio::spawn(async move {
        let (endpoint, _mlx_lease) = match resolve_managed_mlx_endpoint(endpoint).await {
            Ok(resolved) => resolved,
            Err(error) => {
                let _ = tx
                    .send(AiEvent::Error {
                        message: error.to_string(),
                    })
                    .await;
                return;
            }
        };
        if endpoint.protocol == AiProtocol::CodexSubscription {
            let model = endpoint.model.clone();
            let worker_tx = tx.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::codex_subscription::run_turn(
                    &model,
                    endpoint.reasoning_effort.as_deref(),
                    &messages,
                    |event| {
                        let mapped = match event {
                            crate::codex_subscription::TurnEvent::Start { id } => {
                                AiEvent::Start { id }
                            }
                            crate::codex_subscription::TurnEvent::Delta { text } => {
                                AiEvent::Delta { text }
                            }
                            crate::codex_subscription::TurnEvent::Done => AiEvent::Done {
                                reason: "stop".into(),
                            },
                        };
                        worker_tx.blocking_send(mapped).is_ok()
                    },
                )
            })
            .await;
            match result {
                Ok(Ok(_)) | Ok(Err(crate::codex_subscription::TurnFailure::Cancelled)) => {}
                Ok(Err(failure)) => {
                    let _ = tx
                        .send(AiEvent::Error {
                            message: codex_failure_message(failure).to_string(),
                        })
                        .await;
                }
                Err(_) => {
                    let _ = tx
                        .send(AiEvent::Error {
                            message: "Codex answer unavailable".into(),
                        })
                        .await;
                }
            }
            return;
        }
        if crate::deep_lock::endpoint_blocked(
            crate::deep_lock::deep_lock_active(),
            &endpoint.base_url,
        ) {
            let _ = tx
                .send(AiEvent::Error {
                    message: crate::deep_lock::BLOCKED_ERROR.to_string(),
                })
                .await;
            return;
        }
        let _permit = tokio::select! {
            permit = AI_SEMAPHORE.acquire() => permit.ok(),
            () = tx.closed() => return,
        };
        if tx.is_closed() {
            return;
        }
        if let Err(e) = stream_inner(
            endpoint,
            messages,
            max_tokens,
            tx.clone(),
            request_id,
            request_started_at,
        )
        .await
        {
            let _ = tx
                .send(AiEvent::Error {
                    message: format!("{e:#}"),
                })
                .await;
        }
    });

    rx
}

pub(crate) fn codex_failure_message(failure: crate::codex_subscription::TurnFailure) -> &'static str {
    match failure {
        crate::codex_subscription::TurnFailure::RateLimited(message) => message,
        crate::codex_subscription::TurnFailure::Security => "Codex security boundary rejected turn",
        crate::codex_subscription::TurnFailure::Protocol => "Codex protocol error",
        crate::codex_subscription::TurnFailure::Cancelled => "Codex turn cancelled",
    }
}

pub(crate) async fn stream_inner(
    endpoint: AiEndpoint,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    tx: mpsc::Sender<AiEvent>,
    request_id: u64,
    request_started_at: std::time::Instant,
) -> Result<()> {
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), &endpoint.base_url)
    {
        return Err(anyhow!(crate::deep_lock::BLOCKED_ERROR));
    }
    let client = http_client();

    let url = format!(
        "{}/{}",
        endpoint.base_url.trim_end_matches('/'),
        provider::endpoint_path(endpoint.protocol)
    );
    let prompt_cache = PROMPT_CACHE.load(std::sync::atomic::Ordering::Relaxed)
        && endpoint.protocol.supports_prompt_cache_control();
    let mut body = provider::request_body(
        endpoint.protocol,
        &endpoint.model,
        &messages,
        max_tokens,
        true,
        prompt_cache,
    )?;
    if endpoint.protocol == AiProtocol::OpenAiCompatible {
        apply_prompt_cache(&mut body);
        apply_local_no_think(&mut body, false);
        apply_managed_gemma_sampler(&mut body, &endpoint.base_url, &endpoint.model);
    }

    log::info!(
        "AI stream → provider (protocol={}, model={}, msgs={})",
        endpoint.protocol.label(),
        endpoint.model,
        messages.len()
    );

    let request = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(120));
    let mut resp = match provider::authorize(request, endpoint.protocol, &endpoint.bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(error) => {
            log::warn!(
                "AI stream transport error ({})",
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
            crate::http_log::http_error_line("AI stream", status.as_u16(), body.len())
        );
        return Err(anyhow!("HTTP {status}"));
    }

    let mut byte_buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut buf = String::new();
    let mut id_sent = false;
    let mut delta_count: u32 = 0;
    let mut server_tps: Option<f64> = None;
    let mut completion_tokens: Option<u64> = None;
    let mut first_delta_at: Option<std::time::Instant> = None;

    while let Some(chunk) = resp.chunk().await.context("read sse chunk")? {
        byte_buf.extend_from_slice(&chunk);
        let text = drain_complete_frames(&mut byte_buf);
        buf.push_str(&text);

        while let Some(pos) = buf.find("\n\n") {
            let frame = buf[..pos].to_string();
            buf.drain(..pos + 2);

            for line in frame.lines() {
                let line = line.trim();
                if !line.starts_with("data:") {
                    continue;
                }
                let payload = line["data:".len()..].trim();
                if endpoint.protocol == AiProtocol::OpenAiCompatible && payload == "[DONE]" {
                    log::info!("AI stream got [DONE]: deltas={}", delta_count);
                    record_stream_tps(
                        request_id,
                        request_started_at,
                        delta_count,
                        first_delta_at,
                        server_tps,
                        completion_tokens,
                    );
                    let _ = tx
                        .send(AiEvent::Done {
                            reason: "stop".into(),
                        })
                        .await;
                    return Ok(());
                }

                let v: Value = match serde_json::from_str(payload) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let parsed = provider::parse_stream(endpoint.protocol, &v);
                if parsed.failed {
                    return Err(anyhow!("AI provider stream error"));
                }
                if parsed.server_tps.is_some() {
                    server_tps = parsed.server_tps;
                }
                if parsed.completion_tokens.is_some() {
                    completion_tokens = parsed.completion_tokens;
                }
                if !id_sent {
                    if let Some(id) = parsed.id {
                        let _ = tx.send(AiEvent::Start { id: id.to_string() }).await;
                        id_sent = true;
                    }
                }

                if let Some(content) = parsed.delta {
                    if first_delta_at.is_none() {
                        first_delta_at = Some(std::time::Instant::now());
                    }
                    delta_count += 1;
                    if tx.send(AiEvent::Delta { text: content }).await.is_err() {
                        return Ok(());
                    }
                }
                if let Some(reason) = parsed.done {
                    log::info!(
                        "AI stream finished: reason={} deltas={}",
                        reason,
                        delta_count
                    );
                    record_stream_tps(
                        request_id,
                        request_started_at,
                        delta_count,
                        first_delta_at,
                        server_tps,
                        completion_tokens,
                    );
                    let _ = tx.send(AiEvent::Done { reason }).await;
                    return Ok(());
                }
            }
        }
    }

    if crate::mlx_runtime::is_owned_endpoint(&endpoint.base_url) {
        return Err(anyhow!("AI connection error"));
    }

    log::info!("AI stream ended without [DONE]/finish_reason: deltas={delta_count}");
    record_stream_tps(
        request_id,
        request_started_at,
        delta_count,
        first_delta_at,
        server_tps,
        completion_tokens,
    );
    let _ = tx
        .send(AiEvent::Done {
            reason: "eof".into(),
        })
        .await;
    Ok(())
}

pub(crate) fn drain_complete_frames(byte_buf: &mut Vec<u8>) -> String {
    let lf_boundary = byte_buf
        .windows(2)
        .rposition(|w| w == b"\n\n")
        .map(|p| p + 2);
    let crlf_boundary = byte_buf
        .windows(4)
        .rposition(|w| w == b"\r\n\r\n")
        .map(|p| p + 4);
    let last_boundary = match (lf_boundary, crlf_boundary) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    };
    let Some(split_at) = last_boundary else {
        return String::new();
    };
    let decodable: Vec<u8> = byte_buf.drain(..split_at).collect();
    match std::str::from_utf8(&decodable) {
        Ok(s) => s.replace("\r\n", "\n"),
        Err(e) => {
            log::warn!("SSE utf8 error at byte {}: {}", e.valid_up_to(), e);
            std::str::from_utf8(&decodable[..e.valid_up_to()])
                .unwrap_or("")
                .to_string()
        }
    }
}
