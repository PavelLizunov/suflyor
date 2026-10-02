"""Source fixtures for wave1_worker3_config C08 and C09 (confirmed mechanisms).
No filesystem mutation of real configs, no external logging, no live process state.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class ConfigMergeLoggingFixtures(unittest.TestCase):
    def test_merge_server_settings_clobbers_gigaam_dir_from_imported(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn merge_server_settings("):text.index("pub fn import_server_settings_from(")]
        # Unconditionally copies imported.stt_gigaam_dir and stt_gigaam_gpu
        self.assertIn("next.stt_gigaam_dir = imported.stt_gigaam_dir;", body)
        self.assertIn("next.stt_gigaam_gpu = imported.stt_gigaam_gpu;", body)

    def test_apply_server_settings_contrasts_by_preserving_current_gigaam_dir(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn apply_server_settings("):text.index("pub fn shared_from(")]
        # apply_server_settings explicitly overrides gigaam_dir with current (preserving local machine path)
        self.assertIn("let mut next = merge_server_settings(current, imported);", body)
        self.assertIn("next.stt_gigaam_dir = current.stt_gigaam_dir.clone();", body)

    def test_import_server_settings_from_calls_merge_not_apply_before_save(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn import_server_settings_from("):text.index("pub fn export_server_settings_to(")]
        # Calls merge_server_settings directly and saves it
        self.assertIn("let next = merge_server_settings(current, imported);", body)
        self.assertIn('save(&next).context("persist imported server settings")?;', body)

    def test_config_path_logging_includes_raw_filesystem_path(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn config_path() -> Result<PathBuf> {"):text.index("fn parse_config_bytes(")]
        # Logs the unredacted directory path display
        self.assertIn('format!("create config dir {}", dir.display())', body)

    def test_load_resets_to_defaults_and_persists_if_migration_sets_dirty(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn load() -> Config {"):text.index("pub(crate) fn save_to_path(")]
        # On parse failure: preserves copy, falls back to Config::defaults(), then saves if dirty
        self.assertIn("preserve_corrupt_config(&path);", body)
        self.assertIn("Config::defaults()", body)
        self.assertIn("if dirty {", body)
        self.assertIn("if let Err(e) = save(&cfg) {", body)

    def test_original_c08_c09_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker3_config-C08"], "confirmed")
        self.assertEqual(found["wave1_worker3_config-C09"], "confirmed")


if __name__ == "__main__":
    unittest.main()
