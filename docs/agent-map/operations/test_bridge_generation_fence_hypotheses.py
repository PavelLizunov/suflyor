"""Source and sequence fixtures for wave3_worker1_bridge C03 and C04.
No UI thread blocking, no native Slint event loop, no live AI completion.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class BridgeGenerationFenceFixtures(unittest.TestCase):
    def test_maybe_spawn_auto_tile_entry_has_no_initial_gen_check(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("async fn maybe_spawn_auto_tile("):text.index("// ===== Rate-limit =====")]
        # Function takes session_gen: u64
        self.assertIn("session_gen: u64,", body)
        # Checks suppress_tiles, endpoint config, trigger keywords, but NO session_gen check at entry
        self.assertNotIn("session_gen !=", body)
        self.assertNotIn("session_gen ==", body)

    def test_pre_ai_mutations_occur_before_session_gen_check(self):
        text = source("slint-experiment/src/slint_session.rs")
        start_idx = text.index("async fn maybe_spawn_auto_tile(")
        end_idx = text.index("pub fn stop_session(", start_idx)
        func = text[start_idx:end_idx]
        idx_rate_limit = func.index("s.recent_tile_triggers.push_back(now);")
        idx_prefix_dedup = func.index("s.recent_question_prefixes.push((normalized, now));")
        idx_ai_call = func.index("ai::complete_with_usage_endpoint(")
        idx_post_ai_gen_check = func.index("if lock(&rt).session_gen != session_gen {")

        # Mutations to recent_tile_triggers and recent_question_prefixes happen before AI call and post-AI gen check
        self.assertLess(idx_rate_limit, idx_ai_call)
        self.assertLess(idx_prefix_dedup, idx_ai_call)
        self.assertLess(idx_ai_call, idx_post_ai_gen_check)

    def test_post_ai_gen_check_guard_and_separate_lock_reacquisition(self):
        text = source("slint-experiment/src/slint_session.rs")
        start_idx = text.index("async fn maybe_spawn_auto_tile(")
        end_idx = text.index("pub fn stop_session(", start_idx)
        func = text[start_idx:end_idx]
        body = func[func.index("if lock(&rt).session_gen != session_gen {"):]
        # Check guards result discarding
        self.assertIn('log_info("auto-tile: session changed during AI call — discarding result");', body)
        # After check passes, lock is dropped and re-acquired separately for qa_cache and session_cost
        idx_qa_cache = body.index("s.qa_cache\n            .insert(cache_key, (answer.clone(), Instant::now()));")
        idx_cost = body.index("s.session_cost_microcents = s.session_cost_microcents.saturating_add(micro);")
        self.assertLess(0, idx_qa_cache)
        self.assertLess(idx_qa_cache, idx_cost)

    def test_stop_session_bumps_gen_and_aborts_tasks_under_same_lock(self):
        text = source("slint-experiment/src/slint_session.rs")
        start_idx = text.index("pub fn stop_session(")
        end_idx = text.index("s.health.last_audio_frame_ms.store(", start_idx)
        body = text[start_idx:end_idx]
        # session_gen is bumped
        self.assertIn("s.session_gen = s.session_gen.wrapping_add(1);", body)
        # tasks are aborted under the same &mut s lock
        self.assertIn("if let Some(h) = s.transcript_task.take() {\n        h.abort();\n    }", body)
        self.assertIn("if let Some(h) = s.ai_task.take() {\n        h.abort();\n    }", body)

    def test_model_interleaving_forwarder_stamps_post_stop_gen_when_abort_unwinds_at_await(self):
        # Pure model demonstrating C04 TOCTOU:
        # 1. Forwarder receives a line from stt_rx.recv()
        # 2. stop_session runs: session_gen 1 -> 2, abort() issued
        # 3. Forwarder runs non-awaiting code up to tokio::spawn: reads session_gen (reads 2!)
        # 4. Forwarder spawns maybe_spawn_auto_tile with gen=2
        # 5. When maybe_spawn_auto_tile finishes AI call, lock(&rt).session_gen is STILL 2
        # 6. lock(&rt).session_gen != session_gen check PASSES (2 == 2)!
        session_gen = 1
        stt_line_in_flight = True
        # stop_session happens
        session_gen += 1
        aborted = True
        # forwarder resumes synchronous execution before hitting .await:
        gen_for_tile = session_gen  # reads bumped gen 2
        # auto-tile runs with gen_for_tile = 2
        # check at end of auto-tile:
        passes_fence = (session_gen == gen_for_tile)
        self.assertTrue(passes_fence)

    def test_original_c03_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker1_bridge-C03"], "hypothesis")
        self.assertEqual(found["wave3_worker1_bridge-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
