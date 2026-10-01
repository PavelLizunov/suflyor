"""Selected SDK name/scoped import and source cleanup fixtures, not Win32 proof."""
import unittest
from pathlib import Path
import windows_sdk_edges

ROOT=Path(__file__).resolve().parents[3]


class WindowsSdkFixtures(unittest.TestCase):
    def extract(self,source):
        try:return windows_sdk_edges.extract(source.encode(),'fixture.rs')
        except ModuleNotFoundError:self.skipTest('pinned Rust grammar unavailable')

    def test_nested_import_alias_direct_call_and_shadowing_remain_candidates(self):
        imports,calls=self.extract('use windows::Win32::Graphics::Gdi::{GetDC, DeleteDC as DropDC}; fn f(){GetDC(None);DropDC(mem);} fn GetDC() {}')
        self.assertEqual([r['local_name'] for r in imports],['GetDC','DropDC'])
        self.assertEqual(calls[1]['import_path_candidates'],['windows::Win32::Graphics::Gdi::DeleteDC'])
        self.assertTrue(all(r['resolved_symbol'] is False for r in calls))

    def test_macro_member_comments_excluded_qualified_calls_retained(self):
        imports,calls=self.extract('// GetDC(None)\nuse windows::Win32::Graphics::Gdi::GetDC; fn f(){log!(GetDC(None));obj.GetDC();windows::Win32::Graphics::Gdi::GetDC(None);}')
        self.assertEqual(len(calls),1)
        self.assertEqual(calls[0]['kind'],'SDK_qualified_call_syntax')

    def test_utf8_closure_range_and_function_container(self):
        text='// пример\nuse windows::X::GetDC; fn f(){let c=||GetDC(None);}'
        _,calls=self.extract(text);self.assertEqual(len(calls),1)
        self.assertEqual(calls[0]['containing_function']['name'],'f')
        self.assertEqual(text.encode()[calls[0]['start_byte']:calls[0]['end_byte']].decode(),'GetDC(None)')

    def test_invalid_rust_fail_closed(self):
        with self.assertRaises(ValueError):self.extract('use windows::X::{')

    def test_gdi_deselect_before_dib_and_regular_cleanup_partial_copy_limit(self):
        s=(ROOT/'slint-experiment/src/native/windows/screen.rs').read_text()
        self.assertLess(s.index('SelectObject(mem, old)'),s.index('GetDIBits('))
        self.assertIn('if lines == 0',s)
        self.assertNotIn('lines != h',s)
        tail=s[s.index('let _ = DeleteObject(HGDIOBJ(bmp.0))'):]
        for token in ('DeleteObject(HGDIOBJ(bmp.0))','DeleteDC(mem)','ReleaseDC(None, screen)'):self.assertIn(token,tail)

    def test_capture_restores_before_error_dispatch_and_wda_readback(self):
        s=(ROOT/'slint-experiment/src/bin/overlay_host/vision_capture.rs').read_text()
        self.assertLess(s.index('show_windows(&hidden)'),s.index('let (frozen, vx, vy) = match frozen'))
        w=(ROOT/'slint-experiment/src/win32.rs').read_text()
        body=w[w.index('pub fn set_stealth('):w.index('pub fn read_display_affinity(')]
        self.assertIn('read_display_affinity(hwnd)?',body)
        self.assertIn('actual == affinity.0',body)
        hide=w[w.index('pub fn hide_own_windows()'):w.index('pub fn show_windows(')]
        self.assertIn('DwmFlush()',hide)

    def test_tray_ui_thread_contract_drop_and_failed_install_releases_slot(self):
        s=(ROOT/'slint-experiment/src/tray.rs').read_text()
        self.assertIn('thread_local!',s)
        self.assertIn('INSTALL_SLOT.store(false, Ordering::SeqCst)',s)
        drop=s[s.index('impl Drop for TrayHandle'):s.index('pub fn install(')]
        self.assertIn('NIM_DELETE',drop);self.assertIn('DestroyWindow(self.hwnd)',drop)
        install=s[s.index('pub fn install('):s.index('fn install_win32(')]
        self.assertIn('INSTALL_SLOT.store(false, Ordering::SeqCst)',install)
        self.assertIn('publish_availability(false)',install)


if __name__=='__main__':unittest.main()
