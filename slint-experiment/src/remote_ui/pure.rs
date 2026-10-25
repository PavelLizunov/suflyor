//! Value-returning Slint functions the host calls (`invoke_*` with a result).
//! A remote window cannot ask the shell synchronously, so these pure functions
//! are mirrored here; `tools/remote-ui/generate.py` routes each one by name.

use super::global_value;
use slint::SharedString;

fn ru() -> bool {
    global_value("Suflyor", "locale").and_then(|v| v.as_str().map(|s| s == "ru")) == Some(true)
}

/// palette.slint `source-label`: the knowledge-base source in the interface language.
pub fn palette_source_label(source: SharedString) -> SharedString {
    let (en, ru_text) = match source.as_str() {
        "glossary" => ("glossary", "глоссарий"),
        "commands" => ("commands", "команды"),
        "patterns" => ("patterns", "шаблоны"),
        _ => return source,
    };
    SharedString::from(if ru() { ru_text } else { en })
}

#[cfg(test)]
mod tests {
    use super::palette_source_label;

    #[test]
    fn source_label_keeps_unknown_sources_and_names_known_ones() {
        assert_eq!(palette_source_label("notes.md".into()), "notes.md");
        // No language reported yet: English, as Slint's msgid fallback.
        assert_eq!(palette_source_label("glossary".into()), "glossary");
    }
}
