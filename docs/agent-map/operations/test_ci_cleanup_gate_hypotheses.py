"""Source fixtures for CI C08/C10. No Actions run or filesystem deletion."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class CiCleanupGateFixtures(unittest.TestCase):
    def test_recursive_cleanup_has_no_reparse_guard(self):
        text = source("scripts/slint-installer.nsi")
        body = text[text.index('Section "Uninstall"'):]
        for target in ("$APPDATA\\suflyor", "$APPDATA\\overlay-mvp", "$PROFILE\\suflyor-local-ai"):
            self.assertIn(f'RMDir /r "{target}"', body)
        self.assertNotIn("GetFileAttributes", body)
        self.assertNotIn("FILE_ATTRIBUTE_REPARSE_POINT", body)
        self.assertNotIn("IO_REPARSE_TAG", body)

    def test_cleanup_is_opt_in_but_targets_whole_trees(self):
        text = source("scripts/slint-installer.nsi")
        body = text[text.index("MessageBox MB_YESNO"):text.index("uninst_keep_data:")]
        self.assertLess(body.index("MessageBox"), body.index("RMDir /r"))
        self.assertEqual(body.count("RMDir /r"), 3)

    def test_gate_dependencies_exclude_macos_and_validate(self):
        text = source(".github/workflows/ci.yml")
        gate = text[text.index("  gate:"):text.index("  validate:")]
        self.assertIn("needs: [changes, rust]", gate)
        self.assertNotIn("macos", gate)
        self.assertNotIn("validate", gate)
        self.assertIn('test "$RUST_NEEDED" != "true" || test "$RUST_RESULT" = "success"', gate)

    def test_validate_and_macos_are_separate_jobs(self):
        text = source(".github/workflows/ci.yml")
        self.assertIn("  macos:", text)
        self.assertIn("  validate:", text)
        validate = text[text.index("  validate:"):]
        self.assertNotIn("needs:", validate)
        self.assertIn("bash .github/scripts/test-docs-only-detection.sh", validate)

    def test_docs_classifier_is_external_script_not_inline_title(self):
        text = source(".github/workflows/ci.yml")
        self.assertIn("bash .github/scripts/is-docs-only.sh", text)
        self.assertNotIn("github.event.pull_request.title", text)
        self.assertNotIn("github.event.pull_request.body", text)

    def test_original_c08_c10_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C08"], "hypothesis")
        self.assertEqual(found["wave4_worker3_cicd-C10"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
