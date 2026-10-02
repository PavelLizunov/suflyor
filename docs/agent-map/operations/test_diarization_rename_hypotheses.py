"""Temporary SQLite model for persistence C13. No owner catalog or Rust race."""
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class DiarizationRenameFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name) / "catalog.sqlite"
        self.a = sqlite3.connect(self.path)
        self.b = sqlite3.connect(self.path)
        self.addCleanup(self.a.close)
        self.addCleanup(self.b.close)
        for path in sorted((ROOT / "overlay-backend/migrations").glob("*.sql")):
            self.a.executescript(path.read_text())
        self.a.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES('s','s.jsonl',1,'completed',1)")
        self.a.execute("INSERT INTO diarization(session_id,created_at_ms,num_speakers,model_id,segments_json,speaker_names_json) VALUES('s',1,2,'model','[]','{}')")
        self.a.commit()

    def read_names(self, conn):
        raw = conn.execute("SELECT speaker_names_json FROM diarization WHERE session_id='s'").fetchone()[0]
        return json.loads(raw)

    def replace_names(self, conn, names):
        conn.execute("UPDATE diarization SET speaker_names_json=? WHERE session_id='s'", (json.dumps(names),))
        conn.commit()

    def test_stale_read_then_replace_drops_other_name(self):
        first = self.read_names(self.a)
        second = self.read_names(self.b)
        first["1"] = "Alice"
        second["2"] = "Bob"
        self.replace_names(self.a, first)
        self.replace_names(self.b, second)
        self.assertEqual(self.read_names(self.a), {"2": "Bob"})

    def test_blank_name_removes_only_selected_speaker(self):
        self.replace_names(self.a, {"1": "Alice", "2": "Bob"})
        names = self.read_names(self.a)
        names.pop("1", None)
        self.replace_names(self.a, names)
        self.assertEqual(self.read_names(self.a), {"2": "Bob"})

    def test_missing_row_is_noop(self):
        self.a.execute("DELETE FROM diarization WHERE session_id='s'")
        self.a.commit()
        self.assertIsNone(self.a.execute("SELECT session_id FROM diarization WHERE session_id='s'").fetchone())

    def test_source_reads_then_replaces_without_transaction(self):
        text = source("overlay-backend/src/persistence/sqlite_store.rs")
        body = text[text.index("pub fn rename_speaker"):text.index("pub fn backfill_session_models")]
        self.assertLess(body.index("self.get_diarization"), body.index("self.put_diarization"))
        self.assertNotIn("transaction", body)
        self.assertNotIn("BEGIN", body)
        put = text[text.index("pub fn put_diarization"):text.index("pub fn rename_speaker")]
        self.assertIn("INSERT OR REPLACE", put)

    def test_replace_writes_entire_names_blob(self):
        put = source("overlay-backend/src/persistence/sqlite_store.rs")
        self.assertIn("speaker_names_json", put)
        self.assertNotIn("json_set", put)

    def test_original_c13_status_remains_hypothesis(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C13"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
