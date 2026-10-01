"""Textual research seams for integration boundaries, not native/network acceptance."""
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


def source(path):
    return (REPO / path).read_text(encoding="utf-8")


class IntegrationSourceTests(unittest.TestCase):
    def test_hermes_bridge_accepts_configured_host_not_strict_loopback_only(self):
        text = source("overlay-backend/src/bridge.rs")
        body = text[text.index("pub fn start("):text.index("fn handle_request(")]
        self.assertIn("c.hermes_bridge_host", body)
        self.assertIn("tiny_http::Server::http((host.as_str(), port))", body)
        self.assertNotIn("if !is_loopback_host", body)

    def test_hermes_summary_reads_catalog_and_save_failure_still_responds(self):
        text = source("overlay-backend/src/bridge.rs")
        self.assertIn("store.session_ai_turns(id)", text)
        self.assertIn('find(|t| t.purpose == "summary")', text)
        self.assertNotIn("conspect::load", text)
        shell = text[text.index("if needs_save {"):text.index("fn split_query(")]
        self.assertIn("if let Err(e) = crate::config::save", shell)
        self.assertIn("let _ = req.respond(response)", shell)

    def test_mlx_runtime_fast_snapshot_and_exact_model_ready_check(self):
        text = source("overlay-backend/src/mlx_runtime.rs")
        body = text[text.index("fn start_macos("):text.index("fn probe(")]
        self.assertIn("mlx_install::installed_snapshot(model)", body)
        self.assertNotIn("installed_snapshot_verified(model)", body)
        ready = text[text.index("fn parse_ready("):text.index("fn is_valid_mlx_token(")]
        self.assertIn("ready.model != expected_model", ready)
        self.assertIn("ready.port == 0", ready)
        self.assertIn("ready.version != PROTOCOL_VERSION", ready)

    def test_swift_mlx_filters_remote_images_and_serializes_waiters(self):
        protocol = source("suflyor-mlx/Sources/SuflyorMLXCore/Protocol.swift")
        server = source("suflyor-mlx/Sources/SuflyorMLXCore/Server.swift")
        self.assertIn('["data:image/jpeg;base64,", "data:image/png;base64,"]', protocol)
        self.assertIn("where url.hasPrefix(prefix)", protocol)
        self.assertIn("let maxImageBytes = 16 * 1024 * 1024", protocol)
        self.assertIn("data.count <= maxImageBytes", protocol)
        self.assertIn("actor GenerationGate", server)
        self.assertIn("waiters.append", server)
        self.assertIn("await gate.release()", server)
        self.assertIn("group.cancelAll()", server)


if __name__ == "__main__":
    unittest.main()
