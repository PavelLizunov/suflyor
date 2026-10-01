"""Dependency-free tests for research checkpoint mechanics, not application behavior."""
import hashlib
import json
import sqlite3
import subprocess
import sys
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

    def seed_coordinator_row(self):
        row = {"id": "wave1_sample-C01", "report": "docs/audit-grok/wave1_sample.md", "status": "hypothesis", "coordinator_review": "original_claim_inspected", "verification": "source_only", "remaining_check": "native not_run", "counterevidence": "runtime consequence untested", "source_references": [{"path": "source.rs", "start_line": 1, "end_line": 2}]}
        self.write("candidates.json", [row])
        self.write("coordinator-checks.json", {"baseline": self.baseline, "checked_original_candidate_ids": [row["id"]], "counts": {"hypothesis": 1}})
        return row

    def test_coordinator_status_is_measured_separately_from_worker(self):
        self.seed_coordinator_row()
        result = checkpoint.inspect(self.root, self.map)
        self.assertEqual(result["issues"], [])
        self.assertEqual(result["coordinator_inspected_candidates"], 1)
        self.assertEqual(result["coordinator_status_counts"], {"hypothesis": 1})
        self.assertFalse(result["independent_acceptance"])
        checkpoint.checkpoint(self.db, result)
        self.assertEqual(checkpoint.recover(self.db)["last_checkpoint"]["evidence"]["coordinator_inspected_candidates"], 1)

    def test_invalid_coordinator_reference_is_not_accepted(self):
        row = self.seed_coordinator_row()
        row["source_references"][0]["end_line"] = 99
        self.write("candidates.json", [row])
        result = checkpoint.inspect(self.root, self.map)
        self.assertTrue(any(x["kind"] == "coordinator_reference_range" for x in result["issues"]))

    def test_coordinator_receipt_count_mismatch_is_not_accepted(self):
        self.seed_coordinator_row()
        self.write("coordinator-checks.json", {"baseline": self.baseline, "checked_original_candidate_ids": ["wave1_sample-C01"], "counts": {"confirmed": 1}})
        self.assertTrue(any(x["kind"] == "coordinator_count_mismatch" for x in checkpoint.inspect(self.root, self.map)["issues"]))

    def test_coordinator_missing_receipt_is_not_accepted(self):
        self.seed_coordinator_row()
        (self.rec / "coordinator-checks.json").unlink()
        self.assertTrue(any(x["kind"] == "coordinator_missing_receipt" for x in checkpoint.inspect(self.root, self.map)["issues"]))

    def test_recorded_git_text_form_is_accepted_not_arbitrary_drift(self):
        source = self.root / "source.rs"
        crlf = b"fn one() {}\r\nfn two() {}\r\n"
        lf = crlf.replace(b"\r\n", b"\n")
        self.snapshot["source_files"][0]["sha256"] = hashlib.sha256(crlf).hexdigest()
        self.write("snapshot.json", self.snapshot)
        self.write("source-portability.json", {"source_commit": self.baseline, "entries": [{"path": "source.rs", "source_commit": self.baseline, "frozen_worktree_sha256": hashlib.sha256(crlf).hexdigest(), "git_blob_sha256": hashlib.sha256(lf).hexdigest()}]})
        source.write_bytes(lf)
        result = checkpoint.inspect(self.root, self.map)
        self.assertEqual(result["issues"], [])
        self.assertEqual(result["accepted_git_text_forms"], ["source.rs"])
        source.write_bytes(lf + b"extra")
        self.assertTrue(any(x["kind"] == "input_drift" for x in checkpoint.inspect(self.root, self.map)["issues"]))

    def test_wrong_baseline_portability_proof_is_rejected(self):
        source = self.root / "source.rs"
        original = source.read_bytes()
        source.write_bytes(original.replace(b"\n", b"\r\n"))
        self.write("source-portability.json", {"entries": [{"path": "source.rs", "source_commit": "other-source", "frozen_worktree_sha256": self.snapshot["source_files"][0]["sha256"], "git_blob_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}]})
        self.assertTrue(any(x["kind"] == "input_drift" for x in checkpoint.inspect(self.root, self.map)["issues"]))

    def seed_feature_contract(self):
        directory = self.map / "features"
        directory.mkdir()
        (directory / "test.md").write_text("# Source-only contract\n")
        record = {"id": "test", "baseline": self.baseline, "contract": "docs/agent-map/features/test.md", "native_verification": "not_run", "independent_acceptance": False, "source_references": [{"path": "source.rs", "start_line": 1, "end_line": 2}]}
        path = directory / "contracts.json"
        path.write_text(json.dumps({"features": [record]}))
        return path, record

    def test_feature_contract_is_checked_without_completeness_claim(self):
        self.seed_feature_contract()
        result = checkpoint.inspect(self.root, self.map)
        self.assertEqual(result["issues"], [])
        self.assertEqual(result["source_feature_contracts"], 1)
        self.assertFalse(result["feature_contracts_complete"])

    def test_feature_reference_and_fake_acceptance_are_rejected(self):
        path, record = self.seed_feature_contract()
        record["source_references"][0]["end_line"] = 999
        record["independent_acceptance"] = True
        path.write_text(json.dumps({"features": [record]}))
        result = checkpoint.inspect(self.root, self.map)
        kinds = {row["kind"] for row in result["issues"]}
        self.assertIn("feature_reference_range", kinds)
        self.assertIn("feature_acceptance_boundary", kinds)

    def test_recover_cli_checks_current_drift_not_only_saved_report(self):
        report = checkpoint.inspect(self.root, self.map)
        checkpoint.checkpoint(self.db, report)
        (self.root / "source.rs").write_text("changed after checkpoint\n")
        result = subprocess.run([sys.executable, "-B", str(Path(checkpoint.__file__).resolve()), "recover", "--root", str(self.root), "--database", str(self.db)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        recovered = json.loads(result.stdout)
        self.assertTrue(any(row["kind"] == "input_drift" for row in recovered["issues"]))
        self.assertFalse(recovered["automatic_dispatch"])

    def test_canonical_claim_is_bound_to_report_not_editable_register(self):
        report = self.rec / "grok-redacted/wave1_sample.md"
        report.parent.mkdir()
        report.write_text("### Original title\n- **Finding / Hypothesis:** Exact claim\n")
        self.snapshot["grok_reports"] = [{"path": "docs/audit-grok/wave1_sample.md", "sha256": "raw-hash"}]
        self.write("snapshot.json", self.snapshot)
        self.write("grok-redaction-provenance.json", [{"original_path": "docs/audit-grok/wave1_sample.md", "original_sha256": "raw-hash", "redacted_path": str(report.relative_to(self.root)), "redacted_sha256": hashlib.sha256(report.read_bytes()).hexdigest()}])
        row = {"id": "wave1_sample-C01", "report": "docs/audit-grok/wave1_sample.md", "title": "Original title", "original_claim_redacted": "Exact claim  "}
        self.write("candidates.json", [row])
        self.assertEqual(checkpoint.inspect(self.root, self.map)["issues"], [])
        row["original_claim_redacted"] = "Substituted claim"
        self.write("candidates.json", [row])
        self.assertTrue(any(x["kind"] == "canonical_original_claim_mismatch" for x in checkpoint.inspect(self.root, self.map)["issues"]))

    def test_redacted_copy_is_verified_even_when_raw_report_exists(self):
        raw = self.root / "docs/audit-grok/wave1_sample.md"
        raw.parent.mkdir()
        raw.write_text("### Original title\n- **Finding / Hypothesis:** Exact claim\n")
        redacted = self.rec / "grok-redacted/wave1_sample.md"
        redacted.parent.mkdir()
        redacted.write_text(raw.read_text())
        raw_hash = hashlib.sha256(raw.read_bytes()).hexdigest()
        self.snapshot["grok_reports"] = [{"path": str(raw.relative_to(self.root)), "sha256": raw_hash}]
        self.write("snapshot.json", self.snapshot)
        self.write("grok-redaction-provenance.json", [{"original_path": str(raw.relative_to(self.root)), "original_sha256": raw_hash, "redacted_path": str(redacted.relative_to(self.root)), "redacted_sha256": hashlib.sha256(redacted.read_bytes()).hexdigest()}])
        self.write("candidates.json", [{"id": "wave1_sample-C01", "report": str(raw.relative_to(self.root)), "title": "Original title", "original_claim_redacted": "Exact claim"}])
        self.assertEqual(checkpoint.inspect(self.root, self.map)["issues"], [])
        redacted.write_text("tampered copy")
        self.assertTrue(any(row["kind"] == "redacted_report_drift" for row in checkpoint.inspect(self.root, self.map)["issues"]))


if __name__ == "__main__":
    unittest.main()
