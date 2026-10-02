"""Source fixtures for wave2_worker2_stt C03 and C04.
No ONNX model loading, no audio transcription, no sidecar execution.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SttDiarizationRuntimeFixtures(unittest.TestCase):
    def test_gigaam_cache_is_process_global_mutex(self):
        text = source("overlay-backend/src/stt.rs")
        self.assertIn("static GIGAAM_CACHE: std::sync::OnceLock<Mutex<Option<(String, SharedGigaamModel)>>>", text)
        body = text[text.index("fn shared_gigaam_model("):text.index("pub fn reset_gigaam_cache()")]
        self.assertIn("GIGAAM_CACHE", body)
        self.assertIn(".get_or_init(|| Mutex::new(None))", body)

    def test_gigaam_load_failure_drops_speech_without_cloud_fallback(self):
        text = source("overlay-backend/src/stt.rs")
        idx_giga = text.index("let gigaam: Option<SharedGigaamModel> =")
        idx_client = text.index("let client = reqwest::Client::builder()", idx_giga)
        body = text[idx_giga:idx_client]
        # On load error, logs and sets gigaam = None
        self.assertIn("STT GigaAM load FAILED", body)
        self.assertIn("None", body)
        # In chunk processing loop, if gigaam is None and backend was Gigaam, http_target is also None
        loop_body = text[text.index("if let Some(model) = &gigaam {"):text.index("let client = client.clone();")]
        self.assertIn("} else if let Some((url, bearer, model)) = http_target.clone() {", loop_body)

    def test_validate_gigaam_dir_calls_shared_gigaam_model_synchronously(self):
        text = source("overlay-backend/src/stt.rs")
        body = text[text.index("pub fn validate_gigaam_dir("):text.index("const DEFAULT_GROQ_MODEL:")]
        self.assertIn("shared_gigaam_model(model_dir).map(|_model| ())", body)

    def test_diarization_run_sidecar_blocks_on_command_output_without_timeout(self):
        text = source("overlay-backend/src/diarize.rs")
        body = text[text.index("fn run_sidecar("):text.index("fn system_windows(")]
        # Invokes Command::new(exe)...output() synchronously without timeout or kill handle
        self.assertIn("let mut cmd = Command::new(exe);", body)
        self.assertIn(".output()", body)
        self.assertNotIn("timeout", body)
        self.assertNotIn("kill", body)

    def test_diarization_max_duration_and_window_cap_constants(self):
        text = source("overlay-backend/src/diarize.rs")
        self.assertIn("const MAX_DIAR_SECS: u64 = 3 * 60 * 60;", text)
        self.assertIn("const WINDOW_CAP_MS: i64 = 30_000;", text)
        guard_body = text[text.index("fn guard_wav_len("):text.index("pub(crate) fn diarization_exe_path()")]
        self.assertIn("if secs > MAX_DIAR_SECS {", guard_body)
        window_body = text[text.index("fn system_windows("):text.index("pub fn align_all(")]
        self.assertIn("let capped = start + WINDOW_CAP_MS;", window_body)

    def test_original_c03_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker2_stt-C03"], "hypothesis")
        self.assertEqual(found["wave2_worker2_stt-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
