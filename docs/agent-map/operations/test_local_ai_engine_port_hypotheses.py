"""Source fixtures for wave2_worker4_local_ai C02 and C03.
No local server spawning, no HTTP requests, no port listening.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class LocalAiEnginePortFixtures(unittest.TestCase):
    def test_verify_engine_runs_discards_stop_listener_and_omits_owner_check(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn verify_engine_runs("):text.index("fn swap_engine_binaries(")]
        # Discards stop_listener_on_port return value
        self.assertIn("let _ = stop_listener_on_port(ENGINE_VERIFY_PORT, root);", body)
        # Checks readiness via generic wait_ready on /v1/models
        self.assertIn('wait_ready(\n        &format!("http://127.0.0.1:{ENGINE_VERIFY_PORT}/v1/models"),', body)
        # Does NOT verify that the newly launched child owns the listening port or check model identity
        self.assertNotIn("launched_llama_owns_listener", body)
        self.assertNotIn("expected_model_is_ready", body)

    def test_ensure_servers_for_route_pushes_child_without_ownership_check(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn ensure_servers_for_route("):text.index("fn llama_server_args(")]
        # When launch_hidden succeeds, child is pushed directly to started Vec without verifying port binding
        self.assertIn("if let Ok(child) = launch_hidden(&exe, &arg_refs) {", body)
        self.assertIn("started.push(child);", body)
        self.assertNotIn("launched_llama_owns_listener", body)
        self.assertNotIn("free_llama_port", body)

    def test_wait_local_ai_ready_with_probe_contrasts_by_enforcing_ownership(self):
        # In contrast, wait_for_expected_model_at does verify PID ownership and expected model name
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("fn wait_for_expected_model_at("):text.index("pub fn stop_managed_servers")]
        self.assertIn("if launched_llama_owns_listener(llama)", body)
        self.assertIn("&& expected_model_is_ready(", body)

    def test_engine_verify_port_constant_is_8077(self):
        text = source("overlay-backend/src/local_ai.rs")
        self.assertIn('const ENGINE_VERIFY_PORT: &str = "8077";', text)

    def test_whisper_server_also_pushes_child_without_ownership_check(self):
        text = source("overlay-backend/src/local_ai.rs")
        body = text[text.index("if want_whisper && !is_reachable(&format!(\"{WHISPER_BASE_URL}/models\")) {"):text.index("fn llama_server_args(")]
        self.assertIn("if let Ok(child) = launch_hidden(", body)
        self.assertIn("started.push(child);", body)
        self.assertNotIn("owns_listener", body)

    def test_original_c02_c03_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker4_local_ai-C02"], "hypothesis")
        self.assertEqual(found["wave2_worker4_local_ai-C03"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
