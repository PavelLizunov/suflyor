//! Text the host writes on an answer tile, in both UI languages.

/// Title of a tile opened by F9 or by a typed question.
pub(crate) fn ask_title(is_text: bool, is_ru: bool) -> &'static str {
    match (is_text, is_ru) {
        (true, true) => "Вопрос текстом",
        (true, false) => "text · live",
        (false, true) => "Вопрос F9",
        (false, false) => "F9 ask · live",
    }
}

/// Badge of a typed ask.
pub(crate) fn text_ask_badge(is_ru: bool) -> &'static str {
    if is_ru {
        "вопрос текстом"
    } else {
        "text ask"
    }
}

/// Badge of an ask sent to the cloud model with Shift+F9.
pub(crate) fn cloud_ask_badge(is_ru: bool) -> &'static str {
    if is_ru {
        "облако (Shift+F9)"
    } else {
        "cloud (Shift+F9)"
    }
}

/// Badge of a plain F9 ask.
pub(crate) fn f9_ask_badge(is_ru: bool) -> &'static str {
    if is_ru {
        "вопрос F9"
    } else {
        "F9 manual ask"
    }
}

/// Badge of an answer the user sent on to the cloud model.
pub(crate) fn escalated_badge(is_ru: bool) -> &'static str {
    if is_ru {
        "облако (уточнение)"
    } else {
        "cloud (escalated)"
    }
}

/// Label of a tile the backend asked for: what kind of tile it is, plus a
/// mark when it is hidden from screen capture. A tile of the error kind is
/// mostly a notice ("nothing to summarise yet"), so it is labelled as one.
pub(crate) fn kind_label(
    kind: overlay_backend::events::TileKind,
    stealth: bool,
    is_ru: bool,
) -> String {
    use overlay_backend::events::TileKind;
    let name = match (kind, is_ru) {
        (TileKind::Error, true) => "сообщение",
        (TileKind::Error, false) => "notice",
        (TileKind::Ai, true) => "AI",
        (TileKind::Kb, true) => "база знаний",
        (TileKind::Summary, true) => "сводка",
        (TileKind::Debrief, true) => "разбор",
        (TileKind::Translate, true) => "перевод",
        (TileKind::Snippet, true) => "заготовка",
        (TileKind::Bookmark, true) => "закладка",
        (other, _) => other.as_journal_tag(),
    };
    match (stealth, is_ru) {
        (false, _) => name.to_string(),
        (true, true) => format!("{name} · скрыт от захвата"),
        (true, false) => format!("{name} · stealth"),
    }
}

/// State label while the request is in flight.
pub(crate) fn asking_label(is_ru: bool) -> &'static str {
    if is_ru {
        "AI · отвечает…"
    } else {
        "ai · asking…"
    }
}

/// Body shown until the first part of the answer arrives.
pub(crate) fn asking_body(is_ru: bool) -> &'static str {
    if is_ru {
        "Спрашиваю AI…"
    } else {
        "Asking AI…"
    }
}

/// State label while the model uses a tool. Short: the header chip is
/// narrow; `tool_body` names the lookup in the tile body.
pub(crate) fn tool_label(is_ru: bool) -> &'static str {
    if is_ru {
        "AI · ищет…"
    } else {
        "ai · looking up…"
    }
}

/// Body line under the answer while the model uses a tool: what it looks
/// up, and where. The next part of the answer replaces it.
pub(crate) fn tool_body(name: &str, detail: &str, is_ru: bool) -> String {
    use overlay_backend::ai::tools::{KB_SEARCH, MEMORY_RECALL, OPEN_URL, WEB_SEARCH};
    let (ru, en) = match name {
        WEB_SEARCH => ("Ищу в интернете", "Searching the web"),
        OPEN_URL => ("Читаю страницу", "Reading a page"),
        KB_SEARCH => ("Ищу в базе знаний", "Searching the knowledge base"),
        MEMORY_RECALL => ("Вспоминаю из памяти", "Checking personal memory"),
        _ => ("Использую инструмент", "Using a tool"),
    };
    let head = if is_ru { ru } else { en };
    let detail = detail.replace(['*', '_', '`', '\n'], " ");
    if detail.trim().is_empty() {
        format!("*{head}…*")
    } else {
        format!("*{head}: {}*", detail.trim())
    }
}

/// State label of a finished answer. A normal finish says nothing more; any
/// other reason (length, content filter) is shown.
pub(crate) fn done_label(reason: &str, is_ru: bool) -> String {
    let done = if is_ru { "AI · готово" } else { "ai · done" };
    if reason.is_empty() || reason == "stop" {
        done.to_string()
    } else {
        format!("{done} ({reason})")
    }
}

/// State label of a failed answer.
pub(crate) fn error_label(is_ru: bool) -> &'static str {
    if is_ru {
        "ошибка"
    } else {
        "error"
    }
}

/// State label of a tile whose answer a newer ask replaced.
pub(crate) fn superseded_label(is_ru: bool) -> &'static str {
    if is_ru {
        "AI · заменён новым"
    } else {
        "ai · superseded"
    }
}

/// Body of a failed answer; `safe` is already free of addresses and secrets.
pub(crate) fn error_body(safe: &str, is_ru: bool) -> String {
    if is_ru {
        format!("Ошибка AI: {safe}")
    } else {
        format!("AI error: {safe}")
    }
}

/// Note shown when the model finished without a single token.
pub(crate) fn empty_answer_note(is_ru: bool) -> &'static str {
    if is_ru {
        "_(модель не вернула ответ)_"
    } else {
        "_(the model returned no answer)_"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_finish_shows_no_reason() {
        assert_eq!(done_label("stop", false), "ai · done");
        assert_eq!(done_label("", true), "AI · готово");
        assert_eq!(done_label("length", false), "ai · done (length)");
        assert_eq!(done_label("length", true), "AI · готово (length)");
    }

    #[test]
    fn a_tool_body_names_the_lookup() {
        assert_eq!(
            tool_body("web_search", "курс доллара", true),
            "*Ищу в интернете: курс доллара*"
        );
        assert_eq!(tool_body("open_url", "docs.rs", false), "*Reading a page: docs.rs*");
        assert_eq!(tool_body("kb_search", "", false), "*Searching the knowledge base…*");
        assert_eq!(tool_body("web_search", "a*b_c", false), "*Searching the web: a b c*");
    }

    #[test]
    fn a_tile_kind_label_has_no_dangling_separator() {
        use overlay_backend::events::TileKind;
        assert_eq!(kind_label(TileKind::Ai, false, false), "ai");
        assert_eq!(kind_label(TileKind::Ai, true, false), "ai · stealth");
        assert_eq!(kind_label(TileKind::Error, false, false), "notice");
        assert_eq!(kind_label(TileKind::Error, false, true), "сообщение");
        assert_eq!(kind_label(TileKind::Summary, true, true), "сводка · скрыт от захвата");
        assert_eq!(kind_label(TileKind::Manual, false, true), "manual");
    }

    #[test]
    fn every_text_has_both_languages() {
        let pairs = [
            (ask_title(true, false), ask_title(true, true)),
            (ask_title(false, false), ask_title(false, true)),
            (text_ask_badge(false), text_ask_badge(true)),
            (cloud_ask_badge(false), cloud_ask_badge(true)),
            (f9_ask_badge(false), f9_ask_badge(true)),
            (escalated_badge(false), escalated_badge(true)),
            (asking_label(false), asking_label(true)),
            (asking_body(false), asking_body(true)),
            (tool_label(false), tool_label(true)),
            (error_label(false), error_label(true)),
            (superseded_label(false), superseded_label(true)),
            (empty_answer_note(false), empty_answer_note(true)),
        ];
        for (en, ru) in pairs {
            assert!(!en.chars().any(|c| ('а'..='я').contains(&c)), "{en}");
            assert!(ru.chars().any(|c| ('а'..='я').contains(&c)), "{ru}");
        }
        assert_eq!(error_body("x", false), "AI error: x");
        assert_eq!(error_body("x", true), "Ошибка AI: x");
    }
}
