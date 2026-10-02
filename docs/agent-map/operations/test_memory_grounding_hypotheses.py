"""Pure grounding/load models for memory C05/C08. No owner memory or prompts."""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def words_match(a, b):
    need = min(len(a), len(b), 4)
    return need > 0 and sum(x == y for x, y in zip(a, b)) >= need


def grounded_in_order(fact, source_words):
    start = 0
    for word in fact:
        found = next((index for index, item in enumerate(source_words[start:]) if words_match(word, item)), None)
        if found is None:
            return False
        start += found + 1
    return True


def normalize_question(text):
    return " ".join(text.split()).lower()


class MemoryGroundingFixtures(unittest.TestCase):
    def test_ordered_omission_still_grounds(self):
        source_words = ["клиент", "платит", "подрядчику", "сегодня"]
        self.assertTrue(grounded_in_order(["клиент", "подрядчику"], source_words))
        self.assertFalse(grounded_in_order(["подрядчику", "клиент"], source_words))

    def test_prompt_clip_drops_tail_without_semantic_check(self):
        text = "a" * 239 + " SECRET"
        clipped = "".join(list(text)[:240])
        self.assertNotIn("SECRET", clipped)
        self.assertEqual(len(clipped), 240)

    def test_question_dedup_is_whitespace_case_only(self):
        self.assertEqual(normalize_question("What   is Kubernetes?"), normalize_question("what is kubernetes?"))
        self.assertNotEqual(normalize_question("restart service"), normalize_question("service restart"))

    def test_source_loads_all_and_does_not_dedup_formatted_facts(self):
        context = source("overlay-backend/src/memory/context_builder.rs")
        self.assertIn('list_memory_items("default", false, -1)', context)
        rank = context[context.index("fn rank_by_relevance"):context.index("pub fn context_for_meeting")]
        self.assertIn(".clone()", rank)
        formatter = context[context.index("pub fn format_memory_block"):context.index("pub fn merge_context")]
        self.assertNotIn("dedup", formatter)

    def test_grounding_advances_after_each_match(self):
        text = source("overlay-backend/src/memory/normalize.rs")
        body = text[text.index("fn grounded_in_order"):text.index("fn content_words")]
        self.assertIn("from += off + 1", body)
        self.assertNotIn("fact_words.len() == src_words.len()", body)

    def test_original_c05_c08_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker2_memory-C05"], "hypothesis")
        self.assertEqual(found["wave1_worker2_memory-C08"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
