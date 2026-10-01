"""Read-only source seam checks; no compiler/model/native acceptance implied."""
import json
import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


def source(path):
    return (REPO / path).read_text(encoding="utf-8")


class SourceContractTests(unittest.TestCase):
    def test_ai_channel_permit_and_timeout_seams(self):
        stream = source("overlay-backend/src/ai/stream.rs")
        control = source("overlay-backend/src/ai/control.rs")
        completion = source("overlay-backend/src/ai/completion.rs")
        self.assertRegex(stream, r"mpsc::channel::<AiEvent>\(64\)")
        self.assertIn("Semaphore::const_new(2)", control)
        self.assertIn("from_secs(120)", stream)
        self.assertIn("from_secs(180)", completion)
        self.assertLess(stream.index("AiProtocol::CodexSubscription"), stream.index("permit = AI_SEMAPHORE.acquire()"))

    def test_tts_warm_and_windows_job_attachment_are_explicit(self):
        text = source("overlay-backend/src/tts.rs")
        init = text[text.index("pub fn init("):text.index("pub fn speak(text:")]
        self.assertIn("tts.warm();", init)
        spawn = text[text.index("fn spawn_engine_sidecar("):text.index("fn tts_root()")]
        self.assertIn("#[cfg(windows)]", spawn)
        self.assertIn("assign_to_lifetime_job(&proc)", spawn)

    def test_memory_normalizer_has_no_production_call_in_current_sources(self):
        matches = []
        for base in ("overlay-backend/src", "slint-experiment/src"):
            for path in (REPO / base).rglob("*.rs"):
                for line in path.read_text(encoding="utf-8").splitlines():
                    if re.search(r"\bnormalize_fact\s*\(", line):
                        matches.append((str(path.relative_to(REPO)), line.strip()))
        self.assertEqual(len(matches), 1, matches)
        self.assertEqual(matches[0][0], "overlay-backend/src/memory/normalize.rs")
        self.assertTrue(matches[0][1].startswith("pub async fn normalize_fact("))

    def test_memory_candidate_approval_omits_v2_columns(self):
        text = source("overlay-backend/src/persistence/sqlite_store.rs")
        body = text[text.index("pub fn approve_candidate("):text.index("pub fn insert_memory_item(")]
        self.assertIn("WHERE id = ?1 AND status = 'pending'", body)
        for field in ("source_text", "norm_status", "entity"):
            self.assertNotIn(field, body)
        self.assertIn("tx.commit()", body)

    def test_hotkey_registry_labels_are_the_thirteen_dispatch_inputs(self):
        text = source("slint-experiment/src/bin/overlay_host/hotkeys.rs")
        labels = re.findall(r'\("([A-Za-z0-9+]+)",\s*\w+_hotkey\)', text)
        self.assertEqual(len(labels), 13)
        self.assertEqual(len(set(labels)), 13)
        self.assertEqual(set(labels), {"F1", "F3", "F4", "F6", "F7", "F8", "Shift+F8", "Ctrl+F8", "F9", "Shift+F9", "Shift+Alt+1", "Shift+Alt+2", "Shift+Alt+3"})

    def test_historical_error_enum_record_matches_actual_variant_names(self):
        text = source("suflyor-wsola/src/error.rs")
        body = text[text.index("pub enum WsolaError {"):text.index("impl fmt::Display")]
        names = re.findall(r"^    ([A-Z][A-Za-z0-9_]*)\s*(?:\(|\{)", body, re.MULTILINE)
        row = json.loads(source("docs/agent-map/records/errors.jsonl").strip())
        self.assertEqual([variant["name"] for variant in row["variants"]], names)
        self.assertEqual(names, ["InvalidRatio", "InputTooShort", "BufferOverflow", "InvalidState"])

    def test_ocr_uses_guarded_in_memory_stdin_stdout_not_tempfile(self):
        text = source("overlay-backend/src/ocr.rs")
        body = text[text.index("pub fn run_ocr("):text.index("fn normalize_ocr_text(")]
        self.assertIn("checked_mul(height as usize)", body)
        self.assertIn("stdin.write_all(&bmp)", body)
        self.assertIn("child.wait_with_output()", body)
        self.assertIn('cmd.args(["stdin", "stdout"', body)
        self.assertNotIn("std::fs::write", body)
        self.assertNotIn("temp_dir()", body)


if __name__ == "__main__":
    unittest.main()
