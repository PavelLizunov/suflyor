"""Source fixtures for wave4_worker1_privacy C01 and C02 (confirmed mechanisms).
No external logging, no live filesystem access, no live network probing.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class PrivacyDiagnosticsSanitizationFixtures(unittest.TestCase):
    def test_redact_home_all_forms_checks_three_literal_spellings_only(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("fn redact_home_all_forms("):text.index("fn redact_home_in(")]
        # Replaces raw home, double backslash, and forward slash
        self.assertIn("let r = redact_home_in(s, home);", body)
        self.assertIn("let r = redact_home_in(&r, &home.replace('\\\\', \"\\\\\\\\\"));", body)
        self.assertIn("redact_home_in(&r, &home.replace('\\\\', \"/\"))", body)
        # Note documented 8.3 residual in doc comment
        self.assertIn("Conscious residual: an 8.3 short name", text)

    def test_redact_home_in_ignores_short_homes_under_four_chars(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("fn redact_home_in("):text.index("static GPU_CACHE:")]
        self.assertIn("if home.len() < 4 {\n        return s.to_string();\n    }", body)
        self.assertIn("%USERPROFILE%", body)

    def test_redact_urls_finds_only_http_and_https_schemes(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("pub(crate) fn redact_urls("):text.index("fn redact_user_home(")]
        # Searches for http:// and https:// prefixes only
        self.assertIn('[hay.find("http://"), hay.find("https://")]', body)
        # Calls mask_host on URL slice up to whitespace
        self.assertIn("overlay_backend::config::mask_host(&tail[..end])", body)
        # Non-http schemes (ws://, ftp://, file://) are not searched or masked
        self.assertNotIn("ws://", body)
        self.assertNotIn("ftp://", body)
        self.assertNotIn("file://", body)

    def test_redact_ipv4_operates_on_digit_and_dot_runs(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("pub(crate) fn redact_ipv4("):text.index("pub(crate) fn redact_urls(")]
        self.assertIn("if ch.is_ascii_digit() || ch == '.' {", body)
        self.assertIn('out.push_str("<ip>");', body)

    def test_collect_redacted_log_pipeline_order(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("fn collect_redacted_log() -> std::io::Result<std::path::PathBuf> {"):text.index("fn reveal_in_explorer(")]
        # Applied as: redact_secrets(&redact_user_home(&redact_ipv4(&redact_urls(&raw))))
        self.assertIn("redact_secrets(&redact_user_home(&redact_ipv4(&redact_urls(&raw))))", body)

    def test_original_c01_c02_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker1_privacy-C01"], "confirmed")
        self.assertEqual(found["wave4_worker1_privacy-C02"], "confirmed")


if __name__ == "__main__":
    unittest.main()
