//! Text normalization for the TeraTTSv2 encoder, a faithful port of the
//! reference pipeline in `teratts.py` (NFC → punctuation spacing → digit/word
//! spacing → vocabulary filter → language-tag validation → tagged number
//! expansion → NFKD). One deliberate RC17 gap: automatic Russian stress via
//! the bundled RUAccent graphs is not orchestrated yet, so manual `+` markers
//! pass through unchanged and unmarked Russian text is synthesized as-is.

use anyhow::{anyhow, Result};
use unicode_normalization::UnicodeNormalization;

use crate::indexer::UnicodeIndexer;
use crate::num2words::num2words;

/// Encoded-ready text: `model_text` keeps `+` stress markers (NFKD),
/// `duration_text` is the same text with markers stripped.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelText {
    pub model_text: String,
    pub duration_text: String,
}

/// Wrap untagged text in a `<lang>…</lang>` span. Already-tagged text is left
/// alone (validation happens later and reports precise tag errors).
pub fn ensure_language_tags(text: &str, lang: &str) -> String {
    if find_tag_tokens(text).is_empty() {
        format!("<{lang}>{text}</{lang}>")
    } else {
        text.to_string()
    }
}

/// Full reference pipeline minus RUAccent stress marking.
pub fn prepare(raw_text: &str, indexer: &UnicodeIndexer) -> Result<ModelText> {
    let text: String = raw_text.nfc().collect();
    let text = add_punctuation_spaces(&text);
    let text = add_number_word_spaces(&text);
    let text = skip_unsupported(&text, indexer, true);
    validate_language_tags(&text)?;
    let text = expand_tagged_numbers(&text);
    let text = skip_unsupported(&text, indexer, false);
    let model_text: String = text.nfkd().collect();
    let duration_text = model_text.replace('+', "");
    Ok(ModelText {
        model_text,
        duration_text,
    })
}

// ---------------------------------------------------------------------------
// Spacing passes
// ---------------------------------------------------------------------------

/// Separate punctuation from a following non-space character without splitting
/// decimal literals (`3.5`) or closing tags (`.</ru>`).
fn add_punctuation_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);
    for (i, &c) in chars.iter().enumerate() {
        out.push(c);
        if !matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '…') {
            continue;
        }
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let following = chars.get(i + 1).copied();
        let Some(next) = following else { continue };
        if next.is_whitespace() || next == '<' {
            continue;
        }
        if matches!(c, '.' | ',')
            && prev.is_some_and(|p| p.is_ascii_digit())
            && next.is_ascii_digit()
        {
            continue;
        }
        out.push(' ');
    }
    out
}

/// Space between a digit and a following Latin/Cyrillic letter (`21яблоко` →
/// `21 яблоко`), matching `(?<=\d)(?=[A-Za-zА-Яа-яЁё])`.
fn add_number_word_spaces(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_digit() {
            if let Some(&next) = chars.get(i + 1) {
                if is_word_letter(next) {
                    out.push(c);
                    out.push(' ');
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

fn is_word_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
        || ('\u{0410}'..='\u{044F}').contains(&c)
        || c == '\u{0401}'
        || c == '\u{0451}'
}

// ---------------------------------------------------------------------------
// Vocabulary filter
// ---------------------------------------------------------------------------

/// Keep only characters whose NFKD decomposition exists in the indexer table.
/// With `preserve_digits`, digits survive this pass long enough for tagged
/// number expansion to consume them.
fn skip_unsupported(text: &str, indexer: &UnicodeIndexer, preserve_digits: bool) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut skipped = 0usize;
    for c in text.chars() {
        let supported = indexer.supports(c);
        if supported || (preserve_digits && c.is_ascii_digit()) {
            kept.push(c);
        } else {
            skipped += 1;
        }
    }
    if skipped > 0 {
        // Count only: the protocol must not echo user text into logs.
        eprintln!("[suflyor-teratts] skipped {skipped} unsupported char(s)");
    }
    kept
}

// ---------------------------------------------------------------------------
// Language tags
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
struct TagToken {
    closing: bool,
    lang: [char; 2],
}

fn find_tag_tokens(text: &str) -> Vec<TagToken> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut j = i + 1;
            let closing = chars.get(j) == Some(&'/');
            if closing {
                j += 1;
            }
            if j + 2 <= chars.len()
                && chars[j].is_ascii_lowercase()
                && chars[j + 1].is_ascii_lowercase()
                && chars.get(j + 2) == Some(&'>')
            {
                tokens.push(TagToken {
                    closing,
                    lang: [chars[j], chars[j + 1]],
                });
                i = j + 3;
                continue;
            }
        }
        i += 1;
    }
    tokens
}

/// Require balanced `<ru>…</ru>` / `<en>…</en>` spans, mirroring the exact
/// refusal rules of the reference `validate_language_tags`.
pub fn validate_language_tags(text: &str) -> Result<()> {
    let chars: Vec<char> = text.chars().collect();
    let mut stack: Vec<[char; 2]> = Vec::new();
    let mut saw_span = false;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        // Parse one tag-shaped token, or fail.
        let mut j = i + 1;
        let closing = chars.get(j) == Some(&'/');
        if closing {
            j += 1;
        }
        let valid_shape = j + 2 <= chars.len()
            && chars[j].is_ascii_lowercase()
            && chars[j + 1].is_ascii_lowercase()
            && chars.get(j + 2) == Some(&'>');
        if !valid_shape {
            return Err(anyhow!(
                "invalid language tags; use only <ru>...</ru> or <en>...</en>"
            ));
        }
        let lang = [chars[j], chars[j + 1]];
        let lang_str: String = lang.iter().collect();
        if lang_str != "ru" && lang_str != "en" {
            return Err(anyhow!(
                "unsupported language tag <{lang_str}>; use <ru> or <en>"
            ));
        }
        if !closing {
            stack.push(lang);
        } else {
            let Some(top) = stack.pop() else {
                return Err(anyhow!(
                    "language tags must be balanced: use <ru>...</ru> or <en>...</en>"
                ));
            };
            if top != lang {
                return Err(anyhow!(
                    "language tags must be balanced: use <ru>...</ru> or <en>...</en>"
                ));
            }
            saw_span = true;
        }
        i = j + 3;
    }
    if !saw_span || !stack.is_empty() {
        return Err(anyhow!(
            "language tags must be balanced: use <ru>...</ru> or <en>...</en>"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Number expansion
// ---------------------------------------------------------------------------

fn is_wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Spell out numeric literals inside each balanced language span, matching the
/// reference `TAGGED_NUMBER` boundaries.
fn expand_tagged_numbers(text: &str) -> String {
    // Fast path: no digits at all.
    if !text.chars().any(|c| c.is_ascii_digit()) {
        return text.to_string();
    }
    let spans = language_spans(text);
    if spans.is_empty() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 16);
    let mut cursor = 0usize;
    for (lang, content_start, content_end) in spans {
        out.extend(chars[cursor..content_start].iter());
        let mut i = content_start;
        while i < content_end {
            if let Some((literal, len)) = match_number(&chars, i) {
                let lang_str: String = lang.iter().collect();
                match num2words(&literal, &lang_str) {
                    Some(words) => out.push_str(&words),
                    None => out.push_str(&literal),
                }
                i += len;
            } else {
                out.push(chars[i]);
                i += 1;
            }
        }
        cursor = content_end;
    }
    out.extend(chars[cursor..].iter());
    out
}

/// Balanced `(lang, content_start, content_end)` char spans. Input is already
/// validated, so a plain stack walk is exact.
fn language_spans(text: &str) -> Vec<([char; 2], usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<([char; 2], usize, usize)> = Vec::new();
    let mut stack: Vec<([char; 2], usize)> = Vec::new();
    let tokens = tag_token_positions(&chars);
    for (closing, lang, start, end) in tokens {
        if !closing {
            stack.push((lang, end));
        } else if let Some((open_lang, content_start)) = stack.pop() {
            if open_lang == lang {
                spans.push((lang, content_start, start));
            }
        }
    }
    spans.sort_by_key(|s| s.1);
    spans
}

fn tag_token_positions(chars: &[char]) -> Vec<(bool, [char; 2], usize, usize)> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut j = i + 1;
            let closing = chars.get(j) == Some(&'/');
            if closing {
                j += 1;
            }
            if j + 2 <= chars.len()
                && chars[j].is_ascii_lowercase()
                && chars[j + 1].is_ascii_lowercase()
                && chars.get(j + 2) == Some(&'>')
            {
                tokens.push((closing, [chars[j], chars[j + 1]], i, j + 3));
                i = j + 3;
                continue;
            }
        }
        i += 1;
    }
    tokens
}

/// Match `(?<![\w.])[-−]?\d+(?:[.,]\d+)?(?![\w.])` at position `i`, including
/// the regex engine's backtracking: when the fractional form fails the
/// trailing lookahead (`3,14.` — sentence period), the integer alone is
/// tried (`3`), exactly like the reference `TAGGED_NUMBER`.
fn match_number(chars: &[char], i: usize) -> Option<(String, usize)> {
    fn lookahead_ok(chars: &[char], pos: usize) -> bool {
        pos >= chars.len() || !(is_wordish(chars[pos]) || chars[pos] == '.')
    }
    if i > 0 && (is_wordish(chars[i - 1]) || chars[i - 1] == '.') {
        return None;
    }
    let mut j = i;
    if chars.get(j) == Some(&'-') || chars.get(j) == Some(&'−') {
        j += 1;
    }
    let digits_start = j;
    while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
    }
    if j == digits_start {
        return None;
    }
    let int_end = j;
    if matches!(chars.get(j), Some('.') | Some(',')) {
        let mut k = j + 1;
        let frac_start = k;
        while k < chars.len() && chars[k].is_ascii_digit() {
            k += 1;
        }
        if k > frac_start && lookahead_ok(chars, k) {
            let literal: String = chars[i..k].iter().collect();
            return Some((literal, k - i));
        }
    }
    if !lookahead_ok(chars, int_end) {
        return None;
    }
    let literal: String = chars[i..int_end].iter().collect();
    Some((literal, int_end - i))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    /// Indexer table that accepts ASCII, Cyrillic, and the common punctuation
    /// used by these tests — everything else is unsupported.
    fn test_indexer() -> UnicodeIndexer {
        let mut entries = Vec::with_capacity(65_536);
        for cp in 0..65_536u32 {
            let c = char::from_u32(cp);
            let supported = match c {
                Some(c)
                    if c.is_ascii()
                        || ('\u{0400}'..='\u{04FF}').contains(&c)
                        // Combining marks so NFKD decompositions (Ё → Е + ̈,
                        // й → и + ̆) stay supported, like the real table.
                        || ('\u{0300}'..='\u{036F}').contains(&c)
                        || matches!(c, '+' | '<' | '>' | '/' | '…' | '«' | '»') =>
                {
                    cp as i64
                }
                _ => -1,
            };
            entries.push(serde_json::Value::Number(serde_json::Number::from(
                supported,
            )));
        }
        let json = serde_json::Value::Array(entries).to_string();
        UnicodeIndexer::from_json(&json).unwrap()
    }

    #[test]
    fn wraps_untagged_text_and_keeps_tagged() {
        assert_eq!(ensure_language_tags("привет", "ru"), "<ru>привет</ru>");
        assert_eq!(ensure_language_tags("<en>hi</en>", "ru"), "<en>hi</en>");
    }

    #[test]
    fn punctuation_spacing_keeps_decimals_and_tags() {
        assert_eq!(add_punctuation_spaces("Привет,мир"), "Привет, мир");
        assert_eq!(add_punctuation_spaces("3.5"), "3.5");
        assert_eq!(add_punctuation_spaces("конец.</ru>"), "конец.</ru>");
        assert_eq!(add_punctuation_spaces("Раз!Два"), "Раз! Два");
    }

    #[test]
    fn number_word_spacing() {
        assert_eq!(add_number_word_spaces("21яблоко"), "21 яблоко");
        assert_eq!(add_number_word_spaces("21 яблоко"), "21 яблоко");
        assert_eq!(add_number_word_spaces("42apples"), "42 apples");
        assert_eq!(add_number_word_spaces("3.5"), "3.5");
    }

    #[test]
    fn language_tag_validation_rules() {
        assert!(validate_language_tags("<ru>текст</ru>").is_ok());
        assert!(validate_language_tags("<ru>а</ru> <en>b</en>").is_ok());
        assert!(validate_language_tags("текст").is_err());
        assert!(validate_language_tags("<ru>текст").is_err());
        assert!(validate_language_tags("текст</ru>").is_err());
        assert!(validate_language_tags("<de>текст</de>").is_err());
        assert!(validate_language_tags("<ru>текст</en>").is_err());
        assert!(validate_language_tags("<ru>а < б</ru>").is_err());
    }

    #[test]
    fn expands_tagged_numbers_per_language() {
        assert_eq!(
            expand_tagged_numbers("<ru>У меня 21 яблоко.</ru>"),
            "<ru>У меня двадцать один яблоко.</ru>"
        );
        assert_eq!(
            expand_tagged_numbers("<en>I have 42 apples.</en>"),
            "<en>I have forty-two apples.</en>"
        );
        assert_eq!(
            expand_tagged_numbers("<ru>Версия 1.2.3 вышла.</ru>"),
            "<ru>Версия 1.2.3 вышла.</ru>"
        );
        assert_eq!(
            expand_tagged_numbers("<ru>Пи равен 3,14</ru>"),
            "<ru>Пи равен три целые и четырнадцать сотых</ru>"
        );
        // Sentence-final period defeats the fractional lookahead, so the
        // reference regex backtracks and expands only the integer part.
        assert_eq!(
            expand_tagged_numbers("<ru>Пи равен 3,14.</ru>"),
            "<ru>Пи равен три,14.</ru>"
        );
    }

    #[test]
    fn full_pipeline_produces_nfkd_and_duration_text() {
        let idx = test_indexer();
        let mt = prepare("<ru>Ёлка 1.</ru>", &idx).unwrap();
        // NFKD decomposes Ё → Е + combining diaeresis.
        assert!(mt.model_text.contains('\u{0308}'));
        assert!(!mt.duration_text.contains('+'));
        assert_eq!(mt.duration_text, mt.model_text.replace('+', ""));
    }

    #[test]
    fn manual_stress_markers_survive_pipeline() {
        let idx = test_indexer();
        let mt = prepare("<ru>з+амок</ru>", &idx).unwrap();
        assert!(mt.model_text.contains("+а"));
        assert!(!mt.duration_text.contains('+'));
    }

    #[test]
    fn unsupported_characters_are_dropped() {
        let idx = test_indexer();
        let mt = prepare("<ru>ок \u{1F600}</ru>", &idx).unwrap();
        assert!(!mt.model_text.contains('\u{1F600}'));
    }
}
