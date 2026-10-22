use super::types::{AiEndpoint, AiProtocol};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

pub async fn acquire_exclusive_ai() -> Result<super::queue::AiPermit> {
    super::queue::acquire_exclusive()
        .await
        .map_err(|_| anyhow!("AI request queue closed"))
}

pub(crate) fn http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        })
        .clone()
}

pub(crate) fn transport_failure_kind(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_request() {
        "request"
    } else {
        "transport"
    }
}

pub(crate) static PROMPT_CACHE: AtomicBool = AtomicBool::new(false);

pub fn set_prompt_cache(on: bool) {
    PROMPT_CACHE.store(on, Ordering::Relaxed);
}

pub(crate) fn apply_prompt_cache(body: &mut Value) {
    if !PROMPT_CACHE.load(Ordering::Relaxed) {
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

/// Reasoning level per (protocol, base URL) of the remote profiles, set from
/// the config at start and on every change in Settings.
static REASONING_EFFORTS: std::sync::RwLock<Vec<(AiProtocol, String, String)>> =
    std::sync::RwLock::new(Vec::new());

pub fn set_reasoning_efforts(efforts: Vec<(AiProtocol, String, String)>) {
    *REASONING_EFFORTS.write().unwrap_or_else(|p| p.into_inner()) = efforts;
}

/// Add the configured reasoning level for this endpoint to a request body:
/// `reasoning_effort` for Chat Completions, `reasoning.effort` for Responses.
/// Other protocols and endpoints without a level are left unchanged.
pub(crate) fn apply_reasoning_effort(body: &mut Value, protocol: AiProtocol, base_url: &str) {
    let url = base_url.trim().trim_end_matches('/');
    let efforts = REASONING_EFFORTS.read().unwrap_or_else(|p| p.into_inner());
    let Some((_, _, effort)) = efforts
        .iter()
        .find(|(p, u, _)| *p == protocol && u.trim_end_matches('/') == url)
    else {
        return;
    };
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    match protocol {
        AiProtocol::OpenAiCompatible => {
            obj.insert("reasoning_effort".to_string(), json!(effort));
        }
        AiProtocol::OpenAiResponses => {
            obj.insert("reasoning".to_string(), json!({ "effort": effort }));
        }
        AiProtocol::AnthropicMessages | AiProtocol::CodexSubscription => {}
    }
}

pub(crate) static LOCAL_NO_THINK: AtomicBool = AtomicBool::new(false);

pub fn set_local_no_think(on: bool) {
    LOCAL_NO_THINK.store(on, Ordering::Relaxed);
}

pub(crate) fn apply_local_no_think(body: &mut Value, force: bool) {
    if !force && !LOCAL_NO_THINK.load(Ordering::Relaxed) {
        return;
    }
    if let Some(obj) = body.as_object_mut() {
        obj.insert(
            "chat_template_kwargs".to_string(),
            json!({ "enable_thinking": false }),
        );
    }
}

pub(crate) fn apply_managed_gemma_sampler(body: &mut Value, base_url: &str, model: &str) {
    if !crate::local_ai::is_managed_llama_endpoint(base_url)
        || !model.to_ascii_lowercase().contains("gemma")
    {
        return;
    }
    if let Some(obj) = body.as_object_mut() {
        obj.insert("temperature".to_string(), json!(1.0));
        obj.insert("top_p".to_string(), json!(0.95));
        obj.insert("top_k".to_string(), json!(64));
    }
}

pub(crate) fn is_managed_mlx_endpoint(endpoint: &AiEndpoint) -> bool {
    endpoint.protocol == AiProtocol::OpenAiCompatible
        && endpoint.is_local
        && crate::mlx_install::catalog_model(&endpoint.model).is_some()
        && (endpoint.base_url.trim().is_empty()
            || crate::mlx_runtime::is_owned_endpoint(&endpoint.base_url))
}

pub(crate) async fn resolve_managed_mlx_endpoint(
    mut endpoint: AiEndpoint,
) -> Result<(AiEndpoint, Option<crate::mlx_runtime::MlxRequestLease>)> {
    if !is_managed_mlx_endpoint(&endpoint) {
        return Ok((endpoint, None));
    }
    let model = endpoint.model.clone();
    let acquired = tokio::task::spawn_blocking(move || crate::mlx_runtime::acquire_request(&model))
        .await
        .map_err(|_| anyhow!("MLX model unavailable"))?;
    let (owned, lease) = match acquired {
        Ok(acquired) => acquired,
        Err(_) => {
            log::warn!("MLX model activation failed");
            return Err(anyhow!("MLX model unavailable"));
        }
    };
    endpoint.base_url = owned.base_url;
    endpoint.bearer = owned.bearer;
    endpoint.model = owned.model;
    Ok((endpoint, Some(lease)))
}
