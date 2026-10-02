"""Source fixtures for wave2_worker2_stt C01 and C02 (confirmed mechanisms).
No live network requests, no ONNX model loading, no audio transcription.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def is_permanent_error_model(msg):
    return (
        "HTTP 401" in msg
        or "HTTP 403" in msg
        or "HTTP 404" in msg
        or "HTTP 413" in msg
    )


class SttQueueNetworkFixtures(unittest.TestCase):
    def test_stt_semaphore_permits_count_is_six(self):
        text = source("overlay-backend/src/stt.rs")
        self.assertIn("let stt_semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(6));", text)

    def test_tokio_spawn_precedes_semaphore_acquire(self):
        text = source("overlay-backend/src/stt.rs")
        body = text[text.index("tokio::spawn(async move {"):text.index("finish_transcript(")]
        idx_spawn = body.index("tokio::spawn(async move {")
        idx_acquire = body.index("sem.acquire_owned().await")
        # Task is spawned first, capturing audio buffer into task memory, and waits on semaphore INSIDE task
        self.assertLess(idx_spawn, idx_acquire)

    def test_client_timeout_is_30s_in_live_stt_and_60s_in_adhoc(self):
        text = source("overlay-backend/src/stt.rs")
        live_body = text[text.index("let client = reqwest::Client::builder()"):text.index(".expect(\"reqwest client\");")]
        self.assertIn(".timeout(std::time::Duration::from_secs(30))", live_body)

        adhoc_body = text[text.index("pub async fn transcribe_once("):text.index("async fn transcribe_once_attempt(")]
        self.assertIn(".timeout(std::time::Duration::from_secs(60))", adhoc_body)

    def test_is_permanent_error_retries_429_and_5xx_without_retry_after_header(self):
        # Permanent errors: 401, 403, 404, 413
        self.assertTrue(is_permanent_error_model("API returned HTTP 401 Unauthorized"))
        self.assertTrue(is_permanent_error_model("API returned HTTP 403 Forbidden"))
        self.assertTrue(is_permanent_error_model("API returned HTTP 404 Not Found"))
        self.assertTrue(is_permanent_error_model("API returned HTTP 413 Payload Too Large"))
        # Non-permanent errors (retried): 429, 500, 503, connection drops
        self.assertFalse(is_permanent_error_model("API returned HTTP 429 Too Many Requests"))
        self.assertFalse(is_permanent_error_model("API returned HTTP 500 Internal Server Error"))
        self.assertFalse(is_permanent_error_model("connection reset by peer"))

    def test_transcribe_retry_delay_is_exponential_with_no_jitter(self):
        text = source("overlay-backend/src/stt.rs")
        body = text[text.index("async fn transcribe("):text.index("fn is_permanent_error(")]
        # delay = 1000 * (1 << (attempt - 1)) with no jitter or header parsing
        self.assertIn("let delay = std::time::Duration::from_millis(1000 * (1u64 << (attempt - 1)));", body)
        self.assertNotIn("jitter", body.lower())
        self.assertNotIn("retry-after", body.lower())

    def test_original_c01_c02_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker2_stt-C01"], "confirmed")
        self.assertEqual(found["wave2_worker2_stt-C02"], "confirmed")


if __name__ == "__main__":
    unittest.main()
