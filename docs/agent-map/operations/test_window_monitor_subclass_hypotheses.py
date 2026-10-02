"""Source and arithmetic fixtures for wave3_worker2_window C07 and C10.
No native window creation, no Win32 subclasses installed, no monitor querying.
"""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def source(path):
    return (ROOT / path).read_text()


def pack_tile_monitor_pin(left, top):
    # Mirrors window_lifecycle.rs: (i64::from(left) << 32) | i64::from(top as u32)
    MASK_64 = 0xFFFFFFFFFFFFFFFF
    left_64 = (left & 0xFFFFFFFF)
    if left < 0:
        left_64 = left & MASK_64  # sign extend in 64 bits
    packed = ((left_64 << 32) & MASK_64) | (top & 0xFFFFFFFF)
    if packed >= (1 << 63):
        packed -= (1 << 64)
    return packed


class MockMonitor:
    def __init__(self, left, top, right, bottom, is_primary):
        self.left = left
        self.top = top
        self.right = right
        self.bottom = bottom
        self.is_primary = is_primary

    def is_landscape(self):
        return (self.right - self.left) >= (self.bottom - self.top)

    def width(self):
        return self.right - self.left


def pick_monitor_mock(monitors):
    primary = next((m for m in monitors if m.is_primary), None)
    if not primary:
        return None
    upgrade = next((m for m in monitors if not m.is_primary and m.is_landscape() and m.width() >= primary.width()), None)
    return upgrade or primary


class WindowMonitorSubclassFixtures(unittest.TestCase):
    def test_tile_monitor_pin_packing_sentinel_collision(self):
        # TILE_MONITOR_AUTO is i64::MIN = -9223372036854775808
        I64_MIN = -(1 << 63)
        # When left = i32::MIN (-2147483648) and top = 0:
        # In 64-bit two's complement, (i32::MIN as i64) << 32 is exactly i64::MIN
        packed = pack_tile_monitor_pin(-(1 << 31), 0)
        self.assertEqual(packed, I64_MIN)

    def test_pick_monitor_returns_none_when_no_primary_monitor(self):
        monitors = [
            MockMonitor(-1200, 0, 0, 1920, is_primary=False),
            MockMonitor(0, 0, 1920, 1080, is_primary=False),
        ]
        self.assertIsNone(pick_monitor_mock(monitors))

    def test_pick_monitor_source_requires_is_primary(self):
        text = source("slint-experiment/src/win32.rs")
        body = text[text.index("pub fn pick_monitor(monitors: &[MonitorRect]) -> Option<MonitorRect>"):text.index("pub use crate::native::clipboard::{")]
        self.assertIn("let primary = monitors.iter().find(|m| m.is_primary).copied()?;", body)

    def test_win32_subclass_has_set_window_subclass_but_no_remove_window_subclass(self):
        text = source("slint-experiment/src/win32.rs")
        # SetWindowSubclass is imported and used
        self.assertIn("SetWindowSubclass", text)
        self.assertIn("SetWindowSubclass(", text)
        # RemoveWindowSubclass is never imported or called
        self.assertNotIn("RemoveWindowSubclass", text)

    def test_window_registry_has_no_destruction_or_unhook_method(self):
        text = source("slint-experiment/src/bin/overlay_host/window_lifecycle.rs")
        body = text[text.index("pub(crate) struct WindowRegistry {"):text.index("#[cfg(test)]")]
        # Contains apply_stealth, apply_scheme, apply_opacity, but no drop/remove/destroy/unhook
        self.assertNotIn("impl Drop for WindowRegistry", text)
        self.assertNotIn("fn unhook", body)
        self.assertNotIn("fn close", body)

    def test_original_c07_c10_statuses_remain_hypotheses(self):
        rows = json.loads((ROOT / "docs/agent-map/reconciliation/candidates.json").read_text())
        found = {row["id"]: row["status"] for row in rows}
        self.assertEqual(found["wave3_worker2_window-C07"], "hypothesis")
        self.assertEqual(found["wave3_worker2_window-C10"], "hypothesis")


if __name__ == "__main__":
    unittest.main()
