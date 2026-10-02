"""Source fixtures for the 5 rejected claims:
- wave1_worker1_persistence-C07 (retention pruning vs idempotent catalog)
- wave1_worker1_persistence-C11 (FTS5 triggers vs manual deletion)
- wave1_worker3_config-C10 (http_error_line static op call sites)
- wave3_worker1_bridge-C01 (non-blocking invoke_from_event_loop vs supposed Tokio deadlock)
- wave3_worker3_tile-C04 (system prompt stripping and clean clipboard copy)
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class RejectedClaimsFalsificationFixtures(unittest.TestCase):
    def test_fts5_schema_uses_automatic_after_insert_triggers(self):
        # wave1_worker1_persistence-C11: FTS is kept in sync by SQLite triggers, not manual sync
        text = source("overlay-backend/migrations/0002_fts.sql")
        self.assertIn("CREATE TRIGGER trg_utterances_fts_ai AFTER INSERT ON utterances BEGIN", text)
        self.assertIn("CREATE TRIGGER trg_ai_turns_fts_ai AFTER INSERT ON ai_turns BEGIN", text)

    def test_http_error_line_call_sites_use_static_literals_without_injection(self):
        # wave1_worker3_config-C10: op parameter is always a static literal
        stt_text = source("overlay-backend/src/stt.rs")
        self.assertIn('crate::http_log::http_error_line("STT", status.as_u16(), body.len())', stt_text)

    def test_forward_event_uses_nonblocking_invoke_without_mutex_deadlock(self):
        # wave3_worker1_bridge-C01: forward_event takes local lock, drops it, and dispatches via invoke_from_event_loop
        text = source("slint-experiment/src/bin/overlay_host/tile_controller.rs")
        idx_interrupted = text.index("let interrupted = {")
        idx_busy = text.index("tile.set_followup_busy(false);", idx_interrupted)
        body = text[idx_interrupted:idx_busy]
        self.assertIn("let mut slot = match self.current_streaming.lock()", body)
        self.assertIn("slot.take()", body)
        self.assertIn("let _ = slint::invoke_from_event_loop(move || {", body)
        self.assertNotIn("blocking_recv", body)

    def test_format_convo_copy_explicitly_filters_out_system_prompts(self):
        # wave3_worker3_tile-C04: system messages are explicitly filtered out, user turns sanitized
        text = source("slint-experiment/src/bin/overlay_host/tile_copy.rs")
        body = text[text.index("pub(crate) fn format_convo_copy("):text.index("pub(crate) fn wire_copy(")]
        self.assertIn('.filter(|m| m.role != "system")', body)
        self.assertIn("user_question_for_copy(text)", body)

    def test_persistence_mod_defines_catalog_as_idempotent_projection(self):
        # wave1_worker1_persistence-C07: catalog is documented as additive projection preserving history past raw journal pruning
        text = source("overlay-backend/src/persistence/mod.rs")
        self.assertIn("SQLite ([`Store`]) is a queryable PROJECTION for the session archive", text)
        self.assertIn("the indexer\n//!   is additive (it never deletes session rows on its own)", text)
        self.assertIn("This is a\n//!   feature, not drift — the ~few-MB catalog is the long-term searchable", text)

    def test_all_five_original_statuses_remain_rejected(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave1_worker1_persistence-C07"], "rejected")
        self.assertEqual(found["wave1_worker1_persistence-C11"], "rejected")
        self.assertEqual(found["wave1_worker3_config-C10"], "rejected")
        self.assertEqual(found["wave3_worker1_bridge-C01"], "rejected")
        self.assertEqual(found["wave3_worker3_tile-C04"], "rejected")


if __name__ == "__main__":
    unittest.main()
