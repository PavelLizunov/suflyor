"""Source and model fixtures for wave3_worker1_bridge C05 and C06.
No live UI event loop, no network calls, no actual process termination.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class MockAtomicU64:
    def __init__(self, val=0):
        self.val = val

    def load(self):
        return self.val

    def compare_exchange(self, current, new):
        if self.val == current:
            self.val = new
            return True, current
        return False, self.val


class MockAutoTilePermit:
    def __init__(self, state, busy_state):
        self.state = state
        self.busy_state = busy_state
        self.dropped = False

    def drop(self):
        if not self.dropped:
            self.dropped = True
            # Matches slint_session.rs AutoTilePermit::drop
            self.state.compare_exchange(self.busy_state, self.busy_state & ~1)


def try_acquire_auto_tile(state, session_gen):
    # Mirrors slint_session.rs try_acquire_auto_tile
    MASK_64 = 0xFFFFFFFFFFFFFFFF
    busy_state = ((session_gen << 1) & MASK_64) | 1
    current = state.load()
    while True:
        current_gen = current >> 1
        if current_gen > session_gen or (current_gen == session_gen and (current & 1) == 1):
            return None
        ok, observed = state.compare_exchange(current, busy_state)
        if ok:
            return MockAutoTilePermit(state, busy_state)
        current = observed


class BridgeSingleFlightCleanupFixtures(unittest.TestCase):
    def test_auto_tile_permit_newer_generation_supersedes_live_permit(self):
        state = MockAtomicU64(0)
        # Session 1 acquires permit and stays busy
        permit1 = try_acquire_auto_tile(state, 1)
        self.assertIsNotNone(permit1)
        # Session 1 cannot acquire a second permit while busy
        self.assertIsNone(try_acquire_auto_tile(state, 1))

        # Session 2 attempts acquire while session 1 permit is still active:
        # current_gen (1) < session_gen (2), so it succeeds and overwrites busy_state
        permit2 = try_acquire_auto_tile(state, 2)
        self.assertIsNotNone(permit2)

        # Both permits are alive simultaneously in their respective tasks
        self.assertFalse(permit1.dropped)
        self.assertFalse(permit2.dropped)

        # When permit1 drops, CAS fails because state holds permit2's busy_state
        permit1.drop()
        self.assertEqual(state.load() & 1, 1)  # state remains busy with permit2
        self.assertEqual(state.load() >> 1, 2)

        # When permit2 drops, state becomes idle at gen 2
        permit2.drop()
        self.assertEqual(state.load() & 1, 0)
        self.assertEqual(state.load() >> 1, 2)

    def test_auto_tile_permit_wrap_around_latches_shut(self):
        # 63-bit generation representation
        state = MockAtomicU64(0)
        high_gen = (1 << 62) - 1
        permit_high = try_acquire_auto_tile(state, high_gen)
        self.assertIsNotNone(permit_high)
        permit_high.drop()

        # State is now idle with current_gen = high_gen
        self.assertEqual(state.load() >> 1, high_gen)

        # If session_gen rolls over / wraps to 1:
        # current_gen > session_gen holds true, so try_acquire_auto_tile returns None
        permit_wrapped = try_acquire_auto_tile(state, 1)
        self.assertIsNone(permit_wrapped)

    def test_source_auto_tile_single_flight_state_packing_and_cas(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("fn try_acquire_auto_tile"):text.index("pub struct SystemAudioAuxGuard;")]
        # Packs session_gen << 1 | 1
        self.assertIn("let busy_state = session_gen.wrapping_shl(1) | 1;", body)
        # Checks current_gen > session_gen || (current_gen == session_gen && current & 1 == 1)
        self.assertIn("if current_gen > session_gen || (current_gen == session_gen && current & 1 == 1)", body)
        self.assertIn("return None;", body)

    def test_source_aborted_task_handles_are_taken_and_aborted_without_join(self):
        text = source("slint-experiment/src/slint_session.rs")
        start_idx = text.index("pub fn stop_session(")
        end_idx = text.index("s.health.last_audio_frame_ms.store(", start_idx)
        body = text[start_idx:end_idx]
        # Handles are taken and aborted
        self.assertIn("if let Some(h) = s.transcript_task.take() {\n        h.abort();\n    }", body)
        self.assertIn("if let Some(h) = s.ai_task.take() {\n        h.abort();\n    }", body)
        self.assertIn("if let Some(h) = s.health_task.take() {\n        h.abort();\n    }", body)
        # But neither join nor await is called on the handles
        self.assertNotIn(".await", body)
        self.assertNotIn("join()", body)

    def test_source_untracked_task_spawns_discard_join_handles(self):
        text = source("slint-experiment/src/slint_session.rs")
        # 1. forward_audio_chunks spawns without storing JoinHandle
        body_forward = text[text.index("fn forward_audio_chunks("):text.index("async fn transcript_forwarder(")]
        self.assertIn("tokio::spawn(async move {", body_forward)
        self.assertNotIn("let forward_task =", body_forward)

        # 2. transcript_forwarder spawns maybe_spawn_auto_tile per line without saving handle
        body_tile = text[text.index("let gen_for_tile = lock(&rt).session_gen;"):text.index("log_info(\"transcript forwarder exit\");")]
        self.assertIn("tokio::spawn(async move {", body_tile)
        self.assertIn("maybe_spawn_auto_tile(", body_tile)
        self.assertNotIn("lock(&rt).ai_task = Some", body_tile)

        # 3. maybe_run_debrief spawns run_post_meeting_debrief on rt_handle without tracking
        body_debrief = text[text.index("pub fn maybe_run_debrief("):text.index("pub fn meeting_ending_phrase_match(")]
        self.assertIn("rt_handle.spawn(async move {", body_debrief)
        self.assertIn("overlay_backend::runtime::run_post_meeting_debrief(", body_debrief)
        self.assertNotIn("let debrief_task =", body_debrief)

    def test_original_c05_c06_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker1_bridge-C05"], "hypothesis")
        self.assertEqual(found["wave3_worker1_bridge-C06"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
