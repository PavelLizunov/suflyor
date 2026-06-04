//! Vision-AI channel — standalone screenshot understanding.
//!
//! Resolved through a SEPARATE endpoint ([`crate::config::Config::vision_endpoint`])
//! so a local text model can keep answering interview questions while
//! screenshots go to a vision-capable model (cloud Sonnet, or a 2nd local
//! vision server). This module is deliberately thin: it owns the fixed capture
//! prompt + the image-message shape, and reuses [`crate::ai::stream_chat`] for
//! all HTTP / SSE / retry / cost / secret-safe error handling.

use crate::ai::{ChatMessage, ContentPart, ImageUrl, MessageContent};
use anyhow::{anyhow, Result};

/// Max tokens for a vision answer. A capture usually asks a single question
/// (read / solve / explain), so a moderate budget keeps latency + cost down.
pub const VISION_MAX_TOKENS: u32 = 1024;

/// Fixed MVP capture prompt (RU). One prompt for now — presets are a later phase.
pub const DEFAULT_VISION_PROMPT: &str = "Что на этом скриншоте? Если это вопрос или \
     задача — ответь по делу и кратко (маркдаун, конкретика). Если это просто экран или \
     текст — кратко опиши суть.";

/// Translate-mode capture prompt (feature #3, "перевод для игр"). Outputs ONLY
/// the Russian — "describe my screen" is noise when the user just wants
/// subtitles/dialogue translated. Target = RU for v1 (a later pass can read
/// `ui_language` for an EN target).
///
/// Tuned via a live A/B against the user's small LOCAL vision model (gemma-4-E4B):
/// the original "Выведи ТОЛЬКО перевод" made the 4B model ECHO the source English
/// instead of translating. The "перепиши … на русском, НЕ выводи английский
/// оригинал" framing (don't copy, rewrite in Russian) makes it emit Russian-only.
pub const TRANSLATE_VISION_PROMPT: &str = "Перепиши весь текст с изображения на \
     русском языке — выводи СРАЗУ готовый русский текст, по одной строке. НЕ выводи \
     английский оригинал и НЕ дублируй строки: в ответе должен быть ТОЛЬКО русский \
     результат, без единой английской фразы. Имена собственные, команды и названия \
     (git, kubectl, Bash, Docker) оставляй латиницей внутри русской фразы. Если \
     текста на картинке нет — напиши: (текста нет).";

/// Appended to [`TRANSLATE_VISION_PROMPT`] when phonetics is ON (feature #4): IPA
/// only for non-trivial words, so short subtitles stay clean.
pub const TRANSLATE_PHONETICS_SUFFIX: &str = " Для каждого нетривиального \
     английского слова добавь транскрипцию МФА в квадратных скобках сразу после \
     слова, например: schedule [ˈʃedjuːl].";

/// Compose the translate-capture prompt, optionally with the phonetics suffix.
#[must_use]
pub fn translate_prompt(phonetics: bool) -> String {
    if phonetics {
        format!("{TRANSLATE_VISION_PROMPT}{TRANSLATE_PHONETICS_SUFFIX}")
    } else {
        TRANSLATE_VISION_PROMPT.to_string()
    }
}

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

// NOTE: the live F8 capture path calls crate::ai::stream_chat directly with
// build_vision_request() + VISION_MAX_TOKENS and applies the is_local cost
// zeroing itself, so a separate stream_vision() wrapper here was dead code
// (audit) and was removed — keeping a single vision-streaming entry point.

/// A tiny SYNTHETIC test image — a 32×32 PNG with three flat colour blocks on
/// white — embedded as a data URL. [`test_connection`] sends THIS, never a
/// capture of the user's screen, so a vision connectivity check can NEVER leak
/// private pixels off the box (§9 — Secrets). It is multi-tone and 32×32 (not a
/// 1×1 / blank pixel) so servers that reject trivial images as "too small" still
/// accept it.
pub const SYNTHETIC_TEST_IMAGE_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAABSSURBVFhHY/g/wIABXYDeAO6AOxoaJGFqgeHjAI2oO0RhdDDqgFEHjDpg1AGDxwEDBUYdMHgdILfAjaoYFxh1wKgDRh0w6oDB6wB6gVEHDLgDABuFvoF6wlCUAAAAAElFTkSuQmCC";

/// Live-check the SEPARATE vision endpoint: POST the embedded synthetic image +
/// a 1-token prompt to `{base_url}/chat/completions`. Unlike a plain text ping
/// ([`crate::ai::test_connection`]), this actually exercises the IMAGE path — a
/// text-only endpoint rejects the image content — so a "Vision: ready" result
/// means the endpoint can genuinely read a screenshot, not just answer text.
///
/// SECURITY: a transport error's chain embeds the request URL (the LAN base_url +
/// port). Mirroring [`crate::ai::test_connection`], it is logged to the file log
/// and a GENERIC "connection failed" is returned, so no `base_url` / LAN IP can
/// paint into the screen-shareable Settings / Diagnostics result. An HTTP-status
/// error returns the server's own response snippet (capped by the caller).
pub async fn test_connection(base_url: String, bearer: String, model: String) -> Result<String> {
    let client = crate::ai::http_client();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": build_vision_request(SYNTHETIC_TEST_IMAGE_DATA_URL, "Reply with: ok"),
        "max_tokens": 1,
    });
    let resp = match client
        .post(&url)
        .timeout(std::time::Duration::from_secs(15))
        .bearer_auth(&bearer)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("vision endpoint test transport error: {e:#}");
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
    fn translate_prompt_composes_phonetics_suffix() {
        // OFF: exactly the base translate prompt, no IPA ask.
        let plain = translate_prompt(false);
        assert_eq!(plain, TRANSLATE_VISION_PROMPT);
        assert!(!plain.contains("МФА"), "no phonetics ask when off");
        // ON: base + suffix; mentions IPA + the schedule example.
        let with = translate_prompt(true);
        assert!(with.starts_with(TRANSLATE_VISION_PROMPT));
        assert!(
            with.contains("МФА") && with.contains("schedule"),
            "phonetics suffix appended when on"
        );
        // Both must demand Russian-only output (the anti-echo fix — the whole
        // point of feature #3: no source English in the result).
        assert!(plain.contains("ТОЛЬКО русский") && with.contains("ТОЛЬКО русский"));
        assert!(
            plain.contains("НЕ выводи английский") && with.contains("НЕ выводи английский"),
            "translate prompt must forbid echoing the English source"
        );
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

    #[test]
    fn synthetic_test_image_is_a_nontrivial_png_data_url() {
        // The diagnostics vision live-check sends THIS embedded image, never a
        // screen capture. Lock its shape so a botched edit can't ship a broken
        // or oversized asset. Hermetic — no base64 dep, no network.
        let b64 = SYNTHETIC_TEST_IMAGE_DATA_URL
            .strip_prefix("data:image/png;base64,")
            .expect("must be a base64 PNG data URL");
        assert!(
            b64.len() > 120 && b64.len() < 4096,
            "synthetic image base64 should be small but non-trivial (got {})",
            b64.len()
        );
        assert!(
            b64.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'+' || c == b'/' || c == b'='),
            "data URL payload must be valid base64"
        );
        // Base64 of the 8-byte PNG magic (89 50 4E 47 0D 0A 1A 0A) always starts
        // the encoded string with this prefix.
        assert!(
            b64.starts_with("iVBORw0KGgo"),
            "payload must be a PNG (magic-byte prefix)"
        );
        // It round-trips through the request builder as a single 2-part turn.
        let msgs = build_vision_request(SYNTHETIC_TEST_IMAGE_DATA_URL, "Reply with: ok");
        assert_eq!(msgs.len(), 1);
        assert!(matches!(&msgs[0].content, MessageContent::Parts(p) if p.len() == 2));
    }
}
