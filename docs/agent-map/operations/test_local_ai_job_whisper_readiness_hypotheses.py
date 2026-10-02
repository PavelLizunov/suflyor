"""Source fixtures for wave2_worker4_local_ai C10 and C11.
No Windows JobObjects created, no servers spawned, no curl executed.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class LocalAiJobWhisperReadinessFixtures(unittest.TestCase):
    def test_job_object_limit_is_kill_on_close_only_and_best_effort(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("pub(crate) fn assign_to_lifetime_job"):text.index("fn launch_hidden_wait(")]
        # Limits flag is JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        self.assertIn("info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;", body)
        # Any Win32 failure logs warning and returns 0 / ignores error
        self.assertIn('log::warn!("CreateJobObject failed:', body)
        self.assertIn('log::warn!("SetInformationJobObject failed:', body)
        self.assertIn('log::warn!("AssignProcessToJobObject failed:', body)
        self.assertNotIn("panic!", body)

    def test_job_assignment_called_in_launch_hidden_under_windows_cfg(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn launch_hidden(exe: &Path, args: &[&str]) -> Result<Child>"):text.index("pub(crate) fn assign_to_lifetime_job")]
        self.assertIn("#[cfg(windows)]", body)
        self.assertIn("assign_to_lifetime_job(&child);", body)

    def test_whisper_install_readiness_uses_generic_wait_ready(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("pub fn install("):text.index("pub fn ensure_servers(")]
        # Whisper readiness check in install
        self.assertIn('wait_ready(&format!("{WHISPER_BASE_URL}/models"), 60)', body)
        # It does NOT use PID listener ownership verification or model ID verification
        whisper_section = body[body.index("if !opts.skip_whisper"):body.index("let selected_model =")]
        self.assertNotIn("owns_listener", whisper_section)
        self.assertNotIn("expected_model", whisper_section)

    def test_whisper_route_check_uses_only_is_reachable(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn ensure_servers_for_route("):text.index("fn llama_server_args(")]
        self.assertIn('if want_whisper && !is_reachable(&format!("{WHISPER_BASE_URL}/models")) {', body)
        self.assertNotIn("wait_for_expected_whisper", body)

    def test_wait_ready_polls_curl_status_only_without_content_validation(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn wait_ready(url: &str, max_secs: u64) -> Result<()>"):text.index("fn llama_reply_has_text_content(")]
        # Output redirected to dev_null, checks out.status.success() only
        self.assertIn('dev_null()', body)
        self.assertIn("out.status.success()", body)
        self.assertNotIn("serde_json", body)

    def test_original_c10_c11_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C10"], "hypothesis")
        self.assertEqual(found["wave2_worker4_local_ai-C11"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
