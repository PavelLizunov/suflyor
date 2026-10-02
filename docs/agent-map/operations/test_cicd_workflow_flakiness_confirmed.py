"""Source fixtures for wave4_worker3_cicd C09 and C11 (confirmed mechanisms).
No GitHub Actions workflow runs, no network calls, no Cargo builds.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class CicdWorkflowSecurityFlakinessFixtures(unittest.TestCase):
    def test_ci_workflow_uses_unpinned_action_tags(self):
        text = source(".github/workflows/ci.yml")
        # Floating tag references without commit SHAs
        self.assertIn("actions/checkout@v4", text)
        self.assertIn("dtolnay/rust-toolchain@stable", text)
        self.assertIn("Swatinem/rust-cache@v2", text)

    def test_ci_workflow_omits_top_level_permissions_block(self):
        text = source(".github/workflows/ci.yml")
        header = text[:text.index("jobs:")]
        # No top-level permissions: block before jobs declaration
        self.assertNotIn("\npermissions:", header)

    def test_ci_rust_job_omits_teratts_and_wsola_crates(self):
        text = source(".github/workflows/ci.yml")
        rust_job = text[text.index("  rust:"):text.index("  macos:")]
        # Runs overlay-backend, slint-experiment, suflyor-tts
        self.assertIn("overlay-backend", rust_job)
        self.assertIn("slint-experiment", rust_job)
        self.assertIn("suflyor-tts", rust_job)
        # Omission of suflyor-teratts and suflyor-wsola in rust test matrix
        self.assertNotIn("suflyor-teratts", rust_job)
        self.assertNotIn("suflyor-wsola", rust_job)

    def test_ci_rust_job_runs_cargo_without_locked_flag_except_ui_mcp(self):
        text = source(".github/workflows/ci.yml")
        rust_job = text[text.index("  rust:"):text.index("  macos:")]
        # Only the ui-mcp feature check uses --locked
        lines_with_locked = [line for line in rust_job.splitlines() if "--locked" in line]
        self.assertEqual(len(lines_with_locked), 1)
        self.assertIn("ui-mcp", lines_with_locked[0])
        # Standard cargo test and clippy commands run unlocked
        self.assertIn("cargo test --manifest-path overlay-backend/Cargo.toml", rust_job)
        self.assertIn("cargo test --manifest-path slint-experiment/Cargo.toml", rust_job)
        self.assertIn("cargo test --manifest-path suflyor-tts/Cargo.toml", rust_job)

    def test_concurrency_specifies_cancel_in_progress(self):
        text = source(".github/workflows/ci.yml")
        self.assertIn("concurrency:\n  group: ci-${{ github.ref }}\n  cancel-in-progress: true", text)

    def test_original_c09_c11_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C09"], "confirmed")
        self.assertEqual(found["wave4_worker3_cicd-C11"], "confirmed")


if __name__ == "__main__":
    unittest.main()
