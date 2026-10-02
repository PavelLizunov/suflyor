"""Source fixtures for wave3_worker1_bridge C07 and wave3_worker4_settings C09.
No UI thread blocking, no live audio capture, no network connection tests.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class UiBlockingAndErrorLeakFixtures(unittest.TestCase):
    def test_start_session_inner_performs_synchronous_model_validation(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("fn start_session_inner("):text.index("log_info(&format!(")]
        # Validates GigaAM model directory synchronously (~0.5s) on the calling thread
        self.assertIn("stt::validate_gigaam_dir(model_dir)", body)
        self.assertIn("One-shot blocking load (~0.5s)", text)

    def test_stop_session_blocks_on_journal_shutdown_with_timeout(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("pub fn stop_session("):text.index("if cfg.read().session_archive_enabled {")]
        # Synchronously blocks on journal shutdown up to 3s
        self.assertIn("j.shutdown(Duration::from_secs(3))", body)

    def test_start_session_inner_synchronously_starts_audio_capture(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("fn start_session_inner("):text.index("// ===== 4b. Pause gate")]
        # Synchronously starts audio capture on the calling thread
        self.assertIn("audio::start_capture(mic_dev, sys_dev)", body)

    def test_connection_test_formats_raw_error_chain_truncated_to_90_chars(self):
        text_ai = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body_ai = text_ai[text_ai.index("win.on_ai_bridge_test_clicked("):text_ai.index("#[cfg(test)]")]
        # format!("[err] {e:#}").chars().take(90).collect()
        self.assertIn('format!("[err] {e:#}").chars().take(90).collect()', body_ai)

        text_stt = source("slint-experiment/src/bin/overlay_host/settings_stt.rs")
        body_stt = text_stt[text_stt.index("win.on_stt_test_clicked("):text_stt.index("win.on_stt_provider_changed(")]
        self.assertIn('format!("[err] {e:#}").chars().take(90).collect()', body_stt)

    def test_mic_test_interpolates_raw_error_message(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("win.on_mic_test_clicked("):text.index("win.on_always_on_top_changed(")]
        self.assertIn('Err(e) => format!("error: {e}"),', body)

    def test_original_c07_c09_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker1_bridge-C07"], "hypothesis")
        self.assertEqual(found["wave3_worker4_settings-C09"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
