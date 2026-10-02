"""Temporary retention/UTF-8 fixtures for C03/C17. Not owner journals."""
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def prune(directory, keep, max_bytes):
    entries = []
    for path in Path(directory).iterdir():
        if path.suffix != ".jsonl":
            continue
        stat = path.stat()
        entries.append((stat.st_mtime_ns, stat.st_size, path))
    entries.sort(key=lambda item: item[0], reverse=True)
    deleted = []
    for _, _, path in entries[keep:]:
        path.unlink()
        deleted.append(path.name)
    remaining = entries[: len(entries) - len(deleted)]
    total = sum(size for _, size, _ in remaining)
    if max_bytes > 0 and total > max_bytes:
        for _, size, path in reversed(remaining):
            if total <= max_bytes:
                break
            path.unlink()
            total -= size
            deleted.append(path.name)
    return deleted


class JournalRetentionFixtures(unittest.TestCase):
    def test_count_cap_keeps_newest_then_byte_cap_can_remove_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "notes.txt").write_text("keep", encoding="utf-8")
            for name, payload in (("old.jsonl", "a"), ("new.jsonl", "bbbb")):
                (root / name).write_text(payload, encoding="utf-8")
            old, new = root / "old.jsonl", root / "new.jsonl"
            old_time = new.stat().st_mtime_ns - 1_000_000
            import os
            os.utime(old, ns=(old_time, old_time))
            deleted = prune(root, 1, 1)
            self.assertEqual(deleted, ["old.jsonl", "new.jsonl"])
            self.assertTrue((root / "notes.txt").exists())

    def test_zero_byte_cap_disables_second_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "kept.jsonl").write_text("abcdef", encoding="utf-8")
            self.assertEqual(prune(root, 1, 0), [])
            self.assertTrue((root / "kept.jsonl").exists())

    def test_invalid_utf8_fails_whole_file_before_line_parser(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad.jsonl"
            path.write_bytes(b'{"kind":"session_start"}\n\xff\n{"kind":"session_stop"}\n')
            with self.assertRaises(UnicodeDecodeError):
                path.read_text(encoding="utf-8")
            text = path.read_bytes().splitlines()[0].decode()
            self.assertEqual(json.loads(text)["kind"], "session_start")

    def test_source_reads_whole_file_and_skips_metadata_errors(self):
        indexer = source("overlay-backend/src/persistence/indexer.rs")
        body = indexer[indexer.index("pub fn index_journal_file"):indexer.index("for line in content.lines()")]
        self.assertIn("std::fs::read_to_string(path)", body)
        retention = source("overlay-backend/src/journal/retention.rs")
        self.assertIn("let Ok(meta) = e.metadata() else { continue };", retention)
        self.assertIn("entries.sort_by_key(|e| std::cmp::Reverse(e.0));", retention)
        self.assertIn("if max_bytes > 0", retention)

    def test_retention_constants_and_non_jsonl_guard(self):
        text = source("overlay-backend/src/journal/retention.rs")
        self.assertIn("KEEP_LAST_SESSIONS: usize = 100", text)
        self.assertIn("MAX_TOTAL_BYTES: u64 = 500 * 1024 * 1024", text)
        self.assertIn('!= Some("jsonl")', text)

    def test_original_c03_confirmed_and_c17_hypothesis_unchanged(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C03"], "confirmed")
        self.assertEqual(found["wave1_worker1_persistence-C17"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
