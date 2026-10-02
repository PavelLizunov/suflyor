"""Source fixtures for wave2_worker4_local_ai C08 and C09.
No hardware queries, no live process spawning, no VRAM allocations.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class LocalAiHardwareContextFixtures(unittest.TestCase):
    def test_unknown_profile_skips_ngl_and_np_on_windows(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn llama_server_args("):text.index("if let Some(projector) = mmproj")]
        # Windows unknown profile does not push -ngl or -np unless force_cpu is set
        self.assertIn("else if profile != HardwareModelProfile::Unknown || cfg!(target_os = \"macos\")", body)
        self.assertIn('args.extend([\n            "-ngl".to_string(),', body)
        self.assertIn('"-np".to_string(),\n            "1".to_string(),', body)

    def test_vram_normalization_passes_unmatched_values_through(self):
        text = source("overlay-backend/src/local_ai/hardware_profile.rs")
        body = text[text.index("pub(super) fn normalize_vram_gib"):text.index("pub(super) fn normalize_ram_gib")]
        # Only 7..=9 -> 8, 11..=13 -> 12, 15..=17 -> 16; others pass through
        self.assertIn("7..=9 => 8,", body)
        self.assertIn("11..=13 => 12,", body)
        self.assertIn("15..=17 => 16,", body)
        self.assertIn("_ => raw,", body)

    def test_select_hardware_model_profile_strict_matrix_matching(self):
        text = source("overlay-backend/src/local_ai/hardware_profile.rs")
        body = text[text.index("pub const fn select_hardware_model_profile"):text.index("pub(super) fn normalize_vram_gib")]
        # Match table for (vram, ram)
        self.assertIn("(16, 32..) => HardwareModelProfile::Primary26Vram16,", body)
        self.assertIn("(12, 24..) => HardwareModelProfile::Primary26Vram12,", body)
        self.assertIn("(8, 32..) => HardwareModelProfile::Primary26Vram8,", body)
        self.assertIn("(8, 16..=31) => HardwareModelProfile::Fallback12B,", body)
        self.assertIn("_ => HardwareModelProfile::Unknown,", body)

    def test_context_tokens_ignores_prep_flag(self):
        text = source("overlay-backend/src/local_ai/model_choice.rs")
        body = text[text.index("pub fn context_tokens(self, profile: HardwareModelProfile, _prep: bool) -> u32"):text.index("pub fn estimated_vram_delta_mib")]
        # Leading underscore on _prep indicates unused argument
        self.assertIn("profile: HardwareModelProfile, _prep: bool", body)
        # Always delegates to safe_live with false
        self.assertIn("let safe_live = profile.context_tokens(false);", body)

    def test_prep_mode_enables_quantized_kv_for_vram8_and_vram12(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn llama_server_args("):text.index("if let Some(projector) = mmproj")]
        # prep adds -ctk q8_0 -ctv q8_0 for Vram8 and Vram12 only
        self.assertIn("if prep\n        && matches!(\n            profile,\n            HardwareModelProfile::Primary26Vram8 | HardwareModelProfile::Primary26Vram12\n        )", body)
        self.assertIn('"-ctk".to_string(),\n            "q8_0".to_string(),', body)
        self.assertIn('"-ctv".to_string(),\n            "q8_0".to_string(),', body)

    def test_original_c08_c09_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C08"], "hypothesis")
        self.assertEqual(found["wave2_worker4_local_ai-C09"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
