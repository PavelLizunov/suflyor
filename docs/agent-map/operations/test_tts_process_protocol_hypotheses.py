"""Source fixtures for wave2_worker3_tts C03 and C04.
No process execution, no audio synthesis, no real sidecars spawned.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class TtsProcessProtocolFixtures(unittest.TestCase):
    def test_windows_only_job_assignment_on_spawn(self):
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("fn spawn_engine_sidecar"):text.index("fn tts_root(")]
        # assign_to_lifetime_job is under #[cfg(windows)] only
        self.assertIn("#[cfg(windows)]", body)
        self.assertIn("crate::local_ai::assign_to_lifetime_job(&proc);", body)
        # Note absence of POSIX death signals like prctl/PDEATHSIG or kill_on_drop
        self.assertNotIn("PDEATHSIG", body)
        self.assertNotIn("kill_on_drop", body)
        self.assertNotIn("prctl", body)

    def test_sidecar_struct_has_no_explicit_drop_implementation(self):
        text = source("overlay-backend/src/tts.rs")
        # Verify that Sidecar does not implement Drop with kill/wait
        self.assertNotIn("impl Drop for Sidecar", text)

    def test_sidecar_crashed_out_tracks_crash_limit(self):
        text = source("overlay-backend/src/tts.rs")
        self.assertIn("const TERA_CRASH_LIMIT: u32 = 3;", text)
        body = text[text.index("fn crashed_out(&self) -> bool"):text.index("fn write_raw(")]
        self.assertIn("self.crashes >= TERA_CRASH_LIMIT", body)

    def test_playback_tracker_fifo_pending_pop_on_started(self):
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("impl PlaybackTracker"):text.index("fn finish_generation(")]
        # started() pops front of pending queue and maps to id
        self.assertIn("fn started(&mut self, id: u64) -> Option<u64>", body)
        self.assertIn("let generation = self.pending.pop_front()?;", body)
        self.assertIn("self.utterances.insert(id, generation);", body)

    def test_playback_tracker_rejected_only_consumes_base64_and_utf8_errors(self):
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("fn rejected(&mut self, reason: &str) -> Option<u64>"):text.index("fn drain_generations(")]
        # Only invalid-base64 and invalid-utf8 consume pending generations
        self.assertIn('!matches!(reason, "invalid-base64" | "invalid-utf8")', body)
        self.assertIn("self.pending.pop_front()", body)

    def test_original_c03_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker3_tts-C03"], "hypothesis")
        self.assertEqual(found["wave2_worker3_tts-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
