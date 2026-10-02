"""Source fixtures for wave3_worker4_settings C03 and C04.
No UI thread execution, no network requests to model endpoints, no disk writes.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SettingsModelsCodexFixtures(unittest.TestCase):
    def test_fetch_models_has_no_generation_check(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("pub(crate) fn fetch_models("):text.index("pub(crate) fn refresh_local_model_resource_warning(")]
        # Spawns thread, queries models, then calls invoke_from_event_loop directly without a generation counter
        self.assertIn("std::thread::spawn(move || {", body)
        self.assertIn("slint::invoke_from_event_loop(move || {", body)
        self.assertNotIn("generation", body)
        self.assertNotIn("AtomicU64", body)

    def test_codex_snapshot_ui_contrasts_by_enforcing_generation_fencing(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        self.assertIn("static CODEX_SNAPSHOT_UI_GENERATION: AtomicU64 = AtomicU64::new(0);", text)
        body = text[text.index("pub(crate) fn refresh_codex_account_status("):text.index("pub(crate) fn fetch_models(")]
        self.assertIn("let generation = invalidate_codex_snapshot_ui();", body)
        self.assertIn("if codex_snapshot_ui_is_current(generation)", body)

    def test_refresh_codex_account_status_can_save_config_on_refresh(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("pub(crate) fn refresh_codex_account_status("):text.index("pub(crate) fn fetch_models(")]
        # When saved model or effort is empty/different, it mutates config and calls config::save(&c)
        self.assertIn("c.codex_model = id;", body)
        self.assertIn("c.codex_reasoning_effort = normalized_effort.clone();", body)
        self.assertIn("if overlay_backend::config::save(&c).is_err() {", body)
        # Also auto-populates vision model if saved_vision is empty
        self.assertIn("c.codex_vision_model = id;", body)

    def test_refresh_codex_called_on_settings_window_open(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn open_settings("):text.index("fn reset_component_install_state(")]
        # refresh_codex_account_status is invoked when settings window opens (both new and reused)
        self.assertIn("refresh_codex_account_status(existing.as_weak(), cfg.clone());", body)
        self.assertIn("refresh_codex_account_status(win.as_weak(), cfg.clone());", body)

    def test_fetch_models_targets_cloud_and_local_dropdowns_directly(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_ai.rs")
        body = text[text.index("pub(crate) fn fetch_models("):text.index("pub(crate) fn refresh_local_model_resource_warning(")]
        self.assertIn("ModelTarget::Cloud => {", body)
        self.assertIn("w.set_ai_models(model);", body)
        self.assertIn("ModelTarget::Local => {", body)
        self.assertIn("w.set_ai_local_models(model);", body)

    def test_original_c03_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker4_settings-C03"], "hypothesis")
        self.assertEqual(found["wave3_worker4_settings-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
