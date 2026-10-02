"""Source fixtures for wave2_worker4_local_ai C04 and C05.
No local processes spawned, no netstat executed, no model files hashed.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def parse_mock_netstat_listening(line, port):
    cols = line.split()
    suffix = f":{port}"
    if len(cols) >= 5 and cols[3].upper() == "LISTENING" and cols[1].endswith(suffix):
        return cols[4]
    return None


class LocalAiNetstatModelShaFixtures(unittest.TestCase):
    def test_netstat_line_parser_matches_ipv4_and_ipv6_suffix(self):
        # IPv4
        ipv4_line = "  TCP    127.0.0.1:8080         0.0.0.0:0              LISTENING       1234"
        self.assertEqual(parse_mock_netstat_listening(ipv4_line, "8080"), "1234")
        # IPv6
        ipv6_line = "  TCP    [::1]:8080             [::]:0                 LISTENING       5678"
        self.assertEqual(parse_mock_netstat_listening(ipv6_line, "8080"), "5678")
        # Non-matching port
        diff_port = "  TCP    127.0.0.1:8081         0.0.0.0:0              LISTENING       9999"
        self.assertIsNone(parse_mock_netstat_listening(diff_port, "8080"))

    def test_stop_listener_on_port_treats_unknown_exe_as_stranger(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn stop_listener_on_port(port: &str, root: &Path) -> bool"):text.index("pub fn free_llama_port(")]
        # When exe_path_for_pid returns None / other, it marks free_of_strangers = false
        self.assertIn("other => {", body)
        self.assertIn("free_of_strangers = false;", body)
        # Failure of netstat command returns false immediately
        self.assertIn("let Ok(out) = run_capture(\"netstat\", &[\"-ano\", \"-p\", \"tcp\"]) else {", body)
        self.assertIn("return false;", body)

    def test_selected_llama_gguf_hashes_only_26b_not_12b_or_4b(self):
        text = source("overlay-backend/src/local_ai/model_state.rs")
        body = text[text.index("pub(super) fn selected_llama_gguf("):text.index("pub(super) fn fallback_llama_gguf(")]
        # Primary26B uses cached_pinned_file_matches with SHA256
        self.assertIn("ManagedModel::Primary26B => {", body)
        self.assertIn("cached_pinned_file_matches(", body)
        self.assertIn("GEMMA26_SHA256", body)
        # Legacy4B and Fallback12B use size-only checks
        self.assertIn("legacy_gguf_complete(", body)
        self.assertIn("file_has_expected_size(&llama_dir.join(GEMMA_FILE), GEMMA_SIZE)", body)
        self.assertNotIn("GEMMA_SHA256", body)

    def test_quality_model_present_is_size_only_check(self):
        text = source("overlay-backend/src/local_ai/model_state.rs")
        body = text[text.index("pub fn quality_model_present("):text.index("pub fn legacy_model_present(")]
        self.assertIn("file_has_expected_size(&quality_gguf_path(root), GEMMA26_SIZE)", body)
        self.assertNotIn("SHA", body)
        self.assertNotIn("hash", body.lower())

    def test_mmproj_for_model_uses_size_and_build_stamp_without_hash(self):
        text = source("overlay-backend/src/local_ai/model_state.rs")
        body = text[text.index("pub(super) fn mmproj_for_model("):text.index("pub(super) fn managed_model_vision_capable(")]
        self.assertIn("file_len(&proj) == size", body)
        self.assertIn("llama_build_supports_", body)
        self.assertNotIn("sha256", body.lower())

    def test_original_c04_c05_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C04"], "hypothesis")
        self.assertEqual(found["wave2_worker4_local_ai-C05"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
