//! AI client: POST to OpenAI-compatible endpoint (your Claude OAuth bridge)
//! with SSE streaming. Emits AiEvent chunks downstream.

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

/// Process-wide HTTP client, built once and reused across AI calls so the
/// 2nd+ ask in a session reuses a warm TLS/HTTP connection (cuts
/// time-to-first-token). `reqwest::Client` is cheap to clone (Arc inside).
/// Per-call timeouts are applied on the request builder (`.timeout(..)`),
/// NOT on the client, so the existing 10s/120s/180s budgets are preserved.
pub(crate) fn http_client() -> reqwest::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        })
        .clone()
}

/// EXPERIMENTAL prompt-cache toggle (see `Config::ai_prompt_cache`). When
/// on, the system prompt is sent with Anthropic `cache_control: ephemeral`.
/// Default OFF → request body unchanged, so no regression by default.
static PROMPT_CACHE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the prompt-cache toggle (called at startup from config + on the
/// Settings switch). Cheap atomic; safe from any thread.
pub fn set_prompt_cache(on: bool) {
    PROMPT_CACHE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// If prompt-caching is on, attach `cache_control: ephemeral` to the system
/// message so a pass-through bridge caches the static system-prompt prefix
/// (cuts time-to-first-token on repeat/follow-up asks). No-op when off.
fn apply_prompt_cache(body: &mut Value) {
    if !PROMPT_CACHE.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if let Some(msgs) = body.get_mut("messages").and_then(Value::as_array_mut) {
        if let Some(sys) = msgs
            .iter_mut()
            .find(|m| m.get("role").and_then(Value::as_str) == Some("system"))
        {
            if let Some(text) = sys
                .get("content")
                .and_then(Value::as_str)
                .map(str::to_owned)
            {
                sys["content"] = json!([{
                    "type": "text",
                    "text": text,
                    "cache_control": { "type": "ephemeral" },
                }]);
            }
        }
    }
}

/// When the LOCAL AI provider is a hybrid "thinking" model (e.g. Gemma 4 E4B),
/// we send `chat_template_kwargs.enable_thinking=false` so it answers directly
/// instead of emitting long hidden reasoning (≈5× faster). Toggled from config
/// (`ai_local_thinking`): thinking-OFF is the default. Cloud requests leave the
/// flag false, so their bodies are unchanged.
static LOCAL_NO_THINK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the "disable local-model thinking" toggle. Called at startup from config
/// + whenever the AI provider / thinking setting changes. Cheap atomic.
pub fn set_local_no_think(on: bool) {
    LOCAL_NO_THINK.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// If the no-think toggle is on, attach `chat_template_kwargs.enable_thinking
/// = false` (a llama.cpp / OpenAI-compat extension). Servers that don't know
/// the field ignore it, so this is safe. No-op when off.
fn apply_local_no_think(body: &mut Value) {
    if !LOCAL_NO_THINK.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if let Some(obj) = body.as_object_mut() {
        obj.insert(
            "chat_template_kwargs".to_string(),
            json!({ "enable_thinking": false }),
        );
    }
}

/// Frontend-visible event stream.
///
/// Both `Serialize` AND `Deserialize` — the Slint binary's
/// `OverlayBarBridge` round-trips through `serde_json::Value` to
/// extract typed Delta/Done/Error variants from the `ai:event`
/// channel payload that `ask_stream_loop` emits via the trait
/// boundary. Added Deserialize Phase E3 slice 2 (2026-05-27).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    Start { id: String },
    Delta { text: String },
    Done { reason: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String, // "data:image/jpeg;base64,..."
}

/// Streaming chat completion. Returns a Receiver that emits AiEvents.
pub fn stream_chat(
    base_url: String,
    bearer: String,
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> mpsc::Receiver<AiEvent> {
    let (tx, rx) = mpsc::channel::<AiEvent>(64);

    tokio::spawn(async move {
        if let Err(e) =
            stream_inner(base_url, bearer, model, messages, max_tokens, tx.clone()).await
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

/// Phase E6 v27 — lightweight connection test for the Settings "AI
/// bridge" tab. POSTs a 1-token completion to `{base_url}/chat/
/// completions` with the bearer; returns a short status string on
/// HTTP 2xx, or an error with the status + body snippet. 10s timeout
/// so a dead endpoint doesn't hang the UI thread (caller runs this
/// off-thread anyway). Does NOT log the URL or bearer (secrets).
pub async fn test_connection(base_url: String, bearer: String, model: String) -> Result<String> {
    let client = http_client();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "messages": [{ "role": "user", "content": "ping" }],
        "max_tokens": 1,
    });
    // Generic on transport failure: a reqwest error's chain embeds the request
    // `url` (the LAN base_url + port), which `{e:#}` at the Settings AI-bridge /
    // Diagnostics call sites would paint into a screen-capturable field. Log the
    // full detail to the file log; return a secret-free message. Mirrors the
    // stt.rs fix and honours this fn's "does NOT log the URL" contract.
    let resp = match client
        .post(&url)
        .timeout(std::time::Duration::from_secs(10))
        .bearer_auth(&bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("AI bridge test transport error: {e:#}");
            return Err(anyhow!("connection failed"));
        }
    };
    let status = resp.status();
    if status.is_success() {
        Ok(format!("HTTP {}", status.as_u16()))
    } else {
        let txt = resp.text().await.unwrap_or_default();
        let snippet: String = txt.chars().take(100).collect();
        Err(anyhow!("HTTP {} — {}", status.as_u16(), snippet))
    }
}

/// List the model ids a local OpenAI-compatible server (llama.cpp / Ollama)
/// currently serves, via `GET {base_url}/models`. Powers the Settings → AI
/// provider model dropdown so the user picks a loaded model instead of typing
/// its id. 8s timeout (caller runs this off-thread). Returns the ids from the
/// OpenAI-shaped `{ "data": [ { "id": ... } ] }` response (empty vec if the
/// field is missing). Does NOT log the URL or bearer (secrets).
pub async fn list_models(base_url: &str, bearer: &str) -> Result<Vec<String>> {
    let client = http_client();
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let mut req = client.get(&url).timeout(std::time::Duration::from_secs(8));
    if !bearer.is_empty() {
        req = req.bearer_auth(bearer);
    }
    let resp = req.send().await.context("GET models")?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        let snippet: String = txt.chars().take(100).collect();
        return Err(anyhow!("HTTP {} — {}", status.as_u16(), snippet));
    }
    let v: Value = resp.json().await.context("parse models json")?;
    let ids: Vec<String> = v
        .get("data")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(ids)
}

async fn stream_inner(
    base_url: String,
    bearer: String,
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    tx: mpsc::Sender<AiEvent>,
) -> Result<()> {
    let client = http_client();

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut body = json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "max_tokens": max_tokens,
    });
    apply_prompt_cache(&mut body);
    apply_local_no_think(&mut body);

    // SECURITY: do NOT log the full URL — the configured ai_base_url often
    // contains the user's LAN IP / proxy port (network topology leak in
    // crash dumps / support bundles). Surface only model + message count.
    log::info!(
        "AI stream → /chat/completions (model={}, msgs={})",
        model,
        messages.len()
    );

    let resp = match client
        .post(&url)
        .timeout(std::time::Duration::from_secs(120))
        .bearer_auth(&bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        // Generic on transport failure: the reqwest error chain embeds the
        // request url (the LAN base_url), and `{e:#}` would paint it into the
        // streamed error tile (screen-share leak — CLAUDE.md security boundary).
        // Log the detail; return a secret-free, RETRYABLE message (no "HTTP 4xx"
        // → is_permanent_ai_error keeps retrying).
        Err(e) => {
            log::warn!("AI stream POST failed: {e:#}");
            return Err(anyhow!("AI connection error"));
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        // Keep the status (drives is_permanent_ai_error + classify_ai_error) but
        // DROP the body: a server's body can carry paths/internals that would
        // paint into the streamed error tile. Body → file log only.
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "AI stream HTTP {status} body: {}",
            body.chars().take(500).collect::<String>()
        );
        return Err(anyhow!("HTTP {status}"));
    }

    let mut byte_buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut buf = String::new();
    let mut stream = resp.bytes_stream();
    let mut id_sent = false;
    let mut delta_count: u32 = 0;

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.context("read sse chunk")?;
        byte_buf.extend_from_slice(&chunk);
        let text = drain_complete_frames(&mut byte_buf);
        buf.push_str(&text);

        // SSE frames separated by "\n\n"
        while let Some(pos) = buf.find("\n\n") {
            let frame = buf[..pos].to_string();
            buf.drain(..pos + 2);

            for line in frame.lines() {
                let line = line.trim();
                if !line.starts_with("data:") {
                    continue;
                }
                let payload = line["data:".len()..].trim();
                if payload == "[DONE]" {
                    log::info!("AI stream got [DONE]: deltas={}", delta_count);
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

                if !id_sent {
                    if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
                        let _ = tx.send(AiEvent::Start { id: id.to_string() }).await;
                        id_sent = true;
                    }
                }

                if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = choice.get("delta") {
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                if !content.is_empty() {
                                    delta_count += 1;
                                    let _ = tx
                                        .send(AiEvent::Delta {
                                            text: content.to_string(),
                                        })
                                        .await;
                                }
                            }
                        }
                        if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                            log::info!(
                                "AI stream finished: reason={} deltas={}",
                                reason,
                                delta_count
                            );
                            let _ = tx
                                .send(AiEvent::Done {
                                    reason: reason.to_string(),
                                })
                                .await;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    // The stream ended WITHOUT a `[DONE]` sentinel or a finish_reason — some
    // local llama.cpp servers (and dropped proxy connections) just close the
    // body after the answer. Emit a terminal Done anyway so the UI always
    // clears its in-flight state + finalizes the tile (consumers rely on the
    // "exactly one terminal event per stream" contract; otherwise the bar's
    // "AI working" pulse and the follow-up "busy" state stay stuck on).
    log::info!("AI stream ended without [DONE]/finish_reason: deltas={delta_count}");
    let _ = tx
        .send(AiEvent::Done {
            reason: "eof".into(),
        })
        .await;
    Ok(())
}

/// Drain bytes up to the last `\n\n` SSE frame boundary, returning the
/// decoded UTF-8 text. Bytes after the last boundary stay in `byte_buf`
/// — they may contain a partial UTF-8 character that will complete on the
/// next network chunk.
///
/// This is the regression-tested part of the SSE pipeline: it must NEVER
/// panic on UTF-8 split across chunk boundaries.
fn drain_complete_frames(byte_buf: &mut Vec<u8>) -> String {
    let last_boundary = byte_buf
        .windows(2)
        .rposition(|w| w == b"\n\n")
        .map(|p| p + 2);
    let Some(split_at) = last_boundary else {
        return String::new();
    };
    let decodable: Vec<u8> = byte_buf.drain(..split_at).collect();
    match std::str::from_utf8(&decodable) {
        Ok(s) => s.to_string(),
        Err(e) => {
            log::warn!("SSE utf8 error at byte {}: {}", e.valid_up_to(), e);
            std::str::from_utf8(&decodable[..e.valid_up_to()])
                .unwrap_or("")
                .to_string()
        }
    }
}

/// USD price per 1M tokens for each model. Source: anthropic.com pricing
/// page as of 2026-05. Update when prices change.
pub fn pricing_per_million(model: &str) -> (f64, f64) {
    // (input, output)
    match model {
        "claude-haiku-4-5" => (1.0, 5.0),
        "claude-sonnet-4-5" | "claude-sonnet-4-6" => (3.0, 15.0),
        "claude-opus-4-7" => (15.0, 75.0),
        _ => (3.0, 15.0), // safe upper-bound default
    }
}

/// USD float view of cost — convenience wrapper. Internal accounting
/// uses microcents (cost_microcents) to avoid f64 drift over long
/// sessions. UI displays the float.
#[allow(dead_code)]
pub fn cost_usd(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    // 1 USD = 100_000_000 microcents (1 microcent = 10⁻⁸ USD).
    (cost_microcents(model, input_tokens, output_tokens) as f64) / 100_000_000.0
}

/// Cost in microcents (1 USD = 100_000_000 microcents). Use this for
/// internal accumulation to avoid f64 precision loss over long sessions.
pub fn cost_microcents(model: &str, input_tokens: u64, output_tokens: u64) -> u64 {
    let (p_in_per_m, p_out_per_m) = pricing_per_million(model);
    // microcents per token = price_per_million_usd × 100_000_000 / 1_000_000 = price × 100
    let micro_in = (p_in_per_m * 100.0) as u64; // microcents per input token
    let micro_out = (p_out_per_m * 100.0) as u64;
    input_tokens
        .saturating_mul(micro_in)
        .saturating_add(output_tokens.saturating_mul(micro_out))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
}

/// Non-streaming completion — used for prep-context structuring where we
/// want the whole answer at once and latency is acceptable. Returns
/// (text, token_usage) so caller can track cost.
///
/// Wraps `complete_once` with up to 3 retries on transient failures
/// (network errors, HTTP 5xx, 429 rate-limit). Permanent failures (4xx
/// other than 429) short-circuit immediately so we don't waste time on
/// auth/quota errors that won't fix themselves. Backoff: 1s, 2s, 4s.
///
/// Added P1-2 (review 2026-05-25) — previously a single network blip would
/// kill an auto-tile or F9 ask and the user just saw "HTTP timeout" with no
/// auto-recovery. Bridge restart takes ~30s; 3 retries × 4s ≈ 12s window
/// catches most local-bridge hiccups without doubling user-visible latency
/// on the happy path.
pub async fn complete_with_usage(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<(String, TokenUsage)> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        match complete_once(base_url, bearer, model, messages.clone(), max_tokens).await {
            Ok(ok) => {
                if attempt > 1 {
                    log::info!(
                        "AI complete recovered on attempt {}/{}",
                        attempt,
                        MAX_ATTEMPTS
                    );
                }
                return Ok(ok);
            }
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

/// HTTP 4xx (except 429) = permanent: auth, quota, bad model name, oversized
/// request — retry won't fix any of these. Everything else is transient.
fn is_permanent_ai_error(msg: &str) -> bool {
    // Parse the numeric status after "HTTP " (errors are built as
    // anyhow!("HTTP {status}")) and treat any 4xx except 429 as permanent —
    // catches unlisted 4xx (e.g. 422) and avoids misreading a transient body
    // that merely contains an "HTTP 404" substring.
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

/// Single attempt — no retry. Extracted so the retry wrapper above can
/// call it cleanly with a fresh clone of `messages` each time.
async fn complete_once(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<(String, TokenUsage)> {
    let client = http_client();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "max_tokens": max_tokens,
    });
    apply_prompt_cache(&mut body);
    apply_local_no_think(&mut body);

    // SECURITY: don't log the host portion of the URL (LAN IP/topology). See
    // the matching comment on stream_chat above for the rationale.
    log::info!(
        "AI complete → /chat/completions (model={}, msgs={})",
        model,
        messages.len()
    );

    let resp = match client
        .post(&url)
        .timeout(std::time::Duration::from_secs(180))
        .bearer_auth(bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        // Generic on transport failure (see stream_inner): the reqwest url must
        // not reach a UI surface; log the detail, return a retryable message.
        Err(e) => {
            log::warn!("AI complete POST failed: {e:#}");
            return Err(anyhow!("AI connection error"));
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        // Keep status (classification), drop body (see stream_inner).
        let body = resp.text().await.unwrap_or_default();
        log::warn!(
            "AI complete HTTP {status} body: {}",
            body.chars().take(500).collect::<String>()
        );
        anyhow::bail!("HTTP {status}");
    }
    let v: serde_json::Value = resp.json().await.context("parse json")?;
    let text = v
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let usage = TokenUsage {
        input: v
            .get("usage")
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|n| n.as_u64())
            .unwrap_or(0),
        output: v
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|n| n.as_u64())
            .unwrap_or(0),
    };
    Ok((text, usage))
}

pub async fn complete(
    base_url: &str,
    bearer: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
) -> Result<String> {
    let (text, _usage) = complete_with_usage(base_url, bearer, model, messages, max_tokens).await?;
    Ok(text)
}

/// Convenience: build a typical "ask AI" request with system context +
/// rolling transcript + optional screenshot.
pub fn build_request(
    meeting_context: &str,
    response_language: &str,
    transcript_lines: &[String],
    screenshot_data_url: Option<&str>,
    user_question: Option<&str>,
) -> Vec<ChatMessage> {
    let mut messages = Vec::with_capacity(3);

    // System prompt: explicit role + meeting context + strict output rules.
    let lang_block = match response_language {
        "ru" => {
            "ВАЖНО: отвечай ИСКЛЮЧИТЕЛЬНО на русском языке. \
                 Английский только для названий технологий и команд (e.g. `kubectl`)."
        }
        "en" => "Respond exclusively in English.",
        _ => "Respond in the user's language.",
    };
    let ctx_block = if meeting_context.trim().is_empty() {
        "Контекст встречи не задан.".to_string()
    } else {
        format!(
            "Бэкграунд пользователя (фон для понимания уровня — НЕ ограничивай ответ этой темой \
             если вопрос про что-то другое):\n{}",
            meeting_context.trim()
        )
    };
    // RAG: pull curated KB entries for any domain term explicitly named in the
    // question/transcript, so the model answers from the reference instead of
    // guessing (e.g. a term like "Exasol" a small local model wasn't trained on).
    let kb_query = {
        let mut s = transcript_lines.join("\n");
        if let Some(q) = user_question {
            s.push('\n');
            s.push_str(q);
        }
        s
    };
    // Cap is in BYTES; KB bodies are mostly Cyrillic (~2 bytes/char), so a
    // single entry can be ~1.8 KB. Keep the cap generous enough to fit it.
    let kb_block = crate::kb::reference_for(&kb_query, 3, 4000)
        .map(|r| {
            format!(
                "\n\n=== Справка из базы знаний (точные определения терминов из вопроса; \
                 опирайся на них, НЕ выдумывай факты по этим терминам) ===\n{r}"
            )
        })
        .unwrap_or_default();
    let system_prompt = format!(
        "Ты — техничный AI-ассистент пользователя на встрече/интервью в реальном времени. \
         Пользователь нажимает F9 чтобы попросить тебя помочь с ответом на последний \
         вопрос/реплику из транскрипта.\n\n\
         {ctx_block}\n\n\
         === Содержимое ===\n\
         - Отвечай ПО СУТИ вопроса. Если про generic Linux/SQL/Python — отвечай про это, \
           не притягивай Kubernetes/контейнеры без необходимости.\n\
         - Контекст пользователя нужен чтобы понять уровень детализации, а не чтобы каждый \
           ответ строить вокруг его технологий.\n\n\
         === Формат ===\n\
         - БЕЗ преамбулы (\"Хороший вопрос!\", \"Конечно\"). Сразу к делу.\n\
         - Маркдаун: **жирный** для важного, `code` для команд, маркированные списки.\n\
         - Приводи КОНКРЕТНЫЕ команды/утилиты/числа, не общие фразы.\n\
         - Если вопрос неясен — дай вероятную интерпретацию + уточняющий вопрос.\n\
         - {lang_block}\n\
         - В транскрипте могут быть Whisper-артефакты — восстанавливай смысл из контекста \
           (\"К87С\" → \"K8s\", \"лоуд-эвередж\" → \"load average\", \"гинкс\" → \"nginx\").\n\
         - Источник `[System]` — собеседник, `[Mic]` — пользователь.{kb_block}"
    );
    messages.push(ChatMessage {
        role: "system".into(),
        content: MessageContent::Text(system_prompt),
    });

    // ── User turn: rolling transcript + optional explicit question + optional screenshot ──
    let mut parts: Vec<ContentPart> = Vec::new();

    let mut prompt = String::new();
    if !transcript_lines.is_empty() {
        prompt.push_str("Транскрипт последних реплик (внизу — самые свежие):\n\n");
        for line in transcript_lines {
            prompt.push_str(line);
            prompt.push('\n');
        }
        prompt.push('\n');
    }
    if let Some(q) = user_question {
        prompt.push_str("Помоги ответить: ");
        prompt.push_str(q);
        prompt.push('\n');
    } else {
        prompt.push_str(
            "На основе последнего вопроса в транскрипте предложи краткий ответ, \
             который я могу дать. Используй пункты если уместно. Не больше 200 слов.",
        );
    }
    parts.push(ContentPart::Text { text: prompt });

    if let Some(url) = screenshot_data_url {
        parts.push(ContentPart::ImageUrl {
            image_url: ImageUrl { url: url.into() },
        });
    }

    messages.push(ChatMessage {
        role: "user".into(),
        content: if parts.len() == 1 {
            if let ContentPart::Text { text } = &parts[0] {
                MessageContent::Text(text.clone())
            } else {
                MessageContent::Parts(parts)
            }
        } else {
            MessageContent::Parts(parts)
        },
    });

    messages
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    // ── Regression: P0 bug — UTF-8 split across network chunks must NOT panic ──

    #[test]
    fn drain_returns_empty_when_no_complete_frame() {
        let mut b: Vec<u8> = b"data: hello".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "");
        assert_eq!(b, b"data: hello"); // bytes preserved for next chunk
    }

    #[test]
    fn drain_splits_at_double_newline() {
        let mut b: Vec<u8> = b"data: a\n\ndata: b".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "data: a\n\n");
        assert_eq!(b, b"data: b"); // unfinished frame stays
    }

    /// THE bug we're guarding against: a Russian 2-byte char's bytes are
    /// split across two network reads. The first read ends mid-char; the
    /// second completes it. Old code did `from_utf8(&chunk).unwrap()` and
    /// would panic. New code must keep the leftover for the next call.
    #[test]
    fn drain_does_not_panic_when_utf8_split_across_chunks() {
        // "Привет" — П = 0xD0 0x9F. Find the byte offset that lands mid-char.
        let full = "data: \"Привет\"\n\n";
        let bytes = full.as_bytes();
        // First non-ASCII byte should be П's leading 0xD0. Split right after it.
        let p_start = bytes.iter().position(|&b| b == 0xD0).unwrap();
        let split = p_start + 1; // includes 0xD0 (leading byte) but not 0x9F (trailing)
        let chunk1 = &bytes[..split];
        let chunk2 = &bytes[split..];
        assert!(
            std::str::from_utf8(chunk1).is_err(),
            "test setup: chunk1 must be invalid UTF-8 (split mid Cyrillic char)"
        );

        let mut b: Vec<u8> = chunk1.to_vec();
        let s1 = drain_complete_frames(&mut b);
        // No \n\n yet, so nothing decoded, and no panic.
        assert_eq!(s1, "");

        b.extend_from_slice(chunk2);
        let s2 = drain_complete_frames(&mut b);
        // Now we have a complete frame ending in \n\n. Must decode cleanly.
        assert_eq!(s2, full);
        assert!(b.is_empty());
    }

    #[test]
    fn drain_handles_multiple_frames_in_one_chunk() {
        let mut b: Vec<u8> = b"data: a\n\ndata: b\n\ndata: c".to_vec();
        let s = drain_complete_frames(&mut b);
        assert_eq!(s, "data: a\n\ndata: b\n\n");
        assert_eq!(b, b"data: c");
    }

    // ── Smoke check on build_request shape ──

    #[test]
    fn build_request_always_includes_system_prompt() {
        let msgs = build_request("", "ru", &[], None, None);
        // system + user
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "system");
        assert_eq!(msgs[1].role, "user");
        // Russian directive present
        if let MessageContent::Text(s) = &msgs[0].content {
            assert!(s.contains("русском"));
        } else {
            panic!("system message should be text");
        }
    }

    #[test]
    fn build_request_injects_kb_reference_for_named_term() {
        // A question naming a KB term (Exasol) pulls its entry into the system
        // prompt. Regression guard for the byte-cap bug: the Cyrillic Exasol
        // body is ~1.8 KB, so too small a cap silently dropped it.
        let msgs = build_request("", "ru", &[], None, Some("Что такое Exasol?"));
        if let MessageContent::Text(s) = &msgs[0].content {
            assert!(
                s.contains("Справка из базы знаний"),
                "KB reference block missing from system prompt"
            );
            assert!(s.contains("Exasol"), "Exasol entry not injected");
            assert!(
                s.contains("MPP") || s.contains("columnar"),
                "Exasol body not injected"
            );
        } else {
            panic!("system message should be text");
        }
        // A generic question naming no KB key must NOT inject a block (no noise).
        let plain = build_request("", "ru", &[], None, Some("zzqq xkcdq vmwpq blortz"));
        if let MessageContent::Text(s) = &plain[0].content {
            assert!(
                !s.contains("Справка из базы знаний"),
                "KB block wrongly injected for a generic question"
            );
        } else {
            panic!("system message should be text");
        }
    }

    // ── NEW: cost/pricing math ──

    #[test]
    fn cost_microcents_haiku_known_value() {
        // Haiku: $1/M input + $5/M output. 1M input + 1M output = $6 = 600M microcents.
        // microcents per token: input=100, output=500
        assert_eq!(
            cost_microcents("claude-haiku-4-5", 1_000_000, 1_000_000),
            600_000_000
        );
    }

    #[test]
    fn cost_microcents_sonnet_pricing() {
        // Sonnet: $3/M + $15/M. 100k+50k = 300k*3/M + 50k*15/M ≈ $0.3 + $0.75 = $1.05
        // microcents per token: input=300, output=1500
        let m = cost_microcents("claude-sonnet-4-6", 100_000, 50_000);
        assert_eq!(m, 100_000 * 300 + 50_000 * 1500);
        assert!((cost_usd("claude-sonnet-4-6", 100_000, 50_000) - 1.05).abs() < 0.001);
    }

    #[test]
    fn cost_unknown_model_defaults_to_sonnet() {
        // Per pricing_per_million fallback.
        let m_known = cost_microcents("claude-sonnet-4-5", 1000, 1000);
        let m_unknown = cost_microcents("qwen-14b", 1000, 1000);
        assert_eq!(
            m_known, m_unknown,
            "unknown model should fall back to sonnet pricing"
        );
    }

    #[test]
    fn cost_zero_tokens_is_zero() {
        assert_eq!(cost_microcents("claude-haiku-4-5", 0, 0), 0);
        assert_eq!(cost_usd("claude-haiku-4-5", 0, 0), 0.0);
    }

    #[test]
    fn cost_saturating_no_overflow() {
        // Max u64 input shouldn't panic.
        let m = cost_microcents("claude-opus-4-7", u64::MAX, u64::MAX);
        assert_eq!(m, u64::MAX, "should saturate, not panic");
    }

    // ── is_permanent_ai_error classifier (used by retry wrapper) ──

    #[test]
    fn permanent_error_400_no_retry() {
        // 400 = bad request payload (e.g. oversized prompt, malformed JSON).
        // Retrying won't fix the request — fail fast.
        assert!(is_permanent_ai_error("HTTP 400: invalid request"));
    }

    #[test]
    fn permanent_error_auth_no_retry() {
        // 401 = bad bearer token. 403 = forbidden / quota exceeded.
        // User must fix Settings → no retry.
        assert!(is_permanent_ai_error("HTTP 401: unauthorized"));
        assert!(is_permanent_ai_error("HTTP 403: forbidden"));
    }

    #[test]
    fn permanent_error_404_no_retry() {
        // 404 = endpoint missing (typo in ai_base_url) or model not found.
        // Will keep 404'ing on retry — fail fast.
        assert!(is_permanent_ai_error("HTTP 404: not found"));
    }

    #[test]
    fn permanent_error_413_no_retry() {
        // 413 = payload too large. Retry without changing payload pointless.
        assert!(is_permanent_ai_error("HTTP 413: request entity too large"));
    }

    #[test]
    fn transient_error_5xx_retries() {
        // Server-side problems — bridge restart, upstream Claude blip, etc.
        // Retry MAY succeed.
        assert!(!is_permanent_ai_error("HTTP 500: internal server error"));
        assert!(!is_permanent_ai_error("HTTP 502: bad gateway"));
        assert!(!is_permanent_ai_error("HTTP 503: service unavailable"));
        assert!(!is_permanent_ai_error("HTTP 504: gateway timeout"));
    }

    #[test]
    fn transient_error_429_retries() {
        // Rate limit — retry after exponential backoff usually clears it.
        // Note: NOT in the permanent list per the docstring (4xx EXCEPT 429).
        assert!(!is_permanent_ai_error("HTTP 429: rate limited"));
    }

    #[test]
    fn transient_network_errors_retry() {
        // Connection refused, timeout, DNS — all transient.
        assert!(!is_permanent_ai_error("Connection refused"));
        assert!(!is_permanent_ai_error("request timed out"));
        assert!(!is_permanent_ai_error("DNS resolution failed"));
        assert!(!is_permanent_ai_error("connection reset by peer"));
    }

    #[test]
    fn empty_error_does_not_match_permanent() {
        // Defensive: empty error string should NOT be classified as permanent
        // (otherwise we'd suppress retry for any error that gets stringified
        // to "").
        assert!(!is_permanent_ai_error(""));
    }

    #[test]
    fn build_request_attaches_screenshot_as_image_part() {
        let msgs = build_request(
            "",
            "ru",
            &["[System] что такое etcd?".to_string()],
            Some("data:image/jpeg;base64,XXX"),
            None,
        );
        if let MessageContent::Parts(parts) = &msgs[1].content {
            assert!(parts
                .iter()
                .any(|p| matches!(p, ContentPart::ImageUrl { .. })));
        } else {
            panic!("user content should be parts when screenshot attached");
        }
    }
}
