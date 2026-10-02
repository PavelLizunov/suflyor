"""Source fixtures for wave1_worker1_persistence C04 and wave3_worker3_tile C02 (confirmed mechanisms).
No live file writing or UI streaming.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class PersistenceSyncMarkdownParseFixtures(unittest.TestCase):
    def test_finish_writer_calls_flush_without_sync_all(self):
        text = source("overlay-backend/src/journal/writer.rs")
        body = text[text.index("pub(crate) fn finish_writer("):text.index("pub(crate) fn spawn_writer(")]
        # finish_writer invokes file.flush() only
        self.assertIn("let flush_result = file\n        .flush()", body)
        self.assertNotIn("sync_all", body)
        self.assertNotIn("sync_data", body)

    def test_spawn_writer_loop_flushes_buffer_without_fsync(self):
        text = source("overlay-backend/src/journal/writer.rs")
        body = text[text.index("pub(crate) fn spawn_writer("):text.index("pub(crate) fn bump_counters(")]
        # Periodic and command flush is file.flush() without sync_all
        self.assertIn("file.flush()", body)
        self.assertNotIn("sync_all", body)
        self.assertNotIn("sync_data", body)

    def test_markdown_parse_streaming_runs_full_parse_on_prefix(self):
        text = source("slint-experiment/src/markdown.rs")
        body = text[text.index("pub fn parse_streaming("):text.index("fn stable_streaming_prefix(")]
        # Executes parse on entire stable_streaming_prefix on every call
        self.assertIn("parse(stable_streaming_prefix(source))", body)

    def test_markdown_parse_single_pass_instantiates_full_parser(self):
        text = source("slint-experiment/src/markdown.rs")
        body = text[text.index("fn parse_single_pass("):text.index("fn flush(")]
        # Uses Parser::new_ext on entire source string
        self.assertIn("Parser::new_ext(source, options)", body)
        # Unbounded collection vectors without caps on block or table row counts
        self.assertIn("let mut out: Vec<Block> = Vec::new();", body)
        self.assertIn("let mut table_rows: Vec<Vec<String>> = Vec::new();", body)

    def test_nested_list_indentation_repeats_by_depth(self):
        text = source("slint-experiment/src/markdown.rs")
        body = text[text.index("Event::Start(Tag::Item) => {"):text.index("Event::End(TagEnd::Item) => {")]
        # Indents by list_depth without an explicit depth ceiling
        self.assertIn('"  ".repeat(list_depth - 1)', body)

    def test_original_c04_c02_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C04"], "confirmed")
        self.assertEqual(found["wave3_worker3_tile-C02"], "confirmed")


if __name__ == "__main__":
    unittest.main()
