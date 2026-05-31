//! Vision-AI channel — standalone screenshot understanding.
//!
//! Resolved through a SEPARATE endpoint ([`crate::config::Config::vision_endpoint`])
//! so a local text model can keep answering interview questions while
//! screenshots go to a vision-capable model (cloud Sonnet, or a 2nd local
//! vision server). This module is deliberately thin: it owns the fixed capture
//! prompt + the image-message shape, and reuses [`crate::ai::stream_chat`] for
//! all HTTP / SSE / retry / cost / secret-safe error handling.

use crate::ai::{stream_chat, AiEvent, ChatMessage, ContentPart, ImageUrl, MessageContent};
use crate::config::AiEndpoint;
use tokio::sync::mpsc;

/// Max tokens for a vision answer. A capture usually asks a single question
/// (read / solve / explain), so a moderate budget keeps latency + cost down.
pub const VISION_MAX_TOKENS: u32 = 1024;

/// Fixed MVP capture prompt (RU). One prompt for now — presets are a later phase.
pub const DEFAULT_VISION_PROMPT: &str = "Что на этом скриншоте? Если это вопрос или \
     задача — ответь по делу и кратко (маркдаун, конкретика). Если это просто экран или \
     текст — кратко опиши суть.";

/// Build a one-shot vision request: a single user turn with the prompt text +
/// the screenshot as an image part. No transcript/KB — this is the standalone
/// capture channel, NOT the F9 interview-answer flow ([`crate::ai::build_request`]).
/// An empty/whitespace `prompt` falls back to [`DEFAULT_VISION_PROMPT`].
#[must_use]
pub fn build_vision_request(image_data_url: &str, prompt: &str) -> Vec<ChatMessage> {
    let prompt = if prompt.trim().is_empty() {
        DEFAULT_VISION_PROMPT
    } else {
        prompt
    };
    vec![ChatMessage {
        role: "user".into(),
        content: MessageContent::Parts(vec![
            ContentPart::Text {
                text: prompt.to_string(),
            },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_data_url.to_string(),
                },
            },
        ]),
    }]
}

/// Stream a vision answer for a captured image through `ep`. Thin wrapper over
/// [`crate::ai::stream_chat`] (reuses the pooled client, SSE parsing, retry, and
/// the secret-safe error handling). Cost is the caller's concern: zero it when
/// `ep.is_local`, exactly like the text path.
///
/// `image_data_url` must be a complete data URI, e.g.
/// `"data:image/jpeg;base64,…"`.
#[must_use]
pub fn stream_vision(
    ep: AiEndpoint,
    image_data_url: String,
    prompt: String,
) -> mpsc::Receiver<AiEvent> {
    let messages = build_vision_request(&image_data_url, &prompt);
    stream_chat(
        ep.base_url,
        ep.bearer,
        ep.model,
        messages,
        VISION_MAX_TOKENS,
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn vision_request_has_text_then_image_part() {
        let msgs = build_vision_request("data:image/jpeg;base64,AAAA", "прочитай");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
        assert!(
            matches!(&msgs[0].content, MessageContent::Parts(p) if p.len() == 2),
            "vision request must be a 2-part (text + image) user turn"
        );
        if let MessageContent::Parts(parts) = &msgs[0].content {
            assert!(matches!(&parts[0], ContentPart::Text { text } if text.as_str() == "прочитай"));
            assert!(matches!(&parts[1], ContentPart::ImageUrl { image_url }
                if image_url.url.as_str() == "data:image/jpeg;base64,AAAA"));
        }
    }

    #[test]
    fn empty_prompt_falls_back_to_default() {
        let msgs = build_vision_request("data:image/png;base64,ZZ", "   ");
        assert!(matches!(&msgs[0].content, MessageContent::Parts(_)));
        if let MessageContent::Parts(parts) = &msgs[0].content {
            assert!(matches!(&parts[0], ContentPart::Text { text }
                if text.as_str() == DEFAULT_VISION_PROMPT));
        }
    }
}
