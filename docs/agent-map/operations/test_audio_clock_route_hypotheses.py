"""Pure audio clock/route models for C02/C04. No devices or Rust execution."""
import json
import math
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def resample_length(input_len, ratio):
    if input_len == 0 or ratio <= 0:
        return 0
    if abs(ratio - 3.0) < 1e-6:
        return input_len // 3
    return math.floor(input_len / ratio)


def decide(selection, flow, role, event):
    if event == "device_added":
        return "unrelated"
    if event == "default_changed" and selection == "pinned":
        return "ignore_pinned"
    if event == "default_changed" and selection == "default" and flow == "render" and role == "console":
        return "recover"
    return "unrelated"


class AudioClockRouteFixtures(unittest.TestCase):
    def test_non_integer_ratio_drops_remainder(self):
        self.assertEqual(resample_length(48000, 2.999), math.floor(48000 / 2.999))
        self.assertGreater(resample_length(48000, 2.999), 16000)
        self.assertEqual(resample_length(48002, 3.0), 16000)

    def test_timestamp_uses_elapsed_not_sample_position(self):
        text = source("overlay-backend/src/audio.rs")
        self.assertIn("timestamp_ms: start_ts.elapsed().as_millis() as u64", text)
        mac = source("overlay-backend/src/audio_macos.rs")
        self.assertIn("session_start.elapsed().as_millis() as u64", mac)
        self.assertNotIn("IAudioClock", text)

    def test_communications_role_and_device_added_are_unrelated(self):
        self.assertEqual(decide("default", "render", "communications", "default_changed"), "unrelated")
        self.assertEqual(decide("default", "render", "console", "device_added"), "unrelated")
        self.assertEqual(decide("pinned", "render", "console", "default_changed"), "ignore_pinned")
        self.assertEqual(decide("default", "render", "console", "default_changed"), "recover")

    def test_source_maps_non_console_roles_and_ignores_added_devices(self):
        text = source("overlay-backend/src/audio_route.rs")
        self.assertIn("if role == eConsole", text)
        self.assertIn("Self::Other", text)
        callback = text[text.index("fn OnDeviceAdded"):text.index("fn OnDeviceRemoved")]
        self.assertNotIn("send(", callback)
        marker = "*role == RouteRole::Console"
        self.assertIn(marker, text)

    def test_retry_has_no_attempt_ceiling(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn capture_with_recovery"):text.index("enum CaptureExit")]
        self.assertIn("thread::sleep(Duration::from_secs(1))", body)
        self.assertNotIn("MAX_ATTEMPTS", body)

    def test_original_c02_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker1_audio-C02"], "hypothesis")
        self.assertEqual(found["wave2_worker1_audio-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
