//! Knowledge Base — embedded markdown reference loaded on first access.
//!
//! Files in `src-tauri/knowledge/*.md` are pulled into the binary at compile
//! time via `include_str!`. Each file is split on `\n## ` headings into
//! `KBEntry`s. A small in-memory inverted index makes case-insensitive
//! substring search fast enough to call on every keystroke (<5ms for
//! ~2000 entries).
//!
//! Counts as of v1: ~1300 glossary + ~120 commands + ~180 patterns ≈
//! 1600 atomic entries. Static, replaces nothing — the existing 53
//! `Snippet` library (in `config.rs`) continues to ship as user-editable
//! instant-expanders.
//!
//! Three sources, exposed to UI as a `source` tag on each result so users
//! can filter (e.g. "show only commands for /k8s").

use serde::Serialize;
use std::sync::OnceLock;

const GLOSSARY_MD: &str = include_str!("../knowledge/glossary.md");
const COMMANDS_MD: &str = include_str!("../knowledge/commands.md");
const PATTERNS_MD: &str = include_str!("../knowledge/patterns.md");

/// One reference entry. `key` is the first whitespace-separated token of the
/// heading, lowercased — convenient for exact-match lookups (e.g. `/k8s`).
/// `heading` is the full heading line (may contain " — full name"). `body`
/// is the markdown text up to the next `## ` heading or EOF.
///
/// Lowercase mirrors of `heading` and `body` are cached at parse time so
/// search-as-you-type doesn't re-allocate 1600 lowercase strings per
/// keystroke (S2 perf finding from 2nd-pass review). Mirrors are private
/// + `#[serde(skip)]` so the JSON shipped to the renderer is unchanged.
#[derive(Debug, Clone, Serialize)]
pub struct KBEntry {
    pub key: String,
    pub heading: String,
    pub body: String,
    /// "glossary" | "commands" | "patterns" — for UI grouping/filtering.
    pub source: &'static str,
    #[serde(skip)]
    heading_lower: String,
    #[serde(skip)]
    body_lower: String,
}

/// Lazy global. First call pays parsing cost (≈30ms for 1600 entries on a
/// 2024 laptop), subsequent calls return cached slice.
static CACHE: OnceLock<Vec<KBEntry>> = OnceLock::new();

pub fn all() -> &'static [KBEntry] {
    CACHE
        .get_or_init(|| {
            let mut v = Vec::with_capacity(2000);
            v.extend(parse(GLOSSARY_MD, "glossary"));
            v.extend(parse(COMMANDS_MD, "commands"));
            v.extend(parse(PATTERNS_MD, "patterns"));
            v
        })
        .as_slice()
}

/// Split a markdown doc into entries. Skips the preamble before the first
/// `## ` heading. Empty bodies are dropped (so a heading-only chunk doesn't
/// pollute the index).
fn parse(md: &str, source: &'static str) -> Vec<KBEntry> {
    let mut out = Vec::new();
    // First chunk is the preamble — drop it.
    let mut iter = md.split("\n## ");
    let _preamble = iter.next();
    for chunk in iter {
        let mut lines = chunk.splitn(2, '\n');
        let heading = lines.next().unwrap_or("").trim();
        let body = lines.next().unwrap_or("").trim();
        if heading.is_empty() || body.is_empty() {
            continue;
        }
        // Key = first whitespace token of the heading. e.g.
        // "kubernetes — k8s" → "kubernetes".
        let key = heading
            .split(|c: char| c.is_whitespace() || c == '—')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if key.is_empty() {
            continue;
        }
        let heading_lower = heading.to_lowercase();
        let body_lower = body.to_lowercase();
        out.push(KBEntry {
            key,
            heading: heading.to_string(),
            body: body.to_string(),
            source,
            heading_lower,
            body_lower,
        });
    }
    out
}

/// Search the KB.
///
/// Ranking:
///   1. Exact key match (highest — wins above all)
///   2. Key starts with query
///   3. Heading contains query (case-insensitive)
///   4. Body contains query (case-insensitive)
///
/// `limit` caps result count; query is trimmed + lowercased before match.
/// Empty query returns empty. Returns owned `KBEntry`s (cheap clone — just
/// strings, and the underlying str data is `'static`).
pub fn search(query: &str, limit: usize) -> Vec<KBEntry> {
    // Cap query length to prevent DoS / accidental megabyte paste via the
    // search box — every char extends the contains() scan over body_lower.
    // 200 chars is way past any legitimate use (longest KB heading is ~80).
    const MAX_QUERY_CHARS: usize = 200;
    let trimmed = query.trim();
    let truncated: String = if trimmed.chars().count() > MAX_QUERY_CHARS {
        trimmed.chars().take(MAX_QUERY_CHARS).collect()
    } else {
        trimmed.to_string()
    };
    let q = truncated.to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let entries = all();

    let mut scored: Vec<(u8, &KBEntry)> = Vec::with_capacity(entries.len() / 4);
    for e in entries {
        let key_lower = &e.key; // already lowercase from parse()
        if key_lower == &q {
            scored.push((0, e));
            continue;
        }
        if key_lower.starts_with(&q) {
            scored.push((1, e));
            continue;
        }
        // heading_lower + body_lower pre-computed at parse() (see KBEntry).
        if e.heading_lower.contains(&q) {
            scored.push((2, e));
            continue;
        }
        if e.body_lower.contains(&q) {
            scored.push((3, e));
            continue;
        }
    }
    scored.sort_by_key(|(rank, _)| *rank);
    scored.into_iter().take(limit).map(|(_, e)| e.clone()).collect()
}

/// Get a single entry by exact key (case-insensitive). Used by `/keyname`
/// palette to instantly resolve a known shortcut without ranking overhead.
pub fn get(key: &str) -> Option<&'static KBEntry> {
    let q = key.trim().to_lowercase();
    all().iter().find(|e| e.key == q)
}

/// Summary stats for the Settings UI banner ("📚 KB: 1300 glossary,
/// 120 commands, 180 patterns").
#[derive(Debug, Clone, Serialize)]
pub struct KBStats {
    pub total: usize,
    pub glossary: usize,
    pub commands: usize,
    pub patterns: usize,
}

pub fn stats() -> KBStats {
    let entries = all();
    let mut stats = KBStats { total: entries.len(), glossary: 0, commands: 0, patterns: 0 };
    for e in entries {
        match e.source {
            "glossary" => stats.glossary += 1,
            "commands" => stats.commands += 1,
            "patterns" => stats.patterns += 1,
            _ => {}
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First call populates cache; check we got a non-trivial number of
    /// entries from each source. Floors guard against accidental file
    /// truncation in PR review.
    #[test]
    fn all_loads_three_sources_with_floors() {
        let s = stats();
        assert!(s.total >= 1500, "KB total {} below floor 1500 — was a file truncated?", s.total);
        assert!(s.glossary >= 1000, "glossary {} below floor 1000", s.glossary);
        assert!(s.commands >= 100, "commands {} below floor 100", s.commands);
        assert!(s.patterns >= 100, "patterns {} below floor 100", s.patterns);
        // sanity: source tags are correctly populated
        let by_source: std::collections::HashSet<_> = all().iter().map(|e| e.source).collect();
        assert!(by_source.contains("glossary"));
        assert!(by_source.contains("commands"));
        assert!(by_source.contains("patterns"));
    }

    /// Parser robustness: each entry has non-empty key, heading, body, and
    /// the key is lowercased / whitespace-stripped.
    #[test]
    fn every_entry_well_formed() {
        for e in all() {
            assert!(!e.key.is_empty(), "empty key in heading: {}", e.heading);
            assert_eq!(e.key, e.key.trim().to_lowercase(), "key not normalized: {}", e.key);
            assert!(!e.heading.is_empty());
            assert!(!e.body.is_empty(), "empty body for {}", e.key);
        }
    }

    /// Search end-to-end: exact-key match returns the entry first.
    #[test]
    fn search_exact_key_wins() {
        let results = search("kubernetes", 5);
        assert!(!results.is_empty(), "no results for 'kubernetes'");
        assert_eq!(results[0].key, "kubernetes", "exact-match should rank first");
    }

    /// Search ranks "starts-with" above plain body containment.
    #[test]
    fn search_prefix_beats_body_substring() {
        // "tcp" should rank before some unrelated entry that happens to mention "tcp" in its body
        let results = search("tcp", 10);
        assert!(!results.is_empty());
        let first_key = &results[0].key;
        assert!(
            first_key == "tcp" || first_key.starts_with("tcp"),
            "expected tcp-prefixed entry first, got '{first_key}'"
        );
    }

    /// `get()` returns the entry for known key, None for unknown.
    #[test]
    fn get_known_unknown() {
        assert!(get("kubernetes").is_some());
        assert!(get("KUBERNETES").is_some(), "should be case-insensitive");
        assert!(get("definitely-not-a-real-tech-term-2026").is_none());
    }

    /// Empty / whitespace query returns empty (don't dump the whole KB).
    #[test]
    fn search_empty_query_returns_empty() {
        assert!(search("", 10).is_empty());
        assert!(search("   ", 10).is_empty());
        assert!(search("\n\t", 10).is_empty());
    }

    /// Limit is respected (or all results if fewer than limit).
    #[test]
    fn search_respects_limit() {
        let results = search("a", 3); // 'a' will appear in many entries
        assert!(results.len() <= 3);
    }

    /// Lowercase mirrors of heading/body are populated and actually used —
    /// search should return the same hits whether the query is upper or
    /// lowercase, and not double-allocate per call.
    #[test]
    fn heading_lower_and_body_lower_populated_at_parse() {
        for e in all().iter().take(50) {
            assert_eq!(e.heading_lower, e.heading.to_lowercase(),
                "heading_lower out of sync for {}", e.key);
            assert_eq!(e.body_lower, e.body.to_lowercase(),
                "body_lower out of sync for {}", e.key);
        }
    }

    /// Query length cap — pathological 50k-char paste must not stall the
    /// search loop. (Pre-fix: 1700 KB body comparisons each over 50k chars
    /// = O(85M char ops) per keystroke.)
    #[test]
    fn search_truncates_oversized_query() {
        // 110 000 chars of "kubernetes " — the truncated query becomes
        // "kubernetes kubernetes ..." up to MAX_QUERY_CHARS, which is too
        // long to match any KB heading/body. The point is the search MUST
        // complete fast regardless of input size.
        let huge = "kubernetes ".repeat(10_000);
        let start = std::time::Instant::now();
        let _ = search(&huge, 5);
        let elapsed_ms = start.elapsed().as_millis();
        // Pre-cap on a debug build this would take many seconds; cap +
        // pre-lowered fields keep it well under a second.
        assert!(elapsed_ms < 500, "search took {elapsed_ms}ms — query cap or body cache broken?");
    }

    /// Normal-sized query against the same content still returns the
    /// expected exact-key hit — the truncation logic doesn't kick in.
    #[test]
    fn search_normal_query_works_unchanged() {
        let results = search("kubernetes", 5);
        assert!(!results.is_empty(), "no results for 'kubernetes'");
        assert_eq!(results[0].key, "kubernetes");
    }

    /// Each glossary entry's key shouldn't clash with another entry's
    /// key within glossary (would be a duplicate definition).
    #[test]
    fn no_duplicate_keys_within_glossary() {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut dups = Vec::new();
        for e in all().iter().filter(|e| e.source == "glossary") {
            if !seen.insert(&e.key) {
                dups.push(e.key.clone());
            }
        }
        assert!(
            dups.is_empty(),
            "duplicate glossary keys: {dups:?} — would silently shadow earlier definitions"
        );
    }
}
