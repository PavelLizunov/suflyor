"""Pure string models for memory C04/C07. No owner memory or prompts."""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def common_prefix(a, b):
    return sum(x == y for x, y in zip(a, b))


def words_match(a, b):
    need = min(len(a), len(b), 4)
    return need > 0 and common_prefix(a, b) >= need


def term_in_tokens(term, tokens):
    if len(term) >= 5:
        stem = term[:-1]
        return any(token.startswith(stem) for token in tokens)
    return term in tokens


def key_terms(text):
    terms = []
    for match in re.finditer(r"\w+", text):
        token = match.group(0)
        if token.isascii() and token.isalnum() and re.search(r"[А-Яа-я]", text):
            terms.append(token.lower())
    return terms


class MemoryMatchingFixtures(unittest.TestCase):
    def test_four_character_prefix_matches_antonym_pair(self):
        self.assertTrue(words_match("проверили", "провалили"))
        self.assertGreaterEqual(common_prefix("проверили", "провалили"), 4)
        self.assertFalse(words_match("код", "кот"))

    def test_shorter_word_requires_full_prefix(self):
        self.assertTrue(words_match("код", "кода"))
        self.assertFalse(words_match("код", "кот"))
        self.assertTrue(words_match("влад", "владислав"))

    def test_five_character_term_uses_dropped_final_stem(self):
        self.assertTrue(term_in_tokens("альфа", {"альфе", "другой"}))
        self.assertFalse(term_in_tokens("crm", {"acrm"}))
        self.assertTrue(term_in_tokens("crm", {"crm"}))

    def test_latin_token_has_no_minimum_length(self):
        self.assertEqual(key_terms("факт in системе"), ["in"])
        self.assertEqual(key_terms("только кириллица"), [])

    def test_source_contains_exact_match_boundaries(self):
        normalize = source("overlay-backend/src/memory/normalize.rs")
        body = normalize[normalize.index("pub(super) fn words_match"):normalize.index("fn grounded_in_order")]
        self.assertIn(".min(4)", body)
        summary = source("overlay-backend/src/memory/summary_ref.rs")
        stem = summary[summary.index("fn term_in_tokens"):summary.index("pub fn relevant_items")]
        self.assertIn("if n >= 5", stem)
        self.assertIn("term.chars().take(n - 1)", stem)
        keys = summary[summary.index("let latin_in_cyrillic"):summary.index("let first_word_definition")]
        self.assertNotIn("chars().count()", keys)

    def test_original_c04_c07_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker2_memory-C04"], "hypothesis")
        self.assertEqual(found["wave1_worker2_memory-C07"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
