//! What kind of local server answers at a base URL, the request fields that
//! make it answer faster, and keeping the live-answer prompt in its cache.
//!
//! A server the user already runs keeps its own launch settings; Suflyor can
//! still shape each request. Strata (a hybrid-attention engine) only reuses a
//! prompt from a checkpoint, so the system prompt is pinned as a shared prefix
//! and one-shot calls are told not to park a checkpoint. Every server with one
//! resident sequence (Strata, a single-slot llama.cpp) loses the live-answer
//! prompt to any other request, so during a session the prompt is read again
//! in the background once the other request is done ("primed").

use super::control::{apply_local_no_think, http_client, AI_SEMAPHORE};
use super::types::{AiEndpoint, AiProtocol, ChatMessage, MessageContent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// The local server behind a base URL, as far as its replies tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerKind {
    Strata,
    LlamaCpp,
    Ollama,
    Other,
}

/// Requests at or under this many tokens are one-shot calls (probes,
/// classifications): no later request continues them.
const ONE_SHOT_MAX_TOKENS: u64 = 32;
/// How long a probe of one server path may take.
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);
/// Quiet time after another request before the live prompt is read again.
const PRIME_DELAY: Duration = Duration::from_millis(400);

static KINDS: Mutex<Option<HashMap<String, ServerKind>>> = Mutex::new(None);

fn key(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_ascii_lowercase()
}

/// The kind already detected for `base_url`, if any.
#[must_use]
pub fn known_kind(base_url: &str) -> Option<ServerKind> {
    KINDS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .and_then(|m| m.get(&key(base_url)).copied())
}

fn remember_kind(base_url: &str, kind: ServerKind) {
    KINDS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(key(base_url), kind);
}

/// The server root: the base URL without its trailing `/v1`.
fn server_root(base_url: &str) -> &str {
    let trimmed = base_url.trim_end_matches('/');
    trimmed.strip_suffix("/v1").unwrap_or(trimmed)
}

async fn get_json(url: &str) -> Option<Value> {
    let resp = http_client()
        .get(url)
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json().await.ok()
}

/// Tell the kind from what the server's own status paths return.
#[must_use]
pub fn kind_from_replies(
    health: Option<&Value>,
    props: Option<&Value>,
    ollama_version: Option<&Value>,
) -> ServerKind {
    if health
        .and_then(|h| h.get("service"))
        .and_then(Value::as_str)
        == Some("strata")
    {
        return ServerKind::Strata;
    }
    if props.is_some_and(|p| p.get("default_generation_settings").is_some()) {
        return ServerKind::LlamaCpp;
    }
    if ollama_version.is_some_and(|v| v.get("version").is_some()) {
        return ServerKind::Ollama;
    }
    ServerKind::Other
}

/// Probe a local server once and remember what it is.
pub async fn detect_kind(base_url: &str) -> ServerKind {
    if let Some(kind) = known_kind(base_url) {
        return kind;
    }
    let root = server_root(base_url).to_string();
    let (health_url, props_url, version_url) = (
        format!("{root}/health"),
        format!("{root}/props"),
        format!("{root}/api/version"),
    );
    let (health, props, version) = tokio::join!(
        get_json(&health_url),
        get_json(&props_url),
        get_json(&version_url),
    );
    let kind = kind_from_replies(health.as_ref(), props.as_ref(), version.as_ref());
    log::info!("local AI server kind: {kind:?}");
    remember_kind(base_url, kind);
    kind
}

/// Add the fields that make `kind` answer faster to an OpenAI-compatible
/// chat body. Strata: a request that starts with a system prompt pins it as a
/// shared prefix (its next request with the same prompt skips reading it);
/// a one-shot call parks no checkpoint, so it does not push that prefix out.
pub(crate) fn apply_kind_hints(body: &mut Value, kind: ServerKind) {
    if kind != ServerKind::Strata {
        return;
    }
    let one_shot = body
        .get("max_tokens")
        .and_then(Value::as_u64)
        .is_some_and(|n| n <= ONE_SHOT_MAX_TOKENS);
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    if one_shot {
        obj.insert("strata_checkpoint".into(), json!(false));
        return;
    }
    let pinnable = obj
        .get("messages")
        .and_then(Value::as_array)
        .is_some_and(|msgs| {
            msgs.len() >= 2
                && msgs[0].get("role").and_then(Value::as_str) == Some("system")
                && msgs[1]
                    .get("content")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.is_empty())
        });
    if pinnable {
        // Strata's template cannot render a system message alone, so the
        // prefix ends one character into the first user message.
        obj.insert("strata_prefix".into(), json!({ "message": 1, "chars": 1 }));
    }
}

/// `apply_kind_hints` for a local endpoint whose kind is known; an unknown
/// local server is probed in the background for the next request.
pub(crate) fn apply_server_hints(body: &mut Value, endpoint: &AiEndpoint) {
    if endpoint.protocol != AiProtocol::OpenAiCompatible || !endpoint.is_local {
        return;
    }
    match known_kind(&endpoint.base_url) {
        Some(kind) => apply_kind_hints(body, kind),
        None => {
            let base_url = endpoint.base_url.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    detect_kind(&base_url).await;
                });
            }
        }
    }
}

/// The live-answer prompt kept in the local server's cache during a session.
struct HotPrompt {
    endpoint: AiEndpoint,
    system: String,
}

static HOT: Mutex<Option<HotPrompt>> = Mutex::new(None);
/// Bumped by every finished request; a scheduled prime runs only if no other
/// request finished after it was scheduled.
static PRIME_GEN: AtomicU64 = AtomicU64::new(0);

/// Keep `system` (the live-answer system prompt) in the cache of the local
/// server at `endpoint` until `clear_hot_prompt`.
pub fn set_hot_prompt(endpoint: AiEndpoint, system: String) {
    if endpoint.protocol != AiProtocol::OpenAiCompatible || !endpoint.is_local {
        return;
    }
    *HOT.lock().unwrap_or_else(|p| p.into_inner()) = Some(HotPrompt { endpoint, system });
}

/// Stop priming: the session ended.
pub fn clear_hot_prompt() {
    *HOT.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

fn first_system_text(messages: &[ChatMessage]) -> Option<&str> {
    let first = messages.first()?;
    if first.role != "system" {
        return None;
    }
    match &first.content {
        MessageContent::Text(text) => Some(text),
        MessageContent::Parts(_) => None,
    }
}

/// Marks one request to a server; when it is dropped (the request finished,
/// failed or was cancelled) the live prompt is read again if this request
/// was not it.
pub(crate) struct RequestDone {
    base_url: String,
    system: Option<String>,
}

pub(crate) fn request_started(base_url: &str, messages: &[ChatMessage]) -> RequestDone {
    RequestDone {
        base_url: base_url.to_string(),
        system: first_system_text(messages).map(str::to_string),
    }
}

impl Drop for RequestDone {
    fn drop(&mut self) {
        note_request_done(&self.base_url, self.system.as_deref());
    }
}

fn note_request_done(base_url: &str, system: Option<&str>) {
    let generation = PRIME_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let needed = {
        let hot = HOT.lock().unwrap_or_else(|p| p.into_inner());
        hot.as_ref().is_some_and(|h| {
            key(&h.endpoint.base_url) == key(base_url) && system != Some(h.system.as_str())
        })
    };
    if !needed {
        return;
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        tokio::time::sleep(PRIME_DELAY).await;
        if PRIME_GEN.load(Ordering::SeqCst) == generation {
            prime_hot_prompt().await;
        }
    });
}

/// Read the live prompt into the server's cache now, if the server is idle.
/// Best effort: any failure only means the next answer reads it itself.
pub async fn prime_hot_prompt() {
    let Some((endpoint, system)) = HOT
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .map(|h| (h.endpoint.clone(), h.system.clone()))
    else {
        return;
    };
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), &endpoint.base_url)
    {
        return;
    }
    // Both permits free: nothing else is running or queued.
    let Ok(_permit) = AI_SEMAPHORE.try_acquire_many(2) else {
        return;
    };
    let kind = detect_kind(&endpoint.base_url).await;
    let mut body = json!({
        "model": endpoint.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": "…" },
        ],
        "max_tokens": 1,
        "stream": false,
    });
    apply_local_no_think(&mut body, false);
    if kind == ServerKind::Strata {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("strata_prefix".into(), json!({ "message": 1, "chars": 1 }));
        }
    }
    let url = format!(
        "{}/{}",
        endpoint.base_url.trim_end_matches('/'),
        super::provider::endpoint_path(endpoint.protocol)
    );
    let t0 = std::time::Instant::now();
    let sent = super::provider::authorize(
        http_client().post(&url).timeout(Duration::from_secs(60)),
        endpoint.protocol,
        &endpoint.bearer,
    )
    .json(&body)
    .send()
    .await;
    match sent {
        Ok(resp) if resp.status().is_success() => {
            log::info!("live prompt primed in {} ms", t0.elapsed().as_millis());
        }
        Ok(resp) => log::info!("live prompt prime refused: HTTP {}", resp.status().as_u16()),
        Err(e) => log::info!(
            "live prompt prime failed ({})",
            super::control::transport_failure_kind(&e)
        ),
    }
}
