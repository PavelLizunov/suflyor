//! The agent loop for local OpenAI-compatible models (ADR-003, decision 6).
//!
//! Every step is one streamed `chat/completions` request that offers the
//! tools. Text deltas reach the tile as they arrive. When the model asks for
//! tools instead (`delta.tool_calls`, `finish_reason: "tool_calls"`), the
//! calls run here, their results join the history, and the next step asks
//! again. After `MAX_TOOL_STEPS` rounds, or past the time budget, the last
//! request offers no tools, so the model has to answer.
//!
//! A server that rejects the `tools` field answers through the plain path,
//! and is not offered tools again until the app restarts.

use super::control::{
    apply_local_no_think, apply_managed_gemma_sampler, apply_prompt_cache, http_client,
    transport_failure_kind,
};
use super::tools;
use super::tps::record_stream_tps;
use super::types::{AiEndpoint, AiEvent, AiProtocol, ChatMessage};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const MAX_TOOL_STEPS: usize = 4;
/// Tools stop being offered after this long; the answer itself has the
/// usual request timeout.
const TOOL_BUDGET: Duration = Duration::from_secs(60);
const TOOL_TIMEOUT: Duration = Duration::from_secs(20);

/// Base URLs whose server rejected a request with `tools`.
static NO_TOOLS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The agent runs for the local provider's OpenAI-compatible servers when
/// the user turned the tools on.
pub(crate) fn applies(endpoint: &AiEndpoint) -> bool {
    endpoint.protocol == AiProtocol::OpenAiCompatible
        && endpoint.is_local
        && tools::agent_enabled()
        && !NO_TOOLS
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&endpoint.base_url)
}

/// The server said it cannot take `tools` (llama.cpp without `--jinja`,
/// Ollama with a model that has no tool template), as opposed to being busy
/// or refusing the credentials.
fn rejects_tools(status: u16, body: &str) -> bool {
    matches!(status, 400 | 422 | 500 | 501) && body.to_ascii_lowercase().contains("tool")
}

fn mark_no_tools(base_url: &str) {
    let mut list = NO_TOOLS.lock().unwrap_or_else(|p| p.into_inner());
    if !list.iter().any(|u| u == base_url) {
        list.push(base_url.to_string());
    }
}

/// One tool call assembled from streamed fragments.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Collects `delta.tool_calls` fragments by their `index`: the first
/// fragment carries the id and the name, later ones append to the arguments.
#[derive(Debug, Default)]
pub(crate) struct ToolCallAccumulator {
    calls: BTreeMap<u64, ToolCall>,
}

impl ToolCallAccumulator {
    pub(crate) fn push(&mut self, delta: &Value) {
        let Some(parts) = delta.get("tool_calls").and_then(Value::as_array) else {
            return;
        };
        for (pos, part) in parts.iter().enumerate() {
            let index = part
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(pos as u64);
            let call = self.calls.entry(index).or_default();
            if let Some(id) = part.get("id").and_then(Value::as_str) {
                if !id.is_empty() {
                    call.id = id.to_string();
                }
            }
            if let Some(f) = part.get("function") {
                if let Some(name) = f.get("name").and_then(Value::as_str) {
                    call.name.push_str(name);
                }
                match f.get("arguments") {
                    Some(Value::String(s)) => call.arguments.push_str(s),
                    // A server that sends the arguments as an object.
                    Some(v @ Value::Object(_)) => call.arguments.push_str(&v.to_string()),
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn finish(self, step: usize) -> Vec<ToolCall> {
        self.calls
            .into_values()
            .enumerate()
            .filter(|(_, c)| !c.name.is_empty())
            .map(|(i, mut c)| {
                if c.id.is_empty() {
                    c.id = format!("call_{step}_{i}");
                }
                if c.arguments.trim().is_empty() {
                    c.arguments = "{}".to_string();
                }
                c
            })
            .collect()
    }
}

/// What one streamed step produced.
struct StepOutcome {
    text: String,
    calls: Vec<ToolCall>,
    finish: String,
    delta_count: u32,
    first_delta_at: Option<Instant>,
    server_tps: Option<f64>,
    completion_tokens: Option<u64>,
}

pub(crate) async fn stream_agent(
    endpoint: AiEndpoint,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    tx: mpsc::Sender<AiEvent>,
    request_id: u64,
    request_started_at: Instant,
) -> Result<()> {
    let mut history = match serde_json::to_value(&messages)? {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    let defs = tools::definitions(tools::web_enabled(), &tools::today_utc());
    let tools_until = Instant::now() + TOOL_BUDGET;
    let mut id_sent = false;
    let mut step = 0;
    loop {
        if tx.is_closed() {
            return Ok(());
        }
        let offer_tools = step < MAX_TOOL_STEPS && Instant::now() < tools_until;
        let mut body = json!({
            "model": endpoint.model,
            "messages": history,
            "stream": true,
            "max_tokens": max_tokens,
        });
        if offer_tools {
            body["tools"] = defs.clone();
            body["tool_choice"] = json!("auto");
        }
        apply_prompt_cache(&mut body);
        apply_local_no_think(&mut body, false);
        apply_managed_gemma_sampler(&mut body, &endpoint.base_url, &endpoint.model);
        log::info!(
            "AI agent step {step} (model={}, msgs={}, tools={offer_tools})",
            endpoint.model,
            history.len()
        );
        let url = format!(
            "{}/chat/completions",
            endpoint.base_url.trim_end_matches('/')
        );
        let mut request = http_client().post(&url).timeout(Duration::from_secs(120));
        if !endpoint.bearer.is_empty() {
            request = request.bearer_auth(&endpoint.bearer);
        }
        let resp = match request.json(&body).send().await {
            Ok(r) => r,
            Err(error) => {
                let kind = transport_failure_kind(&error);
                log::warn!("AI agent transport error ({kind})");
                return Err(anyhow!("AI connection error ({kind})"));
            }
        };
        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            log::warn!(
                "{}",
                crate::http_log::http_error_line("AI agent", status.as_u16(), error_body.len())
            );
            // The first request is the capability probe: it falls back to the
            // plain path, and a server that says it has no tool support is
            // not offered tools again. A busy or loading server is asked
            // with tools next time.
            if step == 0 && offer_tools && (status.is_client_error() || status.is_server_error()) {
                if rejects_tools(status.as_u16(), &error_body) {
                    mark_no_tools(&endpoint.base_url);
                }
                return super::stream::stream_plain(
                    endpoint,
                    messages,
                    max_tokens,
                    tx,
                    request_id,
                    request_started_at,
                )
                .await;
            }
            return Err(anyhow!("HTTP {status}"));
        }
        let outcome = read_step(resp, &tx, &mut id_sent, step).await?;
        if tx.is_closed() {
            return Ok(());
        }
        if offer_tools && !outcome.calls.is_empty() {
            let mut assistant = json!({
                "role": "assistant",
                "content": outcome.text,
                "tool_calls": outcome.calls.iter().map(|c| json!({
                    "id": c.id,
                    "type": "function",
                    "function": { "name": c.name, "arguments": c.arguments },
                })).collect::<Vec<_>>(),
            });
            if outcome.text.is_empty() {
                assistant["content"] = Value::Null;
            }
            history.push(assistant);
            for call in &outcome.calls {
                // A closed tile stops the loop: no further lookup leaves the PC.
                if tx
                    .send(AiEvent::Tool {
                        name: call.name.clone(),
                        detail: tools::status_detail(&call.name, &call.arguments),
                    })
                    .await
                    .is_err()
                {
                    return Ok(());
                }
                let result = tokio::select! {
                    biased;
                    () = tx.closed() => return Ok(()),
                    r = tokio::time::timeout(TOOL_TIMEOUT, tools::run(&call.name, &call.arguments)) => {
                        r.unwrap_or_else(|_| "error: the tool did not finish in time".to_string())
                    }
                };
                log::info!(
                    "AI agent tool {} -> {} chars",
                    call.name,
                    result.chars().count()
                );
                history.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": result,
                }));
            }
            // Text the model wrote before asking for a tool stays on the
            // tile; the answer continues below it.
            if !outcome.text.is_empty() {
                let _ = tx
                    .send(AiEvent::Delta {
                        text: "\n\n".to_string(),
                    })
                    .await;
            }
            step += 1;
            continue;
        }
        record_stream_tps(
            request_id,
            request_started_at,
            outcome.delta_count,
            outcome.first_delta_at,
            outcome.server_tps,
            outcome.completion_tokens,
        );
        let _ = tx
            .send(AiEvent::Done {
                reason: outcome.finish,
            })
            .await;
        return Ok(());
    }
}

async fn read_step(
    mut resp: reqwest::Response,
    tx: &mpsc::Sender<AiEvent>,
    id_sent: &mut bool,
    step: usize,
) -> Result<StepOutcome> {
    let mut out = StepOutcome {
        text: String::new(),
        calls: Vec::new(),
        finish: "eof".to_string(),
        delta_count: 0,
        first_delta_at: None,
        server_tps: None,
        completion_tokens: None,
    };
    let mut calls = ToolCallAccumulator::default();
    let mut byte_buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut buf = String::new();
    'read: while let Some(chunk) = resp.chunk().await.context("read sse chunk")? {
        byte_buf.extend_from_slice(&chunk);
        buf.push_str(&super::stream::drain_complete_frames(&mut byte_buf));
        while let Some(pos) = buf.find("\n\n") {
            let frame = buf[..pos].to_string();
            buf.drain(..pos + 2);
            for line in frame.lines() {
                let Some(payload) = line.trim().strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                if payload == "[DONE]" {
                    if out.finish == "eof" {
                        out.finish = "stop".to_string();
                    }
                    break 'read;
                }
                let Ok(v) = serde_json::from_str::<Value>(payload) else {
                    continue;
                };
                if !*id_sent {
                    if let Some(id) = v.get("id").and_then(Value::as_str) {
                        let _ = tx.send(AiEvent::Start { id: id.to_string() }).await;
                        *id_sent = true;
                    }
                }
                if let Some(rate) = v
                    .pointer("/timings/predicted_per_second")
                    .and_then(Value::as_f64)
                    .filter(|r| r.is_finite() && *r > 0.0)
                {
                    out.server_tps = Some(rate);
                }
                if let Some(n) = v
                    .pointer("/usage/completion_tokens")
                    .and_then(Value::as_u64)
                {
                    out.completion_tokens = Some(n);
                }
                let choice = v.pointer("/choices/0");
                if let Some(delta) = choice.and_then(|c| c.get("delta")) {
                    calls.push(delta);
                    if let Some(text) = delta
                        .get("content")
                        .and_then(Value::as_str)
                        .filter(|t| !t.is_empty())
                    {
                        out.first_delta_at.get_or_insert_with(Instant::now);
                        out.delta_count += 1;
                        out.text.push_str(text);
                        if tx
                            .send(AiEvent::Delta {
                                text: text.to_string(),
                            })
                            .await
                            .is_err()
                        {
                            break 'read;
                        }
                    }
                }
                if let Some(reason) = choice
                    .and_then(|c| c.get("finish_reason"))
                    .and_then(Value::as_str)
                    .filter(|r| !r.is_empty())
                {
                    out.finish = reason.to_string();
                    // Usage and timings may follow in a later frame; the
                    // calls are complete, so the step ends here.
                    if reason != "tool_calls" {
                        break 'read;
                    }
                }
            }
        }
    }
    out.calls = calls.finish(step);
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn accumulates_streamed_tool_call_fragments() {
        let mut acc = ToolCallAccumulator::default();
        acc.push(&json!({"tool_calls":[{"index":0,"id":"call_a","type":"function","function":{"name":"web_search","arguments":""}}]}));
        acc.push(&json!({"tool_calls":[{"index":0,"function":{"arguments":"{\"query\":"}}]}));
        acc.push(&json!({"tool_calls":[{"index":0,"function":{"arguments":"\"rust\"}"}}]}));
        acc.push(&json!({"tool_calls":[{"index":1,"function":{"name":"kb_search","arguments":{"query":"tokio"}}}]}));
        acc.push(&json!({"content":"text only"}));
        let calls = acc.finish(2);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "call_a");
        assert_eq!(calls[0].name, "web_search");
        assert_eq!(calls[0].arguments, r#"{"query":"rust"}"#);
        assert_eq!(calls[1].id, "call_2_1");
        assert_eq!(calls[1].arguments, r#"{"query":"tokio"}"#);
    }

    #[test]
    fn only_an_explicit_tool_refusal_disables_tools() {
        assert!(rejects_tools(
            400,
            r#"{"error":"model does not support tools"}"#
        ));
        assert!(rejects_tools(500, "tools param requires --jinja flag"));
        assert!(!rejects_tools(503, "Loading model"));
        assert!(!rejects_tools(429, "tools: too many requests"));
        assert!(!rejects_tools(401, "unauthorized"));
        assert!(!rejects_tools(400, "context too long"));
    }

    #[test]
    fn nameless_fragments_are_dropped() {
        let mut acc = ToolCallAccumulator::default();
        acc.push(&json!({"tool_calls":[{"index":0,"function":{"arguments":"{}"}}]}));
        assert!(acc.finish(0).is_empty());
    }
}
