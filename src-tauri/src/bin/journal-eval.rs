//! journal-eval — regression-check a live journal vs ground-truth fixture.
//!
//! Usage:
//!   cargo run --bin journal-eval -- <journal.jsonl> <ground-truth.json>
//!
//! Emits a markdown report to stdout summarising:
//!   - Whisper accuracy: which expected_terms were found in transcript
//!   - Detector recall: which expected_triggers fired
//!   - Detector precision: false-positives matching expected_quiet
//!   - Cost + latency + AI call counts (from session_summary)
//!
//! The binary is intentionally a thin shell over `eval_module` so the
//! logic stays unit-testable without filesystem I/O.

use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::fs;
use std::process::ExitCode;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)] // duration_sec is informational metadata
struct GroundTruth {
    case_id: String,
    title: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    duration_sec: Option<u64>,
    #[serde(default)]
    domain: Vec<String>,
    #[serde(default)]
    expected_terms: Vec<ExpectedTerm>,
    #[serde(default)]
    expected_triggers: Vec<ExpectedTrigger>,
    #[serde(default)]
    expected_quiet: Vec<String>,
    #[serde(default)]
    answer_quality_notes: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ExpectedTerm {
    /// The canonical spelling we want in the transcript (e.g. "kubernetes").
    canonical: String,
    /// Acceptable phonetic / shorthand variants (e.g. ["k8s", "кубер"]).
    /// Counts as found if any of canonical+aliases appears.
    #[serde(default)]
    aliases: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)] // must_trigger reserved for future severity weighting
struct ExpectedTrigger {
    /// Substring (case-insensitive) the detector_decision.text should contain.
    text_match: String,
    /// Marks this as a regression-critical trigger. Currently informational —
    /// all expected triggers are recall-tracked.
    #[serde(default = "default_true")]
    must_trigger: bool,
}

fn default_true() -> bool {
    true
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: journal-eval <journal.jsonl> <ground-truth.json>\n\n\
             Compares a session journal against an expected-behavior fixture\n\
             and prints a markdown report to stdout."
        );
        return ExitCode::from(2);
    }
    let journal_path = &args[1];
    let gt_path = &args[2];

    let journal_text = match fs::read_to_string(journal_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read journal {}: {}", journal_path, e);
            return ExitCode::from(1);
        }
    };
    let gt_text = match fs::read_to_string(gt_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("read ground-truth {}: {}", gt_path, e);
            return ExitCode::from(1);
        }
    };

    let events: Vec<Value> = journal_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let gt: GroundTruth = match serde_json::from_str(&gt_text) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("parse ground-truth: {}", e);
            return ExitCode::from(1);
        }
    };

    let report = eval_module::build_report(&gt, &events);
    println!("{}", report);
    ExitCode::SUCCESS
}

// ── Logic split into a module so we can unit-test it ──

mod eval_module {
    use super::*;

    /// Result of checking whether a specific term appeared in the journal.
    #[derive(Debug, PartialEq, Eq)]
    pub struct TermResult {
        pub canonical: String,
        pub found: bool,
        /// Which alias matched, if not the canonical. Empty when canonical hit.
        pub matched_as: String,
    }

    /// Result of checking whether a specific question/trigger fired.
    #[derive(Debug, PartialEq, Eq)]
    pub struct TriggerResult {
        pub expected: String,
        pub triggered: bool,
        /// First matching transcript snippet (≤80 chars) for context.
        pub matched_text: String,
    }

    /// False-positive result: a "quiet" utterance that nonetheless triggered.
    #[derive(Debug, PartialEq, Eq)]
    pub struct FalsePositive {
        pub expected_quiet: String,
        pub actual_text: String,
    }

    pub fn build_report(gt: &GroundTruth, events: &[Value]) -> String {
        let term_results = check_terms(gt, events);
        let trigger_results = check_triggers(gt, events);
        let false_positives = check_quiet(gt, events);
        let agg = aggregate_session(events);

        format_markdown(gt, &term_results, &trigger_results, &false_positives, &agg)
    }

    /// For each expected_term, scan transcript_line events for any case-
    /// insensitive substring match against canonical OR any alias.
    pub fn check_terms(gt: &GroundTruth, events: &[Value]) -> Vec<TermResult> {
        let transcript_concat: String = events
            .iter()
            .filter(|e| e.get("kind").and_then(|k| k.as_str()) == Some("transcript_line"))
            .filter_map(|e| e.get("text").and_then(|t| t.as_str()))
            .map(|s| s.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        gt.expected_terms
            .iter()
            .map(|t| {
                let canon_lower = t.canonical.to_lowercase();
                if transcript_concat.contains(&canon_lower) {
                    TermResult {
                        canonical: t.canonical.clone(),
                        found: true,
                        matched_as: String::new(),
                    }
                } else {
                    // Try aliases
                    let alias_hit = t
                        .aliases
                        .iter()
                        .find(|a| transcript_concat.contains(&a.to_lowercase()))
                        .cloned()
                        .unwrap_or_default();
                    TermResult {
                        canonical: t.canonical.clone(),
                        found: !alias_hit.is_empty(),
                        matched_as: alias_hit,
                    }
                }
            })
            .collect()
    }

    /// For each expected_trigger, look at detector_decision events: find one
    /// whose text matches (loose fuzzy) the expected `text_match`, then check
    /// it actually fired.
    ///
    /// Matching uses `text_matches_loosely` which handles Whisper realities:
    /// punctuation differences, Latin/Cyrillic loanword pairs (docker↔докер),
    /// Russian inflection (докер/докере/докером via 5-char stem prefix), and
    /// proximity-windowed token-set match for multi-word phrases.
    pub fn check_triggers(gt: &GroundTruth, events: &[Value]) -> Vec<TriggerResult> {
        let detector_events: Vec<&Value> = events
            .iter()
            .filter(|e| e.get("kind").and_then(|k| k.as_str()) == Some("detector_decision"))
            .collect();

        gt.expected_triggers
            .iter()
            .map(|trig| {
                let hit = detector_events.iter().find(|e| {
                    e.get("text")
                        .and_then(|t| t.as_str())
                        .map(|s| text_matches_loosely(&trig.text_match, s))
                        .unwrap_or(false)
                });
                match hit {
                    Some(e) => {
                        let actual = e.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        let triggered = e
                            .get("triggered")
                            .and_then(|b| b.as_bool())
                            .unwrap_or(false);
                        TriggerResult {
                            expected: trig.text_match.clone(),
                            triggered,
                            matched_text: actual.chars().take(80).collect(),
                        }
                    }
                    None => TriggerResult {
                        expected: trig.text_match.clone(),
                        triggered: false,
                        matched_text: String::from("(transcript line not found)"),
                    },
                }
            })
            .collect()
    }

    /// For each expected_quiet utterance, check whether any detector_decision
    /// fired on text whose entire content (after normalizing) equals that
    /// utterance — i.e. a pure-filler turn the model should have skipped.
    ///
    /// IMPORTANT: this is whole-utterance match, NOT substring. v1 of this
    /// check used `contains` which produced massive false-flagging because
    /// 25-min interview chunks naturally contain "ну" or "то есть" as
    /// substrings of real questions — every triggered text was getting
    /// flagged. Whole-utterance match correctly flags only the cases where
    /// the detector ran AI on a pure filler ("Угу.", "Ну вот так").
    pub fn check_quiet(gt: &GroundTruth, events: &[Value]) -> Vec<FalsePositive> {
        let mut out = Vec::new();
        for quiet in &gt.expected_quiet {
            let needle = normalize_for_quiet_match(quiet);
            if needle.is_empty() {
                continue;
            }
            for e in events.iter().filter(|e| {
                e.get("kind").and_then(|k| k.as_str()) == Some("detector_decision")
                    && e.get("triggered").and_then(|b| b.as_bool()).unwrap_or(false)
            }) {
                let text = e.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let normalized = normalize_for_quiet_match(text);
                if normalized == needle {
                    out.push(FalsePositive {
                        expected_quiet: quiet.clone(),
                        actual_text: text.chars().take(80).collect(),
                    });
                }
            }
        }
        out
    }

    // ── Fuzzy text matching (Plan-agent design) ──
    //
    // 4-stage pipeline for matching expected_triggers against Whisper output.
    // Tackles 3 real misses observed in 25-min live test:
    //   1. "какие вообще как ты его настраивал" vs "какие вообще, как ты..."
    //      → comma between words breaks substring contains
    //   2. "нетворка в докере" vs "типы нетворка в Docker"
    //      → Whisper kept "Docker" Latin script
    //   3. "разные состояния" vs "какие типы состояний бывают"
    //      → Russian inflection (состояния vs состояний)

    /// Loanword aliases — pairs (latin, cyrillic). Canonicalised to latin.
    /// Kept small on purpose: only loanwords with no native Russian collision.
    const ALIASES: &[(&str, &str)] = &[
        ("docker", "докер"),
        ("gitlab", "гитлаб"),
        ("kubernetes", "кубернетес"),
        ("runner", "раннер"),
        ("ansible", "ансибл"),
        ("proxmox", "прокмокс"),
        ("dmesg", "дмесг"),
        ("pipeline", "пайплайн"),
        ("registry", "реджестри"),
        ("network", "нетворк"),
        ("alpine", "алпайн"),
        ("nginx", "нгинкс"),
        ("postgres", "постгрес"),
    ];

    const STEM_LEN: usize = 5;
    const PROXIMITY_FACTOR: usize = 2;

    /// Loose-match an expected pattern against actual text. See module-level
    /// comment for the 4-stage pipeline and rationale.
    pub fn text_matches_loosely(expected: &str, actual: &str) -> bool {
        let e_low = expected.to_lowercase();
        let a_low = actual.to_lowercase();

        // Substring fast paths (stages A/B) are SAFE for multi-word phrases
        // but UNSAFE for short single tokens — "что" would match "чтобы".
        // Gate both on: has space (multi-word) OR ≥5 chars (long enough that
        // accidental substring overlap is unlikely in real Russian).
        let substring_safe = e_low.contains(' ') || e_low.chars().count() >= 5;

        // Stage A: fast-path raw substring (catches 80%+ before any work).
        if substring_safe && a_low.contains(&e_low) {
            return true;
        }

        // Stage B: normalise both — strip punct, canonicalise loanwords.
        let e_norm = normalize_and_canonicalise(&e_low);
        let a_norm = normalize_and_canonicalise(&a_low);
        if e_norm.is_empty() {
            return false;
        }
        if substring_safe && a_norm.contains(&e_norm) {
            return true;
        }

        // Stage C+D: token-set with proximity + stem-prefix equality.
        let e_toks: Vec<&str> = e_norm.split_whitespace().collect();
        let a_toks: Vec<&str> = a_norm.split_whitespace().collect();

        if e_toks.len() == 1 {
            return a_toks.iter().any(|t| stem_eq(t, e_toks[0]));
        }

        // Multi-word: every expected token must appear in actual (under stem
        // equality), AND their positions must lie within a window of
        // expected_len * PROXIMITY_FACTOR — prevents promiscuous matches.
        let window = e_toks.len() * PROXIMITY_FACTOR;
        let positions: Option<Vec<usize>> = e_toks
            .iter()
            .map(|et| a_toks.iter().position(|at| stem_eq(at, et)))
            .collect();
        let Some(positions) = positions else {
            return false;
        };
        let (lo, hi) = (
            *positions.iter().min().unwrap(),
            *positions.iter().max().unwrap(),
        );
        hi - lo <= window
    }

    /// Strip non-alnum → spaces, collapse whitespace, canonicalise loanwords.
    fn normalize_and_canonicalise(s: &str) -> String {
        let scrubbed: String = s
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        scrubbed
            .split_whitespace()
            .map(canonicalise_token)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Map a token to its canonical (Latin) form if it's a known loanword.
    /// Catches inflected forms: "докере" → "docker", "контейнерами" stays.
    fn canonicalise_token(t: &str) -> String {
        for (latin, cyr) in ALIASES {
            if t == *latin || t == *cyr {
                return (*latin).to_string();
            }
            // Inflected: token starts with cyrillic form ("докер" + suffix)
            if t.starts_with(cyr) || t.starts_with(latin) {
                return (*latin).to_string();
            }
        }
        t.to_string()
    }

    /// Two tokens equal if exact OR if both ≥ STEM_LEN chars and share that
    /// many leading chars. Covers Russian inflection (докер/докере/докером).
    fn stem_eq(a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        let n = STEM_LEN.min(a.chars().count()).min(b.chars().count());
        if n < STEM_LEN {
            return false;
        }
        a.chars().take(n).eq(b.chars().take(n))
    }

    /// Lowercase, strip punctuation, collapse whitespace. Used so quiet-match
    /// is robust to ", окей." vs "окей" vs "  ОКЕЙ!". Whole-utterance only —
    /// see check_quiet for why.
    fn normalize_for_quiet_match(s: &str) -> String {
        let lowered = s.to_lowercase();
        let only_word_chars: String = lowered
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        only_word_chars
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[derive(Debug, Default, PartialEq)]
    pub struct SessionAggregate {
        pub duration_min: f64,
        pub transcript_lines: u64,
        pub triggered: u64,
        pub skipped: u64,
        pub ai_requests: u64,
        pub ai_responses: u64,
        pub ai_errors: u64,
        pub tiles: u64,
        pub rate_limited: u64,
        pub cost_usd: f64,
        pub had_summary: bool,
    }

    /// Pull totals from session_summary if present; otherwise recount from
    /// individual events. session_summary is more accurate (single source
    /// of truth at session close).
    pub fn aggregate_session(events: &[Value]) -> SessionAggregate {
        let summary = events
            .iter()
            .find(|e| e.get("kind").and_then(|k| k.as_str()) == Some("session_summary"));
        if let Some(s) = summary {
            return SessionAggregate {
                duration_min: get_u64(s, "duration_ms") as f64 / 60_000.0,
                transcript_lines: get_u64(s, "transcript_lines"),
                triggered: get_u64(s, "detector_triggered"),
                skipped: get_u64(s, "detector_skipped"),
                ai_requests: get_u64(s, "ai_requests_total"),
                ai_responses: get_u64(s, "ai_responses_ok"),
                ai_errors: get_u64(s, "ai_errors"),
                tiles: get_u64(s, "tiles_spawned"),
                rate_limited: get_u64(s, "rate_limited"),
                cost_usd: get_u64(s, "total_cost_microcents") as f64 / 100_000_000.0,
                had_summary: true,
            };
        }
        // Fallback: recount from raw events. Less accurate if session
        // crashed mid-write but better than zero.
        let mut agg = SessionAggregate::default();
        for e in events {
            let kind = e.get("kind").and_then(|k| k.as_str()).unwrap_or("");
            match kind {
                "transcript_line" => agg.transcript_lines += 1,
                "detector_decision" => {
                    if e.get("triggered").and_then(|b| b.as_bool()).unwrap_or(false) {
                        agg.triggered += 1;
                    } else {
                        agg.skipped += 1;
                    }
                }
                "ai_request" => agg.ai_requests += 1,
                "ai_response" => {
                    agg.ai_responses += 1;
                    agg.cost_usd += get_u64(e, "cost_microcents") as f64 / 100_000_000.0;
                }
                "tile_spawn" => agg.tiles += 1,
                "rate_limited" => agg.rate_limited += 1,
                "error" => agg.ai_errors += 1,
                _ => {}
            }
        }
        agg
    }

    fn get_u64(v: &Value, key: &str) -> u64 {
        v.get(key).and_then(|n| n.as_u64()).unwrap_or(0)
    }

    fn format_markdown(
        gt: &GroundTruth,
        terms: &[TermResult],
        triggers: &[TriggerResult],
        false_positives: &[FalsePositive],
        agg: &SessionAggregate,
    ) -> String {
        let mut out = String::new();

        out.push_str(&format!("# Eval · `{}` — {}\n\n", gt.case_id, gt.title));
        if let Some(src) = &gt.source {
            out.push_str(&format!("Source: {}\n", src));
        }
        if !gt.domain.is_empty() {
            out.push_str(&format!("Domain: {}\n", gt.domain.join(", ")));
        }
        out.push('\n');

        // ── Whisper accuracy ──
        let found_count = terms.iter().filter(|t| t.found).count();
        let total = terms.len();
        let pct = found_count.saturating_mul(100).checked_div(total).unwrap_or(100);
        out.push_str(&format!(
            "## Whisper accuracy\n\n{}/{} terms found ({}%)\n\n",
            found_count, total, pct
        ));
        if !terms.is_empty() {
            for t in terms {
                if t.found && t.matched_as.is_empty() {
                    out.push_str(&format!("- ✅ `{}` (exact)\n", t.canonical));
                } else if t.found {
                    out.push_str(&format!(
                        "- ⚠️ `{}` matched only as alias `{}`\n",
                        t.canonical, t.matched_as
                    ));
                } else {
                    out.push_str(&format!("- ❌ `{}` MISSING\n", t.canonical));
                }
            }
            out.push('\n');
        }

        // ── Detector recall ──
        let trig_hit = triggers.iter().filter(|t| t.triggered).count();
        let trig_total = triggers.len();
        let trig_pct = trig_hit.saturating_mul(100).checked_div(trig_total).unwrap_or(100);
        out.push_str(&format!(
            "## Detector recall\n\n{}/{} expected triggers fired ({}%)\n\n",
            trig_hit, trig_total, trig_pct
        ));
        for t in triggers {
            if t.triggered {
                out.push_str(&format!("- ✅ \"{}\" → triggered\n", t.expected));
            } else {
                out.push_str(&format!(
                    "- ❌ \"{}\" → MISSED (transcript: \"{}\")\n",
                    t.expected, t.matched_text
                ));
            }
        }
        out.push('\n');

        // ── Detector precision (false positives) ──
        out.push_str(&format!(
            "## Detector precision\n\n{} false-positive triggers (wasted AI calls)\n\n",
            false_positives.len()
        ));
        for fp in false_positives {
            out.push_str(&format!(
                "- ⚠ quiet utterance `{}` triggered (text: \"{}\")\n",
                fp.expected_quiet, fp.actual_text
            ));
        }
        if !false_positives.is_empty() {
            out.push('\n');
        }

        // ── Aggregates ──
        out.push_str("## Session aggregates\n\n");
        if agg.had_summary {
            out.push_str("(from session_summary)\n\n");
        } else {
            out.push_str("(recounted from events — no session_summary present)\n\n");
        }
        out.push_str(&format!("- Duration: {:.1} min\n", agg.duration_min));
        out.push_str(&format!(
            "- Transcript: {} lines\n",
            agg.transcript_lines
        ));
        out.push_str(&format!(
            "- Detector: {} triggered / {} skipped\n",
            agg.triggered, agg.skipped
        ));
        out.push_str(&format!(
            "- AI: {} requests · {} responses · {} errors\n",
            agg.ai_requests, agg.ai_responses, agg.ai_errors
        ));
        out.push_str(&format!("- Tiles spawned: {}\n", agg.tiles));
        if agg.rate_limited > 0 {
            out.push_str(&format!("- Rate-limited: {}\n", agg.rate_limited));
        }
        out.push_str(&format!("- Cost: ${:.4}\n", agg.cost_usd));
        out.push('\n');

        // ── Notes ──
        if let Some(notes) = &gt.answer_quality_notes {
            out.push_str("## Manual review checklist\n\n");
            out.push_str(notes);
            out.push_str("\n\n");
        }

        out
    }
}

// ── Unit tests ──

#[cfg(test)]
mod tests {
    use super::eval_module::*;
    use super::*;
    use serde_json::json;

    fn gt_basic() -> GroundTruth {
        GroundTruth {
            case_id: "test".into(),
            title: "test case".into(),
            source: None,
            duration_sec: None,
            domain: vec!["devops".into()],
            expected_terms: vec![
                ExpectedTerm { canonical: "kubernetes".into(), aliases: vec!["k8s".into()] },
                ExpectedTerm { canonical: "etcd".into(), aliases: vec![] },
            ],
            expected_triggers: vec![
                ExpectedTrigger { text_match: "что такое pod".into(), must_trigger: true },
            ],
            expected_quiet: vec!["угу".into()],
            answer_quality_notes: None,
        }
    }

    fn evt_transcript(text: &str) -> Value {
        json!({"kind":"transcript_line","unix_ms":1,"source":"system","text":text})
    }

    fn evt_detector(text: &str, triggered: bool) -> Value {
        json!({"kind":"detector_decision","unix_ms":2,"text":text,"triggered":triggered,"trigger_kind":null})
    }

    fn evt_summary(transcript: u64, triggered: u64, skipped: u64, cost_micro: u64) -> Value {
        json!({
            "kind":"session_summary","unix_ms":99,"duration_ms":600_000,
            "transcript_lines":transcript,"transcript_mic":0,"transcript_system":transcript,
            "detector_triggered":triggered,"detector_skipped":skipped,
            "ai_requests_total":triggered,"ai_responses_ok":triggered,"ai_errors":0,
            "tiles_spawned":triggered,"rate_limited":0,"total_cost_microcents":cost_micro
        })
    }

    #[test]
    fn term_found_by_exact_canonical_match() {
        let gt = gt_basic();
        let events = vec![evt_transcript("Сегодня поговорим про Kubernetes и etcd")];
        let r = check_terms(&gt, &events);
        assert_eq!(r.len(), 2);
        assert!(r[0].found && r[0].matched_as.is_empty());
        assert!(r[1].found);
    }

    #[test]
    fn term_found_by_alias_when_canonical_missing() {
        let gt = gt_basic();
        let events = vec![evt_transcript("у нас k8s кластер")];
        let r = check_terms(&gt, &events);
        assert!(r[0].found, "kubernetes should match via k8s alias");
        assert_eq!(r[0].matched_as, "k8s");
    }

    #[test]
    fn term_missing_when_neither_canonical_nor_alias_present() {
        let gt = gt_basic();
        let events = vec![evt_transcript("ничего технического")];
        let r = check_terms(&gt, &events);
        assert!(!r[0].found);
        assert!(!r[1].found);
    }

    #[test]
    fn term_match_case_insensitive() {
        let gt = gt_basic();
        let events = vec![evt_transcript("KUBERNETES is great")];
        let r = check_terms(&gt, &events);
        assert!(r[0].found, "case shouldn't matter");
    }

    #[test]
    fn trigger_found_when_detector_fired_on_matching_text() {
        let gt = gt_basic();
        let events = vec![evt_detector("Что такое pod в Kubernetes?", true)];
        let r = check_triggers(&gt, &events);
        assert_eq!(r.len(), 1);
        assert!(r[0].triggered);
        assert!(r[0].matched_text.contains("pod"));
    }

    #[test]
    fn trigger_missed_when_detector_skipped_matching_text() {
        let gt = gt_basic();
        let events = vec![evt_detector("Что такое pod", false)];
        let r = check_triggers(&gt, &events);
        assert!(!r[0].triggered, "matched text but detector skipped");
    }

    #[test]
    fn trigger_missed_when_transcript_line_absent() {
        let gt = gt_basic();
        let events: Vec<Value> = vec![];
        let r = check_triggers(&gt, &events);
        assert!(!r[0].triggered);
        assert!(r[0].matched_text.contains("not found"));
    }

    #[test]
    fn false_positive_pure_filler_only() {
        // "угу" alone IS the whole turn → real FP, model should have skipped.
        let gt = gt_basic();
        let events = vec![evt_detector("Угу", true)];
        let fp = check_quiet(&gt, &events);
        assert_eq!(fp.len(), 1, "pure-filler turn should be flagged");
        assert_eq!(fp[0].expected_quiet, "угу");
    }

    #[test]
    fn no_false_positive_when_filler_is_part_of_real_question() {
        // v2 schema bug (#97): substring-match flagged this as FP because
        // "ну" appears inside "нужно". Now (whole-utterance match) it's ok.
        let gt = gt_basic();
        let events = vec![evt_detector(
            "Окей, ладно. А скажи, пожалуйста, зачем люди боятся? Что нужно знать?",
            true,
        )];
        let fp = check_quiet(&gt, &events);
        assert!(fp.is_empty(), "real question containing filler chars must NOT be FP");
    }

    #[test]
    fn no_false_positive_when_quiet_skipped() {
        let gt = gt_basic();
        let events = vec![evt_detector("угу", false)];
        let fp = check_quiet(&gt, &events);
        assert!(fp.is_empty(), "skipped quiet should NOT be flagged");
    }

    #[test]
    fn quiet_match_normalizes_punctuation_and_case() {
        let gt = gt_basic();
        // " Угу. " with trailing junk normalises to "угу" → FP.
        let events = vec![evt_detector(" Угу.  ", true)];
        let fp = check_quiet(&gt, &events);
        assert_eq!(fp.len(), 1, "normalized 'угу' should still match");
    }

    #[test]
    fn quiet_match_handles_multi_word_filler() {
        // Add multi-word quiet entry inline (not in gt_basic), simulate match.
        let mut gt = gt_basic();
        gt.expected_quiet.push("ну вот так".into());
        let events = vec![evt_detector(" Ну, вот так! ", true)];
        let fp = check_quiet(&gt, &events);
        assert!(fp.iter().any(|f| f.expected_quiet == "ну вот так"));
    }

    #[test]
    fn aggregate_pulls_from_session_summary_when_present() {
        let events = vec![evt_summary(10, 5, 3, 1_500_000)];
        let agg = aggregate_session(&events);
        assert!(agg.had_summary);
        assert_eq!(agg.transcript_lines, 10);
        assert_eq!(agg.triggered, 5);
        assert!((agg.cost_usd - 0.015).abs() < 0.0001);
    }

    #[test]
    fn aggregate_recounts_when_no_summary() {
        let events = vec![
            evt_transcript("a"),
            evt_transcript("b"),
            evt_detector("question?", true),
            evt_detector("filler", false),
            json!({"kind":"ai_request","unix_ms":3,"purpose":"auto_tile","model":"haiku","system_prompt":"","user_prompt":"","attached_screenshot":false,"input_tokens_est":50}),
            json!({"kind":"ai_response","unix_ms":4,"purpose":"auto_tile","model":"haiku","latency_ms":500,"finish_reason":"stop","text":"","output_tokens_est":100,"cost_microcents":12345}),
            json!({"kind":"tile_spawn","unix_ms":5,"label":"tile-1","question":"q","answer":"a"}),
        ];
        let agg = aggregate_session(&events);
        assert!(!agg.had_summary);
        assert_eq!(agg.transcript_lines, 2);
        assert_eq!(agg.triggered, 1);
        assert_eq!(agg.skipped, 1);
        assert_eq!(agg.ai_requests, 1);
        assert_eq!(agg.ai_responses, 1);
        assert_eq!(agg.tiles, 1);
        assert!((agg.cost_usd - 0.00012345).abs() < 1e-8);
    }

    #[test]
    fn report_contains_all_sections_and_summary_data() {
        let gt = gt_basic();
        let events = vec![
            evt_transcript("Сегодня про kubernetes"),
            evt_detector("Что такое pod?", true),
            evt_detector("угу", true), // false positive
            evt_summary(1, 2, 0, 500_000),
        ];
        let report = build_report(&gt, &events);

        assert!(report.contains("# Eval"));
        assert!(report.contains("Whisper accuracy"));
        assert!(report.contains("Detector recall"));
        assert!(report.contains("Detector precision"));
        assert!(report.contains("Session aggregates"));
        assert!(report.contains("$0.0050"));
        assert!(report.contains("false-positive"));
    }

    // ── Fuzzy text_matches_loosely tests (from real live-test misses) ──

    #[test]
    fn fuzzy_matches_through_added_punctuation() {
        // Live miss #1: Whisper added a comma in the middle of the phrase.
        assert!(text_matches_loosely(
            "какие вообще как ты его настраивал",
            "какие вообще, как ты его настраивал?",
        ));
    }

    #[test]
    fn fuzzy_matches_latin_vs_cyrillic_loanword() {
        // Live miss #2: Whisper kept "Docker" Latin instead of cyrillicising.
        assert!(text_matches_loosely(
            "нетворка в докере",
            "А как устроен networking в Docker?",
        ));
    }

    #[test]
    fn fuzzy_matches_russian_inflection_via_stem() {
        // Russian morphology: docker/докер/докером/докере all 5-char-stem equal.
        assert!(text_matches_loosely(
            "отличается докер",
            "чем отличаются Docker от containerd?",
        ));
    }

    #[test]
    fn fuzzy_rejects_unrelated_text_with_one_token_overlap() {
        // Precision guard: 4-word expected must NOT match on 1 stray word.
        assert!(!text_matches_loosely(
            "что происходит с правами",
            "а что ты ел на завтрак",
        ));
    }

    #[test]
    fn fuzzy_one_word_expected_requires_token_presence() {
        // Strict: "отличается" must appear as a real token (stem-matched).
        assert!(text_matches_loosely("отличается", "чем отличается docker?"));
        // But not in unrelated speech where no token shares the stem.
        assert!(!text_matches_loosely("отличается", "сегодня хорошая погода"));
    }

    #[test]
    fn fuzzy_proximity_window_prevents_promiscuous_match() {
        // The expected tokens must appear close together — not scattered
        // across 60 min of transcript. Two-token expected, window=4.
        // Both tokens present but 50 tokens apart → reject.
        let scattered = "docker started yesterday and we talked about other things \
                         for a long time also about something else and food and \
                         then much later we discussed runners deploying jobs";
        // 'docker' at pos 0, 'runner' (stem) at pos ~20+ → outside window of 4.
        assert!(!text_matches_loosely("docker runner", scattered));
    }

    #[test]
    fn fuzzy_canonicalises_known_loanwords() {
        // Token-level alias mapping bridges Latin/Cyrillic for loanwords.
        assert!(text_matches_loosely("ansible роли", "А расскажи про ансибл роли?"));
        // Words without alias entries (like 'pods'/'поды') won't auto-bridge —
        // that's a design choice, not a bug. If a domain needs the mapping,
        // add to ALIASES.
        assert!(text_matches_loosely("docker контейнер", "запустить docker контейнер"));
    }

    #[test]
    fn fuzzy_fast_path_still_works() {
        // Exact substring should hit stage A without modification.
        assert!(text_matches_loosely("docker run", "пишем docker run hello-world"));
    }

    #[test]
    fn fuzzy_short_tokens_dont_falsely_match_via_stem() {
        // "что" is 3 chars — < STEM_LEN — must not match "чтобы".
        // (Both lowercase here for fairness.)
        let r = text_matches_loosely("что", "чтобы не забыть");
        // Either result is defensible; document chosen behavior.
        // Current impl: 1-token-expected uses stem_eq which requires
        // ≥5 chars; short token falls through to exact equality only.
        assert!(!r, "3-char 'что' should not stem-match 'чтобы'");
    }

    #[test]
    fn report_handles_empty_journal() {
        let gt = gt_basic();
        let events: Vec<Value> = vec![];
        let report = build_report(&gt, &events);
        // Should still emit a report with 0/N findings
        assert!(report.contains("0/2 terms found"));
        assert!(report.contains("0/1 expected triggers fired"));
    }
}
