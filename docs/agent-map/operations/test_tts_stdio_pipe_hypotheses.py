"""Source fixtures for wave2_worker3_tts C01 and C02.
No sidecar process execution, no ONNX loading, no audio output.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class TtsStdioPipeFixtures(unittest.TestCase):
    def test_tts_speak_encodes_entire_text_as_single_speak_line(self):
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("pub fn speak(&self, text: &str)"):text.index("pub fn pause(&self)")]
        # Base64 encodes whole spoken text and writes format!("SPEAK {b64}") without length check or chunking
        self.assertIn("let b64 = base64::engine::general_purpose::STANDARD.encode(&spoken);", body)
        self.assertIn('self.send_tera_speak(&format!("SPEAK {b64}"), spoken.chars().count())', body)
        self.assertIn('self.send_piper_speak(&format!("SPEAK {b64}"), spoken.chars().count())', body)
        # Note absence of text chunking or line length clamp before encoding
        self.assertNotIn("MAX_SPEAK_CHARS", body)
        self.assertNotIn("chunk", body.lower())

    def test_sidecar_write_raw_flushes_under_sidecar_mutex(self):
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("fn write_raw(&mut self, line: &str) -> bool"):text.index("fn send(&mut self, line: &str)")]
        # Writes directly to stdin and flushes immediately
        self.assertIn('writeln!(si, "{line}").and_then(|_| si.flush()).is_ok()', body)

    def test_suflyor_tts_stdin_reader_grows_unbounded_line_string(self):
        text = source("suflyor-tts/src/main.rs")
        body = text[text.index("let stdin = std::io::stdin();"):text.index("let voices = engine::scan_voices")]
        # Uses standard BufRead::lines() without take() limit
        self.assertIn("for line in stdin.lock().lines()", body)
        self.assertNotIn("take(", body)
        self.assertNotIn("MAX_LINE_BYTES", body)

    def test_suflyor_teratts_stdin_reader_grows_unbounded_line_string(self):
        text = source("suflyor-teratts/src/main.rs")
        body = text[text.index("let stdin = std::io::stdin();"):text.index("worker(controller, events_rx);")]
        # TeraTTS sidecar also reads stdin with BufRead::lines() without take() limit
        self.assertIn("for line in stdin.lock().lines()", body)
        self.assertNotIn("take(", body)
        self.assertNotIn("MAX_LINE_BYTES", body)

    def test_host_sidecar_reader_thread_consumes_stdout_lines_without_deadlock(self):
        # C02 analysis: child stdout is drained in a dedicated background thread spawned at sidecar startup
        text = source("overlay-backend/src/tts.rs")
        body = text[text.index("match spawn_engine_sidecar(&self.exe, self.kind)"):text.index("self.write_raw(&format!(\"RATE {r}\"));")]
        self.assertIn("let thread_name = match engine {", body)
        self.assertIn("std::thread::Builder::new()", body)
        self.assertIn("for line in std::io::BufReader::new(stdout).lines()", body)
        self.assertIn("handle_playback_line(engine, &playback, &line);", body)

    def test_original_c01_c02_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave2_worker3_tts-C01"], "hypothesis")
        self.assertEqual(found["wave2_worker3_tts-C02"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
