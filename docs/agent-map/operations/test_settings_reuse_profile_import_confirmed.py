"""Source fixtures for wave3_worker4_settings C01 and C08 (confirmed mechanisms).
No GUI event loops, no file dialogs, no disk configuration overwriting.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class SettingsReuseProfileImportFixtures(unittest.TestCase):
    def test_reused_settings_window_path_skips_persistent_control_seeding(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        reused_body = text[text.index("if let Some(existing) = settings_slot.as_ref() {"):text.index("let win = match SettingsWindow::new()")]
        # Reused branch calls populate_token_status, refresh_profiles, populate_component_rows, reveal
        self.assertIn("populate_token_status(existing, cfg);", reused_body)
        self.assertIn("refresh_profiles(existing, &snap);", reused_body)
        # But DOES NOT reseed coaching, retention, trigger keywords, or auto-tiles
        self.assertNotIn("set_coaching_debrief", reused_body)
        self.assertNotIn("set_coaching_live_tiles", reused_body)
        self.assertNotIn("set_record_audio", reused_body)
        self.assertNotIn("set_auto_tiles_enabled", reused_body)
        self.assertNotIn("set_trigger_keywords_input", reused_body)
        self.assertNotIn("set_retention_mode", reused_body)
        self.assertNotIn("set_retention_value", reused_body)

    def test_fresh_settings_window_seeds_all_persistent_controls(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        fresh_body = text[text.index("let win = match SettingsWindow::new()"):text.index("win.on_component_install(move |idx| {")]
        # Fresh construction branch seeds all of these
        self.assertIn("win.set_coaching_debrief(snap.post_meeting_debrief_enabled);", fresh_body)
        self.assertIn("win.set_coaching_live_tiles(snap.live_coaching_tiles_enabled);", fresh_body)
        self.assertIn("win.set_record_audio(snap.record_audio_enabled);", fresh_body)
        self.assertIn("win.set_auto_tiles_enabled(snap.auto_tiles_enabled);", fresh_body)
        self.assertIn("win.set_suppress_tiles(snap.suppress_tiles);", fresh_body)
        self.assertIn("win.set_trigger_keywords_input(SharedString::from(snap.trigger_keywords.as_str()));", fresh_body)
        self.assertIn("win.set_retention_mode(mode);", fresh_body)
        self.assertIn("win.set_retention_value(SharedString::from(value));", fresh_body)

    def test_import_profile_clicked_overwrites_shared_config_without_path_filtering(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("win.on_import_profile_clicked(move || {"):text.index("wire_import_export(")]
        # Assigns full imported struct into live shared config
        self.assertIn("*cfg_c.write() = imported;", body)
        # Calls msg_refresh_after_import
        self.assertIn("msg_refresh_after_import(&w, &cfg_c)", body)
        # Formats raw error chain if import fails
        self.assertIn('Err(e) => format!("[err] {e:#}")', body)

    def test_msg_refresh_after_import_only_refreshes_token_status(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn msg_refresh_after_import("):text.index("pub(crate) fn component_row_copy(")]
        self.assertIn("populate_token_status(win, cfg);", body)
        # Does not call refresh_profiles or reseed controls after full import
        self.assertNotIn("refresh_profiles", body)
        self.assertNotIn("set_coaching", body)
        self.assertNotIn("set_retention", body)

    def test_export_profile_and_server_logs_interpolate_path_display(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        profile_export = text[text.index("win.on_export_profile_clicked("):text.index("win.on_import_profile_clicked(")]
        self.assertIn('format!("[ok] exported to {}", path.display())', profile_export)

        text_exp = source("slint-experiment/src/bin/overlay_host/settings_import_export.rs")
        self.assertIn('format!("[ok] server settings exported to {}", path.display())', text_exp)

    def test_populate_token_status_seeds_raw_whisper_bearer(self):
        text = source("slint-experiment/src/bin/overlay_host/settings_controller.rs")
        body = text[text.index("pub(crate) fn populate_token_status("):text.index("#[cfg(test)]")]
        # Whisper bearer is set into input property verbatim
        self.assertIn("win.set_stt_whisper_bearer_input(SharedString::from(c.stt_whisper_bearer.clone()));", body)

    def test_original_c01_c08_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker4_settings-C01"], "confirmed")
        self.assertEqual(found["wave3_worker4_settings-C08"], "confirmed")


if __name__ == "__main__":
    unittest.main()
