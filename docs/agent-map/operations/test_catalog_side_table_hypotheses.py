"""Temporary side-table fixtures for confirmed C10/C12. Not owner catalogs."""
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class CatalogSideTableFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.conn = sqlite3.connect(Path(self.tmp.name) / "catalog.sqlite")
        self.addCleanup(self.conn.close)
        self.conn.execute("PRAGMA foreign_keys=ON")
        for path in sorted((ROOT / "overlay-backend/migrations").glob("*.sql")):
            self.conn.executescript(path.read_text())
        self.conn.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES('s','s.jsonl',1,'completed',1)")
        self.conn.execute("INSERT INTO utterances(session_id,unix_ms,source,text) VALUES('s',1,'mic','hello')")
        self.conn.execute("INSERT INTO diarization(session_id,created_at_ms,num_speakers,model_id,segments_json,speaker_names_json) VALUES('s',1,1,'model','[]','{}')")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,approved_at_ms) VALUES('default','note','owned',1)")

    def replace_projection(self):
        self.conn.execute("DELETE FROM search_index WHERE session_id='s'")
        self.conn.execute("DELETE FROM sessions WHERE id='s'")
        self.conn.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES('s','s.jsonl',1,'completed',2)")

    def test_replace_shape_keeps_diarization_and_memory_but_cascades_utterances(self):
        self.replace_projection()
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM utterances").fetchone()[0], 0)
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM diarization").fetchone()[0], 1)
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM memory_items").fetchone()[0], 1)

    def test_hard_delete_removes_diarization_but_not_memory(self):
        self.conn.execute("DELETE FROM search_index WHERE session_id='s'")
        self.conn.execute("DELETE FROM diarization WHERE session_id='s'")
        self.conn.execute("DELETE FROM sessions WHERE id='s'")
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM sessions").fetchone()[0], 0)
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM diarization").fetchone()[0], 0)
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM memory_items").fetchone()[0], 1)

    def test_invalid_diarization_json_is_error_not_missing_row(self):
        self.conn.execute("UPDATE diarization SET segments_json='{' WHERE session_id='s'")
        with self.assertRaises(json.JSONDecodeError):
            json.loads(self.conn.execute("SELECT segments_json FROM diarization WHERE session_id='s'").fetchone()[0])
        self.assertIsNotNone(self.conn.execute("SELECT session_id FROM diarization WHERE session_id='s'").fetchone())

    def test_replace_source_does_not_clear_side_tables_delete_clears_diarization(self):
        text = source("overlay-backend/src/persistence/sqlite_store.rs")
        replace = text[text.index("pub fn replace_session"):text.index("pub fn delete_session")]
        delete = text[text.index("pub fn delete_session"):text.index("pub fn list_sessions")]
        self.assertNotIn("DELETE FROM diarization", replace)
        self.assertNotIn("DELETE FROM memory_items", replace)
        self.assertIn("DELETE FROM diarization WHERE session_id = ?1", delete)
        self.assertNotIn("DELETE FROM memory_items", delete)

    def test_indexer_projects_only_four_journal_kinds(self):
        text = source("overlay-backend/src/persistence/indexer.rs")
        body = text[text.index("match v.get(\"kind\")"):text.index("if !has_event")]
        for kind in ("session_start", "session_stop", "transcript_line", "ai_response"):
            self.assertIn(f'"{kind}"', body)
        for kind in ("detector_decision", "tile_spawn", "rate_limited", "error", "diarization", "memory"):
            self.assertNotIn(f'"{kind}"', body)
        types = source("overlay-backend/src/journal/types.rs")
        for kind in ("SessionSummary", "DetectorDecision", "TileSpawn", "RateLimited", "Error"):
            self.assertIn(kind, types)

    def test_doc_contract_disagrees_with_error_return_and_statuses_stay_confirmed(self):
        text = source("overlay-backend/src/persistence/sqlite_store.rs")
        body = text[text.index("pub fn get_diarization") - 500:text.index("pub fn put_diarization")]
        self.assertIn("or the row is unreadable JSON", body)
        self.assertIn('.context("parse segments_json")?', body)
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C10"], "confirmed")
        self.assertEqual(found["wave1_worker1_persistence-C12"], "confirmed")


if __name__ == "__main__":
    unittest.main()
