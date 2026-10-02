"""Source fixtures for wave1_worker3_config C06 and wave2_worker1_audio C05.
No live audio, no CoreAudio FFI, no Windows Credential Manager interaction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SecretZeroizationMacosRecoveryFixtures(unittest.TestCase):
    def test_windows_credentials_read_does_not_zero_credential_blob_before_credfree(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("pub fn read("):text.index("pub fn delete(")]
        # Reads bytes from credential.CredentialBlob into Vec and converts to String
        self.assertIn("std::slice::from_raw_parts(", body)
        self.assertIn("credential.CredentialBlob", body)
        self.assertIn("String::from_utf8(bytes.to_vec())", body)
        # Calls CredFree without zeroizing CredentialBlob memory first
        self.assertIn("CredFree(raw.cast())", body)
        self.assertNotIn("fill(0)", body)
        self.assertNotIn("zeroize", body.lower())

    def test_windows_credentials_write_zeros_blob_after_credwrite_without_drop_guard(self):
        text = source("overlay-backend/src/credentials.rs")
        body = text[text.index("pub fn write("):text.index("pub fn read(")]
        self.assertIn("let mut blob = trimmed.as_bytes().to_vec();", body)
        # blob.fill(0) only executed after CredWriteW, not via ZeroizeOnDrop wrapper
        idx_write = body.index("CredWriteW(&credential, 0)")
        idx_fill = body.rindex("blob.fill(0)")
        self.assertLess(idx_write, idx_fill)
        self.assertNotIn("ZeroizeOnDrop", body)

    def test_secret_redacted_clears_strings_retaining_capacity_without_zeroization(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("fn secret_redacted(cfg: &Config) -> Config {"):text.index("pub fn export_to(")]
        # Clones full config and calls .clear() on secret strings
        self.assertIn("let mut c = cfg.clone();", body)
        self.assertIn("c.ai_bearer.clear();", body)
        self.assertIn("c.groq_api_key.clear();", body)
        # String::clear keeps allocated buffer capacity without zeroing memory
        self.assertNotIn("zeroize", body.lower())

    def test_macos_reopen_system_has_no_retry_stop_on_error(self):
        text = source("overlay-backend/src/audio_macos.rs")
        reopen_sys = text[text.index("fn reopen_system("):text.index("fn capture_worker(")]
        # reopen_system retries forever until stop: no RetryResult::Stop branch
        self.assertIn("RetryResult::Success((controller, native_rate))", reopen_sys)
        self.assertIn("RetryResult::Retry", reopen_sys)
        self.assertNotIn("RetryResult::Stop", reopen_sys)

    def test_macos_reopen_mic_stops_on_permission_error_in_contrast(self):
        text = source("overlay-backend/src/audio_macos.rs")
        reopen_mic = text[text.index("fn reopen_mic("):text.index("fn reopen_system(")]
        # reopen_mic contrasts by explicitly returning RetryResult::Stop on permission denial
        self.assertIn("error_code == MIC_START_PERMISSION", reopen_mic)
        self.assertIn("RetryResult::Stop", reopen_mic)

    def test_original_c06_c05_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker3_config-C06"], "hypothesis")
        self.assertEqual(found["wave2_worker1_audio-C05"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
