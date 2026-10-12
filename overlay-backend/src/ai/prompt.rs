use super::types::{ChatMessage, ContentPart, ImageUrl, MessageContent};

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
            "Профиль/контекст пользователя — применяй его ОДИНАКОВО к каждому ответу (и на вопрос \
             голосом, и на введённый текстом). Если профиль задаёт РОЛЬ или стиль общения \
             (например «отвечай как психолог», «говори кратко») — следуй ему во всех ответах. \
             Если это бэкграунд/опыт — используй для уровня детализации, НЕ ограничивая тему \
             ответа этим, если вопрос про другое:\n{}",
            meeting_context.trim()
        )
    };
    let kb_query = {
        let mut s = transcript_lines.join("\n");
        if let Some(q) = user_question {
        prompt.push_str("Помоги ответить: ");
        prompt.push_str(q);
        prompt.push('\n');
    } else {
        prompt.push_str(
            "На основе последнего вопроса в транскрипте предложи краткий ответ, \
             который я могу дать. Используй пункты если уместно. Не больше 120 слов.",
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
        content: match parts.as_slice() {
            [ContentPart::Text { text }] => MessageContent::Text(text.clone()),
            _ => MessageContent::Parts(parts),
        },
    });

    messages
}
