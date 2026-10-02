"""Source and mock fixtures for wave1_worker3_config C03 and C04 (confirmed mechanisms).
No live credentials written, no external logging, no live filesystem corruption.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class ConfigDiagnosticsBackupFixtures(unittest.TestCase):
    def test_readiness_report_embeds_raw_base_url_and_paths_without_masking(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn readiness(&self) -> ReadinessReport {"):text.index("pub enum SttBackendCfg {")]
        # ai_detail formats raw ep.base_url and ep.model without mask_host
        self.assertIn('format!("{} · {} · {}", provider, ep.base_url, ep.model)', body)
        # stt_detail formats raw stt_gigaam_dir without path redaction
        self.assertIn('format!("gigaam · {}", self.stt_gigaam_dir)', body)
        # stt_detail formats raw stt_whisper_url without mask_host
        self.assertIn('format!("whisper · {}", self.stt_whisper_url)', body)
        self.assertNotIn("mask_host", body)

    def test_preview_server_settings_copies_raw_urls_and_gigaam_dir(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub fn preview_server_settings("):text.index("pub fn preview_server_settings_from(")]
        # Copies raw URLs into preview structs
        self.assertIn("gigaam_dir_current: current.stt_gigaam_dir.clone(),", body)
        self.assertIn("gigaam_dir_incoming: imported.stt_gigaam_dir.clone(),", body)
        self.assertNotIn("mask_host", body)

    def test_save_to_path_bak_write_omits_unix_mode_permissions(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("if path.exists() {"):text.index("std::fs::rename(&tmp, path)")]
        # live .tmp uses mode(0o600) under #[cfg(unix)]
        tmp_body = text[text.index("let tmp = path.with_extension(\"json.tmp\");"):text.index("if path.exists() {")]
        self.assertIn(".mode(0o600)", tmp_body)
        # .bak uses standard fs::write without mode(0o600) or PermissionsExt
        self.assertIn("let bak = path.with_extension(\"json.bak\");", body)
        self.assertIn("std::fs::write(&bak, redacted)", body)
        self.assertNotIn(".mode(0o600)", body)

    def test_preserve_corrupt_config_renames_without_redaction_or_retention(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("fn preserve_corrupt_config(path: &std::path::Path) {"):text.index("pub fn load() -> Config {")]
        # Renames raw unparseable config to json.broken-<ts>
        self.assertIn('path.with_extension(format!("json.broken-{ts}"))', body)
        self.assertIn("std::fs::rename(path, &backup)", body)
        # No redaction or retention pruning of previous broken-* files
        self.assertNotIn("secret_redacted", body)
        self.assertNotIn("prune", body)
        self.assertNotIn("read_dir", body)

    def test_save_to_path_atomic_rename_flow(self):
        text = source("overlay-backend/src/config.rs")
        body = text[text.index("pub(crate) fn save_to_path("):text.index("pub fn save(")]
        self.assertIn('let tmp = path.with_extension("json.tmp");', body)
        self.assertIn("std::fs::rename(&tmp, path)", body)

    def test_original_c03_c04_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker3_config-C03"], "confirmed")
        self.assertEqual(found["wave1_worker3_config-C04"], "confirmed")


if __name__ == "__main__":
    unittest.main()
