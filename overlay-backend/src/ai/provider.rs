use super::{AiProtocol, ChatMessage, ContentPart, MessageContent, TokenUsage};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub(super) struct ParsedStreamEvent {
    pub id: Option<String>,
    pub delta: Option<String>,
    pub done: Option<String>,
    pub failed: bool,
}

pub(super) fn endpoint_path(protocol: AiProtocol) -> &'static str {
    match protocol {
        AiProtocol::OpenAiCompatible => "chat/completions",
        AiProtocol::OpenAiResponses => "responses",
        AiProtocol::AnthropicMessages => "messages",
    }
}

pub(super) fn authorize(
    request: reqwest::RequestBuilder,
    protocol: AiProtocol,
    credential: &str,
) -> reqwest::RequestBuilder {
    match protocol {
        AiProtocol::OpenAiCompatible | AiProtocol::OpenAiResponses => {
            request.bearer_auth(credential)
        }
        AiProtocol::AnthropicMessages => request
            .header("x-api-key", credential)
            .header("anthropic-version", "2023-06-01"),
    }
}

pub(super) fn request_body(
    protocol: AiProtocol,
    model: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
    stream: bool,
    prompt_cache: bool,
) -> Result<Value> {
    match protocol {
        AiProtocol::OpenAiCompatible => Ok(json!({
            "model": model,
            "messages": messages,
            "stream": stream,
            "max_tokens": max_tokens,
        })),
        AiProtocol::OpenAiResponses => {
            let instructions = messages
                .iter()
                .filter(|message| message.role == "system")
                .filter_map(|message| match &message.content {
                    MessageContent::Text(text) => Some(text.as_str()),
                    MessageContent::Parts(_) => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let input = messages
                .iter()
                .filter(|message| message.role != "system")
                .map(|message| {
                    Ok(json!({
                        "role": message.role,
                        "content": openai_content(&message.content),
                    }))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut body = json!({
                "model": model,
                "input": input,
                "stream": stream,
                "max_output_tokens": max_tokens,
            });
            if !instructions.is_empty() {
                body["instructions"] = Value::String(instructions);
            }
            Ok(body)
        }
        AiProtocol::AnthropicMessages => {
            let system = messages
                .iter()
                .filter(|message| message.role == "system")
                .filter_map(|message| match &message.content {
                    MessageContent::Text(text) => Some(text.as_str()),
                    MessageContent::Parts(_) => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let api_messages = messages
                .iter()
                .filter(|message| message.role != "system")
                .map(|message| {
                    Ok(json!({
                        "role": message.role,
                        "content": anthropic_content(&message.content)?,
                    }))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut body = json!({
                "model": model,
                "messages": api_messages,
                "stream": stream,
                "max_tokens": max_tokens,
            });
            if !system.is_empty() {
                body["system"] = if prompt_cache {
                    json!([{
                        "type": "text",
                        "text": system,
                        "cache_control": { "type": "ephemeral" },
                    }])
                } else {
                    Value::String(system)
                };
            }
            Ok(body)
        }
    }
}

fn openai_content(content: &MessageContent) -> Value {
    match content {
        MessageContent::Text(text) => Value::String(text.clone()),
        MessageContent::Parts(parts) => Value::Array(
            parts
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => json!({ "type": "input_text", "text": text }),
                    ContentPart::ImageUrl { image_url } => json!({
                        "type": "input_image",
                        "image_url": image_url.url,
                    }),
                })
                .collect(),
        ),
    }
}

fn anthropic_content(content: &MessageContent) -> Result<Value> {
    let parts = match content {
        MessageContent::Text(text) => vec![json!({ "type": "text", "text": text })],
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                ContentPart::Text { text } => Ok(json!({ "type": "text", "text": text })),
                ContentPart::ImageUrl { image_url } => {
                    let (media_type, data) = parse_data_url(&image_url.url)?;
                    Ok(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": data,
                        }
                    }))
                }
            })
            .collect::<Result<Vec<_>>>()?,
    };
    Ok(Value::Array(parts))
}

fn parse_data_url(url: &str) -> Result<(&str, &str)> {
    let payload = url
        .strip_prefix("data:")
        .ok_or_else(|| anyhow!("unsupported image source"))?;
    let (metadata, data) = payload
        .split_once(',')
        .ok_or_else(|| anyhow!("invalid image source"))?;
    let media_type = metadata
        .strip_suffix(";base64")
        .ok_or_else(|| anyhow!("image must be base64 encoded"))?;
    if !matches!(
        media_type,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp"
    ) {
        return Err(anyhow!("unsupported image format"));
    }
    Ok((media_type, data))
}

pub(super) fn parse_stream(protocol: AiProtocol, value: &Value) -> ParsedStreamEvent {
    match protocol {
        AiProtocol::OpenAiCompatible => {
            let choice = value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first());
            ParsedStreamEvent {
                id: value.get("id").and_then(Value::as_str).map(str::to_string),
                delta: choice
                    .and_then(|item| item.get("delta"))
                    .and_then(|delta| delta.get("content"))
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(str::to_string),
                done: choice
                    .and_then(|item| item.get("finish_reason"))
                    .and_then(Value::as_str)
                    .filter(|reason| !reason.is_empty())
                    .map(str::to_string),
                failed: false,
            }
        }
        AiProtocol::OpenAiResponses => {
            let kind = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            ParsedStreamEvent {
                id: (kind == "response.created")
                    .then(|| value.get("response"))
                    .flatten()
                    .and_then(|response| response.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                delta: (kind == "response.output_text.delta")
                    .then(|| value.get("delta"))
                    .flatten()
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(str::to_string),
                done: if kind == "response.completed" {
                    Some("stop".to_string())
                } else if kind == "response.incomplete" {
                    Some("length".to_string())
                } else {
                    None
                },
                failed: kind == "response.failed" || kind == "error",
            }
        }
        AiProtocol::AnthropicMessages => {
            let kind = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            ParsedStreamEvent {
                id: (kind == "message_start")
                    .then(|| value.get("message"))
                    .flatten()
                    .and_then(|message| message.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                delta: (kind == "content_block_delta")
                    .then(|| value.get("delta"))
                    .flatten()
                    .and_then(|delta| delta.get("text"))
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(str::to_string),
                done: if kind == "message_delta" {
                    value
                        .get("delta")
                        .and_then(|delta| delta.get("stop_reason"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                } else if kind == "message_stop" {
                    Some("stop".to_string())
                } else {
                    None
                },
                failed: kind == "error",
            }
        }
    }
}

pub(super) fn parse_completion(
    protocol: AiProtocol,
    value: &Value,
    elapsed_secs: f64,
) -> (String, TokenUsage) {
    let (text, input, output, finish_reason, server_tps) = match protocol {
        AiProtocol::OpenAiCompatible => (
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            value
                .pointer("/usage/prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            value
                .pointer("/usage/completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            value
                .pointer("/choices/0/finish_reason")
                .and_then(Value::as_str)
                .filter(|reason| !reason.is_empty())
                .unwrap_or("stop")
                .to_string(),
            value
                .pointer("/timings/predicted_per_second")
                .and_then(Value::as_f64),
        ),
        AiProtocol::OpenAiResponses => (
            value
                .get("output")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|item| item.get("content").and_then(Value::as_array))
                .flatten()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>(),
            value
                .pointer("/usage/input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            if value.get("status").and_then(Value::as_str) == Some("incomplete") {
                value
                    .pointer("/incomplete_details/reason")
                    .and_then(Value::as_str)
                    .unwrap_or("length")
                    .to_string()
            } else {
                "stop".to_string()
            },
            None,
        ),
        AiProtocol::AnthropicMessages => (
            value
                .get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>(),
            value
                .pointer("/usage/input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            value
                .pointer("/usage/output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            value
                .get("stop_reason")
                .and_then(Value::as_str)
                .filter(|reason| !reason.is_empty())
                .unwrap_or("stop")
                .to_string(),
            None,
        ),
    };
    let tok_per_sec = server_tps
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or_else(|| {
            if elapsed_secs > 0.0 && output > 0 {
                output as f64 / elapsed_secs
            } else {
                0.0
            }
        });
    (
        text,
        TokenUsage {
            input,
            output,
            tok_per_sec,
            finish_reason,
        },
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::ai::{ImageUrl, MessageContent};

    fn messages() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: "system".into(),
                content: MessageContent::Text("be brief".into()),
            },
            ChatMessage {
                role: "user".into(),
                content: MessageContent::Parts(vec![
                    ContentPart::Text {
                        text: "what?".into(),
                    },
                    ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: "data:image/png;base64,AAAA".into(),
                        },
                    },
                ]),
            },
        ]
    }

    #[test]
    fn responses_body_uses_native_fields_and_image_shape() {
        let body = request_body(
            AiProtocol::OpenAiResponses,
            "gpt-test",
            &messages(),
            77,
            true,
            true,
        )
        .unwrap();
        assert_eq!(body["instructions"], "be brief");
        assert_eq!(body["max_output_tokens"], 77);
        assert_eq!(body["input"][0]["content"][1]["type"], "input_image");
        assert!(body.get("messages").is_none());
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn anthropic_body_uses_header_protocol_shape_and_cache_control() {
        let body = request_body(
            AiProtocol::AnthropicMessages,
            "claude-test",
            &messages(),
            88,
            false,
            true,
        )
        .unwrap();
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(
            body["messages"][0]["content"][1]["source"]["media_type"],
            "image/png"
        );
        assert_eq!(body["max_tokens"], 88);
    }

    #[test]
    fn parses_responses_and_anthropic_stream_events() {
        let openai = parse_stream(
            AiProtocol::OpenAiResponses,
            &json!({"type":"response.output_text.delta","delta":"hi"}),
        );
        assert_eq!(openai.delta.as_deref(), Some("hi"));
        let anthropic = parse_stream(
            AiProtocol::AnthropicMessages,
            &json!({"type":"content_block_delta","delta":{"type":"text_delta","text":"ok"}}),
        );
        assert_eq!(anthropic.delta.as_deref(), Some("ok"));
    }

    #[test]
    fn parses_native_usage_and_text() {
        let (text, usage) = parse_completion(
            AiProtocol::OpenAiResponses,
            &json!({
                "status":"completed",
                "output":[{"content":[{"type":"output_text","text":"answer"}]}],
                "usage":{"input_tokens":10,"output_tokens":2}
            }),
            1.0,
        );
        assert_eq!(text, "answer");
        assert_eq!(usage.input, 10);
        assert_eq!(usage.output, 2);
    }
}
