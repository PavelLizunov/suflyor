//! memory::normalize (M1 of docs/memory-architecture.md §3) — turn a RAW captured span
//! (messy STT / a verbatim block) into a clean, atomic fact.
//!
//! Three pieces — two PURE + deterministic (unit-tested), plus the async orchestrator that
//! composes them with `ai::complete`:
//!
//! - [`heuristic_clean`] — cheap layer-1 pre-clean (collapse whitespace, drop an immediate
//!   duplicate STT-stutter word). Deliberately conservative: it makes garbage *shorter/tidier*
//!   and never drops numbers or short words (a repeated digit/short word may be a real double);
//!   it does NOT touch semantics (pronouns, mishearings, entity grouping — that's the LLM).
//! - [`is_grounded`] — the anti-hallucination GATE for an LLM rewrite. An LLM proposal is
//!   accepted ONLY if it's grounded in the source: every identifier token (a digit-bearing
//!   id — IP / subnet / port / number — or an all-caps ASCII acronym like VPN / DNS) equals a
//!   WHOLE source token verbatim, content words are contained, negation polarity is preserved,
//!   and the shape is sane. This is what makes an LLM normalization safe to run unattended — a
//!   failing rewrite is rejected and the caller keeps the un-rewritten (heuristic) text.
//! - [`normalize_fact`] — async layer-2 orchestrator: heuristic-clean → one AI rewrite → gate
//!   with `is_grounded`. Returns the grounded fact, or `None` so the caller keeps the heuristic
//!   text (never a hallucinated rewrite).
//!
//! The two helpers are pure (no I/O), unit-tested like `key_terms`; `normalize_fact` is the only
//! I/O path (one AI call) — its JSON parse (`parse_first_fact`) is tested, the network call not.

use crate::ai::{self, ChatMessage, MessageContent};

/// The lowercase alphanumeric "core" of a token (strips surrounding punctuation/case) —
/// used to detect immediate duplicate words: «Даже,» and «даже» share the core «даже».
fn word_core(tok: &str) -> String {
    tok.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Layer-1 heuristic pre-clean (pure, always cheap): collapse whitespace runs to single
/// spaces and drop an IMMEDIATE duplicate STT-stutter word — a repeated ALPHABETIC word of
/// ≥4 chars, e.g. «Даже,  даже писана» → «Даже, писана». Numbers and short words are NEVER
/// collapsed («порт 80 80», «код два два», «5 5» may be real doubles), so this can't lose a
/// fact. Conservative on purpose — no semantic edits (that's the LLM layer).
#[must_use]
pub fn heuristic_clean(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for tok in text.split_whitespace() {
        if let Some(prev) = out.last() {
            let core = word_core(tok);
            let is_stutter_word =
                core.chars().count() >= 4 && core.chars().all(char::is_alphabetic);
            if is_stutter_word && core == word_core(prev) {
                continue; // immediate repeat of a ≥4-letter word → an STT stutter
            }
        }
        out.push(tok);
    }
    out.join(" ")
}

/// True if `tok` must survive normalization VERBATIM (matched by WHOLE-TOKEN equality, not
/// substring): it bears a digit (IP / subnet / port / number / mixed id like `z14-4443`) or
/// is an all-caps ASCII acronym (VPN / DNS / CRM / API) — the classes where a dropped digit
/// or a swap fabricates a fact. Cyrillic names and lowercase words are content words
/// (containment, declension-tolerant). NB: an acronym the source spells in Cyrillic («айпи»)
/// won't match an ASCII rewrite («IP»); we reject that canonicalization (keeping the raw
/// text) rather than risk letting a genuine swap through — a false reject is only annoying.
fn is_identifier(tok: &str) -> bool {
    let has_digit = tok.chars().any(|c| c.is_ascii_digit());
    let alpha = tok.chars().filter(|c| c.is_alphabetic()).count();
    let all_caps_ascii = alpha >= 2
        && tok
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_ascii_uppercase());
    has_digit || all_caps_ascii
}

/// Does `src_lower` contain content `word` (already lowercased), tolerating RU declension? A
/// word of ≥5 chars also matches on its stem (last char dropped): «альфа» matches «альфе».
/// Substring is intentional here (content words only) — it over-accepts grammatical variants,
/// which is safe; identifiers use whole-token equality instead (see [`is_grounded`]).
fn source_contains(src_lower: &str, word: &str) -> bool {
    if src_lower.contains(word) {
        return true;
    }
    let n = word.chars().count();
    if n >= 5 {
        let stem: String = word.chars().take(n - 1).collect();
        return src_lower.contains(&stem);
    }
    false
}

/// Any negation marker present? Used to reject a rewrite that introduces or drops negation.
/// Covers the explicit particles plus the productive «не…»/«ни…» prefix (невозможно,
/// небезопасно, недоступно). ponytail: not exhaustive for Russian (lexical negations like
/// «отказался», English contractions like «don't» are missed) — false positives here only
/// cause a harmless false REJECT, so erring broad is safe; the LLM prompt (M1-b) also
/// instructs polarity preservation. Upgrade to a morphology lib only if this proves leaky.
fn has_negation(text: &str) -> bool {
    text.to_lowercase().split_whitespace().any(|w| {
        let t = w.trim_matches(|c: char| !c.is_alphanumeric());
        matches!(
            t,
            "не" | "нет" | "ни" | "нельзя" | "без" | "no" | "not" | "never" | "никогда" | "никак"
        ) || ((t.starts_with("не") || t.starts_with("ни")) && t.chars().count() >= 5)
    })
}

/// The anti-hallucination gate: is the LLM `rewrite` grounded in `source`? Rejects unless
/// (a) SHAPE is sane — non-empty, ≤300 chars, no markdown header, not ballooned past ~1.5×
/// the source; (b) every IDENTIFIER token (digit-bearing id or all-caps ASCII acronym) equals
/// a WHOLE source token verbatim — zero tolerance (a truncated IP `10.0.0.11` must NOT match
/// `10.0.0.116`, so substring won't do); (c) ≥90% of content words (≥4 letters) are contained
/// in the source (RU-declension-aware); (d) negation polarity is preserved; (e) SOMETHING was
/// actually grounded (a rewrite that verified no identifier and has no checkable content word
/// is rejected, not vacuously accepted). Pure → tested.
#[must_use]
pub fn is_grounded(source: &str, rewrite: &str) -> bool {
    let r = rewrite.trim();
    // (a) shape
    if r.is_empty() || r.chars().count() > 300 || r.starts_with('#') {
        return false;
    }
    if r.chars().count() > source.chars().count() * 3 / 2 + 20 {
        return false; // a "normalization" that grows the text is inventing
    }
    // (d) negation must not be introduced or dropped
    if has_negation(source) != has_negation(r) {
        return false;
    }
    // Tokenize the source ONCE into trimmed, lowercased WHOLE tokens — identifiers are matched
    // against these by equality (substring would accept a truncated number).
    let src_tokens: Vec<String> = source
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    let src_lower = source.to_lowercase();
    let mut content_total = 0usize;
    let mut content_missing = 0usize;
    let mut identifier_seen = false;
    for raw in r.split_whitespace() {
        let tok = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if tok.is_empty() {
            continue;
        }
        if is_identifier(tok) {
            // (b) identifier — must equal a WHOLE source token, verbatim. Zero tolerance.
            let low = tok.to_lowercase();
            if !src_tokens.contains(&low) {
                return false;
            }
            identifier_seen = true;
            continue;
        }
        // (c) split a compound («Бекап-сервер») into words; each content word (≥4 letters)
        // must be contained in the source (declension-aware).
        for word in tok.split(|c: char| !c.is_alphanumeric()) {
            if word.chars().filter(|c| c.is_alphabetic()).count() >= 4 {
                content_total += 1;
                if !source_contains(&src_lower, &word.to_lowercase()) {
                    content_missing += 1;
                }
            }
        }
    }
    // (e) something must actually be grounded — otherwise «ок да» would pass vacuously (0 ≤ 0).
    if content_total == 0 && !identifier_seen {
        return false;
    }
    // (c) ≤10% of content words may be novel grammatical glue.
    content_missing * 10 <= content_total
}

/// A normalized fact from [`normalize_fact`]: clean atomic text + its primary entity (the thing
/// the fact is about), when the model could name one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFact {
    pub text: String,
    pub entity: Option<String>,
}

/// System prompt for the layer-2 rewrite. Russian (the app + the facts are Russian). The hard
/// guarantees (identifiers verbatim, no invention, negation preserved) are RE-CHECKED by
/// [`is_grounded`] — the prompt asks for good behaviour, the gate enforces it.
const NORMALIZE_SYSTEM: &str = "Ты нормализуешь короткий фрагмент из личной памяти пользователя. \
Фрагмент — сырая расшифровка речи: в нём бывают оговорки, повторы, слова-паразиты, разорванные слова.\n\n\
Задача: превратить его в ОДИН чёткий короткий факт на том же языке, что и вход.\n\n\
Строгие правила:\n\
- Сохрани ДОСЛОВНО все идентификаторы: числа, IP-адреса, подсети, порты, имена, логины, даты, версии, коды. \
Нельзя менять ни одной цифры или буквы в них.\n\
- Ничего не выдумывай. Если смысл неясен — оставь формулировку как есть.\n\
- Убери шум речи (эканье, повторы, самоисправления), собери разорванные слова, сделай ясное утверждение.\n\
- Сохрани отрицание («не», «нельзя», «без») — не переворачивай смысл.\n\
- Без пояснений, без markdown, без тройных кавычек.\n\n\
Ответь ТОЛЬКО строгим JSON:\n\
{\"facts\":[{\"entity\":\"<главный предмет факта, коротко>\",\"text\":\"<чистый факт>\"}]}\n\
Обычно один факт. entity — о чём факт (имя/система/человек), или \"\" если неясно.";

/// Async layer-2 normalization (M1, docs/memory-architecture.md §3): heuristic-clean the raw
/// span, ask the AI to rewrite it into one clean atomic fact (no-think JSON via [`ai::complete`]),
/// and accept the rewrite ONLY if [`is_grounded`] passes against the RAW source. Returns the
/// grounded fact, or `None` on any AI error / parse failure / grounding rejection — the caller
/// then keeps the heuristic-cleaned text, never a hallucinated rewrite.
pub async fn normalize_fact(
    raw: &str,
    base_url: &str,
    bearer: &str,
    model: &str,
) -> Option<NormalizedFact> {
    let cleaned = heuristic_clean(raw);
    if cleaned.trim().is_empty() {
        return None;
    }
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: MessageContent::Text(NORMALIZE_SYSTEM.into()),
        },
        ChatMessage {
            role: "user".into(),
            content: MessageContent::Text(cleaned),
        },
    ];
    let resp = ai::complete(base_url, bearer, model, messages, 400)
        .await
        .ok()?;
    let fact = parse_first_fact(&resp)?;
    // The gate: reject any rewrite not grounded in the RAW source (which carries every
    // identifier). On rejection the caller keeps the heuristic text.
    if !is_grounded(raw, &fact.text) {
        return None;
    }
    Some(fact)
}

/// Extract the first fact from a model reply that SHOULD be `{"facts":[{entity,text}]}` but may
/// be wrapped in prose or ```json fences. Pure → tested. `None` if nothing parseable.
fn parse_first_fact(resp: &str) -> Option<NormalizedFact> {
    #[derive(serde::Deserialize)]
    struct FactsDto {
        #[serde(default)]
        facts: Vec<FactDto>,
    }
    #[derive(serde::Deserialize)]
    struct FactDto {
        #[serde(default)]
        entity: String,
        #[serde(default)]
        text: String,
    }
    // Slice from the first '{' to the last '}' — drops ```json fences / surrounding prose.
    let start = resp.find('{')?;
    let end = resp.rfind('}')?;
    if end < start {
        return None;
    }
    let parsed: FactsDto = serde_json::from_str(&resp[start..=end]).ok()?;
    let first = parsed.facts.into_iter().next()?;
    let text = first.text.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let entity = {
        let e = first.entity.trim();
        (!e.is_empty()).then(|| e.to_string())
    };
    Some(NormalizedFact { text, entity })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn heuristic_collapses_ws_and_dedups_stutter_words() {
        assert_eq!(
            heuristic_clean("Даже,  даже   писана мэру.сь."),
            "Даже, писана мэру.сь."
        );
        assert_eq!(heuristic_clean("  a   b  "), "a b");
        assert_eq!(heuristic_clean(""), "");
        // Numbers and short words are NEVER collapsed (may be real doubles).
        assert_eq!(heuristic_clean("порт 80 80"), "порт 80 80");
        assert_eq!(heuristic_clean("код два два"), "код два два");
        assert_eq!(heuristic_clean("5 5"), "5 5");
        // Clean text is unchanged.
        assert_eq!(heuristic_clean("сервер бэкапов"), "сервер бэкапов");
    }

    #[test]
    fn grounded_accepts_faithful_rewrite() {
        let src = "ну это бекап сервер z14-4443-backup подсеть 10.255.28.96/27 IP 10.255.28.116";
        // Reworded prose, but every identifier is a verbatim whole token and content is present.
        assert!(is_grounded(
            src,
            "Бекап-сервер z14-4443-backup: подсеть 10.255.28.96/27, IP 10.255.28.116"
        ));
    }

    #[test]
    fn grounded_rejects_truncated_or_altered_identifier() {
        // Substring must NOT count: a truncated IP / CIDR / port / id is a fabricated fact.
        assert!(!is_grounded("адрес 10.255.28.116", "адрес 10.255.28.11"));
        assert!(!is_grounded(
            "подсеть 10.255.28.96/27",
            "подсеть 10.255.28.96/2"
        ));
        assert!(!is_grounded("порт 8080", "порт 808"));
        assert!(!is_grounded("сервер z14-4443", "сервер z14"));
        assert!(!is_grounded("бюджет 1000 долларов", "бюджет 100 долларов"));
        // An invented subnet not present at all — rejected.
        assert!(!is_grounded(
            "подсеть 10.255.28.96/27",
            "подсеть 10.0.0.0/8"
        ));
    }

    #[test]
    fn grounded_rejects_acronym_swap() {
        // An all-caps ASCII acronym must be verbatim — a protocol swap is a fabricated fact.
        assert!(!is_grounded("используем VPN", "используем DNS"));
        assert!(is_grounded("используем VPN дома", "дома используем VPN"));
    }

    #[test]
    fn grounded_rejects_negation_flip() {
        assert!(!is_grounded("я не пью кофе", "пьёт кофе"));
        assert!(!is_grounded("пьёт кофе по утрам", "не пьёт кофе"));
        assert!(!is_grounded(
            "это невозможно сделать быстро",
            "это возможно сделать быстро"
        ));
        assert!(!is_grounded("без ошибок прошло", "с ошибками прошло"));
    }

    #[test]
    fn grounded_rejects_bad_shape_and_vacuous() {
        assert!(!is_grounded("что-то сказано", "")); // empty
        assert!(!is_grounded("тема", "## Заголовок")); // markdown header
        assert!(!is_grounded("кот", &"длинно ".repeat(60))); // ballooned
                                                             // Nothing grounded: no identifier verified, no ≥4-letter content word → reject.
        assert!(!is_grounded("договорились обо всём", "ок да"));
    }

    #[test]
    fn grounded_accepts_declension_and_all_identifiers() {
        // RU declension: «Альфе» matches the stem of «Альфа»; «CRM» is a verbatim acronym.
        assert!(is_grounded("проект Альфа наша CRM", "Альфе — CRM проекта"));
        // All-identifier fact (no content words) is fine when they're verbatim.
        assert!(is_grounded("порт 8080 API", "8080 API"));
    }

    #[test]
    fn parse_first_fact_handles_clean_fenced_and_prose() {
        let f =
            parse_first_fact(r#"{"facts":[{"entity":"z14","text":"Бекап-сервер z14"}]}"#).unwrap();
        assert_eq!(f.text, "Бекап-сервер z14");
        assert_eq!(f.entity.as_deref(), Some("z14"));
        // ```json fences + surrounding prose are tolerated; empty entity → None.
        let f =
            parse_first_fact("Вот:\n```json\n{\"facts\":[{\"entity\":\"\",\"text\":\"кофе по утрам\"}]}\n```\nготово")
                .unwrap();
        assert_eq!(f.text, "кофе по утрам");
        assert_eq!(f.entity, None);
    }

    #[test]
    fn parse_first_fact_rejects_empty_and_garbage() {
        assert!(parse_first_fact(r#"{"facts":[]}"#).is_none());
        assert!(parse_first_fact("не json вообще").is_none());
        // Present but empty text → not a fact.
        assert!(parse_first_fact(r#"{"facts":[{"entity":"x"}]}"#).is_none());
    }
}
