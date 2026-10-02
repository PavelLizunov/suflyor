"""Source fixtures for CI C12/C13. No Actions, Cargo, or installer build."""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class CiVersionSecurityFixtures(unittest.TestCase):
    def test_security_scans_are_not_gate_dependencies(self):
        workflow = source(".github/workflows/ci.yml")
        gate = workflow[workflow.index("  gate:"):workflow.index("  validate:")]
        for token in ("gitleaks", "cargo-deny", "CodeQL", "security.yml"):
            self.assertNotIn(token, gate)
        self.assertNotIn("needs: [changes, rust, macos]", workflow)
        security = source(".github/workflows/security.yml")
        self.assertIn("gitleaks", security)
        self.assertIn("cargo-deny", security)

    def test_macos_failure_cannot_fail_gate(self):
        workflow = source(".github/workflows/ci.yml")
        gate = workflow[workflow.index("  gate:"):workflow.index("  validate:")]
        self.assertNotIn("macos", gate)
        self.assertIn("needs: [changes, rust]", gate)

    def test_versions_match_but_build_does_not_inject_them(self):
        cargo = source("slint-experiment/Cargo.toml")
        installer = source("scripts/slint-installer.nsi")
        cargo_version = re.search(r'^version = "([^"]+)"', cargo, re.M).group(1)
        installer_version = re.search(r'!define PRODUCT_VERSION "([^"]+)"', installer).group(1)
        self.assertEqual(cargo_version, installer_version)
        build = source("scripts/build-slint-release.ps1")
        self.assertNotIn("/DPRODUCT_VERSION", build)
        self.assertNotIn("Info.plist", build)

    def test_version_guard_is_slint_only_and_nsi_is_not_crate_input(self):
        gate = source("scripts/git-gate-native.ps1")
        body = gate[gate.index("$affectedCrates"):gate.index("Write-Host")]
        self.assertNotIn("slint-installer.nsi", body)
        tests = gate[gate.index("'version_guard'"):]
        self.assertIn("--manifest-path", tests)

    def test_security_workflow_is_separate_file(self):
        ci = source(".github/workflows/ci.yml")
        self.assertNotIn("jobs:\n  gitleaks:", ci)
        security = source(".github/workflows/security.yml")
        self.assertIn("name: security", security)
        self.assertIn("pull_request:", security)

    def test_original_c12_hypothesis_and_c13_confirmed_unchanged(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C12"], "hypothesis")
        self.assertEqual(found["wave4_worker3_cicd-C13"], "confirmed")


if __name__ == "__main__":
    unittest.main()
