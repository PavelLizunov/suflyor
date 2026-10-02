"""Source fixtures for wave3_worker4_settings C02 and C07.
No GUI event loops, no background installers, no thread execution.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SettingsResetSeedingFixtures(unittest.TestCase):
    def test_unconditional_installer_flag_resets_in_populate_token_status(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn populate_token_status("):text.index("#[cfg(test)]")]
        # Unconditionally forces installing = false on reopen for OCR, Diar, TTS, Tera, Updates
        self.assertIn("win.set_ocr_installing(false);", body)
        self.assertIn("win.set_diar_installing(false);", body)
        self.assertIn("win.set_update_checking(false);", body)
        self.assertIn("win.set_tts_installing(false);", body)
        self.assertIn("win.set_tera_installing(false);", body)

    def test_gigaam_installing_exception_guard(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn populate_token_status("):text.index("#[cfg(test)]")]
        # GigaAM is the only installer guarded with a check before overwriting state
        self.assertIn("if !win.get_stt_gigaam_installing() {", body)
        self.assertIn("win.set_stt_gigaam_install_failed(false);", body)

    def test_component_install_busy_reset(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("fn reset_component_install_state(win: &SettingsWindow) {"):text.index("pub(crate) fn refresh_profiles(")]
        self.assertIn("win.set_component_busy_index(-1);", body)
        self.assertIn("win.set_component_busy_phase(0);", body)
        self.assertIn('win.set_component_busy_label(SharedString::from(""));', body)

    def test_missing_seeds_for_local_ai_profile_and_vision_properties(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn populate_token_status("):text.index("#[cfg(test)]")]
        # Neither property is seeded in populate_token_status (relying on Slint default)
        self.assertNotIn("set_ai_local_model_profile_index", body)
        self.assertNotIn("set_ai_local_vision_available", body)

    def test_server_preview_and_context_dictating_partial_resets(self):
        settings_text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = settings_text[settings_text.index("pub(crate) fn populate_token_status("):settings_text.index("#[cfg(test)]")]
        # server_preview_ready is set to false, but preview strings are not cleared
        self.assertIn("win.set_server_preview_ready(false);", body)
        self.assertNotIn("set_server_preview_ai_model", body)
        self.assertNotIn("set_context_processing", body)
        self.assertNotIn("set_context_dictating", body)

    def test_original_c02_c07_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker4_settings-C02"], "hypothesis")
        self.assertEqual(found["wave3_worker4_settings-C07"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
