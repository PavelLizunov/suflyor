use super::control::{http_client, resolve_managed_mlx_endpoint, transport_failure_kind};
use super::provider;
use super::stream::codex_failure_message;
use super::types::{AiEndpoint, AiProtocol, ChatMessage, ContentPart, MessageContent};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

pub async fn test_connection(base_url: String, bearer: String, model: String) -> Result<String> {
    test_connection_endpoint(AiEndpoint {
        protocol: AiProtocol::OpenAiCompatible,
        base_url,
        bearer,
        model,
        reasoning_effort: None,
        is_local: false,
    })
    .await
}

pub async fn test_connection_endpoint(endpoint: AiEndpoint) -> Result<String> {
    test_connection_messages(
        endpoint,
        vec![ChatMessage {
            role: "user".into(),
            content: MessageContent::Text("ping".into()),
        }],
    )
    .await
}

pub async fn test_connection_messages(
    endpoint: AiEndpoint,
    messages: Vec<ChatMessage>,
) -> Result<String> {
    if endpoint.protocol == AiProtocol::CodexSubscription {
        let selected_model = endpoint.model.clone();
        let snapshot = tokio::task::spawn_blocking(crate::codex_subscription::provider_snapshot)
            .await
            .map_err(|_| anyhow!("Codex account unavailable"))?;
        let signed_in = matches!(
            snapshot.account,
            crate::codex_subscription::AccountState::SignedIn { .. }
        );
        let selected = snapshot
            .models
            .iter()
            .find(|model| model.id == selected_model);
        let has_image = messages.iter().any(|message| {
            matches!(&message.content, MessageContent::Parts(parts)
                if parts.iter().any(|part| matches!(part, ContentPart::ImageUrl { .. })))
        });
        if signed_in && selected.is_none() {
            return Err(anyhow!("Selected Codex model unavailable"));
        }
        if signed_in {
            if !has_image {
                return Ok("Codex account ready".into());
            }
            if !selected.is_some_and(|model| {
                model
                    .input_modalities
                    .iter()
                    .any(|modality| modality == "image")
            }) {
                return Err(anyhow!("Selected Codex model does not accept images"));
            }
            let effort = endpoint.reasoning_effort.clone();
            tokio::task::spawn_blocking(move || {
                crate::codex_subscription::run_turn(
                    &selected_model,
                    effort.as_deref(),
                    &messages,
                    |_| true,
                )
                .map_err(|failure| anyhow!(codex_failure_message(failure)))
            })
            .await
            .map_err(|_| anyhow!("Codex vision unavailable"))??;
            return Ok("Codex vision ready".into());
        }
        return Err(anyhow!("Codex account unavailable"));
    }
    let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(endpoint).await?;
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
    let body = provider::request_body(
        endpoint.protocol,
        &endpoint.model,
        &messages,
        1,
        false,
        false,
    )?;
    let request = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(10));
    let resp = match provider::authorize(request, endpoint.protocol, &endpoint.bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(error) => {
            log::warn!(
                "AI provider test transport error ({})",
                transport_failure_kind(&error)
            );
            return Err(anyhow!("connection failed"));
        }
    };
    let status = resp.status();
    if status.is_success() {
        Ok(format!("HTTP {}", status.as_u16()))
    } else {
        let txt = resp.text().await.unwrap_or_default();
        log::warn!(
            "{}",
            crate::http_log::http_error_line("AI bridge test", status.as_u16(), txt.len())
        );
        Err(anyhow!("HTTP {}", status.as_u16()))
    }
}

pub async fn list_models(base_url: &str, bearer: &str) -> Result<Vec<String>> {
    list_models_endpoint(&AiEndpoint {
        protocol: AiProtocol::OpenAiCompatible,
        base_url: base_url.to_string(),
        bearer: bearer.to_string(),
        model: String::new(),
        reasoning_effort: None,
        is_local: false,
    })
    .await
}

pub async fn list_models_endpoint(endpoint: &AiEndpoint) -> Result<Vec<String>> {
    if endpoint.protocol == AiProtocol::CodexSubscription {
        let snapshot = tokio::task::spawn_blocking(crate::codex_subscription::provider_snapshot)
            .await
            .map_err(|_| anyhow!("Codex model catalog unavailable"))?;
        return Ok(snapshot.models.into_iter().map(|model| model.id).collect());
    }
    if !endpoint.protocol.supports_model_listing() {
        return Ok(Vec::new());
    }
    let (endpoint, _mlx_lease) = resolve_managed_mlx_endpoint(endpoint.clone()).await?;
    if crate::deep_lock::endpoint_blocked(crate::deep_lock::deep_lock_active(), &endpoint.base_url)
    {
        return Err(anyhow!(crate::deep_lock::BLOCKED_ERROR));
    }
    let client = http_client();
    let url = format!("{}/models", endpoint.base_url.trim_end_matches('/'));
    let mut req = client.get(&url).timeout(std::time::Duration::from_secs(8));
    if !endpoint.bearer.is_empty() {
        req = provider::authorize(req, endpoint.protocol, &endpoint.bearer);
    }
    let resp = match req.send().await {
        Ok(response) => response,
        Err(error) => {
            log::warn!(
                "AI model-list transport error ({})",
                transport_failure_kind(&error)
            );
            return Err(anyhow!("connection failed"));
        }
    };
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        log::warn!(
            "{}",
            crate::http_log::http_error_line("list_models", status.as_u16(), txt.len())
        );
        return Err(anyhow!("HTTP {}", status.as_u16()));
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
