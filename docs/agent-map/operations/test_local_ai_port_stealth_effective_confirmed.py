"""Source fixtures for wave2_worker4_local_ai C01 and wave3_worker2_window C11 (confirmed mechanisms).
No network calls, no process spawning, no live display server interaction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class LocalAiPortStealthEffectiveFixtures(unittest.TestCase):
    def test_is_reachable_uses_curl_without_fail_flag(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn is_reachable(url: &str) -> bool {"):text.index("fn models_list_expected_model")]
        # curl -s -o dev_null --max-time 2 without -f (fail flag)
        self.assertIn('"curl.exe"', text)
        self.assertIn('&["-s", "-o", dev_null(), "--max-time", "2", url]', body)
        self.assertNotIn('"-f"', body)
        self.assertNotIn('"--fail"', body)

    def test_ensure_llama_serving_returns_switched_with_empty_children_if_reachable(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("pub fn ensure_llama_serving("):text.index("pub fn restart_llama_server(")]
        # If llama_reachable() is true, immediately returns (ModelSwitch::Switched, Vec::new())
        self.assertIn("if llama_reachable() {", body)
        self.assertIn("return (ModelSwitch::Switched, Vec::new());", body)

    def test_ensure_servers_for_route_skips_spawn_when_models_endpoint_is_reachable(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn ensure_servers_for_route("):text.index("fn llama_server_args(")]
        self.assertIn("if want_llama && !is_reachable(&format!(\"{LLAMA_BASE_URL}/models\")) {", body)

    def test_stealth_supported_is_hardcoded_const_bool(self):
        text_win = source("slint-experiment/src/win32.rs")
        # On Windows: returns true const
        self.assertIn("pub const fn stealth_supported() -> bool {\n        true\n    }", text_win)
        # On non-Windows: returns false const
        self.assertIn("pub const fn stealth_supported() -> bool {\n        false\n    }", text_win)
        # Does not query Windows OS build number (build 2004 check)
        body = text_win[text_win.index("pub const fn stealth_supported() -> bool {"):text_win.index("pub fn set_stealth(")]
        self.assertNotIn("RtlGetVersion", body)
        self.assertNotIn("GetVersionEx", body)

    def test_global_stealth_effective_only_records_bar_wda_outcome(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        self.assertIn("static STEALTH_EFFECTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);", text)
        body = text[text.index("pub(crate) fn set_global_stealth_effective(on: bool)"):text.index("pub(crate) fn surface_stealth_unavailable(")]
        self.assertIn("STEALTH_EFFECTIVE.store(on, std::sync::atomic::Ordering::Relaxed);", body)
        # Confirms documented limitation: per-window exclusion failures are not aggregated into STEALTH_EFFECTIVE
        self.assertIn("verifies the BAR's WDA only", text)
        self.assertIn("exclusion failures are logged", text)
        self.assertIn("apply_stealth_one", text)

    def test_original_c01_c11_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C01"], "confirmed")
        self.assertEqual(found["wave3_worker2_window-C11"], "confirmed")


if __name__ == "__main__":
    unittest.main()
