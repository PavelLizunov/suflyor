"""Name-candidate and source setting-chain fixtures, not resolved/native graph."""
import unittest
from pathlib import Path

import config_ui_edges

ROOT=Path(__file__).resolve().parents[3]


class ConfigUiEdgeFixtures(unittest.TestCase):
    def extract(self,text):
        try:return config_ui_edges.extract(text.encode(),'fixture.rs',{'value','opacity'},{'set_value':[{'kind':'property','name':'value'}]})
        except ModuleNotFoundError:self.skipTest('pinned Rust grammar unavailable')

    def test_nonconfig_collision_nested_receiver_and_write_lhs_explicitly_unresolved(self):
        rows=self.extract('fn f() { c.value=1; self.value; unrelated.value; cfg.read().opacity; }')
        self.assertEqual(len(rows),4)
        self.assertEqual([r['direct_assignment_lhs'] for r in rows],[True,False,False,False])
        self.assertEqual(rows[2]['receiver_identifier'],'unrelated')
        self.assertEqual(rows[3]['receiver_node_type'],'call_expression')
        self.assertTrue(all(r['resolved_type'] is False for r in rows))

    def test_method_name_is_not_field_read_and_ui_calls_have_candidate_decl_only(self):
        rows=self.extract('fn f() { c.value(); win.set_value(c.value); }')
        self.assertEqual([r['kind'] for r in rows],['Slint_generated_method_name_candidate','Config_field_name_candidate'])
        self.assertEqual(rows[0]['ui_declaration_candidates'][0]['name'],'value')
        self.assertFalse(rows[0]['semantic_acceptance'])

    def test_macro_strings_comments_excluded_utf8_closure_ranges(self):
        text='// пример c.value\nfn f() { let x="c.value"; log!(c.value); let y=|| c.value; }'
        rows=self.extract(text);self.assertEqual(len(rows),1)
        self.assertTrue(rows[0]['inside_closure'])
        self.assertEqual(text.encode()[rows[0]['start_byte']:rows[0]['end_byte']].decode(),'c.value')
        self.assertEqual(rows[0]['containing_function']['name'],'f')

    def test_invalid_rust_fail_closed(self):
        with self.assertRaises(ValueError):self.extract('fn f( {')

    def test_opacity_mutates_before_save_but_global_apply_after_success(self):
        s=(ROOT/'slint-experiment/src/bin/overlay_host/settings_controller.rs').read_text()
        body=s[s.index('win.on_tile_body_opacity_changed'):s.index('// Tile-placement monitor picker')]
        self.assertLess(body.index('c.tile_body_opacity = clamped'),body.index('config::save(&c)'))
        self.assertLess(body.index('return;'),body.index('set_global_tile_opacity(clamped)'))
        self.assertLess(body.index('config::save(&c)'),body.index('tile.set_body_opacity(clamped)'))

    def test_scheme_save_then_apply_but_monitor_runtime_first(self):
        s=(ROOT/'slint-experiment/src/bin/overlay_host/settings_controller.rs').read_text()
        scheme=s[s.index('win.on_color_scheme_selected'):s.index('// P0 — Diagnostics')]
        self.assertLess(scheme.index('c.color_scheme = scheme'),scheme.index('config::save(&c)'))
        self.assertLess(scheme.index('return;'),scheme.index('set_global_scheme(scheme)'))
        monitor=s[s.index('win.on_tile_monitor_changed'):s.index('// Phase E6 v38')]
        self.assertLess(monitor.index('set_global_tile_monitor(pin)'),monitor.index('c.tile_monitor_name = pin.map('))
        self.assertLess(monitor.index('c.tile_monitor_name = pin.map('),monitor.index('config::save(&c)'))
        self.assertNotIn('return;',monitor)

    def test_language_select_before_config_save_and_runtime_default_exact_tokens(self):
        s=(ROOT/'slint-experiment/src/bin/overlay_host/settings_controller.rs').read_text()
        body=s[s.index('win.on_language_selected'):s.index('// Colour-scheme switch.')]
        self.assertLess(body.index('slint::select_bundled_translation(lang)'),body.index('c.ui_language = lang.to_string()'))
        self.assertLess(body.index('c.ui_language = lang.to_string()'),body.index('config::save(&c)'))
        runtime=(ROOT/'slint-experiment/src/bin/overlay_host_windows.rs').read_text()
        self.assertIn('set_global_tile_opacity(cfg.read().tile_body_opacity)',runtime)
        self.assertIn('set_global_scheme(cfg.read().color_scheme)',runtime)
        self.assertIn('slint::select_bundled_translation(&lang)',runtime)


if __name__=='__main__':unittest.main()
