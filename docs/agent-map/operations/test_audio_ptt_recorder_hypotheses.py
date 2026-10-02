"""Source fixtures for wave2_worker1_audio C06/C08.
No audio hardware, no audio threads, no WASAPI/CoreAudio calls.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def plan_pad(written, skew, timestamp_ms, chunk_len, pad_budget):
    SAMPLE_RATE = 16000
    MAX_PAD_SAMPLES = 10 * 60 * 16000  # 10 minutes
    target_end = max(0, (timestamp_ms * SAMPLE_RATE // 1000) - skew)
    target_start = max(0, target_end - chunk_len)
    gap = max(0, target_start - written)
    pad = min(gap, MAX_PAD_SAMPLES, pad_budget)
    return pad, skew + (gap - pad)


class AudioPttRecorderFixtures(unittest.TestCase):
    def test_windows_ptt_accumulates_without_size_cap(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("pub fn record_source_until_stop"):text.index("pub fn record_sys_blocking")]
        # Initial reservation is 30s
        self.assertIn("Vec::with_capacity((actual_rate as usize) * 30)", body)
        # Loop pushes every sample into all_f32 while !stop.load()
        self.assertIn("while !stop.load(Ordering::Acquire)", body)
        self.assertIn("all_f32.push(f32::from_le_bytes(b));", body)
        # Note absence of buffer length limit or break before stop flag
        self.assertNotIn("all_f32.len() >=", body)
        self.assertNotIn("MAX_PTT_SAMPLES", body)

    def test_macos_ptt_accumulates_without_size_cap(self):
        text = source("overlay-backend/src/audio_macos.rs")
        body = text[text.index("pub fn record_source_until_stop"):text.index("pub fn record_sys_blocking")]
        self.assertIn("let mut accum: Vec<f32> = Vec::new();", body)
        self.assertIn("while !stop.load(Ordering::Acquire)", body)
        self.assertIn("accum.extend_from_slice(&scratch[..n]);", body)
        self.assertNotIn("accum.len() >=", body)
        self.assertNotIn("MAX_PTT_SAMPLES", body)

    def test_plan_pad_caps_single_gap_and_absorbs_excess_into_skew(self):
        # 16 kHz sample rate: 10 mins = 9,600,000 samples
        written = 16000
        skew = 0
        timestamp_ms = 15 * 60 * 1000  # 15 minutes mark
        chunk_len = 3200  # 200ms
        pad_budget = 30 * 60 * 16000  # 30 mins budget
        pad, new_skew = plan_pad(written, skew, timestamp_ms, chunk_len, pad_budget)
        # Gap is (15*60*16000 - 3200) - 16000 = 14,400,000 - 19,200 = 14,380,800
        # Capped at 10 minutes (9,600,000)
        self.assertEqual(pad, 10 * 60 * 16000)
        self.assertEqual(new_skew, 14380800 - 9600000)

    def test_plan_pad_backwards_timestamp_yields_zero_pad(self):
        # When timestamp indicates earlier than written, no pad is added (WAV never seeks back)
        written = 32000
        skew = 0
        timestamp_ms = 1000  # 1 sec = 16000 samples
        chunk_len = 3200
        pad, new_skew = plan_pad(written, skew, timestamp_ms, chunk_len, 100000)
        self.assertEqual(pad, 0)
        self.assertEqual(new_skew, 0)

    def test_recorder_source_constants_and_plan_pad_guards(self):
        text = source("overlay-backend/src/recorder.rs")
        self.assertIn("MAX_PAD_SAMPLES: u64 = 10 * 60 * SAMPLE_RATE as u64;", text)
        self.assertIn("MAX_TOTAL_PAD_SAMPLES: u64 = 30 * 60 * SAMPLE_RATE as u64;", text)
        body = text[text.index("fn plan_pad("):text.index("fn writer_loop(")]
        self.assertIn("let gap = target_start.saturating_sub(written);", body)
        self.assertIn("let pad = gap.min(MAX_PAD_SAMPLES).min(pad_budget);", body)

    def test_original_c06_c08_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker1_audio-C06"], "hypothesis")
        self.assertEqual(found["wave2_worker1_audio-C08"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
