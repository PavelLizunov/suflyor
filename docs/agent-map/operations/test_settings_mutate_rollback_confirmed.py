"""Source fixtures for wave3_worker4_settings C05 and C06 (confirmed mechanisms).
No GUI event loops, no real config writes, no display/translation mutations.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SettingsMutateRollbackFixtures(unittest.TestCase):
    def test_ai_provider_changed_mutates_before_save_without_rollback(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("win.on_ai_provider_changed(move |idx| {"):text.index("let codex_needed =")]
        # Mutates c.ai_provider and other fields in RwLock write guard before save
        self.assertIn("let mut c = cfg_c.write();", body)
        self.assertIn("c.ai_provider = provider.to_string();", body)
        # On save failure, simply prints error and returns without reverting c
        self.assertIn("if let Err(e) = overlay_backend::config::save(&c) {", body)
        self.assertIn("return;", body)
        self.assertNotIn("previous", body)

    def test_stt_cloud_model_contrasts_by_implementing_explicit_rollback(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_stt.rs")
        body = text[text.index("fn update_cloud_model<E>("):text.index("pub(crate) fn wire_stt_settings(")]
        # Replaces and retains previous, rolling back if save fails
        self.assertIn("let previous = std::mem::replace(", body)
        self.assertIn("config.stt_model = previous;", body)

    def test_language_selected_applies_translation_before_save(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("win.on_language_selected(move |idx| {"):text.index("win.on_color_scheme_selected(move |idx| {")]
        idx_translation = body.index("slint::select_bundled_translation(lang)")
        idx_save = body.index("overlay_backend::config::save(&c)")
        # Translation is switched live in UI memory before persisting to config
        self.assertLess(idx_translation, idx_save)

    def test_tile_monitor_changed_applies_pin_to_runtime_before_save(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("win.on_tile_monitor_changed(move |idx| {"):text.index("win.on_language_selected(")]
        idx_pin = body.index("set_global_tile_monitor(pin);")
        idx_save = body.index("overlay_backend::config::save(&c)")
        # Applies pin to runtime global before disk save
        self.assertLess(idx_pin, idx_save)

    def test_stealth_changed_applies_to_windows_regardless_of_save_error(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("win.on_stealth_changed(move |on| {"):text.index("win.on_tile_monitor_changed(")]
        # Stealth is persisted best-effort: let _ = config::save(&c)
        self.assertIn("let _ = config::save(&c);", body)
        self.assertIn("set_global_stealth(on);", body)
        self.assertIn("registry_stealth.apply_stealth(on);", body)

    def test_original_c05_c06_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker4_settings-C05"], "confirmed")
        self.assertEqual(found["wave3_worker4_settings-C06"], "confirmed")


if __name__ == "__main__":
    unittest.main()
