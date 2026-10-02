"""Source fixtures for wave1_worker2_memory C03 and C06 (confirmed mechanisms).
No AI prompt execution, no memory DB access.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class MemoryNegationRecencyFixtures(unittest.TestCase):
    def test_negations_list_contains_only_cyrillic_particles(self):
        text = source("overlay-backend/src/memory/normalize.rs")
        body = text[text.index("const NEGATIONS: &[&str] = &["):text.index("fn common_prefix_len")]
        # Contains Russian negation particles
        for word in ("не", "нет", "ни", "нельзя", "без", "никак", "никогда", "ничего", "никто", "никакой"):
            self.assertIn(f'"{word}"', body)
        # Does NOT contain English negation words like not, never, no, without, cannot
        for en_word in ("not", "never", "without", "cannot"):
            self.assertNotIn(f'"{en_word}"', body)

    def test_validate_rewrite_checks_negation_count_only(self):
        text = source("overlay-backend/src/memory/normalize.rs")
        body = text[text.index("pub fn validate_rewrite("):text.index("pub struct NormalizedFact")]
        # Only enforces negation_words(f).len() != negation_words(span).len()
        self.assertIn("if negation_words(f).len() != negation_words(span).len() {", body)
        self.assertNotIn("polarity", body)

    def test_query_terms_filter_threshold_drops_short_names(self):
        text = source("overlay-backend/src/memory/context_builder.rs")
        body = text[text.index("fn query_terms(query: &str) -> Vec<String> {"):text.index("fn score_item(")]
        # Filter requires char count >= 4 and not in STOPWORDS
        self.assertIn("t.chars().count() >= 4", body)
        self.assertIn("!STOPWORDS.contains(&t.as_str())", body)

    def test_rank_by_relevance_falls_back_to_none_when_no_match(self):
        text = source("overlay-backend/src/memory/context_builder.rs")
        body = text[text.index("fn rank_by_relevance("):text.index("pub fn context_for_meeting(")]
        # When terms are empty or scored is empty, returns None
        self.assertIn("if terms.is_empty() {\n        return None;\n    }", body)
        self.assertIn("if scored.is_empty() {\n        return None;\n    }", body)

    def test_context_for_meeting_unconditional_recency_fallback(self):
        text = source("overlay-backend/src/memory/context_builder.rs")
        body = text[text.index("pub fn context_for_meeting("):text.index("#[cfg(test)]")]
        # query.and_then(|q| rank_by_relevance(q, &items)) -> None => format_memory_block(&items)
        self.assertIn("match query.and_then(|q| rank_by_relevance(q, &items)) {", body)
        self.assertIn("Some(relevant) => format_memory_block(&relevant),", body)
        self.assertIn("None => format_memory_block(&items),", body)

    def test_original_c03_c06_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker2_memory-C03"], "confirmed")
        self.assertEqual(found["wave1_worker2_memory-C06"], "confirmed")


if __name__ == "__main__":
    unittest.main()
