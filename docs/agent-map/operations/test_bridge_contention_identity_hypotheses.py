"""Source fixtures for wave3_worker1_bridge C02 and C08.
No live audio, no GUI event loop, no network requests.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class BridgeContentionIdentityFixtures(unittest.TestCase):
    def test_forward_audio_chunks_locks_rt_on_every_incoming_chunk(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("fn forward_audio_chunks("):text.index("async fn transcript_forwarder(")]
        # Takes lock(&rt) on each chunk for paused/muted check and health metrics
        self.assertIn("while let Some(chunk) = src_rx.recv().await {", body)
        self.assertIn("let (paused, mic_muted) = {", body)
        self.assertIn("let mut state = lock(&rt);", body)
        # Also takes lock again after stt_tx.reserve().await
        self.assertIn("let permit = match stt_tx.reserve().await {", body)
        self.assertIn("if lock(&rt).paused {", body)

    def test_start_session_inner_locks_rt_during_bulk_clear_operations(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("fn start_session_inner("):text.index("if prior_capture.is_some() {")]
        # Holds lock(&rt) while clearing full_transcript, qa_cache, etc.
        self.assertIn("let mut s = lock(&rt);", body)
        self.assertIn("s.full_transcript.clear();", body)
        self.assertIn("s.qa_cache.clear();", body)

    def test_qa_cache_eviction_sorts_under_lock(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("if s.qa_cache.len() >= QA_CACHE_MAX_ENTRIES {"):text.index("s.qa_cache\n            .insert(cache_key")]
        # Inside lock(&rt), sorts qa_cache entries by age when capacity reached
        self.assertIn("let mut by_age: Vec<(String, Duration)> = s", body)
        self.assertIn("by_age.sort_by_key(|(_, age)| std::cmp::Reverse(*age));", body)
        self.assertIn("s.qa_cache.remove(&k);", body)

    def test_debrief_spawn_passes_session_id_without_session_gen(self):
        text = source("slint-experiment/src/slint_session.rs")
        body = text[text.index("pub fn maybe_run_debrief("):text.index("pub fn meeting_ending_phrase_match(")]
        # Passes session_id to debrief task without session_gen
        self.assertIn("rt_handle.spawn(async move {", body)
        self.assertIn("overlay_backend::runtime::run_post_meeting_debrief(", body)
        self.assertIn("session_id,", body)
        self.assertNotIn("session_gen", body)

    def test_session_namer_captures_and_verifies_generation(self):
        # Counterevidence to claim that namer has no generation stamp:
        text = source("slint-experiment/src/session_namer.rs")
        body = text[text.index("pub fn maybe_spawn_namer("):text.index("pub async fn generate_name(")]
        # Captures s.session_gen
        self.assertIn("(action, s.session_gen, lines)", body)
        # Inside async completion, checks s.session_gen != gen before persisting
        self.assertIn("if s.session_gen != gen {", body)
        self.assertIn("s.session_name_inflight = false;", body)

    def test_original_c02_c08_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker1_bridge-C02"], "hypothesis")
        self.assertEqual(found["wave3_worker1_bridge-C08"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
