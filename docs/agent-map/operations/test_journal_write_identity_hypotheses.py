"""Temporary JSONL write/identity fixtures. Not owner journals or Rust I/O faults."""
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class JournalWriteIdentityFixtures(unittest.TestCase):
    def test_writer_continues_after_line_error_and_keeps_first_error(self):
        text = source("overlay-backend/src/journal/writer.rs")
        body = text[text.index("pub(crate) fn spawn_writer"):]
        self.assertIn("note_write_error(&mut first_error, e)", body)
        self.assertIn("journal write failed (continuing)", text)
        note = text[text.index("pub(crate) fn note_write_error"):text.index("pub(crate) fn finish_writer")]
        self.assertIn("if first_error.is_none()", note)
        finish = text[text.index("pub(crate) fn finish_writer"):text.index("pub(crate) fn spawn_writer")]
        self.assertLess(finish.index("Some(e) => Err"), finish.index("None => flush_result"))

    def test_partial_middle_line_is_skipped_and_later_json_survives(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.jsonl"
            path.write_text(
                '{"kind":"session_start","unix_ms":1}\n'
                '{"kind":"transcript","text":"torn\n'
                '{"kind":"session_stop","unix_ms":2}\n',
                encoding="utf-8",
            )
            kinds = []
            for line in path.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if not line:
                    continue
                try:
                    kinds.append(json.loads(line)["kind"])
                except json.JSONDecodeError:
                    kinds.append("SKIP")
        self.assertEqual(kinds, ["session_start", "SKIP", "session_stop"])

    def test_open_uses_append_not_exclusive_create(self):
        text = source("overlay-backend/src/journal/writer.rs")
        body = text[text.index("let file = OpenOptions::new()"):text.index("log::info!(\"journal opened")]
        self.assertIn(".create(true)", body)
        self.assertIn(".append(true)", body)
        self.assertNotIn(".create_new(true)", body)
        self.assertNotIn("O_EXCL", body)

    def test_same_second_suffix_collides_and_append_interleaves_sessions(self):
        stamp = "2026-08-01_00-00-00"
        millis = 0xABCDEF
        names = {f"{stamp}_{(millis & 0xFFFFFF):06x}.jsonl" for _ in range(2)}
        self.assertEqual(len(names), 1)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / next(iter(names))
            path.write_text("", encoding="utf-8")
            with path.open("a", encoding="utf-8") as first, path.open("a", encoding="utf-8") as second:
                first.write('{"kind":"session_start","session":"A"}\n')
                second.write('{"kind":"session_start","session":"B"}\n')
                first.write('{"kind":"session_stop","session":"A"}\n')
                second.write('{"kind":"session_stop","session":"B"}\n')
            sessions = [json.loads(line)["session"] for line in path.read_text(encoding="utf-8").splitlines()]
        self.assertCountEqual(sessions, ["A", "A", "B", "B"])
        self.assertEqual(len(sessions), 4)

    def test_suffix_uses_only_low_24_bits_of_same_clock(self):
        text = source("overlay-backend/src/journal/writer.rs")
        body = text[text.index("let stamp = chrono_like_stamp()"):text.index("let file = OpenOptions")]
        self.assertIn("now_unix_ms() & 0xFFFFFF", body)
        self.assertIn("{stamp}_{rand:06x}.jsonl", body)
        self.assertEqual(f"{0x01000001 & 0xFFFFFF:06x}", f"{0x00000001 & 0xFFFFFF:06x}")

    def test_original_c05_c06_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C05"], "hypothesis")
        self.assertEqual(found["wave1_worker1_persistence-C06"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
