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

/// Russian/English stopwords + STT fillers — carry no fact content, so they don't gate grounding.
const STOPWORDS: &[&str] = &[
    "и",
    "в",
    "во",
    "на",
    "с",
    "со",
    "по",
    "о",
    "об",
    "а",
    "но",
    "что",
    "чтобы",
    "как",
    "это",
    "эта",
    "этот",
    "эти",
    "этом",
    "этой",
    "того",
    "том",
    "же",
    "бы",
    "ли",
    "вот",
    "ну",
    "там",
    "тут",
    "то",
    "так",
    "уж",
    "из",
    "за",
    "до",
    "от",
    "для",
    "при",
    "про",
    "мы",
    "ты",
    "он",
    "она",
    "они",
    "оно",
    "вы",
    "типа",
    "короче",
    "значит",
    "самое",
    "был",
    "было",
    "были",
    "есть",
    "будет",
    "будем",
    "если",
    "или",
    "уже",
    "ещё",
    "еще",
    "the",
    "and",
    "for",
    "that",
    "this",
    "with",
    "are",
];

/// Negation particles — a rewrite must NOT INTRODUCE one absent from the span (meaning inversion).
const NEGATIONS: &[&str] = &[
    "не",
    "нет",
    "ни",
    "нельзя",
    "без",
    "никак",
    "никогда",
    "ничего",
    "никто",
    "никакой",
];

/// Length of the shared leading char-run — a crude stem test for inflected Russian (the root is
/// prefix-stable, endings inflect). договорились/договорённости → 6; взломан/перезагружен → 0.
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

/// True if fact word `w` is rooted in `source_words`: a shared prefix ≥ min(len_w, len_s, 4). So a
/// 4+‑char word needs a 4‑char shared root (catches inflection: сервер/сервера), a 3‑char word needs
/// all 3 (код/кода ✓, код/кот ✗). A fabricated word (взломан) shares no root → not grounded.
fn word_grounded(w: &str, source_words: &[String]) -> bool {
    let wl = w.chars().count();
    source_words.iter().any(|s| {
        let need = wl.min(s.chars().count()).min(4);
        need > 0 && common_prefix_len(w, s) >= need
    })
}

/// Content words of `s`: lowercased ALPHABETIC tokens ≥3 chars, minus stopwords/negations. (Numbers
/// & identifiers are checked by [`digit_tokens`]; negations by [`negation_words`].)
fn content_words(s: &str) -> Vec<String> {
    tokenize(s)
        .into_iter()
        .map(|(t, _, _)| t)
        .filter(|t| {
            t.chars().count() >= 3
                && t.chars().all(char::is_alphabetic)
                && !STOPWORDS.contains(&t.as_str())
                && !NEGATIONS.contains(&t.as_str())
        })
        .collect()
}

/// Maximal identifier-ish runs of `s` that contain a digit — IPs/versions/ports/codes
/// («10.0.0.116», «z14-4443», «8080»). A rewrite must reproduce each VERBATIM (a truncated/altered
/// number is a different token). Leading/trailing separators are trimmed (sentence dots etc.).
fn digit_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() || matches!(ch, '.' | '-' | ':' | '_') {
            cur.push(ch);
        } else {
            push_digit_token(&cur, &mut out);
            cur.clear();
        }
    }
    push_digit_token(&cur, &mut out);
    out
}

/// Push `cur` to `out` (lowercased, separator-trimmed) iff it holds a digit. Helper for [`digit_tokens`].
fn push_digit_token(cur: &str, out: &mut Vec<String>) {
    let t = cur.trim_matches(|c| matches!(c, '.' | '-' | ':' | '_'));
    if t.chars().any(|c| c.is_ascii_digit()) {
        out.push(t.to_lowercase());
    }
}

/// Negation particles present in `s`.
fn negation_words(s: &str) -> Vec<String> {
    tokenize(s)
        .into_iter()
        .map(|(t, _, _)| t)
        .filter(|t| NEGATIONS.contains(&t.as_str()))
        .collect()
}

/// True if every element of `needles` appears in `haystack` IN ORDER (a subsequence). Greedy: each
/// needle consumes `haystack` forward from the previous match, so ORDER — not just membership — is
/// enforced. `[20, 10]` is NOT a subsequence of `[10, 20]`.
fn is_ordered_subsequence(needles: &[String], haystack: &[String]) -> bool {
    let mut it = haystack.iter();
    needles.iter().all(|n| it.any(|h| h == n))
}

/// M1 — deterministic grounding gate for a REWRITE: the clean `fact` must be a faithful rewrite of
/// `span` (a contiguous source quote from [`locate_span`]). It may drop filler / inflect / keep order,
/// but must NOT: (1) use a content word not rooted in the span (no synonyms/fabrication); (2) change,
/// invent, or REORDER a number/identifier (digit-tokens must be an ordered subsequence of the span's —
/// so «с 10 до 20» → «с 20 до 10» is rejected); (3) change the NEGATION count (rejects both ADDING
/// «не» — «работает» → «не работает» — AND DROPPING it — «не отвечает» → «отвечает», the meaning
/// inversion a cleaning model most often makes); (4) exceed ~200 chars. Because `span` is ONE short
/// contiguous quote, a faithful rewrite of it can't recombine distant source parts, and a rejected
/// rewrite falls back to the verbatim `span`. RESIDUAL (accepted, per owner — over-rejecting here
/// would just re-roughen facts): a near-prefix synonym swap that shares a ≥4-char root (проверили ↔
/// провалили) or an intra-clause content-word REORDER (клиент платит подрядчику ↔ подрядчик платит
/// клиенту) — the prompt asks to keep word order + reuse words to make these unlikely, but neither is
/// blocked deterministically without over-rejecting legitimate cleaning.
#[must_use]
pub fn validate_rewrite(span: &str, fact: &str) -> bool {
    let f = fact.trim();
    if f.is_empty() || f.chars().count() > 200 {
        return false;
    }
    let src_words = content_words(span);
    if content_words(f)
        .iter()
        .any(|w| !word_grounded(w, &src_words))
    {
        return false;
    }
    if !is_ordered_subsequence(&digit_tokens(f), &digit_tokens(span)) {
        return false;
    }
    if negation_words(f).len() != negation_words(span).len() {
        return false;
    }
    true
}

/// A normalized fact from [`normalize_fact`]: clean atomic text + its primary entity (the thing
/// the fact is about), when the model named one. `text` is either a validated clean rewrite of a
/// verbatim span, or (when the rewrite fails validation) the verbatim span itself. `entity` is always
/// a verbatim source slice (via `locate_span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFact {
    pub text: String,
    pub entity: Option<String>,
}

/// System prompt for the rewrite pass (M1). Russian. The model returns, per fact, a VERBATIM
/// contiguous `quote` (anchored by [`locate_span`] → no recombination) AND a clean `fact` (a rewrite
/// of THAT quote). [`validate_rewrite`] then gates the clean fact against the quote; a failed rewrite
/// falls back to the verbatim quote. So the prompt asks for clean facts, but safety is structural
/// (quote anchor + validator), not prompt-trust.
const REWRITE_SYSTEM: &str =
    "Ты извлекаешь 1–3 самых важных факта для личной памяти пользователя. \
Вход — ответ ИИ ИЛИ сырая расшифровка речи.\n\n\
Для КАЖДОГО факта верни ДВА поля:\n\
- \"quote\" — ДОСЛОВНЫЙ непрерывный кусок исходного текста (скопируй БУКВА В БУКВУ, одно место \
подряд), который содержит факт. Бери КОРОТКИЙ кусок — ОДНО предложение, БЕЗ точек «.», «!», «?», «;» \
внутри (не захватывай соседние предложения — иначе факт потеряется);\n\
- \"fact\" — та же мысль, но ЧИСТО: выкинь ВСЕ слова-паразиты и повторы (а-а, ну, вот, значит, это \
самое, как бы, типа, как их там, так-то, в общем), оставь только суть.\n\n\
СТРОГИЕ правила для \"fact\":\n\
- Используй ТОЛЬКО слова из \"quote\" (можно менять их форму и убирать лишние, но СОХРАНЯЙ порядок \
слов). НЕ добавляй новых слов, НЕ подбирай синонимы, НЕ переводи.\n\
- Числа, коды, адреса (IP, порты, версии, имена) — ТОЧНО как в тексте, ни цифры не меняй.\n\
- Сохраняй отрицание (не/нет/ни), если оно есть — не переворачивай смысл.\n\
- Коротко: одно ясное утверждение.\n\n\
Ответь ТОЛЬКО строгим JSON:\n\
{\"facts\":[{\"entity\":\"<о чём, коротко, из текста>\",\"quote\":\"<дословная цитата>\",\
\"fact\":\"<чистый факт>\"}]}\n\
entity — имя/система/человек ИЗ ТЕКСТА, или \"\" если неясно.";

/// Async normalize (M1): heuristic-clean the raw span, ask the AI for 1–3 `{quote, fact}` pairs
/// (no-think JSON via [`ai::complete`]) — `quote` a VERBATIM contiguous span, `fact` its clean
/// rewrite. Per fact: [`locate_span`] anchors the quote to an exact source slice (drops it if the
/// quote isn't a contiguous fragment → no recombination); then [`validate_rewrite`] gates the clean
/// `fact` against that span (clean tier) or falls back to the verbatim span (safe tier). Joins ≤3 with
/// «; ». Three outcomes (P3 offline-reliability): `Ok(Some)` fact(s); `Ok(None)` TERMINAL — the AI
/// replied but nothing parsed/anchored, OR failed PERMANENTLY (4xx bad bearer/model) so a retry is
/// pointless (caller keeps the heuristic text, marks `'heuristic'`); `Err` a TRANSIENT AI failure
/// (offline/timeout/5xx — RETRYABLE, caller leaves the row `'pending'`). Never a fabricated fact.
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
            content: MessageContent::Text(REWRITE_SYSTEM.into()),
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
    let facts: Vec<(String, Option<String>)> = parse_facts(&resp)
        .into_iter()
        .filter_map(|f| {
            // Anchor to a VERBATIM contiguous span (drops a quote that isn't a real fragment → the
            // fact can't recombine distant source parts).
            let span = locate_span(&cleaned, &f.quote)?;
            // Clean tier: a validated rewrite of the span; else safe tier: the verbatim span itself.
            let text = if validate_rewrite(&span, &f.fact) {
                f.fact.trim().to_string()
            } else {
                span
            };
            // entity is metadata — keep it only if it too is a source quote (never fabricated).
            let entity = f.entity.and_then(|e| locate_span(&cleaned, &e));
            Some((text, entity))
        })
        .take(3)
        .collect();
    if facts.is_empty() {
        return Ok(None); // AI replied but nothing parsed/anchored → terminal 'heuristic', not a retry.
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

/// A raw fact parsed from the model reply (before grounding): the clean `fact`, the verbatim `quote`
/// it is anchored to, and an optional `entity`. [`normalize_fact`] then anchors + validates.
struct ParsedFact {
    entity: Option<String>,
    quote: String,
    fact: String,
}

/// Parse ALL facts from a model reply that SHOULD be `{"facts":[{entity,quote,fact},…]}` but may be
/// wrapped in prose or ```json fences. Pure → tested. Empty vec if nothing parseable; a fact with an
/// empty `quote` is dropped (the quote is the required verbatim anchor; `fact` may be empty → the
/// caller then falls back to the located span).
fn parse_facts(resp: &str) -> Vec<ParsedFact> {
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
        quote: String,
        #[serde(default)]
        fact: String,
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
            let quote = f.quote.trim().to_string();
            if quote.is_empty() {
                return None; // no verbatim anchor → can't ground → drop
            }
            let e = f.entity.trim();
            Some(ParsedFact {
                entity: (!e.is_empty()).then(|| e.to_string()),
                quote,
                fact: f.fact.trim().to_string(),
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
        let fs = parse_facts(
            r#"{"facts":[{"entity":"z14","quote":"ну это бекап сервер z14","fact":"бекап-сервер z14"}]}"#,
        );
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].quote, "ну это бекап сервер z14");
        assert_eq!(fs[0].fact, "бекап-сервер z14");
        assert_eq!(fs[0].entity.as_deref(), Some("z14"));
        // ```json fences + surrounding prose tolerated; empty entity → None; MULTIPLE facts kept.
        let fs = parse_facts(
            "Вот:\n```json\n{\"facts\":[{\"entity\":\"\",\"quote\":\"кофе по утрам\",\"fact\":\"кофе по утрам\"},\
             {\"entity\":\"порт\",\"quote\":\"порт 8080\",\"fact\":\"порт 8080\"}]}\n```\nготово",
        );
        assert_eq!(fs.len(), 2);
        assert_eq!(fs[0].quote, "кофе по утрам");
        assert_eq!(fs[0].entity, None);
        assert_eq!(fs[1].fact, "порт 8080");
    }

    #[test]
    fn parse_facts_rejects_empty_and_garbage() {
        assert!(parse_facts(r#"{"facts":[]}"#).is_empty());
        assert!(parse_facts("не json вообще").is_empty());
        // Present but empty QUOTE (the required verbatim anchor) → dropped even if `fact` is set.
        assert!(parse_facts(r#"{"facts":[{"entity":"x","fact":"есть факт"}]}"#).is_empty());
    }

    #[test]
    fn validate_rewrite_accepts_faithful_clean_rewrite() {
        // Drops filler, reorders, inflects — every content word rooted in the span, number verbatim.
        assert!(validate_rewrite(
            "ну это бекап сервер z14 наверное",
            "бекап-сервер z14"
        ));
        assert!(validate_rewrite(
            "мы как бы провели встречу и договорились двигаться",
            "провели встречу, договорились двигаться"
        ));
        // Inflection: договорённости shares a ≥4-char root with договорились.
        assert!(validate_rewrite("в итоге договорились", "договорённости"));
    }

    #[test]
    fn validate_rewrite_rejects_fabrication_number_and_negation_change() {
        // A synonym/fabricated content word not rooted in the span.
        assert!(!validate_rewrite(
            "сервер вчера перезагружен",
            "сервер взломан"
        ));
        // A truncated / wrong number is a different digit-token.
        assert!(!validate_rewrite("айпи 10.0.0.116 готов", "айпи 10.0.0.11"));
        assert!(!validate_rewrite("порт 8080 открыт", "порт 9090 открыт"));
        // ADDING a negation absent from the span (meaning inversion).
        assert!(!validate_rewrite("сервер работает", "сервер не работает"));
        // DROPPING a negation the span had — the inversion a cleaning model most often makes (F1).
        assert!(!validate_rewrite("сервер не отвечает", "сервер отвечает"));
        assert!(!validate_rewrite("доступ не дали", "доступ дали"));
        // REORDERING a numeric range keeps the digit-token SET but flips meaning (F2).
        assert!(!validate_rewrite("бэкап с 10 до 20", "бэкап с 20 до 10"));
        // Over-long (a whole paragraph is not one clean fact).
        let long = "слово ".repeat(60);
        assert!(!validate_rewrite(&long, long.trim()));
    }
}
