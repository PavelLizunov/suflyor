"""Source fixtures for wave4_worker2_installers C03/C04.
No live downloads, no network calls, no process execution.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class InstallerNetworkProtocolFixtures(unittest.TestCase):
    def test_curl_download_flags_and_starts_with_https_check(self):
        text = source("overlay-backend/src/download.rs")
        body = text[text.index("pub(crate) fn curl_download"):text.index("pub(crate) fn extract_tar_bz2")]
        # Checks trimmed_url starts with https://
        self.assertIn('trimmed_url = url.trim()', body)
        self.assertIn('!trimmed_url.starts_with("https://")', body)
        # Check curl arguments: -L is present (follows redirects)
        self.assertIn('"-L"', body)
        self.assertIn('"--fail"', body)
        self.assertIn('"--retry"', body)
        # Note absence of protocol restrictions like --proto =https or --proto-redir =https
        self.assertNotIn("--proto", body)
        self.assertNotIn("--proto-redir", body)
        self.assertNotIn("--max-redirs", body)

    def test_update_is_trusted_download_domain_and_port_allowlist(self):
        text = source("overlay-backend/src/update.rs")
        body = text[text.index("fn is_trusted_download"):text.index("pub async fn download_installer")]
        # Validates https scheme, standard port, no userinfo
        self.assertIn('parsed.scheme() != "https"', body)
        self.assertIn('parsed.username().is_empty()', body)
        self.assertIn('parsed.port_or_known_default() != Some(443)', body)
        # Allowed domains: github.com, raw.githubusercontent.com, objects.githubusercontent.com
        self.assertIn('"github.com"', body)
        self.assertIn('"objects.githubusercontent.com"', body)

    def test_reqwest_client_builder_in_update_has_no_explicit_redirect_policy(self):
        text = source("overlay-backend/src/update.rs")
        body = text[text.index("pub async fn download_installer"):text.index("pub fn run_installer")]
        # Client builder sets user_agent and timeout, but doesn't set redirect policy (default is follow up to 10)
        self.assertIn("reqwest::Client::builder()", body)
        self.assertIn(".timeout(std::time::Duration::from_secs(300))", body)
        self.assertNotIn("redirect(", body)
        self.assertNotIn("Policy::none()", body)

    def test_teratts_atomic_staging_and_marker_quarantine(self):
        # C04 counterevidence / design contrast: teratts_install implements staging dir,
        # atomic fs::rename, manifest.json marker check, and broken directory quarantine
        text = source("overlay-backend/src/teratts_install.rs")
        self.assertIn('const MARKER: &str = "manifest.json"', text)
        self.assertIn(".staging", text)
        self.assertIn(".broken-", text)
        body = text[text.index("pub fn install_with"):text.index("pub fn verify_file")]
        self.assertIn("check_dir(manifest, &release).is_ok()", body)
        self.assertIn("std::fs::rename(&staging, &release)", body)

    def test_ocr_install_in_place_extraction_contrast(self):
        # C04 contrast: ocr_install downloads tarball into root, extracts directly,
        # and cleans up dest only if dest_has_engine fails
        text = source("overlay-backend/src/ocr_install.rs")
        body = text[text.index("pub fn install("):text.index("fn dest_has_engine")]
        self.assertIn("let dest = root.join(\"tesseract\");", body)
        self.assertIn("extract_tar_bz2(&tarball, &root)", body)
        self.assertIn("if !dest_has_engine(&dest)", body)
        self.assertIn("std::fs::remove_dir_all(&dest)", body)

    def test_original_c03_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker2_installers-C03"], "hypothesis")
        self.assertEqual(found["wave4_worker2_installers-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
