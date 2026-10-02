"""Source and numerical fixtures for wave2_worker1_audio C01 and C07 (confirmed mechanisms).
No live WASAPI audio capture, no device initialization, no hardware interaction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def resample_and_quantise_model(input_samples, ratio):
    if not input_samples or ratio <= 0.0:
        return []
    out_len = int(len(input_samples) / ratio)
    out = []
    if abs(ratio - 3.0) < 1e-6:
        inv_3 = 1.0 / 3.0
        full_chunks = len(input_samples) // 3
        for i in range(full_chunks):
            idx = i * 3
            mean = (input_samples[idx] + input_samples[idx + 1] + input_samples[idx + 2]) * inv_3
            clamped = max(-1.0, min(1.0, mean))
            out.append(int(clamped * 32767))
        return out

    for i in range(out_len):
        start = int(i * ratio)
        end = min(len(input_samples), int((i + 1) * ratio))
        if start >= end:
            continue
        mean = sum(input_samples[start:end]) / (end - start)
        clamped = max(-1.0, min(1.0, mean))
        out.append(int(clamped * 32767))
    return out


class AudioResampleFormatFixtures(unittest.TestCase):
    def test_3_to_1_resample_discards_modulo_3_tail_samples(self):
        # 10 samples with ratio=3.0 -> full_chunks = 3, out_len = 3, drops 1 sample
        samples = [0.1] * 10
        out = resample_and_quantise_model(samples, 3.0)
        self.assertEqual(len(out), 3)

    def test_f32_buf_and_byte_q_clear_on_each_loop_iteration(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn capture_thread("):text.index("fn record_source_until_stop(")]
        # Clears byte_q at start of loop, wiping leftover bytes if len % 4 != 0
        self.assertIn("byte_q.clear();", body)
        self.assertIn("while byte_q.len() >= 4 {", body)
        # Clears f32_buf after resample, dropping fractional phase
        self.assertIn("let pcm_i16 = resample_and_quantise(&f32_buf, ratio);", body)
        self.assertIn("f32_buf.clear();", body)

    def test_wasapi_client_requests_autoconvert_without_reading_back_negotiated_format(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn capture_thread("):text.index("fn record_source_until_stop(")]
        # Requests Float, 1 channel with autoconvert = true
        self.assertIn("&SampleType::Float, actual_rate as usize, 1, None", body)
        self.assertIn("autoconvert: true", body)
        self.assertIn(".initialize_client(&desired, &init_dir, &mode)", body)
        # Never calls get_mixformat or inspects actual format after initialize_client
        init_to_loop = body[body.index(".initialize_client"):body.index("while !stop.load")]
        self.assertNotIn("get_mixformat", init_to_loop)
        self.assertNotIn("get_format", init_to_loop)

    def test_record_source_until_stop_requests_autoconvert_float_mono(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("pub fn record_source_until_stop("):text.index("pub fn record_sys_blocking(")]
        self.assertIn("WaveFormat::new(32, 32, &SampleType::Float, actual_rate as usize, 1, None)", body)
        self.assertIn("autoconvert: true", body)

    def test_resample_and_quantise_is_stateless_per_call(self):
        text = source("overlay-backend/src/audio.rs")
        body = text[text.index("fn resample_and_quantise("):text.index("#[cfg(test)]")]
        # Takes slice &[f32] and ratio f64 without any mutable state / accumulator reference
        self.assertIn("fn resample_and_quantise(input: &[f32], ratio: f64) -> Vec<i16>", body)
        self.assertNotIn("&mut", body[:body.index("->")])

    def test_original_c01_c07_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker1_audio-C01"], "confirmed")
        self.assertEqual(found["wave2_worker1_audio-C07"], "confirmed")


if __name__ == "__main__":
    unittest.main()
