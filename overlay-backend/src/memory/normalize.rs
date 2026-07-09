//! memory::normalize (M1, docs/memory-architecture.md §3) — turn a RAW captured span (messy STT /
//! a verbatim block) into 1–3 clean SHORT facts, safely.
//!
//! **M1′ architecture — SEGMENT → SELECT → validate (fable).** The earlier P4 protocol had the model
//! return a VERBATIM quote it then rewrote; a weak model autocorrects while "quoting" garbled STT, so
//! the quote never matched and the whole raw line was stored. M1′ inverts it — CODE segments, the
//! model only SELECTS + rewrites, and a deterministic validator grounds the result:
//! - [`heuristic_clean`] — cheap pre-clean (collapse whitespace, drop an immediate ≥4-letter STT
//!   stutter). Never drops numbers/short words. No semantic edits.
//! - [`segment_clauses`] — split the cleaned source into clauses (sentence/`;`/newline boundaries,
//!   comma-repacked ≤200 chars, pure-filler dropped). The model is shown the NUMBERED clause list.
//! - [`validate_rewrite`] — the anti-hallucination CORE: the model's clean `fact` must be a faithful
//!   rewrite of ONE clause — content words rooted in it AND in the same ORDER (an ordered subsequence:
//!   no synonyms/fabrication, no reorder/recombination), digit-tokens a verbatim ordered subsequence,
//!   negation count preserved, ≤200 chars. CROSS-clause fusion is impossible (the span is one clause,
//!   chosen by code before the model sees it); WITHIN-clause reorder is impossible (the order check).
//! - [`normalize_fact`] — async orchestrator (segment → AI selects `{clause,fact}` → validate → join
//!   ≤3). `Ok(Some)` grounded fact(s); `Ok(None)` TERMINAL (AI replied but nothing grounded, OR a
//!   PERMANENT 4xx — caller stores [`heuristic_condense`] = the best clauses, marks `'heuristic'`);
//!   `Err` a TRANSIENT AI failure (RETRYABLE — row left `'pending'`). Never a fabricated fact.
//! - [`locate_span`] — kept for the ENTITY path only (a verbatim contiguous source slice).
//!
//! A recognizer-fused mixed-script token («LLМоткрытых») is un-merged in [`heuristic_clean`] via
//! [`split_fused_token`] so the real words ground. Residuals (accepted): a near-prefix synonym sharing
//! a ≥4-char root (проверили↔провалили); lossy OMISSION of a non-negated word (a shorter-but-true fact).
//!
//! The pure pieces are unit-tested; `normalize_fact`'s one AI call is not (its JSON parse + the
//! segmenter + the validator are).

use crate::ai::{self, ChatMessage, MessageContent};

/// The lowercase alphanumeric "core" of a token (strips surrounding punctuation/case) —
/// used to detect immediate duplicate words: «Даже,» and «даже» share the core «даже».
fn word_core(tok: &str) -> String {
    tok.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// In the Unicode Cyrillic block.
fn is_cyr(c: char) -> bool {
    ('\u{0400}'..='\u{04FF}').contains(&c)
}

/// Uppercase Cyrillic → its glyph-IDENTICAL Latin lookalike (confusables only). Used to repair a
/// recognizer that emitted a Cyrillic letter where a visually-identical Latin one belongs.
fn cyr_upper_confusable(c: char) -> Option<char> {
    Some(match c {
        'А' => 'A',
        'В' => 'B',
        'Е' => 'E',
        'К' => 'K',
        'М' => 'M',
        'Н' => 'H',
        'О' => 'O',
        'Р' => 'P',
        'С' => 'C',
        'Т' => 'T',
        'У' => 'Y',
        'Х' => 'X',
        _ => return None,
    })
}

/// De-garble a recognizer-FUSED mixed-script token, e.g. «LLМоткрытых» → [«LLM», «открытых»] (the STT
/// glued a Latin acronym to a Cyrillic word via a confusable «М»). Only touches a token that has BOTH
/// Latin and Cyrillic LETTERS: (1) fold a BRIDGING uppercase Cyrillic confusable — Latin before it,
/// lowercase-Cyrillic after it — to Latin («М»→«M»); (2) split into maximal Latin / Cyrillic runs where
/// each side is ≥2 letters. A pure-script token (сервер, GigaChat, недоступен), a numeric/identifier
/// token (10.0.0.116, z14), or a short token is returned UNCHANGED. Letters only — digits, punctuation
/// and negation particles (не/…, always pure-Cyrillic) can never be touched, so no number, negation, or
/// meaning can change; it only inserts a space / swaps a glyph-identical codepoint. Pure → tested.
fn split_fused_token(tok: &str) -> Vec<String> {
    let chars: Vec<char> = tok.chars().collect();
    if !chars.iter().any(|c| c.is_ascii_alphabetic()) || !chars.iter().copied().any(is_cyr) {
        return vec![tok.to_string()]; // pure-script / no-letter → untouched
    }
    let folded: Vec<char> = (0..chars.len())
        .map(|i| {
            let lat_before = i > 0 && chars[i - 1].is_ascii_alphabetic();
            let low_cyr_after = chars
                .get(i + 1)
                .is_some_and(|&n| is_cyr(n) && n.is_lowercase());
            if lat_before && low_cyr_after {
                cyr_upper_confusable(chars[i]).unwrap_or(chars[i])
            } else {
                chars[i]
            }
        })
        .collect();
    let script = |c: char| -> u8 {
        if c.is_ascii_alphabetic() {
            1
        } else if is_cyr(c) {
            2
        } else {
            0
        }
    };
    // Group into maximal same-script runs, then split ONLY between a Latin letter-run and a Cyrillic
    // letter-run when both are ≥2 letters (a shorter side stays attached — no orphan 1-char splits).
    let mut runs: Vec<(u8, String)> = Vec::new();
    for &c in &folded {
        let k = script(c);
        match runs.last_mut() {
            Some((rk, s)) if *rk == k => s.push(c),
            _ => runs.push((k, c.to_string())),
        }
    }
    let mut out: Vec<String> = vec![String::new()];
    let (mut prev_kind, mut prev_len) = (0u8, 0usize);
    for (k, s) in &runs {
        let len = s.chars().count();
        if (*k == 1 || *k == 2) && prev_kind != 0 && prev_kind != *k && prev_len >= 2 && len >= 2 {
            out.push(String::new());
        }
        if let Some(last) = out.last_mut() {
            last.push_str(s);
        }
        if *k != 0 {
            prev_kind = *k;
            prev_len = len;
        }
    }
    out.retain(|t| !t.is_empty());
    out
}

/// Layer-1 heuristic pre-clean (pure, always cheap): collapse whitespace runs to single spaces,
/// [`split_fused_token`] a recognizer-fused mixed-script token («LLМоткрытых» → «LLM открытых»), and
/// drop an IMMEDIATE duplicate STT-stutter word — a repeated ALPHABETIC word of ≥4 chars, e.g.
/// «Даже,  даже писана» → «Даже, писана». Numbers and short words are NEVER collapsed («порт 80 80»,
/// «код два два» may be real doubles), so this can't lose a fact. No semantic edits (that's the LLM
/// layer); the fused-split only inserts a space / swaps a glyph-identical codepoint.
#[must_use]
pub fn heuristic_clean(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        for tok in split_fused_token(raw) {
            if let Some(prev) = out.last() {
                let core = word_core(&tok);
                let is_stutter_word =
                    core.chars().count() >= 4 && core.chars().all(char::is_alphabetic);
                if is_stutter_word && core == word_core(prev) {
                    continue; // immediate repeat of a ≥4-letter word → an STT stutter
                }
            }
            out.push(tok);
        }
    }
    out.join(" ")
}

/// Split `s` into maximal alphanumeric tokens (lowercased) with their source byte range. Punctuation
/// and whitespace are separators, not tokens. `«Бекап-сервер z14»` → `[(бекап,..),(сервер,..),(z14,..)]`.
/// `pub(super)`: reused by context_builder's relevance scorer (ТЗ 2026-07-06 A).
pub(super) fn tokenize(s: &str) -> Vec<(String, usize, usize)> {
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

/// True if the char at `chars[i]` is a clause/sentence boundary: `; ! ? \n \r` always, and `.` at
/// end-of-string or before whitespace (a dot BETWEEN digits like `10.255.28.116` is NOT a boundary).
/// The single source of truth shared by [`crosses_clause_boundary`] and [`segment_clauses`].
fn is_boundary_at(chars: &[char], i: usize) -> bool {
    match chars[i] {
        ';' | '!' | '?' | '\n' | '\r' => true,
        '.' => chars.get(i + 1).is_none_or(|n| n.is_whitespace()),
        _ => false,
    }
}

/// True if `s` bridges a sentence/clause boundary — a contiguous token run must NOT fuse two clauses
/// into a misleading fact (e.g. «ушёл. Она», «жив; база», «включи прод; выключи тест»).
fn crosses_clause_boundary(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    (0..chars.len()).any(|i| is_boundary_at(&chars, i))
}

/// M1′ (fable) — segment the (cleaned) source into grounding CLAUSES: the units a fact may ground in.
/// Split at [`is_boundary_at`] boundaries (each clause keeps its trailing punctuation, trimmed); repack
/// a clause > `MAX` chars into comma-windows WHERE COMMAS ALLOW (a comma-free run-on stays one oversized
/// window — harmless: the ≤200 fact cap + ordered grounding in [`validate_rewrite`] bound the stored
/// text regardless). Drop clauses that are pure filler — < 2 content words AND no digit-token («Вот
/// так-то да-да», «ыбочку и»). If
/// filtering would drop everything, keep the unfiltered windows. Capped at 20 (a long tile block).
/// Pure → tested. The model later SELECTS a clause by index and rewrites it — so a fact can never fuse
/// two clauses (the grounding span never contains a boundary), by construction.
fn segment_clauses(s: &str) -> Vec<String> {
    const MAX: usize = 200;
    let chars: Vec<char> = s.chars().collect();
    let mut raw: Vec<String> = Vec::new();
    let mut start = 0usize;
    for i in 0..chars.len() {
        if is_boundary_at(&chars, i) {
            let clause: String = chars[start..=i].iter().collect();
            push_trimmed(&clause, &mut raw);
            start = i + 1;
        }
    }
    if start < chars.len() {
        let clause: String = chars[start..].iter().collect();
        push_trimmed(&clause, &mut raw);
    }
    // Repack over-long clauses into ≤MAX comma-windows so a grounding span stays bounded.
    let mut windows: Vec<String> = Vec::new();
    for clause in raw {
        if clause.chars().count() <= MAX {
            windows.push(clause);
        } else {
            let mut cur = String::new();
            for chunk in clause.split_inclusive(',') {
                if !cur.is_empty() && cur.chars().count() + chunk.chars().count() > MAX {
                    push_trimmed(&cur, &mut windows);
                    cur.clear();
                }
                cur.push_str(chunk);
            }
            push_trimmed(&cur, &mut windows);
        }
    }
    // Drop pure-filler clauses; if that empties the list, keep the windows (better a filler clause
    // than nothing). Cap the count so a huge tile block can't fan out unboundedly.
    let kept: Vec<String> = windows
        .iter()
        .filter(|c| content_words(c).len() >= 2 || !digit_tokens(c).is_empty())
        .cloned()
        .collect();
    let mut out = if kept.is_empty() { windows } else { kept };
    out.truncate(20);
    out
}

/// Trim `s` and push to `out` if non-empty. Helper for [`segment_clauses`].
fn push_trimmed(s: &str, out: &mut Vec<String>) {
    let t = s.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
}

/// M1′ (fable) — deterministic fallback when the AI declines / is not configured: the 2 most
/// content-bearing clauses (by content-word count), emitted in SOURCE order, joined «; ». Each is a
/// verbatim contiguous source slice (same safety as a `locate_span` output) — just the best clauses
/// instead of the whole raw ramble. Empty → the (already heuristic-cleaned) input unchanged.
#[must_use]
pub fn heuristic_condense(cleaned: &str) -> String {
    let clauses = segment_clauses(cleaned);
    if clauses.len() <= 1 {
        return clauses
            .into_iter()
            .next()
            .unwrap_or_else(|| cleaned.trim().to_string());
    }
    // Rank by content-word count, keep the top 2, then restore SOURCE order.
    let mut idx: Vec<usize> = (0..clauses.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(content_words(&clauses[i]).len()));
    let mut top: Vec<usize> = idx.into_iter().take(2).collect();
    top.sort_unstable();
    top.iter()
        .map(|&i| clauses[i].as_str())
        .collect::<Vec<_>>()
        .join("; ")
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
/// `pub(super)`: also filters the question's content terms in context_builder (ТЗ 2026-07-06 A).
pub(super) const STOPWORDS: &[&str] = &[
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

/// True if words `a` and `b` share a root: a common prefix ≥ min(len_a, len_b, 4). So a 4+‑char word
/// needs a 4‑char shared root (catches inflection: сервер/сервера), a 3‑char word needs all 3
/// (код/кода ✓, код/кот ✗). A fabricated word (взломан) shares no root with the source → no match.
/// `pub(super)`: reused by context_builder's relevance scorer (ТЗ 2026-07-06 A) — the symmetry is
/// what catches влад↔владислав AND влада↔владислав (either side may be the shorter).
pub(super) fn words_match(a: &str, b: &str) -> bool {
    let need = a.chars().count().min(b.chars().count()).min(4);
    need > 0 && common_prefix_len(a, b) >= need
}

/// True if `fact_words` appear in `src_words` IN ORDER — an ordered subsequence under [`words_match`].
/// Greedy: each fact word consumes `src_words` forward from the previous match. This is what makes a
/// clean rewrite provably a faithful one: it may DROP source words (filler) and inflect, but it can't
/// REORDER them — so «клиент платит подрядчику» can't become «подрядчик платит клиенту», and a
/// within-clause recombination («тест…поднят, прод…стабилен» → «прод…поднят») is rejected. A
/// rejected fact falls back to the verbatim clause (`heuristic_condense`), so over-rejection is cheap.
fn grounded_in_order(fact_words: &[String], src_words: &[String]) -> bool {
    let mut from = 0usize;
    for w in fact_words {
        match src_words[from..].iter().position(|s| words_match(w, s)) {
            Some(off) => from += off + 1,
            None => return false,
        }
    }
    true
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

/// M1′ — deterministic grounding gate for a REWRITE: the clean `fact` must be a faithful rewrite of
/// ONE source clause `span`. It may drop filler and inflect, but must NOT: (1) use a content word not
/// rooted in the span, OR REORDER the content words (they must be an ordered subsequence of the span's
/// — [`grounded_in_order`]) — this blocks both fabrication/synonyms AND recombination («клиент платит
/// подрядчику» → «подрядчик платит клиенту», or «тест…поднят, прод…стабилен» → «прод…поднят»);
/// (2) change/invent/REORDER a number-identifier (digit-tokens an ordered subsequence — «с 10 до 20» →
/// «с 20 до 10» rejected); (3) change the NEGATION count (rejects ADDING «не» — «работает» → «не
/// работает» — AND DROPPING it — «не отвечает» → «отвечает»); (4) exceed ~200 chars. The caller only
/// ever passes ONE clause (from [`segment_clauses`]), so cross-clause fusion is impossible by
/// construction and intra-clause reorder is blocked by (1). A rejected fact falls back to the verbatim
/// clause (`heuristic_condense`), so over-rejection is cheap. RESIDUAL (accepted): a near-prefix
/// synonym sharing a ≥4-char root (проверили ↔ провалили), and lossy OMISSION of a non-negated content
/// word (a shorter-but-true fact).
#[must_use]
pub fn validate_rewrite(span: &str, fact: &str) -> bool {
    let f = fact.trim();
    if f.is_empty() || f.chars().count() > 200 {
        return false;
    }
    // Content words must be an ORDERED subsequence of the span's (rooted + in order) — no fabrication,
    // no reorder/recombination. Then numbers verbatim-in-order, then negation-count preserved.
    if !grounded_in_order(&content_words(f), &content_words(span)) {
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

/// System prompt for the SELECT-and-rewrite pass (M1′, fable). Russian. The user message is a NUMBERED
/// list of source clauses. The model SELECTS a clause by index and rewrites it cleanly — it never
/// copies verbatim (a weak model autocorrects while "quoting", which broke the old protocol on garbled
/// STT). [`validate_rewrite`] then gates the clean fact against the SELECTED clause; the index is only
/// a hint (the caller re-scans all clauses if it's wrong), so safety is structural, not prompt-trust.
const REWRITE_SYSTEM: &str =
    "Ты извлекаешь 1–3 самых важных факта для личной памяти пользователя. \
Тебе дан СПИСОК пронумерованных фрагментов текста (по одному в строке, вида «N: текст»).\n\n\
Для КАЖДОГО факта: выбери НОМЕР фрагмента, который его содержит, и перепиши ЭТОТ фрагмент ЧИСТО.\n\n\
СТРОГИЕ правила для \"fact\":\n\
- Бери слова ТОЛЬКО из выбранного фрагмента (можно менять их форму и убирать лишние, но СОХРАНЯЙ \
порядок слов). НЕ добавляй новых слов, НЕ подбирай синонимы, НЕ переводи, НЕ бери слова из других \
фрагментов.\n\
- Числа, коды, адреса (IP, порты, версии, имена) — ТОЧНО как во фрагменте, ни цифры не меняй.\n\
- Сохраняй отрицание (не/нет/ни), если оно есть — не переворачивай смысл.\n\
- Выкинь слова-паразиты и повторы (а-а, ну, вот, значит, это самое, как бы, типа, как их там, \
так-то, в общем), оставь суть — одно короткое ясное утверждение.\n\n\
Ответь ТОЛЬКО строгим JSON:\n\
{\"facts\":[{\"clause\":<номер фрагмента>,\"entity\":\"<о чём, из фрагмента>\",\
\"fact\":\"<чистый факт>\"}]}\n\
entity — имя/система/человек ИЗ фрагмента, или \"\" если неясно.";

/// Async normalize (M1′, fable): heuristic-clean the raw span, [`segment_clauses`] it, and ask the AI
/// (no-think JSON via [`ai::complete`]) to SELECT a clause by index and rewrite it cleanly — 1–3
/// `{clause, entity, fact}`. The model never copies verbatim (a weak model autocorrects while quoting,
/// which broke the old protocol on garbled STT). Per item: [`validate_rewrite`] grounds the clean
/// `fact` in the selected clause (or, if the index is wrong, ANY single clause) — a fact fusing two
/// clauses fails every clause → dropped. Joins ≤3 with «; ». Three outcomes (P3 offline-reliability):
/// `Ok(Some)` fact(s); `Ok(None)` TERMINAL — the AI replied but nothing grounded, OR failed PERMANENTLY
/// (4xx bad bearer/model) so a retry is pointless (caller stores [`heuristic_condense`], marks
/// `'heuristic'`); `Err` a TRANSIENT AI failure (offline/timeout/5xx — RETRYABLE, row left `'pending'`).
/// Never a fabricated fact — every stored word roots in ONE source clause.
pub async fn normalize_fact(
    raw: &str,
    base_url: &str,
    bearer: &str,
    model: &str,
) -> anyhow::Result<Option<NormalizedFact>> {
    let cleaned = heuristic_clean(raw);
    let clauses = segment_clauses(&cleaned);
    if clauses.is_empty() {
        return Ok(None);
    }
    // The user message is the NUMBERED clause list the model SELECTS from (it never copies verbatim).
    let numbered = clauses
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}: {c}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: MessageContent::Text(REWRITE_SYSTEM.into()),
        },
        ChatMessage {
            role: "user".into(),
            content: MessageContent::Text(numbered),
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
            let fact = f.fact.as_str();
            // Ground the clean fact in ONE clause: the model's selected index (a hint), else scan ALL
            // clauses. A fact fusing two clauses' words fails EVERY single clause → dropped. So the
            // stored text is the model's clean rewrite, but only if it's a faithful rewrite of a
            // single source clause (no fabrication / cross-clause fusion) — safety is structural.
            let grounded = f
                .clause
                .and_then(|n| n.checked_sub(1))
                .and_then(|i| clauses.get(i))
                .is_some_and(|c| validate_rewrite(c, fact))
                || clauses.iter().any(|c| validate_rewrite(c, fact));
            if !grounded {
                return None;
            }
            // entity is metadata — keep it only if it too is a source quote (never fabricated).
            let entity = f.entity.and_then(|e| locate_span(&cleaned, &e));
            Some((fact.to_string(), entity))
        })
        .take(3)
        .collect();
    if facts.is_empty() {
        return Ok(None); // AI replied but nothing grounded → terminal 'heuristic', not a retry.
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

/// A raw fact parsed from the model reply (before grounding): the clean `fact`, the 1-based `clause`
/// index the model selected (a HINT — `None`/wrong is fine, the caller re-scans all clauses), and an
/// optional `entity`. [`normalize_fact`] then grounds `fact` against a clause via [`validate_rewrite`].
struct ParsedFact {
    clause: Option<usize>,
    entity: Option<String>,
    fact: String,
}

/// Parse ALL facts from a model reply that SHOULD be `{"facts":[{clause,entity,fact},…]}` but may be
/// wrapped in prose or ```json fences. Pure → tested. Empty vec if nothing parseable; a fact with an
/// empty `fact` is dropped (nothing to ground). `clause` is coerced tolerantly (number OR string OR
/// missing → `None`), since it's only a hint and a weak model may format it loosely.
fn parse_facts(resp: &str) -> Vec<ParsedFact> {
    #[derive(serde::Deserialize)]
    struct FactsDto {
        #[serde(default)]
        facts: Vec<FactDto>,
    }
    #[derive(serde::Deserialize)]
    struct FactDto {
        #[serde(default)]
        clause: serde_json::Value, // number OR string OR missing — coerced below, never fails parse
        #[serde(default)]
        entity: String,
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
            let fact = f.fact.trim().to_string();
            if fact.is_empty() {
                return None; // nothing to ground → drop
            }
            let clause = f
                .clause
                .as_u64()
                .or_else(|| f.clause.as_str().and_then(|s| s.trim().parse().ok()))
                .and_then(|n| usize::try_from(n).ok());
            let e = f.entity.trim();
            Some(ParsedFact {
                clause,
                entity: (!e.is_empty()).then(|| e.to_string()),
                fact,
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
    fn split_fused_token_de_garbles_mixed_script_only() {
        // The owner's case: Latin acronym fused to a Cyrillic word via a confusable «М» → split.
        assert_eq!(
            split_fused_token("LLМоткрытых"),
            vec!["LLM".to_string(), "открытых".to_string()]
        );
        // All-caps fused → split without needing the fold (no confusable bridges into a word).
        assert_eq!(
            split_fused_token("APIСЕРВЕР"),
            vec!["API".to_string(), "СЕРВЕР".to_string()]
        );
        // Pure-script / identifiers / negation words are UNTOUCHED — no meaning can change.
        for t in [
            "сервер",
            "GigaChat",
            "недоступен",
            "перезагружен",
            "10.0.0.116",
            "z14",
            "не",
        ] {
            assert_eq!(split_fused_token(t), vec![t.to_string()], "untouched: {t}");
        }
        // End-to-end via heuristic_clean: the fused token becomes two clean tokens.
        assert_eq!(
            heuristic_clean("используем LLМоткрытых моделей"),
            "используем LLM открытых моделей"
        );
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
    fn parse_facts_handles_clause_index_and_multi() {
        let fs =
            parse_facts(r#"{"facts":[{"clause":2,"entity":"z14","fact":"бекап-сервер z14"}]}"#);
        assert_eq!(fs.len(), 1);
        assert_eq!(fs[0].clause, Some(2));
        assert_eq!(fs[0].fact, "бекап-сервер z14");
        assert_eq!(fs[0].entity.as_deref(), Some("z14"));
        // ```json fences + prose tolerated; a STRING clause is coerced; a missing clause → None.
        let fs = parse_facts(
            "Вот:\n```json\n{\"facts\":[{\"clause\":\"1\",\"entity\":\"\",\"fact\":\"кофе по утрам\"},\
             {\"entity\":\"порт\",\"fact\":\"порт 8080\"}]}\n```\nготово",
        );
        assert_eq!(fs.len(), 2);
        assert_eq!(fs[0].clause, Some(1)); // "1" string coerced
        assert_eq!(fs[0].entity, None);
        assert_eq!(fs[1].clause, None); // missing → None (caller re-scans all clauses)
        assert_eq!(fs[1].fact, "порт 8080");
    }

    #[test]
    fn parse_facts_rejects_empty_and_garbage() {
        assert!(parse_facts(r#"{"facts":[]}"#).is_empty());
        assert!(parse_facts("не json вообще").is_empty());
        // Present but empty `fact` (nothing to ground) → dropped even if a clause is set.
        assert!(parse_facts(r#"{"facts":[{"clause":1,"entity":"x"}]}"#).is_empty());
    }

    #[test]
    fn segment_clauses_splits_drops_filler_and_repacks() {
        // The owner's real failing line: garbled multi-sentence STT. Segmentation must ISOLATE the
        // content clause and DROP the filler/garbage sentences deterministically.
        let line =
            "А-аа, вот, значит, так. А-а-а, да, это самое, мы, в общем, тогда решили идти в \
                    сторону это самое, использования этих самых, как их там, открытых моделей. \
                    Вот так-то да-да. ыбочку и";
        let cl = segment_clauses(line);
        // «А-аа, вот, значит, так.» / «Вот так-то да-да.» / «ыбочку и» are <2 content words → dropped.
        assert!(
            cl.iter()
                .any(|c| c.contains("решили идти") && c.contains("открытых моделей")),
            "content clause kept: {cl:?}"
        );
        assert!(
            !cl.iter().any(|c| c.contains("ыбочку")),
            "garbage clause dropped: {cl:?}"
        );
        // A clause that validates a clean rewrite exists (the whole point — the fact can ground now).
        let content = cl.iter().find(|c| c.contains("решили идти")).unwrap();
        assert!(validate_rewrite(
            content,
            "решили идти в сторону использования открытых моделей"
        ));
    }

    #[test]
    fn owner_garbled_line_cleans_end_to_end() {
        // The owner's ACTUAL failing line, verbatim (with the recognizer-FUSED «LLМоткрытых»).
        let raw = "А-аа, вот, значит, так. А-а-а, да, это самое, мы, в общем, тогда решили идти в \
                   сторону это самое, так-то этого направления использования этих самых, как их \
                   там, LLМоткрытых моделей. Вот так-то да-да. ыбочку и";
        // heuristic_clean un-fuses «LLМоткрытых» → «LLM открытых»; segment isolates the content clause.
        let cleaned = heuristic_clean(raw);
        assert!(
            cleaned.contains("LLM открытых"),
            "fused token un-merged: {cleaned}"
        );
        let clauses = segment_clauses(&cleaned);
        let content = clauses.iter().find(|c| c.contains("LLM открытых")).unwrap();
        // gemma-4-12B's actual output for this clause (validated live against :8080) now GROUNDS,
        // because «открытых» is a real token after the un-merge. Before this fix it was rejected.
        assert!(validate_rewrite(content, "использования открытых моделей"));
        assert!(validate_rewrite(
            content,
            "решили идти в сторону использования открытых моделей"
        ));
    }

    #[test]
    fn heuristic_condense_picks_best_clauses_not_whole_line() {
        let line = "А-аа, вот, значит, так. мы решили купить сервер за 500 рублей в этом месяце. \
                    ыбочку и";
        let out = heuristic_condense(line);
        assert!(
            out.contains("решили купить сервер"),
            "kept the content clause: {out}"
        );
        assert!(!out.contains("ыбочку"), "dropped garbage: {out}");
        // A single clean clause passes through unchanged.
        assert_eq!(
            heuristic_condense("сервер бэкапов готов"),
            "сервер бэкапов готов"
        );
    }

    #[test]
    fn validate_rewrite_accepts_faithful_clean_rewrite() {
        // Drops filler, keeps word order, inflects — content words rooted + in order, number verbatim.
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

    #[test]
    fn validate_rewrite_rejects_reorder_and_within_clause_recombination() {
        // Role swap — same words, REORDERED → not an ordered subsequence → rejected (safety review).
        assert!(!validate_rewrite(
            "клиент платит подрядчику двести тысяч",
            "подрядчик платит клиенту"
        ));
        // Within-clause recombination: «поднят» belongs to «тест», «прод» is «стабилен»; the fact
        // reorders across the comma → not an ordered subsequence → rejected.
        assert!(!validate_rewrite(
            "тест сервер поднят, прод сервер стабилен",
            "прод сервер поднят"
        ));
        // But a faithful order-preserving pick from the same clause IS accepted.
        assert!(validate_rewrite(
            "тест сервер поднят, прод сервер стабилен",
            "прод сервер стабилен"
        ));
    }
}
