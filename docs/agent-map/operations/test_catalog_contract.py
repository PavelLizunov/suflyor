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

    def test_legacy_approve_projection_does_not_transfer_v2_candidate_fields(self):
        # Execute SQL matching approve_candidate's current selected/inserted columns.
        self.conn.execute("INSERT INTO memory_candidates(profile_id,kind,text,status,created_at_ms,source_text,entity,norm_status) VALUES ('default','fact','normalized fixture','pending',1,'verbatim fixture','subject','llm')")
        profile, kind, text, source = self.conn.execute("SELECT profile_id,kind,text,source_session_id FROM memory_candidates WHERE id=1 AND status='pending'").fetchone()
        self.conn.execute("UPDATE memory_candidates SET status='approved' WHERE id=1")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,source_session_id,approved_at_ms,embedding_status) VALUES (?,?,?,?,2,'none')", (profile, kind, text, source))
        self.assertEqual(self.conn.execute("SELECT source_text,entity,norm_status FROM memory_items").fetchone(), (None, None, "none"))
        self.assertEqual(self.conn.execute("SELECT source_text,entity,norm_status,status FROM memory_candidates").fetchone(), ("verbatim fixture", "subject", "llm", "approved"))

    def test_manual_item_restore_returns_verbatim_provenance_once(self):
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,approved_at_ms,source_text,entity,norm_status) VALUES ('default','note','rewritten fixture',1,'verbatim fixture','subject','llm')")
        self.conn.execute("UPDATE memory_items SET text=source_text,entity=NULL,norm_status='none',source_text=NULL WHERE id=1 AND source_text IS NOT NULL")
        self.assertEqual(self.conn.execute("SELECT text,source_text,entity,norm_status FROM memory_items").fetchone(), ("verbatim fixture", None, None, "none"))
        before = self.conn.total_changes
        self.conn.execute("UPDATE memory_items SET text=source_text,entity=NULL,norm_status='none',source_text=NULL WHERE id=1 AND source_text IS NOT NULL")
        self.assertEqual(self.conn.total_changes, before)

    def test_active_memory_query_excludes_pending_candidates_and_archived_items(self):
        self.conn.execute("INSERT INTO memory_candidates(profile_id,kind,text,created_at_ms) VALUES ('default','fact','pending fixture',1)")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,approved_at_ms) VALUES ('default','note','active fixture',1)")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,approved_at_ms,archived_at_ms) VALUES ('default','note','archived fixture',2,3)")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,approved_at_ms) VALUES ('other','note','other profile',4)")
        active = self.conn.execute("SELECT text FROM memory_items WHERE profile_id='default' AND archived_at_ms IS NULL ORDER BY approved_at_ms DESC LIMIT -1").fetchall()
        self.assertEqual(active, [("active fixture",)])
        self.assertEqual(self.conn.execute("SELECT status FROM memory_candidates").fetchone()[0], "pending")


if __name__ == "__main__":
    unittest.main()
