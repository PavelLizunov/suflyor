"""Source ownership assertions only, no native callback/device/ABI proof."""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text(encoding="utf-8")


class NativeOwnershipSourceTests(unittest.TestCase):
    def test_screenshot_length_validated_before_copy_and_free_on_invalid(self):
        s = source("slint-experiment/src/native/macos/screen.rs")
        body = s[s.index("pub fn capture_display_bgra_with_dimensions"):s.index("pub fn recognize_text_from_bgra")]
        self.assertLess(body.index("checked_mul"), body.index("from_raw_parts"))
        self.assertLess(body.index("if expected != Some(len)"), body.index("slice.to_vec()"))
        self.assertGreaterEqual(body.count("suflyor_macos_free_screenshot_buffer(bytes_ptr)"), 2)

    def test_native_timeout_does_not_cancel_late_image_callback(self):
        s = source("slint-experiment/src/native/macos/screen.m")
        body = s[s.index("int32_t suflyor_macos_capture_display_bgra"):s.index("void suflyor_macos_free_screenshot_buffer")]
        self.assertIn("CGImageRetain(image)", body)
        self.assertIn("5 * NSEC_PER_SEC", body)
        self.assertIn("return -2", body)
        self.assertNotIn("cancel", body)
        # Source-only absence assertion, not a native memory-leak reproducer.

    def test_ocr_null_success_and_matching_text_free(self):
        s = source("slint-experiment/src/native/macos/screen.rs")
        body = s[s.index("pub fn recognize_text_from_bgra"):s.index("#[cfg(test)]")]
        self.assertIn("if out_text_ptr.is_null()", body)
        self.assertIn("return Ok(String::new())", body)
        self.assertIn("to_string_lossy().into_owned()", body)
        self.assertIn("suflyor_macos_free_string(out_text_ptr)", body)

    def test_host_screenshot_ui_path_and_ocr_spawn_blocking_are_distinct(self):
        s = source("slint-experiment/src/bin/overlay_host/vision_capture.rs")
        freeze = s[s.index("pub(crate) fn fire_f8_vision_capture"):s.index("let (frozen, vx, vy)")]
        self.assertIn("capture_virtual_desktop()", freeze)
        self.assertNotIn("spawn_blocking", freeze)
        self.assertIn("tokio::task::spawn_blocking(move || run_local_ocr", s)

    def test_pending_system_worker_detaches_but_mic_start_wait_is_unbounded(self):
        s = source("overlay-backend/src/audio_macos.rs")
        self.assertIn("state != SYSTEM_WORKER_PENDING", s)
        drop_body = s[s.index("impl Drop for CaptureHandle"):s.index("pub fn start_capture(")]
        self.assertIn("if system_worker_joinable", drop_body)
        self.assertIn("worker detached", drop_body)
        self.assertIn("match started_rx.recv()", s)
        self.assertNotIn("started_rx.recv_timeout", s)
        self.assertIn("let callback = unsafe { Box::from_raw(context.cast::<F>()) }", s)
        self.assertIn("catch_unwind", s)

    def test_native_build_flags_preserve_mic_manual_ownership_vs_arc_system(self):
        s = source("overlay-backend/build.rs")
        mic = s[s.index("fn build_mic_capture()"):s.index("fn build_system_capture()")]
        system = s[s.index("fn build_system_capture()"):s.index("fn build_process_memory()")]
        self.assertNotIn('flag("-fobjc-arc")', mic)
        self.assertIn('flag("-fobjc-arc")', system)
        native = source("overlay-backend/native/macos/system_capture.m")
        self.assertIn("SuflyorSystemCaptureState *callback_state = state", native)
        self.assertIn("if (safe_release && c->state_ref != NULL)", native)
        self.assertIn("CFRelease((CFTypeRef)c->state_ref)", native)
        self.assertIn("initStereoGlobalTapButExcludeProcesses:@[]", native)
        status = source("slint-experiment/src/native/macos/status.rs")
        self.assertIn("PhantomData<Rc<()>>", status)
        self.assertIn("suflyor_macos_status_remove()", status)


if __name__ == "__main__":
    unittest.main()
