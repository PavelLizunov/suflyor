# Original privacy C04/C05: bounded screen capture and clipboard handling evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_privacy_capture_clipboard_hypotheses.py) inspect frozen screen capture buffer conversion and clipboard read-aloud logic. They do not trigger screen capture, access OS clipboard history, or synthesize speech. C04 and C05 remain hypotheses.

## C04 — screen capture buffer in-memory lifecycle and journal serialization

In `vision_capture.rs`, [bgra_to_slint_image](<../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L51-L75>) constructs a Slint RGBA buffer directly in memory, swapping blue and red channels (`slot[0] = px[2]`, `slot[2] = px[0]`) and forcing the alpha channel to `255` (`slot[3] = 255`).
The capture workflow does not write unencrypted raw pixel dumps to disk temp directories.
When recording AI vision queries to the journal, [AiRequest event](<../../../slint-experiment/src/bin/overlay_host/vision_capture.rs#L867-L876>) records `attached_screenshot: true` along with text prompts, without embedding raw pixel buffers into the journal.

## C05 — clipboard snapshot restoration and unredacted read-aloud storage

In `read_aloud.rs`, [restore_text_clipboard](<../../../slint-experiment/src/bin/overlay_host/read_aloud.rs#L27-L40>) only manages a single saved string snapshot, restoring it via `clipboard_write_text` or clearing it, without accessing Windows or macOS multi-entry clipboard history APIs.
When spawning a read-aloud tile, [spawn_text_tile](<../../../slint-experiment/src/bin/overlay_host/read_aloud.rs#L70-L85>) and `fill_ocr_tile` pass the raw captured text into `bridge.store_conversation` and invoke `speak_explicit` without filtering through secret redaction rules.
Closing a read-aloud tile preserves the single most recent window in the thread-local [LAST_CLOSED_READ_TILE](<../../../slint-experiment/src/bin/overlay_host/read_aloud.rs#L20-L25>) slot to allow restoration via the overlay bar.

## Limits

No screen capture APIs were executed, no clipboard history was modified, and no voice synthesis audio was generated. Original statuses in `candidates.json` remain `hypothesis`.
