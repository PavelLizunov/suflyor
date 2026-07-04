//! memory::normalize (M1, docs/memory-architecture.md §3) — turn a RAW captured span (messy STT /
//! a verbatim block) into 1–3 clean SHORT facts, safely.
//!
//! **P4 architecture — QUOTE-SPAN extraction** (replaces the old bag-of-words grounding gate, which
//! an adversarial review showed could neither tell a synonym from a lie nor catch recombination):
//! - [`heuristic_clean`] — cheap pre-clean (collapse whitespace, drop an immediate ≥4-letter STT
//!   stutter). Never drops numbers/short words → can't lose a fact. No semantic edits.
//! - [`locate_span`] — the anti-hallucination CORE: the model returns VERBATIM QUOTES, and this finds
//!   each as a CONTIGUOUS run of whole tokens in the source and returns the exact SOURCE SLICE. We
//!   store the slice, never the model's text → a fact cannot fabricate a word, truncate an identifier
//!   («10.0.0.11» ≠ «10.0.0.116»), or recombine distant words into a false claim. It IS a piece of
//!   the source. Residual (accept): a semantic inversion the source itself contains is copied as-is.
//! - [`normalize_fact`] — async orchestrator: heuristic-clean → AI quotes (no-think JSON) → locate
//!   each quote → join ≤3. Returns `Ok(Some)` (located facts), `Ok(None)` (AI replied but nothing
//!   located, OR failed PERMANENTLY (4xx bad config) — either way TERMINAL, keep heuristic text) or
//!   `Err` (a TRANSIENT AI failure → offline/timeout/5xx, RETRYABLE — the caller leaves the row
//!   `'pending'` for `sweep_pending` to retry). Never a hallucinated fact.
//!
//! The pure pieces are unit-tested like `key_terms`; `normalize_fact`'s one AI call is not (its JSON
//! parse `parse_facts` is).

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

/// Split `s` into maximal alphanumeric tokens (lowercased) with their source byte range. Punctuation
/// and whitespace are separators, not tokens. `«Бекап-сервер z14»` → `[(бекап,..),(сервер,..),(z14,..)]`.
fn tokenize(s: &str) -> Vec<(String, usize, usize)> {
    let mut toks = Vec::new();
    let mut start: Option<usize> = None;
    for (b, ch) in s.char_indices() {
        if ch.is_alphanumeric() {
            start.get_or_insert(b);
        } else if let Some(st) = start.take() {
            toks.push((s[st..b].to_lowercase(), st, b));
        }
    }
    if let Some(st) = start {
        toks.push((s[st..].to_lowercase(), st, s.len()));
    }
    toks
}

/// True if `s` bridges a sentence/clause boundary — a contiguous token run must NOT fuse two clauses
/// into a misleading fact (e.g. «ушёл. Она», «жив; база», «включи прод; выключи тест»). A dot BETWEEN
/// digits (an IP/version like `10.255.28.116`) is NOT a boundary; only `.!?` at end-of-span or
/// followed by whitespace, plus `;` / newline, count.
fn crosses_clause_boundary(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    chars.iter().enumerate().any(|(i, &c)| match c {
        ';' | '!' | '?' | '\n' | '\r' => true,
        '.' => chars.get(i + 1).is_none_or(|n| n.is_whitespace()),
        _ => false,
    })
}

/// P4 — quote-span grounding (the anti-hallucination core). Find the model's `quote` as a CONTIGUOUS
/// run of whole tokens inside `source` (case-insensitive), and return the EXACT source slice (original
/// case, spacing, and any punctuation BETWEEN the tokens). `None` if the quote is not a contiguous
/// fragment of the source. Because we store the source slice — never the model's text — a stored fact
/// cannot fabricate a word, cannot truncate an identifier (a partial token ≠ the whole token → no
/// match, so «10.0.0.11» never matches «10.0.0.116»), and cannot RECOMBINE distant words into a false
/// claim (non-adjacent tokens are not a contiguous run). Two extra guards keep a fact to ONE short
/// utterance: reject a span that bridges a clause/sentence boundary (else two clauses fuse into a
/// misleading claim) or that exceeds ~200 chars (else a long tile answer becomes one giant "fact").
/// The fact IS a short piece of the source. Residual: a semantic inversion within one clause is copied.
#[must_use]
pub fn locate_span(source: &str, quote: &str) -> Option<String> {
    let src = tokenize(source);
    let q: Vec<String> = tokenize(quote).into_iter().map(|(t, _, _)| t).collect();
    if q.is_empty() || q.len() > src.len() {
        return None;
    }
    (0..=src.len() - q.len())
        .find(|&start| (0..q.len()).all(|k| src[start + k].0 == q[k]))
        .map(|start| source[src[start].1..src[start + q.len() - 1].2].to_string())
        .filter(|span| span.chars().count() <= 200 && !crosses_clause_boundary(span))
}

/// A normalized fact from [`normalize_fact`]: clean atomic text + its primary entity (the thing
/// the fact is about), when the model named one. Both are verbatim source slices (via `locate_span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFact {
    pub text: String,
    pub entity: Option<String>,
}

/// System prompt for the condense pass (M1-b-2, feature A / P4). Russian. QUOTE-SPAN: the model
/// returns VERBATIM contiguous quotes, not free text — [`locate_span`] then verifies each is a
/// contiguous fragment of the source and stores the source slice. So the prompt only has to make
/// the model COPY (choose start/end), not rewrite; the safety is structural, not prompt-trust.
const CONDENSE_SYSTEM: &str = "Ты извлекаешь из текста 1–3 самых важных факта для личной памяти \
пользователя. Вход — ответ ИИ ИЛИ сырая расшифровка речи.\n\n\
Для каждого факта верни ДОСЛОВНУЮ ЦИТАТУ — НЕПРЕРЫВНЫЙ кусок текста, скопированный БУКВА В БУКВУ. \
Ты можешь только ВЫБРАТЬ, где цитата начинается и заканчивается (чтобы отрезать слова-паразиты и \
воду вокруг факта). НЕЛЬЗЯ: перефразировать, менять слова, менять цифры или буквы, склеивать \
НЕСМЕЖНЫЕ части текста.\n\n\
Правила:\n\
- Цитата — СПЛОШНОЙ фрагмент оригинала (одно место, подряд). Не объединяй разные куски.\n\
- Копируй ТОЧНО: ни одной буквы или цифры не меняй, ничего не добавляй, не переводи.\n\
- 1–3 цитаты, каждая — одно короткое ясное утверждение, без окружающего мусора.\n\n\
Ответь ТОЛЬКО строгим JSON:\n\
{\"facts\":[{\"entity\":\"<о чём, коротко>\",\"text\":\"<дословная цитата из текста>\"}]}\n\
entity — имя/система/человек ИЗ ТЕКСТА, или \"\" если неясно.";

/// Async condense (M1-b-2, feature A / P4): heuristic-clean the raw span, ask the AI for 1–3 VERBATIM
/// quotes (no-think JSON via [`ai::complete`]), and [`locate_span`] each against the cleaned source —
/// keeping the exact SOURCE SLICE (dropping any quote that isn't a contiguous fragment). Joins ≤3 with
/// «; ». Three outcomes (P3 offline-reliability): `Ok(Some)` located fact(s); `Ok(None)` TERMINAL —
/// the AI replied but nothing parsed/located, OR failed PERMANENTLY (4xx bad bearer/model) so a retry
/// is pointless (caller keeps the heuristic text, marks `'heuristic'`); `Err` a TRANSIENT AI failure
/// (offline/timeout/5xx — RETRYABLE, caller leaves the row `'pending'`). Never a hallucinated fact.
/// A long tile answer → a few short quoted facts; a messy STT line → one clean quote.
pub async fn normalize_fact(
    raw: &str,
    base_url: &str,
    bearer: &str,
    model: &str,
) -> anyhow::Result<Option<NormalizedFact>> {
    let cleaned = heuristic_clean(raw);
    if cleaned.trim().is_empty() {
        return Ok(None);
    }
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: MessageContent::Text(CONDENSE_SYSTEM.into()),
        },
        ChatMessage {
            role: "user".into(),
            content: MessageContent::Text(cleaned.clone()),
        },
    ];
    // P3 pivot: split the AI-call failure by whether a RETRY could ever help.
    // - TRANSIENT (offline / timeout / 5xx / rate-limit) → Err: the caller leaves the row 'pending'
    //   so a later sweep retries once the AI is back — the whole offline-reliability fix (D1).
    // - PERMANENT (4xx: bad bearer/model, oversized req) → Ok(None): give up cleanly (keep the
    //   heuristic text, mark terminal), else the row would stick 'pending' FOREVER and be re-hammered
    //   on every sweep trigger for an error retry can't fix — and a permanent-error row at the head of
    //   the queue would break the whole sweep, starving the retryable rows behind it.
    let resp = match ai::complete(base_url, bearer, model, messages, 500).await {
        Ok(r) => r,
        Err(e) if ai::is_permanent_ai_error(&format!("{e:#}")) => return Ok(None),
        Err(e) => return Err(e),
    };
    // Each fact's text must be a VERBATIM CONTIGUOUS quote of the (cleaned) source: locate_span
    // returns the exact source slice or drops the fact — we store slices, never the model's text.
    let facts: Vec<(String, Option<String>)> = parse_facts(&resp)
        .into_iter()
        .filter_map(|f| {
            locate_span(&cleaned, &f.text).map(|span| {
                // entity is metadata — keep it only if it too is a source quote (never fabricated).
                let entity = f.entity.and_then(|e| locate_span(&cleaned, &e));
                (span, entity)
            })
        })
        .take(3)
        .collect();
    if facts.is_empty() {
        return Ok(None); // AI replied but nothing located → terminal 'heuristic', not a retry.
    }
    let entity = facts.iter().find_map(|(_, e)| e.clone());
    // Join with "; " (not "\n") so a multi-fact record stays single-line-editable in the
    // Настройки→Память LineEdit — same convention as `join_marked_text`.
    let text = facts
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    Ok(Some(NormalizedFact { text, entity }))
}

/// Parse ALL facts from a model reply that SHOULD be `{"facts":[{entity,text},…]}` but may be
/// wrapped in prose or ```json fences. Pure → tested. Empty vec if nothing parseable; facts with
/// an empty `text` are dropped.
fn parse_facts(resp: &str) -> Vec<NormalizedFact> {
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
    let (Some(start), Some(end)) = (resp.find('{'), resp.rfind('}')) else {
        return Vec::new();
    };
    if end < start {
        return Vec::new();
    }
    let Ok(parsed) = serde_json::from_str::<FactsDto>(&resp[start..=end]) else {
        return Vec::new();
    };
    parsed
        .facts
        .into_iter()
        .filter_map(|f| {
            let text = f.text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let e = f.entity.trim();
            Some(NormalizedFact {
                text,
                entity: (!e.is_empty()).then(|| e.to_string()),
            })
        })
        .collect()
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
    fn locate_span_returns_verbatim_source_slice() {
        // Case + whitespace tolerant; returns the ORIGINAL slice incl. inner punctuation/spacing.
        assert_eq!(
            locate_span(
                "Ну это Бекап-Сервер  z14-4443, ага",
                "бекап-сервер z14-4443"
            )
            .as_deref(),
            Some("Бекап-Сервер  z14-4443")
        );
    }

    #[test]
    fn locate_span_rejects_truncated_identifier_and_fabrication() {
        // Truncated IP: token «11» ≠ «116» → no contiguous match → None (never a wrong IP).
        assert_eq!(locate_span("айпи 10.255.28.116", "10.255.28.11"), None);
        // A word not in the source → None.
        assert_eq!(locate_span("кот не ест таблетки", "собака"), None);
    }

    #[test]
    fn locate_span_rejects_recombination() {
        // «Бета» and «продакшн» both exist but are NOT adjacent — the false claim «Бета продакшн»
        // (Бета was the TEST server) is not a contiguous run → None. A bag-of-words gate can't
        // catch this; contiguous-token matching does, by construction.
        let src = "сервер Альфа продакшн, сервер Бета тестовый";
        assert_eq!(locate_span(src, "Бета продакшн"), None);
        // The true contiguous claim IS found (verbatim slice).
        assert_eq!(
            locate_span(src, "Бета тестовый").as_deref(),
            Some("Бета тестовый")
        );
    }

    #[test]
    fn locate_span_rejects_boundary_and_overlong() {
        // Bridging a sentence («ушёл. Она») or clause («жив; база») → rejected: a fact is ONE
        // clause, not two fused by verbatim inner punctuation.
        assert_eq!(locate_span("Он ушёл. Она осталась", "ушёл. Она"), None);
        assert_eq!(locate_span("сервер жив; база мертва", "жив; база"), None);
        // But a dot BETWEEN digits (IP/version) is NOT a boundary — must still locate.
        assert_eq!(
            locate_span("айпи 10.255.28.116 готов", "10.255.28.116").as_deref(),
            Some("10.255.28.116")
        );
        // An over-long contiguous span (a whole tile paragraph) → rejected (one short utterance).
        let long = "слово ".repeat(60); // ~360 chars, one contiguous token run
        assert_eq!(locate_span(&long, long.trim()), None);
    }

    #[test]
    fn parse_facts_handles_clean_fenced_and_multi() {
        let fs = parse_facts(r#"{"facts":[{"entity":"z14","text":"Бекап-сервер z14"}]}"#);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].text, "Бекап-сервер z14");
        assert_eq!(fs[0].entity.as_deref(), Some("z14"));
        // ```json fences + surrounding prose tolerated; empty entity → None; MULTIPLE facts kept.
        let fs = parse_facts(
            "Вот:\n```json\n{\"facts\":[{\"entity\":\"\",\"text\":\"кофе по утрам\"},\
             {\"entity\":\"порт\",\"text\":\"порт 8080\"}]}\n```\nготово",
        );
        assert_eq!(fs.len(), 2);
        assert_eq!(fs[0].text, "кофе по утрам");
        assert_eq!(fs[0].entity, None);
        assert_eq!(fs[1].text, "порт 8080");
    }

    #[test]
    fn parse_facts_rejects_empty_and_garbage() {
        assert!(parse_facts(r#"{"facts":[]}"#).is_empty());
        assert!(parse_facts("не json вообще").is_empty());
        // Present but empty text → dropped.
        assert!(parse_facts(r#"{"facts":[{"entity":"x"}]}"#).is_empty());
    }
}
