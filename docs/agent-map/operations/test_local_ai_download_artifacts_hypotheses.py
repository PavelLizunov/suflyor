"""Source fixtures for wave2_worker4_local_ai C06 and C07.
No network requests, no model downloads, no binary extraction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class LocalAiDownloadArtifactsFixtures(unittest.TestCase):
    def test_is_trusted_release_url_allowlist_hosts(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn is_trusted_release_url("):text.index("fn download_and_extract(")]
        self.assertIn('url.starts_with("https://github.com/")', body)
        self.assertIn('url.starts_with("https://objects.githubusercontent.com/")', body)
        self.assertIn('url.starts_with("https://release-assets.githubusercontent.com/")', body)

    def test_download_and_extract_has_no_binary_sha_or_authenticode(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn download_and_extract("):text.index("fn system_tar(")]
        # Flow: is_trusted_release_url -> curl_resumable -> extract_zip -> remove zip
        self.assertIn("curl_resumable(url, &zip, size, label, cancel, on)?;", body)
        self.assertIn("extract_zip(&zip, dest_dir)?;", body)
        # Note absence of SHA-256 calculation or Authenticode verification on the zip or extracted files
        self.assertNotIn("verify_file_sha256", body)
        self.assertNotIn("authenticode", body.lower())

    def test_curl_resumable_stops_on_greater_or_equal_size(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn curl_resumable("):text.index("fn bail_if_cancelled(")]
        self.assertIn("if cur >= expected {", body)
        self.assertIn('"-L"', body)
        self.assertNotIn("--proto", body)

    def test_mutable_main_branch_urls_for_auxiliary_models(self):
        text = source("overlay-backend/src/local_ai.rs")
        # MMPROJ_URL, WHISPER_URL, and GIGAAM_MODEL_URL point to /resolve/main/
        self.assertIn('MMPROJ_URL: &str =\n    "https://huggingface.co/unsloth/gemma-4-12B-it-qat-GGUF/resolve/main/mmproj-F16.gguf";', text)
        self.assertIn('WHISPER_URL: &str =\n    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin";', text)
        self.assertIn('GIGAAM_MODEL_URL: &str =\n    "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc.int8.onnx";', text)

    def test_sha_pins_exist_and_verified_during_install(self):
        text = source("overlay-backend/src/local_ai.rs")
        # SHA pins are defined
        self.assertIn("const MMPROJ_SHA256: &str =", text)
        self.assertIn("const WHISPER_SHA256: &str =", text)
        self.assertIn("const GIGAAM_SHA256: &str =", text)
        # Installed with verify_sha256 in install flow
        body = text[text.index("pub fn install("):text.index("pub fn ensure_servers(")]
        self.assertIn('verify_sha256(&mmproj_dest, MMPROJ_SHA256, "Vision projector")', body)
        self.assertIn('verify_sha256(&whisper_dest, WHISPER_SHA256, "Whisper model")', body)

    def test_original_c06_c07_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C06"], "hypothesis")
        self.assertEqual(found["wave2_worker4_local_ai-C07"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
