"""Source and arithmetic fixtures for wave3_worker2_window C05 and C06.
No Win32 window creation, no monitor reconfiguration, no DWM calls.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


class WindowGeometryTaskbarFixtures(unittest.TestCase):
    def test_set_skip_taskbar_unconditionally_shows_window_on_style_change(self):
        text = source("slint-experiment/src/win32.rs")
        body = text[text.index("pub fn set_skip_taskbar("):text.index("pub fn set_window_owner(")]
        # When ex_style changes, it calls ShowWindow(SW_HIDE) then ShowWindow(SW_SHOWNOACTIVATE)
        idx_diff = body.index("if after == before")
        idx_hide = body.index("ShowWindow(hwnd, SW_HIDE)")
        idx_show = body.index("ShowWindow(hwnd, SW_SHOWNOACTIVATE)")
        idx_topmost = body.index("SetWindowPos(")

        self.assertLess(idx_diff, idx_hide)
        self.assertLess(idx_hide, idx_show)
        self.assertLess(idx_show, idx_topmost)
        self.assertIn("SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE", body)

    def test_apply_bar_stealth_always_invokes_set_skip_taskbar(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("pub(crate) fn apply_bar_stealth("):text.index("fn apply_stealth_one(")]
        # apply_bar_stealth invokes set_skip_taskbar without checking BAR_TRAY_HIDDEN
        self.assertIn("slint_replay::win32::set_skip_taskbar(hwnd, effective)", body)
        self.assertNotIn("BAR_TRAY_HIDDEN", body)

    def test_apply_overlay_hwnd_bar_w_fallback_zero_centers_at_half_width(self):
        # Simulation of lines 418-422 in bar_tray.rs:
        # let primary = ...;
        # let bar_w = get_window_rect(hwnd).map(|(_, _, w, _)| w).unwrap_or(0);
        # let (x, y) = match primary { Some(p) => (p.left + ((p.width() - bar_w) / 2).max(0), p.top + 24), None => (60, 24) };
        class MockMon:
            left = 0
            top = 0
            w = 1920

            def width(self):
                return self.w

        p = MockMon()
        # Normal sized bar: w = 1200
        normal_bar_w = 1200
        normal_x = p.left + max(0, (p.width() - normal_bar_w) // 2)
        self.assertEqual(normal_x, 360)  # centered properly

        # Unset / un-sized bar fallback w = 0
        unset_bar_w = 0
        half_off_x = p.left + max(0, (p.width() - unset_bar_w) // 2)
        self.assertEqual(half_off_x, 960)  # Left edge placed at monitor midpoint!

    def test_apply_overlay_hwnd_source_calculates_pin_with_raw_rect_fallback_zero(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("pub(super) fn apply_overlay_hwnd("):]
        # Evaluates bar_w using get_window_rect unwrap_or(0)
        self.assertIn("let bar_w = get_window_rect(hwnd).map(|(_, _, w, _)| w).unwrap_or(0);", body)
        self.assertIn("Some(p) => (p.left + ((p.width() - bar_w) / 2).max(0), p.top + 24),", body)

    def test_recenter_when_sized_contrasts_by_waiting_for_target_width(self):
        text = source("slint-experiment/src/bin/overlay_host/bar_tray.rs")
        body = text[text.index("fn recenter_when_sized("):text.index("pub(super) fn tray_menu_action(")]
        # Waits until (cur_w - target_width).abs() <= 24 with retry loop
        self.assertIn("if (cur_w - target_width).abs() > 24 && attempt < 12 {", body)
        self.assertIn("recenter_when_sized(weak.clone(), target_w_logical, attempt + 1);", body)

    def test_original_c05_c06_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker2_window-C05"], "hypothesis")
        self.assertEqual(found["wave3_worker2_window-C06"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
