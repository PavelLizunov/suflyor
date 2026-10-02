"""Source and mock fixtures for wave4_worker1_privacy C03 and wave3_worker4_settings C10 (confirmed mechanisms).
No live API token manipulation, no external logging, no UI event loop execution.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def redact_secrets_mock(s: str) -> str:
    out = []
    rest = s
    while rest:
        if rest.startswith("Bearer "):
            out.append("Bearer <redacted>")
            rest = rest[7:]
            tok_len = -1
            for i, ch in enumerate(rest):
                if ch.isspace():
                    tok_len = i
                    break
            if tok_len == -1:
                tok_len = len(rest)
            rest = rest[tok_len:]
        elif rest.startswith("gsk_"):
            out.append("gsk_<redacted>")
            rest = rest[4:]
            tok_len = -1
            for i, ch in enumerate(rest):
                if not (ch.isalnum() or ch == "_" or ch == "-"):
                    tok_len = i
                    break
            if tok_len == -1:
                tok_len = len(rest)
            rest = rest[tok_len:]
        elif rest.startswith("sk-") and not (out and (out[-1][-1].isalnum() or out[-1][-1] == "_")):
            out.append("sk-<redacted>")
            rest = rest[3:]
            tok_len = -1
            for i, ch in enumerate(rest):
                if not (ch.isalnum() or ch == "_" or ch == "-"):
                    tok_len = i
                    break
            if tok_len == -1:
                tok_len = len(rest)
            rest = rest[tok_len:]
        else:
            out.append(rest[0])
            rest = rest[1:]
    return "".join(out)


class PrivacySecretsTokenClearFixtures(unittest.TestCase):
    def test_redact_secrets_double_space_bearer_leak_bug(self):
        # When two spaces follow Bearer, the second space makes find(is_whitespace) == 0, leaking the token
        sample = "Authorization: Bearer  secret_live_key_value"
        result = redact_secrets_mock(sample)
        self.assertIn("secret_live_key_value", result)
        self.assertEqual(result, "Authorization: Bearer <redacted> secret_live_key_value")

    def test_redact_secrets_source_matches_only_bearer_gsk_and_sk(self):
        text = source("slint-experiment/src/bin/overlay_host/diagnostics.rs")
        body = text[text.index("pub(crate) fn redact_secrets("):text.index("pub(crate) fn redact_user_home(")]
        # Matches exact string literals
        self.assertIn('rest.starts_with("Bearer ")', body)
        self.assertIn('rest.starts_with("gsk_")', body)
        self.assertIn('rest.starts_with("sk-")', body)
        # Omission of other vendor token prefixes
        self.assertNotIn("ghp_", body)
        self.assertNotIn("hf_", body)
        self.assertNotIn("xai-", body)
        self.assertNotIn("anthropic", body.lower())

    def test_redact_secrets_case_sensitivity_and_delimiters(self):
        # Case variations and tabs/colons are unredacted
        self.assertEqual(redact_secrets_mock("bearer 12345678"), "bearer 12345678")
        self.assertEqual(redact_secrets_mock("BEARER 12345678"), "BEARER 12345678")
        self.assertEqual(redact_secrets_mock("Bearer:\t12345678"), "Bearer:\t12345678")

    def test_on_ai_bearer_save_returns_on_empty_input_preventing_clear(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("win.on_ai_bearer_save(move |new_value| {"):text.index("win.on_openai_key_save(")]
        self.assertIn("let trimmed = new_value.trim().to_string();", body)
        self.assertIn("if trimmed.is_empty() {\n                eprintln!(\"[overlay-host] ai_bearer save skipped: empty input\");\n                return;\n            }", body)

    def test_on_groq_api_key_save_returns_on_empty_input_preventing_clear(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("win.on_groq_api_key_save(move |new_value| {"):text.index("win.on_ai_base_url_save(")]
        self.assertIn("let trimmed = new_value.trim().to_string();", body)
        self.assertIn("if trimmed.is_empty() {\n                eprintln!(\"[overlay-host] groq_api_key save skipped: empty input\");\n                return;\n            }", body)

    def test_original_c03_c10_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker1_privacy-C03"], "confirmed")
        self.assertEqual(found["wave3_worker4_settings-C10"], "confirmed")


if __name__ == "__main__":
    unittest.main()
