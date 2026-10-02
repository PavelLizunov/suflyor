"""Source fixtures for wave2_worker1_audio C03 and C09 (confirmed mechanisms).
No live WASAPI audio capture, no device initialization, no hardware interaction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class AudioEventLoopRecoveryFixtures(unittest.TestCase):
    def test_wait_for_event_timeout_logs_heartbeat_and_continues(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("if event.wait_for_event(250).is_err() {"):text.index("byte_q.clear();")]
        # Timeout branches to continue without exiting loop or transitioning to recover
        self.assertIn("log::info!(", body)
        self.assertIn("audio_no_frame heartbeat idle_s=", body)
        self.assertIn("action=wait", body)
        self.assertIn("continue;", body)
        self.assertNotIn("CaptureExit::Recover", body)
        self.assertNotIn("break", body)

    def test_capture_thread_only_checks_device_invalidation_on_read(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("byte_q.clear();"):text.index("while byte_q.len() >= 4 {")]
        # Read from device is the only place error from cap_client is checked and returned
        self.assertIn("if let Err(e) = cap_client.read_from_device_to_deque(&mut byte_q) {", body)
        self.assertIn("return Err(e).context(\"read_from_device_to_deque\");", body)

    def test_capture_with_recovery_always_returns_ok_after_loop(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn capture_with_recovery("):text.index("enum CaptureExit")]
        # Terminal return statement is Ok(())
        self.assertTrue(any("Ok(())" in line for line in body.strip().splitlines()[-3:]))
        # Errors from capture_thread are caught and retried in sleep(1s) loop
        self.assertIn("Err(e) if !stop.load(Ordering::Acquire) => {", body)
        self.assertIn("recovery_attempt = recovery_attempt.saturating_add(1);", body)
        self.assertIn("thread::sleep(Duration::from_secs(1));", body)

    def test_start_capture_returns_ok_immediately_without_synchronous_device_open(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("pub fn start_capture("):text.index("fn capture_with_recovery(")]
        # Spawns threads for System and Mic and immediately returns Ok((rx, CaptureHandle { stop }))
        self.assertIn("thread::Builder::new()", body)
        self.assertIn("capture_with_recovery(", body)
        self.assertIn("Ok((rx, CaptureHandle { stop }))", body)
        self.assertNotIn(".join()", body)

    def test_capture_exit_stopped_breaks_recovery_loop(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn capture_with_recovery("):text.index("fn capture_thread(")]
        self.assertIn("Ok(CaptureExit::Stopped) => break,", body)
        self.assertIn("selection = DeviceSelection::from_configured(device_name);", body)

    def test_original_c03_c09_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker1_audio-C03"], "confirmed")
        self.assertEqual(found["wave2_worker1_audio-C09"], "confirmed")


if __name__ == "__main__":
    unittest.main()
