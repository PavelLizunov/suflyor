"""Actual SQLite temporary WAL fixtures; NOT native rusqlite/UI/product fault tests."""
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]


class SQLiteContentionFixtures(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.path=Path(self.tmp.name)/'fixture.sqlite'
        self.a=sqlite3.connect(self.path,isolation_level=None);self.b=sqlite3.connect(self.path,isolation_level=None)
        self.addCleanup(self.a.close);self.addCleanup(self.b.close)
        for conn in (self.a,self.b):
            conn.execute('PRAGMA journal_mode=WAL');conn.execute('PRAGMA foreign_keys=ON');conn.execute('PRAGMA busy_timeout=20')
        for path in sorted((ROOT/'overlay-backend/migrations').glob('*.sql')):self.a.executescript(path.read_text())
        self.a.execute("INSERT INTO sessions(id,journal_path,started_at_ms,status,indexed_at_ms) VALUES('s','fixture',1,'completed',1)")
        self.a.execute("INSERT INTO memory_candidates(id,profile_id,kind,text,status,created_at_ms) VALUES(1,'default','fact','dummy','pending',1)")

    def test_deferred_read_snapshot_upgrade_fails_busy_snapshot_after_other_commit(self):
        self.a.execute('BEGIN DEFERRED')
        self.assertEqual(self.a.execute('SELECT status FROM memory_candidates WHERE id=1').fetchone()[0],'pending')
        self.b.execute("UPDATE memory_candidates SET status='approved' WHERE id=1")
        with self.assertRaises(sqlite3.OperationalError) as error:self.a.execute("UPDATE memory_candidates SET status='approved' WHERE id=1")
        self.assertEqual(error.exception.sqlite_errorcode,sqlite3.SQLITE_BUSY_SNAPSHOT)
        self.a.execute('ROLLBACK')
        self.assertEqual(self.a.execute('SELECT status FROM memory_candidates WHERE id=1').fetchone()[0],'approved')

    def test_write_first_conflict_busy_not_snapshot_while_first_writer_open(self):
        self.a.execute('BEGIN DEFERRED');self.a.execute('UPDATE sessions SET indexed_at_ms=2 WHERE id="s"')
        self.b.execute('BEGIN DEFERRED')
        with self.assertRaises(sqlite3.OperationalError) as error:self.b.execute('UPDATE sessions SET indexed_at_ms=3 WHERE id="s"')
        self.assertEqual(error.exception.sqlite_errorcode,sqlite3.SQLITE_BUSY)
        self.b.execute('ROLLBACK');self.a.execute('COMMIT')
        self.b.execute('UPDATE sessions SET indexed_at_ms=3 WHERE id="s"')
        self.assertEqual(self.b.execute('SELECT indexed_at_ms FROM sessions').fetchone()[0],3)

    def test_reader_snapshot_does_not_block_writer_wal_and_refresh_needs_transaction_end(self):
        self.a.execute('BEGIN DEFERRED');self.assertEqual(self.a.execute('SELECT indexed_at_ms FROM sessions').fetchone()[0],1)
        self.b.execute('UPDATE sessions SET indexed_at_ms=8 WHERE id="s"')
        self.assertEqual(self.a.execute('SELECT indexed_at_ms FROM sessions').fetchone()[0],1)
        self.a.execute('ROLLBACK');self.assertEqual(self.a.execute('SELECT indexed_at_ms FROM sessions').fetchone()[0],8)

    def test_default_auto_checkpoint_enabled_not_no_checkpoint_policy(self):
        self.assertEqual(self.a.execute('PRAGMA wal_autocheckpoint').fetchone()[0],1000)
        source=(ROOT/'overlay-backend/src/persistence/sqlite_store.rs').read_text()
        self.assertIn('PRAGMA busy_timeout = 2000',source)
        self.assertNotIn('wal_autocheckpoint',source)
        self.assertNotIn('SQLITE_BUSY_SNAPSHOT',source)
        self.assertNotIn('transaction_with_behavior',source)

    def test_active_reader_prevents_truncate_then_release_allows_zero_wal(self):
        self.a.execute('PRAGMA wal_checkpoint(TRUNCATE)')
        self.b.execute('BEGIN DEFERRED');self.b.execute('SELECT indexed_at_ms FROM sessions').fetchone()
        for value in range(2,6):self.a.execute('UPDATE sessions SET indexed_at_ms=? WHERE id="s"',(value,))
        status,log,done=self.a.execute('PRAGMA wal_checkpoint(TRUNCATE)').fetchone()
        self.assertEqual(status,1);self.assertGreater(log,done)
        self.b.execute('ROLLBACK')
        self.assertEqual(self.a.execute('PRAGMA wal_checkpoint(TRUNCATE)').fetchone(),(0,0,0))
        self.assertEqual(Path(str(self.path)+'-wal').stat().st_size,0)

    def test_approval_read_before_update_but_replace_write_first_source_preconditions(self):
        source=(ROOT/'overlay-backend/src/persistence/sqlite_store.rs').read_text()
        approve=source[source.index('pub fn approve_candidate'):source.index('pub fn insert_memory_item')]
        self.assertLess(approve.index('.query_row('),approve.index('tx.execute('))
        replacement=source[source.index('pub fn replace_session'):source.index('pub fn delete_session')]
        self.assertLess(replacement.index('DELETE FROM sessions'),replacement.index('INSERT INTO sessions'))
        self.assertNotIn('query_row(',replacement)
        candidates=json.loads((ROOT/'docs/agent-map/reconciliation/candidates.json').read_text())
        for row in candidates:
            if row['id'] in {'wave1_worker1_persistence-C02','wave1_worker1_persistence-C18'}:self.assertEqual(row['status'],'hypothesis')


if __name__=='__main__':unittest.main()
