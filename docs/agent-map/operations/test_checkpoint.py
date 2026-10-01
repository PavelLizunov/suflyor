"""Dependency-free tests for research checkpoint mechanics, not application behavior."""
import hashlib
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

import checkpoint


class CheckpointTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.map = self.root / "docs/agent-map"
        self.rec = self.map / "reconciliation"
        self.rec.mkdir(parents=True)
        source = self.root / "source.rs"
        source.write_text("fn one() {}\nfn two() {}\n")
        self.baseline = "frozen-test-source"
        self.snapshot = {"source_commit": self.baseline, "candidate_count": 1, "source_files": [{"path": "source.rs", "sha256": hashlib.sha256(source.read_bytes()).hexdigest()}], "grok_reports": []}
        self.write("snapshot.json", self.snapshot)
        self.write("candidates.json", [{"id": "wave1_sample-C01", "report": "docs/audit-grok/wave1_sample.md"}])
        self.report = {"baseline": self.baseline, "lane": "data", "candidates": [{"id": "wave1_sample-C01", "status": "hypothesis", "source_references": [{"path": "source.rs", "start_line": 1, "end_line": 2}], "verification": "source inspected; native test not_run", "remaining_check": "Run native bounded reproduction."}]}
        self.write("data.json", self.report)
        self.db = self.root / "state.sqlite"

    def write(self, name, value):
        (self.rec / name).write_text(json.dumps(value))

    def test_idempotent_checkpoint_and_process_recovery(self):
        report = checkpoint.inspect(self.root, self.map)
        self.assertEqual(report["issues"], [])
        checkpoint.checkpoint(self.db, report)
        checkpoint.checkpoint(self.db, report)
        recovered = checkpoint.recover(self.db)
        self.assertEqual(len(recovered["preserved_artifacts"]), 1)
        self.assertFalse(recovered["automatic_dispatch"])
        self.assertEqual(recovered["last_checkpoint"]["source_commit"], self.baseline)

    def test_unknown_attempt_is_not_silently_retried(self):
        checkpoint.checkpoint(self.db, checkpoint.inspect(self.root, self.map))
        with sqlite3.connect(self.db) as db:
            db.execute("INSERT INTO attempts VALUES ('attempt-1','data','workflow-test','running')")
        recovered = checkpoint.recover(self.db)
        self.assertEqual(recovered["attempts"][0][3], "unknown")
        self.assertFalse(recovered["automatic_dispatch"])

    def test_source_drift_does_not_replace_preserved_artifact(self):
        report = checkpoint.inspect(self.root, self.map)
        checkpoint.checkpoint(self.db, report)
        old = checkpoint.recover(self.db)["preserved_artifacts"]
        (self.root / "source.rs").write_text("changed\n")
        self.report["candidates"][0]["rationale"] = "new untrusted answer"
        self.write("data.json", self.report)
        changed = checkpoint.inspect(self.root, self.map)
        self.assertTrue(any(x["kind"] == "input_drift" for x in changed["issues"]))
        checkpoint.checkpoint(self.db, changed)
        self.assertEqual(checkpoint.recover(self.db)["preserved_artifacts"], old)

    def test_missing_candidate_is_rejected(self):
        self.report["candidates"] = []
        self.write("data.json", self.report)
        report = checkpoint.inspect(self.root, self.map)
        self.assertTrue(report["issues"])
        self.assertEqual(report["artifacts"][0]["state"], "invalid")

    def test_invalid_source_line_is_rejected(self):
        self.report["candidates"][0]["source_references"][0]["end_line"] = 500
        self.write("data.json", self.report)
        report = checkpoint.inspect(self.root, self.map)
        self.assertTrue(any("out_of_range_reference" in x.get("detail", "") for x in report["issues"]))

    def test_pending_lane_is_not_completed(self):
        (self.rec / "data.json").unlink()
        report = checkpoint.inspect(self.root, self.map)
        self.assertEqual(report["reported_candidates"], 0)
        self.assertEqual(report["artifacts"][0]["state"], "pending")
        checkpoint.checkpoint(self.db, report)
        self.assertEqual(checkpoint.recover(self.db)["preserved_artifacts"], [])

    def test_null_worker_output_cannot_be_accepted(self):
        self.report["transport_status"] = "final_result_null; partial_file_output"
        self.write("data.json", self.report)
        result = checkpoint.inspect(self.root, self.map)
        self.assertEqual(result["artifacts"][0]["state"], "unaccepted_proposal")
        checkpoint.checkpoint(self.db, result)
        self.assertEqual(checkpoint.recover(self.db)["preserved_artifacts"], [])

    def test_original_claim_substitution_is_not_accepted(self):
        self.write("candidates.json", [{"id": "wave1_sample-C01", "report": "docs/audit-grok/wave1_sample.md", "title": "Original title", "original_claim_redacted": "Original consequence"}])
        self.report["candidates"][0]["title"] = "Different title"
        self.report["candidates"][0]["original_claim_redacted"] = "Other consequence"
        self.write("data.json", self.report)
        result = checkpoint.inspect(self.root, self.map)
        self.assertEqual(result["artifacts"][0]["state"], "unaccepted_proposal")
        self.assertEqual(len(result["artifacts"][0]["semantic_warnings"]), 2)

    def test_portable_redacted_report_recovers_without_raw_file(self):
        original_hash = hashlib.sha256(b"private report").hexdigest()
        portable = self.rec / "grok-redacted/report.md"
        portable.parent.mkdir()
        portable.write_text("Redacted report without private endpoint")
        self.snapshot["grok_reports"] = [{"path": "docs/audit-grok/report.md", "sha256": original_hash}]
        self.write("snapshot.json", self.snapshot)
        self.write("grok-redaction-provenance.json", [{"original_path": "docs/audit-grok/report.md", "original_sha256": original_hash, "redacted_path": str(portable.relative_to(self.root)), "redacted_sha256": hashlib.sha256(portable.read_bytes()).hexdigest()}])
        self.assertEqual(checkpoint.inspect(self.root, self.map)["issues"], [])
        portable.write_text("changed copy")
        self.assertTrue(checkpoint.inspect(self.root, self.map)["issues"])


if __name__ == "__main__":
    unittest.main()
