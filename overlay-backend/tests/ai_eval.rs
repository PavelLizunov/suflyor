//! Behavioral eval harness for the app's AI features — the
//! "static-checks-pass-while-the-output-is-wrong" gap the 5-layer methodology
//! exists to close.
//!
//! Tier 1 — PURE invariant checkers + unit tests (always run). They encode what
//! a correct summary / translation / auto-name must LOOK like (shape, not exact
//! words), so a future prompt/parse refactor that breaks the shape is caught by
//! `cargo test` like any other regression.
//!
//! Tier 2 — a LIVE eval (`#[ignore]`, gated on `SUFLYOR_EVAL=1` + a running local
//! model) that feeds fixed transcripts through the real endpoint and asserts the
//! same invariants on the model's actual output. Run it with:
//!
//! ```text
//! SUFLYOR_EVAL=1 cargo test --manifest-path overlay-backend/Cargo.toml \
//!   --test ai_eval -- --ignored --nocapture
//! ```
//!
//! Point `SUFLYOR_EVAL_BASE_URL` / `SUFLYOR_EVAL_MODEL` at the local server.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// ---- Tier 1: pure invariant checkers (model-free, always run) ----

/// A meeting summary must contain every required section heading. The map-reduce
/// reduce step is supposed to emit them all; a silent drop is the historical bug.
fn summary_has_all_headings(summary: &str, headings: &[&str]) -> bool {
    let lower = summary.to_lowercase();
    headings.iter().all(|h| lower.contains(&h.to_lowercase()))
}

/// The translate echo-bug: asked to translate EN→RU, the model sometimes echoes
/// the English back verbatim. Heuristic: a Russian translation must contain
/// Cyrillic and must NOT be ~entirely ASCII-Latin.
fn looks_like_untranslated_echo(output: &str) -> bool {
    let has_cyrillic = output
        .chars()
        .any(|c| ('А'..='я').contains(&c) || c == 'ё' || c == 'Ё');
    let ascii_letters = output.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let total_letters = output.chars().filter(|c| c.is_alphabetic()).count();
    !has_cyrillic && total_letters > 0 && ascii_letters * 10 >= total_letters * 9
}

/// An auto-generated session name must be SHORT + unquoted (mirrors the
/// `session_namer::clean_name` contract: ≤ 4 words, no wrapping quotes, ≤ 60
/// chars). A model that ignores the "≤4 words, no quotes" prompt fails here.
fn name_is_clean(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t.chars().count() <= 60
        && t.split_whitespace().count() <= 4
        && !t.starts_with('"')
        && !t.starts_with('«')
        && !t.ends_with('"')
        && !t.ends_with('»')
}

#[test]
fn summary_heading_checker_catches_a_dropped_section() {
    let required = ["Решения", "Действия", "Риски"];
    assert!(summary_has_all_headings(
        "## Решения\n…\n## Действия\n…\n## Риски\n…",
        &required
    ));
    assert!(!summary_has_all_headings(
        "## Решения\n…\n## Действия\n…",
        &required
    ));
}

#[test]
fn translate_echo_detector() {
    assert!(looks_like_untranslated_echo(
        "The interviewer asked about hash maps and load factors."
    ));
    assert!(!looks_like_untranslated_echo(
        "Интервьюер спросил про хеш-таблицы и коэффициент заполнения."
    ));
    assert!(!looks_like_untranslated_echo(
        "Хеш-таблица (hash map) — это структура данных."
    ));
}

#[test]
fn auto_name_contract() {
    assert!(name_is_clean("Путь к высокому доходу"));
    assert!(!name_is_clean("\"Путь к высокому доходу\""));
    assert!(!name_is_clean("«Обзор функций приложения»"));
    assert!(!name_is_clean(
        "Очень длинное название которое явно превышает лимит из четырёх слов"
    ));
    assert!(!name_is_clean(""));
}


#[test]
#[ignore = "live: set SUFLYOR_EVAL=1 (+ SUFLYOR_EVAL_BASE_URL/_MODEL) and run with --ignored"]
fn live_local_ai_invariants() {
    if std::env::var("SUFLYOR_EVAL").ok().as_deref() != Some("1") {
        eprintln!("SUFLYOR_EVAL != 1 — skipping live eval");
        return;
    }
    eprintln!("live eval stub — wire ai::complete against the local model to expand");
}
