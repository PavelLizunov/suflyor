"""Research source-seam assertions for retrospective features; no native tests."""
import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


def source(path):
    return (REPO / path).read_text(encoding="utf-8")


class RetrospectiveSourceTests(unittest.TestCase):
    def test_palette_empty_search_matches_actual_empty_list(self):
        kb = source("overlay-backend/src/kb.rs")
        body = kb[kb.index("pub fn search("):kb.index("pub fn get(")]
        self.assertIn("const MAX_QUERY_CHARS: usize = 200", body)
        self.assertIn("trimmed.chars().take(MAX_QUERY_CHARS)", body)
        self.assertIn("q.is_empty() || limit == 0", body)
        self.assertIn('kb::search("", 20)', source("slint-experiment/src/bin/overlay_host/aux_windows/help_palette.rs"))

    def test_kb_reference_budget_counts_utf8_bytes(self):
        text = source("overlay-backend/src/kb.rs")
        body = text[text.index("pub fn reference_for("):text.index("#[cfg(test)]")]
        self.assertIn("out.len() + block.len() > max_chars", body)
        self.assertNotIn("block.chars().count()", body)
        self.assertIn("out.push_str(&block)", body)

    def test_offline_stt_windows_and_aggregated_channel_shape(self):
        text = source("overlay-backend/src/re_transcribe.rs")
        self.assertRegex(text, r"SttBackendCfg::Cloud\s*\{\s*\.\.\s*\}\s*=>\s*600")
        self.assertRegex(text, r"SttBackendCfg::Whisper\s*\{\s*\.\.\s*\}\s*=>\s*300")
        self.assertRegex(text, r"SttBackendCfg::Gigaam\s*\{\s*\.\.\s*\}\s*=>\s*60")
        assemble = text[text.index("pub fn assemble_lines("):text.index("pub async fn transcribe_session(")]
        self.assertIn("timestamp_ms: 0", assemble)
        self.assertIn("timestamp_ms: 1", assemble)
        self.assertIn("const MAX_REASONABLE_SECS: u64 = 24 * 60 * 60", text)

    def test_retranscribe_summary_uses_nonforce_even_with_force_comment(self):
        text = source("overlay-backend/src/re_transcribe.rs")
        body = text[text.index("pub async fn retranscribe_and_summarize("):text.index("#[cfg(test)]")]
        self.assertIn("session_id.to_string(), false)", body)
        self.assertIn("run_meeting_summary", body)
        self.assertIn("Ok(n)", body)

    def test_conspect_cache_key_is_transcript_only(self):
        text = source("overlay-backend/src/runtime.rs")
        body = text[text.index("pub async fn run_meeting_summary("):text.index("pub async fn retry_meeting_summary(")]
        self.assertIn("conspect::fingerprint(&formatted)", body)
        self.assertIn("saved.fingerprint == fp", body)
        self.assertIn("saved.final_summary.clone()", body)
        self.assertIn("!force", body)

    def test_session_delete_has_ignored_sidecar_failures(self):
        text = source("overlay-backend/src/session_admin.rs")
        body = text[text.index("pub fn delete_session_everywhere("):text.index("#[cfg(test)]")]
        self.assertIn("delete_session_files_in", body)
        self.assertIn("crate::conspect::delete(session_id);", body)
        self.assertIn("crate::conspect::delete_debrief(session_id);", body)
        self.assertNotIn("session_names::", body)
        self.assertLess(body.index("delete_session_files_in"), body.index("store.delete_session"))

    def test_updater_compares_suffix_class_not_rc_number(self):
        text = source("overlay-backend/src/update.rs")
        body = text[text.index("fn parse_ver("):text.index("pub async fn check_latest(")]
        self.assertIn("parse_ver(a) > parse_ver(b)", body)
        self.assertIn("s.split('-').next()", body)
        self.assertIn("u8::from(!has_pre)", body)
        self.assertNotIn("semver::", body)

    def test_updater_checks_initial_url_and_digest_not_final_redirect_url(self):
        text = source("overlay-backend/src/update.rs")
        body = text[text.index("pub async fn download_installer("):text.index("pub fn run_installer(")]
        self.assertIn("if !is_trusted_download(url)", body)
        self.assertNotIn(".url()", body)
        self.assertNotIn("final_url", body)
        self.assertLess(body.index("if got != expected"), body.index("std::fs::write"))
        self.assertNotIn("redirect::Policy", body)


if __name__ == "__main__":
    unittest.main()
