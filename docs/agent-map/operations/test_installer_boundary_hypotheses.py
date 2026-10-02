"""Installer source/quoting fixtures for C06/C07. No NSIS build or deletion."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class InstallerBoundaryFixtures(unittest.TestCase):
    def test_installer_is_per_user_without_signature_or_acl_policy(self):
        text = source("scripts/slint-installer.nsi")
        self.assertIn("RequestExecutionLevel user", text)
        self.assertIn('!define PRODUCT_INSTALL_DIR "$LOCALAPPDATA\\suflyor-slint"', text)
        self.assertIn("Page directory", text)
        self.assertNotIn("!uninstfinalize", text)
        self.assertNotIn("AccessControl::", text)
        self.assertNotIn("SetOverwrite", text)

    def test_process_stop_quotes_paths_and_limits_exact_executables(self):
        text = source("scripts/slint-installer.nsi")
        self.assertIn('-File "$PLUGINSDIR\\stop-installed-suflyor.ps1" -InstallDir "$INSTDIR"', text)
        helper = source("scripts/stop-installed-suflyor.ps1")
        self.assertIn("[IO.Path]::GetFullPath", helper)
        self.assertIn("$targets.ContainsKey($path)", helper)
        self.assertIn("exit 10", helper)
        self.assertIn("exit 11", helper)

    def test_install_dir_is_interpolated_inside_quotes_without_escape(self):
        text = source("scripts/slint-installer.nsi")
        command = text[text.index("nsExec::ExecToStack"):text.index("Pop $0")]
        self.assertIn('-InstallDir "$INSTDIR"', command)
        self.assertNotIn("$" + '\\"$INSTDIR$\\"', command)

    def test_uninstall_recursive_targets_are_profile_wide(self):
        text = source("scripts/slint-installer.nsi")
        body = text[text.index('Section "Uninstall"'):]
        for target in ("$APPDATA\\suflyor", "$APPDATA\\overlay-mvp", "$PROFILE\\suflyor-local-ai"):
            self.assertIn(f'RMDir /r "{target}"', body)
        self.assertIn("MessageBox MB_YESNO", body)

    def test_uninstall_registry_strings_are_quoted(self):
        text = source("scripts/slint-installer.nsi")
        self.assertIn('UninstallString" "$\\"$INSTDIR\\uninstall.exe$\\""', text)
        self.assertIn('InstallLocation" "$\\"$INSTDIR$\\""', text)
        self.assertNotIn("HKLM", text)

    def test_original_c06_c07_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker3_cicd-C06"], "hypothesis")
        self.assertEqual(found["wave4_worker3_cicd-C07"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
