"""Source fixtures for wave3_worker2_window C02 and C04.
No display affinity modification, no window creation, no native GUI calls.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class WindowStealthLifecycleFixtures(unittest.TestCase):
    def test_present_window_stealth_aware_at_parks_offscreen_before_show(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("pub(crate) fn present_window_stealth_aware_at<W, F>("):text.index("let do_reveal: Rc<dyn Fn(&W) -> bool>")]
        # Parking at (-32000, -32000) precedes win.show()
        idx_pos = body.index("set_platform_window_position(win.window(), -32000, -32000);")
        idx_show = body.index("let _ = win.show();")
        self.assertLess(idx_pos, idx_show)

    def test_do_reveal_applies_stealth_before_moving_onscreen(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("let do_reveal: Rc<dyn Fn(&W) -> bool> = Rc::new(move |w: &W| -> bool {"):text.index("let fallback_reveal: Rc<dyn Fn(&W)>")]
        # In do_reveal: grab_hwnd -> decorate -> apply_stealth_one -> move_window_pos_only
        idx_grab = body.index("let Ok(hwnd) = grab_hwnd(w.window())")
        idx_stealth = body.index("apply_stealth_one(hwnd, true);")
        idx_move = body.index("move_window_pos_only(hwnd,")
        self.assertLess(idx_grab, idx_stealth)
        self.assertLess(idx_stealth, idx_move)

    def test_fallback_reveal_keeps_window_parked_offscreen_under_stealth(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("let fallback_reveal: Rc<dyn Fn(&W)> = Rc::new(move |w: &W| {"):text.index("realize_with_retries(win, do_reveal, fallback_reveal);")]
        # When global_stealth() is true, fallback_reveal returns without moving window on-screen
        self.assertIn("if global_stealth() {", body)
        self.assertIn("parked off-screen under stealth", body)
        self.assertIn("return;", body)

    def test_window_registry_apply_stealth_does_not_include_tray_menu_or_capture(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("pub(crate) fn apply_stealth(&self, on: bool) {"):text.index("pub(crate) fn apply_scheme(&self, scheme: i32)")]
        # Iterates tiles, settings, palette, text_ask, wizard, help, recover_offer, transcript, archive, lock_menu
        self.assertIn("for t in self.tiles.borrow().iter()", body)
        self.assertIn("if let Some(sw) = self.settings.borrow().as_ref()", body)
        self.assertIn("if let Some(p) = self.palette.borrow().as_ref()", body)
        self.assertIn("if let Some(t) = self.text_ask.borrow().as_ref()", body)
        # Note absence of self.tray_menu or self.capture in the walk
        self.assertNotIn("self.tray_menu", body)
        self.assertNotIn("self.capture", body)

    def test_open_tray_menu_applies_stealth_at_realization_only(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("fn open_tray_menu("):text.index("pub(super) fn hide_bar_to_tray(")]
        # Parks at (-32000, -32000) then shows
        self.assertIn("PhysicalPosition::new(-32000, -32000)", body)
        self.assertIn("menu.show()", body)
        # In reveal closure, applies set_stealth before move_window_pos_only
        idx_stealth = body.index("if global_stealth() && set_stealth(hwnd, true).is_err()")
        idx_move = body.index("if move_window_pos_only(hwnd, x, y).is_err()")
        self.assertLess(idx_stealth, idx_move)

    def test_original_c02_c04_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker2_window-C02"], "hypothesis")
        self.assertEqual(found["wave3_worker2_window-C04"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
