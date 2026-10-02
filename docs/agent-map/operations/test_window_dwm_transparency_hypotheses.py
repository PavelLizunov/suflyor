"""Source and bitmask fixtures for wave3_worker2_window C08 and C09.
No Win32 window creation, no DWM composition calls, no live display server.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


# Win32 constants from win32.rs
WS_EX_TOOLWINDOW = 0x00000080
WS_EX_APPWINDOW = 0x00040000
WS_EX_TRANSPARENT = 0x00000020


def skip_taskbar_exstyle(before, skip):
    if skip:
        return (before | WS_EX_TOOLWINDOW) & ~WS_EX_APPWINDOW
    else:
        return (before | WS_EX_TOOLWINDOW) & ~WS_EX_APPWINDOW


def transparency_exstyle(before, click_through):
    taskbar_safe = skip_taskbar_exstyle(before, True)
    if click_through:
        return taskbar_safe | WS_EX_TRANSPARENT
    else:
        return taskbar_safe & ~WS_EX_TRANSPARENT


class WindowDwmTransparencyFixtures(unittest.TestCase):
    def test_transparency_exstyle_click_through_toggles_transparent_bit(self):
        base_style = 0x00040000  # WS_EX_APPWINDOW
        # When click_through is True (overlay bar), WS_EX_TRANSPARENT is set and APPWINDOW is cleared
        overlay_style = transparency_exstyle(base_style, click_through=True)
        self.assertTrue(overlay_style & WS_EX_TRANSPARENT)
        self.assertTrue(overlay_style & WS_EX_TOOLWINDOW)
        self.assertFalse(overlay_style & WS_EX_APPWINDOW)

        # When click_through is False (tile window), WS_EX_TRANSPARENT is cleared
        tile_style = transparency_exstyle(overlay_style, click_through=False)
        self.assertFalse(tile_style & WS_EX_TRANSPARENT)
        self.assertTrue(tile_style & WS_EX_TOOLWINDOW)
        self.assertFalse(tile_style & WS_EX_APPWINDOW)

    def test_apply_transparency_calls_dwm_extend_and_blur_behind_without_layered_attribute(self):
        text = source("slint-experiment/src/win32.rs")
        body = text[text.index("fn apply_transparency("):text.index("fn transparency_exstyle(")]
        # DwmExtendFrameIntoClientArea with -1 margins
        self.assertIn("cxLeftWidth: -1", body)
        self.assertIn("DwmExtendFrameIntoClientArea(hwnd, &margins)", body)
        # DwmEnableBlurBehindWindow with -1,-1 rect region
        self.assertIn("CreateRectRgn(0, 0, -1, -1)", body)
        self.assertIn("DwmEnableBlurBehindWindow(hwnd, &bb)", body)
        # Note absence of SetLayeredWindowAttributes in apply_transparency
        self.assertNotIn("SetLayeredWindowAttributes", body)

    def test_apply_transparency_strips_ghost_caption_buttons(self):
        text = source("slint-experiment/src/win32.rs")
        body = text[text.index("fn apply_transparency("):text.index("fn transparency_exstyle(")]
        # Clears WS_SYSMENU, WS_MAXIMIZEBOX, WS_MINIMIZEBOX
        self.assertIn("WS_SYSMENU.0 as isize | WS_MAXIMIZEBOX.0 as isize | WS_MINIMIZEBOX.0 as isize", body)
        self.assertIn("SWP_FRAMECHANGED", body)

    def test_make_transparent_overlay_and_tile_dispatch_boolean_click_through(self):
        text = source("slint-experiment/src/win32.rs")
        self.assertIn("pub fn make_transparent_overlay(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>> {\n        apply_transparency(hwnd, /* click_through */ true)\n    }", text)
        self.assertIn("pub fn make_transparent_tile(hwnd: HWND) -> Result<(), Box<dyn std::error::Error>> {\n        apply_transparency(hwnd, /* click_through */ false)\n    }", text)

    def test_set_always_on_top_swp_nomove_nosize_flags(self):
        text = source("slint-experiment/src/win32.rs")
        body = text[text.index("pub fn set_always_on_top("):text.index("pub fn set_stealth(")]
        self.assertIn("if on { HWND_TOPMOST } else { HWND_NOTOPMOST }", body)
        self.assertIn("SWP_NOMOVE | SWP_NOSIZE", body)

    def test_original_c08_c09_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker2_window-C08"], "hypothesis")
        self.assertEqual(found["wave3_worker2_window-C09"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
