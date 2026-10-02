"""Source fixtures for wave4_worker1_privacy C04 and C05.
No screen capture, no clipboard API access, no audio synthesis.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class PrivacyCaptureClipboardFixtures(unittest.TestCase):
    def test_bgra_to_slint_image_forces_opaque_alpha_and_swaps_channels(self):
        text = source("slint-experiment/src/bin/overlay_host/vision_capture.rs")
        body = text[text.index("pub(crate) fn bgra_to_slint_image"):text.index("fn local_ocr_available")]
        # Pixel channel swap: slot[0] = B->R (px[2]), slot[1] = G (px[1]), slot[2] = R->B (px[0]), slot[3] = 255 (A)
        self.assertIn("slot[0] = px[2]; // R", body)
        self.assertIn("slot[1] = px[1]; // G", body)
        self.assertIn("slot[2] = px[0]; // B", body)
        self.assertIn("slot[3] = 255; // A", body)
        # Note absence of temporary file writes or raw dumps in vision_capture
        self.assertNotIn("std::fs::write", body)
        self.assertNotIn("temp_dir", body)

    def test_journal_event_marks_attached_screenshot_boolean_without_raw_pixels(self):
        text = source("slint-experiment/src/bin/overlay_host/vision_capture.rs")
        body = text[text.index("attached_screenshot: true,"):text.index("let ai_rx = ai::stream_chat_endpoint")]
        self.assertIn("attached_screenshot: true,", body)
        # Does not serialize raw bitmap data into journal event
        self.assertNotIn("raw_pixels", body)
        self.assertNotIn("bgra_bytes", body)

    def test_restore_text_clipboard_handles_single_string_snapshot(self):
        text = source("slint-experiment/src/bin/overlay_host/read_aloud.rs")
        body = text[text.index("pub(super) fn restore_text_clipboard("):text.index("pub(super) fn spawn_text_tile(")]
        self.assertIn("Some(text) => slint_replay::win32::clipboard_write_text(text),", body)
        self.assertIn("None => slint_replay::win32::clipboard_clear(),", body)
        # Only restores single text snapshot, no clipboard history scraping
        self.assertNotIn("history", body.lower())

    def test_spawn_text_tile_stores_conversation_and_speaks_unredacted(self):
        text = source("slint-experiment/src/bin/overlay_host/read_aloud.rs")
        body = text[text.index("pub(super) fn spawn_text_tile("):text.index("pub(super) fn fill_ocr_tile(")]
        # Stores verbatim text in conversation
        self.assertIn("content: ai::MessageContent::Text(text.to_string()),", body)
        self.assertIn("speak_explicit(text, convo_id);", body)
        # No redaction filter applied to text before speaking
        self.assertNotIn("redact", body)

    def test_last_closed_read_tile_retains_single_recent_tile(self):
        text = source("slint-experiment/src/bin/overlay_host/read_aloud.rs")
        # thread-local single RefCell slot
        self.assertIn("pub(super) static LAST_CLOSED_READ_TILE: RefCell<Option<TileWindow>> = const { RefCell::new(None) };", text)
        body = text[text.index("tile.on_close_clicked("):text.index("o.set_can_restore_tile(true);")]
        self.assertIn("LAST_CLOSED_READ_TILE.with(|slot| slot.borrow_mut().replace(t))", body)

    def test_original_c04_c05_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave4_worker1_privacy-C04"], "hypothesis")
        self.assertEqual(found["wave4_worker1_privacy-C05"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
