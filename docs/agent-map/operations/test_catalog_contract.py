"""Research SQL fixtures against shipped migrations, not native rusqlite acceptance."""
import sqlite3
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


class CatalogContractTests(unittest.TestCase):
    def setUp(self):
        self.conn = sqlite3.connect(":memory:")
        self.addCleanup(self.conn.close)
        self.conn.execute("PRAGMA foreign_keys=ON")
        for path in sorted((REPO / "overlay-backend/migrations").glob("*.sql")):
            self.conn.executescript(path.read_text(encoding="utf-8"))
        self.conn.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES ('s1','fixture',1,'completed',1)")

    def test_fts_delete_by_unindexed_session_id_is_valid(self):
        self.conn.execute("INSERT INTO utterances(session_id,unix_ms,source,text) VALUES ('s1',1,'system','alpha text')")
        self.assertEqual(self.conn.execute("SELECT count(*) FROM search_index WHERE session_id='s1'").fetchone()[0], 1)
        self.conn.execute("DELETE FROM search_index WHERE session_id='s1'")
        self.assertEqual(self.conn.execute("SELECT count(*) FROM search_index WHERE session_id='s1'").fetchone()[0], 0)

    def test_session_replacement_does_not_cascade_curated_data(self):
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,source_session_id,approved_at_ms) VALUES ('default','fact','approved fixture','s1',1)")
        self.conn.execute("INSERT INTO memory_candidates(source_session_id,profile_id,kind,text,created_at_ms) VALUES ('s1','default','fact','pending fixture',1)")
        self.conn.execute("INSERT INTO diarization(session_id,created_at_ms,model_id,num_speakers,segments_json,speaker_names_json) VALUES ('s1',1,'fixture',1,'[]','{}')")
        self.conn.execute("DELETE FROM sessions WHERE id='s1'")
        self.conn.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES ('s1','fixture2',1,'completed',2)")
        for table in ("memory_items", "memory_candidates", "diarization"):
            self.assertEqual(self.conn.execute("SELECT count(*) FROM " + table).fetchone()[0], 1)
            self.assertEqual(self.conn.execute("PRAGMA foreign_key_list(" + table + ")").fetchall(), [])

    def test_session_children_have_foreign_key_cascade(self):
        self.conn.execute("INSERT INTO utterances(session_id,unix_ms,source,text) VALUES ('s1',1,'system','alpha text')")
        self.conn.execute("INSERT INTO ai_turns(session_id,unix_ms,purpose,model,question,answer) VALUES ('s1',1,'fixture','fixture','q','a')")
        self.conn.execute("DELETE FROM sessions WHERE id='s1'")
        self.assertEqual(self.conn.execute("SELECT count(*) FROM utterances").fetchone()[0], 0)
        self.assertEqual(self.conn.execute("SELECT count(*) FROM ai_turns").fetchone()[0], 0)
        # FTS requires explicit deletion by Store::replace_session/delete_session.
        self.assertGreater(self.conn.execute("SELECT count(*) FROM search_index").fetchone()[0], 0)

    def test_replace_diarization_row_replaces_manual_names(self):
        self.conn.execute("INSERT INTO diarization(session_id,created_at_ms,model_id,num_speakers,segments_json,speaker_names_json) VALUES ('s1',1,'fixture',1,'[]',?)", ('{"0":"renamed fixture"}',))
        self.conn.execute("INSERT OR REPLACE INTO diarization(session_id,created_at_ms,model_id,num_speakers,segments_json,speaker_names_json) VALUES ('s1',2,'rerun-fixture',1,'[]','{}')")
        self.assertEqual(self.conn.execute("SELECT speaker_names_json FROM diarization WHERE session_id='s1'").fetchone()[0], "{}")


if __name__ == "__main__":
    unittest.main()
