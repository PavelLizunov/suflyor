"""Temporary schema fixtures for C14/C15. Not owner catalogs or Rust migrations."""
import sqlite3
import tempfile
import unittest
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class MemoryMigrationFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.conn = sqlite3.connect(Path(self.tmp.name) / "catalog.sqlite")
        self.addCleanup(self.conn.close)
        self.conn.execute("PRAGMA foreign_keys=ON")
        for path in sorted((ROOT / "overlay-backend/migrations").glob("*.sql")):
            self.conn.executescript(path.read_text())
        self.conn.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES('s','s.jsonl',1,'completed',1)")
        self.conn.execute("INSERT INTO memory_candidates(profile_id,kind,text,created_at_ms) VALUES('default','note','owned',1)")

    def test_status_accepts_non_contract_value_and_duplicate_approve_mints_two_items(self):
        self.conn.execute("UPDATE memory_candidates SET status='banana' WHERE id=1")
        self.assertEqual(self.conn.execute("SELECT status FROM memory_candidates").fetchone()[0], "banana")
        self.conn.execute("UPDATE memory_candidates SET status='pending'")
        row = tuple(self.conn.execute("SELECT profile_id,kind,text,source_session_id FROM memory_candidates WHERE id=1 AND status='pending'").fetchone())
        for _ in range(2):
            self.conn.execute("UPDATE memory_candidates SET status='approved' WHERE id=1")
            self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,source_session_id,approved_at_ms) VALUES(?,?,?,?,1)", row)
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM memory_items").fetchone()[0], 2)

    def test_non_pending_guard_blocks_second_item_when_status_checked(self):
        row = self.conn.execute("SELECT profile_id,kind,text,source_session_id FROM memory_candidates WHERE status='pending'").fetchone()
        self.conn.execute("UPDATE memory_candidates SET status='approved'")
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,source_session_id,approved_at_ms) VALUES(?,?,?,?,1)", tuple(row))
        self.assertIsNone(self.conn.execute("SELECT id FROM memory_candidates WHERE status='pending'").fetchone())
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM memory_items").fetchone()[0], 1)

    def test_memory_has_no_session_fk_and_survives_session_delete(self):
        self.conn.execute("INSERT INTO memory_items(profile_id,kind,text,source_session_id,approved_at_ms) VALUES('default','note','kept','s',1)")
        self.conn.execute("DELETE FROM sessions WHERE id='s'")
        self.assertEqual(self.conn.execute("SELECT COUNT(*) FROM memory_items").fetchone()[0], 1)
        self.assertIsNone(self.conn.execute("SELECT id FROM sessions").fetchone())

    def test_shipped_memory_sql_has_no_check_or_session_reference(self):
        memory = (ROOT / "overlay-backend/migrations/0003_memory.sql").read_text()
        self.assertNotIn("CHECK", memory)
        self.assertNotIn("REFERENCES sessions", memory)
        catalog = (ROOT / "overlay-backend/migrations/0001_session_catalog.sql").read_text()
        self.assertEqual(catalog.count("REFERENCES sessions(id) ON DELETE CASCADE"), 2)

    def test_status_writer_is_unvalidated_and_migration_has_no_downgrade(self):
        store = source("overlay-backend/src/persistence/sqlite_store.rs")
        status = store[store.index("pub fn set_candidate_status"):store.index("pub fn update_candidate_text")]
        self.assertIn("SET status = ?2", status)
        self.assertNotIn("pending", status)
        migrations = source("overlay-backend/src/persistence/migrations.rs")
        self.assertIn("if *version <= current", migrations)
        self.assertNotIn("downgrade", migrations.lower())

    def test_original_c14_c15_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C14"], "hypothesis")
        self.assertEqual(found["wave1_worker1_persistence-C15"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
