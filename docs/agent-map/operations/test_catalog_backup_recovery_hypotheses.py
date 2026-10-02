"""Temporary backup/recovery fixtures for C01/C16. Not owner catalogs."""
import json
import sqlite3
import tempfile
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MAX_AGE_MS = 12 * 60 * 60 * 1000


def source(path):
    return (ROOT / path).read_text()


def classify(path, now_ms):
    started = stop = summary = False
    started_ms = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = event.get("kind")
        if kind == "session_start":
            started = True
            started_ms = started_ms or event.get("unix_ms")
        elif kind == "session_stop":
            stop = True
        elif kind == "session_summary":
            summary = True
    if not started or stop or summary or started_ms is None:
        return None
    if now_ms - started_ms > MAX_AGE_MS:
        return None
    return path.stem


class CatalogBackupRecoveryFixtures(unittest.TestCase):
    def test_backup_copies_main_file_but_not_wal_sidecar(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            db = root / "catalog.sqlite"
            conn = sqlite3.connect(db)
            conn.execute("PRAGMA journal_mode=WAL")
            conn.execute("CREATE TABLE rows(id INTEGER)")
            conn.execute("BEGIN")
            conn.execute("INSERT INTO rows VALUES(1)")
            conn.commit()
            self.assertTrue(Path(str(db) + "-wal").exists())
            backup = db.with_suffix(".sqlite.bak")
            backup.write_bytes(db.read_bytes())
            self.assertFalse(Path(str(backup) + "-wal").exists())
            conn.close()

    def test_recovery_requires_start_without_stop_summary_and_within_age(self):
        now = int(time.time() * 1000)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            valid = root / "valid.jsonl"
            valid.write_text(json.dumps({"kind": "session_start", "unix_ms": now - 1000}) + "\n", encoding="utf-8")
            stopped = root / "stopped.jsonl"
            stopped.write_text(json.dumps({"kind": "session_start", "unix_ms": now}) + "\n" + json.dumps({"kind": "session_stop"}) + "\n", encoding="utf-8")
            old = root / "old.jsonl"
            old.write_text(json.dumps({"kind": "session_start", "unix_ms": now - MAX_AGE_MS - 1}) + "\n", encoding="utf-8")
            self.assertEqual(classify(valid, now), "valid")
            self.assertIsNone(classify(stopped, now))
            self.assertIsNone(classify(old, now))

    def test_summary_marker_rejects_even_without_stop(self):
        now = int(time.time() * 1000)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "summary.jsonl"
            path.write_text(json.dumps({"kind": "session_start", "unix_ms": now}) + "\n" + json.dumps({"kind": "session_summary", "summary": "done"}) + "\n", encoding="utf-8")
            self.assertIsNone(classify(path, now))

    def test_source_backup_discards_checkpoint_result_and_copies_main_only(self):
        text = source("overlay-backend/src/persistence/sqlite_store.rs")
        body = text[text.index("if current < migrations::LATEST_VERSION"):text.index("migrations::run_migrations")]
        self.assertIn('execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")', body)
        self.assertIn("std::fs::copy(path, &bak)", body)
        self.assertNotIn("-wal", body)
        self.assertNotIn("-shm", body)

    def test_recovery_source_age_and_marker_order(self):
        text = source("overlay-backend/src/journal/recovery.rs")
        self.assertIn("RECOVERY_MAX_AGE_MS: u64 = 12 * 60 * 60 * 1000", text)
        body = text[text.index("if !has_start || has_stop || has_summary"):text.index("let session_id = path")]
        self.assertLess(body.index("return None"), body.index("RECOVERY_MAX_AGE_MS"))

    def test_original_c01_and_c16_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C01"], "confirmed")
        self.assertEqual(found["wave1_worker1_persistence-C16"], "confirmed")


if __name__ == "__main__":
    unittest.main()
