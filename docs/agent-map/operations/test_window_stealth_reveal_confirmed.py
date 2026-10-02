"""Source fixtures for wave3_worker2_window C01 and C03 (confirmed mechanisms).
No live Win32 window creation, no display server interaction.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class WindowStealthRevealFixtures(unittest.TestCase):
    def test_do_reveal_in_window_lifecycle_unconditionally_moves_window_on_screen(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("let do_reveal: Rc<dyn Fn(&W) -> bool> ="):text.index("let fallback_reveal:")]
        # apply_stealth_one is called if global_stealth()
        self.assertIn("apply_stealth_one(hwnd, true);", body)
        # Window is moved on-screen without checking if apply_stealth_one succeeded
        self.assertIn("move_window_pos_only(hwnd, cx, cy)", body)
        self.assertIn("set_platform_window_position(w.window(), cx, cy)", body)
        self.assertNotIn("if apply_stealth_one", body)

    def test_open_tray_menu_aborts_reveal_and_hides_window_on_stealth_failure(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("let reveal = Rc::new(move |window: &TrayMenuWindow| {"):text.index("pub(super) fn hide_bar_to_tray")]
        # Tray menu checks set_stealth and returns false on error
        self.assertIn("if global_stealth() && set_stealth(hwnd, true).is_err() {", body)
        self.assertIn("return false;", body)
        # Fallback explicitly hides the window if native reveal was aborted
        self.assertIn("let _ = window.hide();", body)

    def test_bar_fallback_reveals_on_screen_even_under_stealth(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("let fallback: Rc<dyn Fn(&OverlayBarWindow)> ="):text.index("realize_with_retries(overlay, attempt, fallback);")]
        # Sets effective stealth to false and disables active stealth
        self.assertIn("set_global_stealth_effective(false);", body)
        self.assertIn("o.set_stealth_active(false);", body)
        # Still moves the bar on-screen to avoid lockout
        self.assertIn("set_platform_window_position(o.window(), x, y);", body)

    def test_aux_window_fallback_keeps_window_parked_off_screen_under_stealth(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("let fallback_reveal: Rc<dyn Fn(&W)> ="):text.index("realize_with_retries(win, do_reveal, fallback_reveal);")]
        # Under stealth, aux window stays parked off-screen and returns early
        self.assertIn("if global_stealth() {", body)
        self.assertIn("parked off-screen under stealth", body)
        self.assertIn("return;", body)

    def test_apply_overlay_hwnd_parks_bar_at_32000_initially(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("pub(super) fn apply_overlay_hwnd("):text.index("let attempt: Rc<dyn Fn(&OverlayBarWindow) -> bool> =")]
        self.assertIn("set_platform_window_position(overlay.window(), -32000, -32000);", body)

    def test_original_c01_c03_statuses_remain_confirmed(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker2_window-C01"], "confirmed")
        self.assertEqual(found["wave3_worker2_window-C03"], "confirmed")


if __name__ == "__main__":
    unittest.main()
