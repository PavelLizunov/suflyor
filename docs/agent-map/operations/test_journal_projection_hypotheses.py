"""Temporary projection fixtures for C08/C09. Not owner journals or Rust indexing."""
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class JournalProjectionFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.conn = sqlite3.connect(Path(self.tmp.name) / "catalog.sqlite")
        self.addCleanup(self.conn.close)
        self.conn.execute("PRAGMA foreign_keys=ON")
        for path in sorted((ROOT / "overlay-backend/migrations").glob("*.sql")):
            self.conn.executescript(path.read_text())

    def project(self, name, lines):
        events = [json.loads(line) for line in lines if json.loads(line)]
        stop = next((event.get("unix_ms") for event in events if event["kind"] == "session_stop"), None)
        start = next((event for event in events if event["kind"] == "session_start"), None)
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions(id,journal_path,started_at_ms,finished_at_ms,status,ai_model,indexed_at_ms) VALUES(?,?,?,?,?,?,?)",
            (name, f"{name}.jsonl", start.get("unix_ms") if start else None, stop, "completed" if stop else "crashed", start.get("ai_model") if start else None, 1),
        )
        self.conn.execute("DELETE FROM ai_turns WHERE session_id=?", (name,))
        for event in events:
            if event["kind"] == "ai_turn":
                self.conn.execute(
                    "INSERT INTO ai_turns(session_id,unix_ms,model,question,answer) VALUES(?,?,?,?,?)",
                    (name, event["unix_ms"], event["model"], "q", "a"),
                )

    def test_skip_snapshot_uses_non_crashed_rows_and_exact_active_stem(self):
        text = source("overlay-backend/src/persistence/indexer.rs")
        body = text[text.index("pub fn index_all"):text.index("fn num(")]
        self.assertLess(body.index("store.finalized_session_ids()"), body.index("std::fs::read_dir"))
        self.assertIn("Some(stem.as_str()) == skip_active", body)
        self.assertIn("finalized.contains(&stem)", body)
        query = source("overlay-backend/src/persistence/sqlite_store.rs")
        self.assertIn("SELECT id FROM sessions WHERE status <> 'crashed'", query)

    def test_missing_stop_projects_crashed_then_later_stop_heals(self):
        self.project("live", ['{"kind":"session_start","unix_ms":1,"ai_model":"configured"}'])
        self.assertEqual(self.conn.execute("SELECT status FROM sessions WHERE id='live'").fetchone()[0], "crashed")
        finalized = {row[0] for row in self.conn.execute("SELECT id FROM sessions WHERE status <> 'crashed'")}
        self.assertNotIn("live", finalized)
        self.project("live", ['{"kind":"session_start","unix_ms":1,"ai_model":"configured"}', '{"kind":"session_stop","unix_ms":2}'])
        self.assertEqual(self.conn.execute("SELECT status,finished_at_ms FROM sessions WHERE id='live'").fetchone(), ("completed", 2))

    def test_rebuild_keeps_session_start_model_until_separate_backfill(self):
        self.project("s", [
            '{"kind":"session_start","unix_ms":1,"ai_model":"configured-cloud"}',
            '{"kind":"ai_turn","unix_ms":2,"model":"actual-local"}',
            '{"kind":"session_stop","unix_ms":3}',
        ])
        self.assertEqual(self.conn.execute("SELECT ai_model FROM sessions WHERE id='s'").fetchone()[0], "configured-cloud")
        self.conn.execute(
            "UPDATE sessions SET ai_model=(SELECT model FROM ai_turns WHERE session_id=sessions.id AND model<>'' GROUP BY model ORDER BY COUNT(*) DESC, MAX(unix_ms) DESC LIMIT 1)"
        )
        self.assertEqual(self.conn.execute("SELECT ai_model FROM sessions WHERE id='s'").fetchone()[0], "actual-local")

    def test_turnless_backfill_clears_headline_model(self):
        self.project("empty", ['{"kind":"session_start","unix_ms":1,"ai_model":"configured"}', '{"kind":"session_stop","unix_ms":2}'])
        self.conn.execute(
            "UPDATE sessions SET ai_model=(SELECT model FROM ai_turns WHERE session_id=sessions.id AND model<>'' GROUP BY model ORDER BY COUNT(*) DESC, MAX(unix_ms) DESC LIMIT 1)"
        )
        self.assertIsNone(self.conn.execute("SELECT ai_model FROM sessions WHERE id='empty'").fetchone()[0])

    def test_reindex_calls_backfill_after_index_all(self):
        text = source("overlay-backend/src/persistence/mod.rs")
        body = text[text.index("pub fn reindex_default"):text.index("pub fn open_default_store")]
        self.assertLess(body.index("index_all("), body.index("store.backfill_session_models()"))
        self.assertIn("if let Err(e)", body)

    def test_original_c08_c09_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C08"], "hypothesis")
        self.assertEqual(found["wave1_worker1_persistence-C09"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
