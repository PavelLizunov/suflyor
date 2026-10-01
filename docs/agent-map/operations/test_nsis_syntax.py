"""Prebuilt NSIS WASM syntax fixtures; never native installer/preprocessor proof."""
import hashlib
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import syntax_index


class NsisSyntaxFixtures(unittest.TestCase):
    def parse(self, source):
        if not os.environ.get("SUFLYOR_RESEARCH_NSIS_WASM") or not shutil.which("node"):
            self.skipTest("hash-pinned NSIS WASM runtime unavailable")
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "fixture.nsi"
            path.write_text(source, encoding="utf-8")
            result = syntax_index.isolated_rust_index(path, "fixture.nsi", "--nsis-worker")
            self.assertFalse(result.get("native_parser_failure"), result)
            return result, source.encode()

    def test_utf16_to_utf8_ranges_emoji_and_cyrillic(self):
        result, data = self.parse('; текст 😀\nFunction real\n MessageBox MB_OK "строка"\nFunctionEnd\n')
        self.assertFalse(result['has_parse_error'])
        row = next(r for r in result['declarations'] if r['kind']=='function_definition')
        self.assertEqual(row['name'], 'real')
        self.assertEqual(row['start_line'], 2)
        self.assertEqual(row['source_sha256'], hashlib.sha256(data).hexdigest())
        self.assertEqual(data[row['start_byte']:row['end_byte']].decode(), 'Function real\n MessageBox MB_OK "строка"\nFunctionEnd')

    def test_comments_strings_do_not_create_functions_and_labels_keep_scope(self):
        result, _ = self.parse('; Function fake\nFunction real\nlabel:\n MessageBox MB_OK "Function fake2"\nFunctionEnd\n')
        self.assertFalse(result['has_parse_error'])
        rows = result['declarations']
        self.assertEqual([r['name'] for r in rows if r['kind']=='function_definition'], ['real'])
        label = next(r for r in rows if r['kind']=='label')
        self.assertEqual(label['scope_names'], ['real'])
        self.assertEqual(label['parent_ids'], [rows[0]['id']])

    def test_macro_sections_variable_and_preprocessor_not_expanded(self):
        result, _ = self.parse('!define VALUE "yes"\nVar target\n!macro M ARG\n DetailPrint "${ARG}"\n!macroend\nSectionGroup "Group"\nSection "Main" ID\n !insertmacro M x\nSectionEnd\nSectionGroupEnd\n')
        self.assertFalse(result['has_parse_error'])
        kinds = {r['kind'] for r in result['declarations']}
        self.assertTrue({'preproc_directive','variable_declaration','macro_definition','section_group','section_definition'} <= kinds)
        section = next(r for r in result['declarations'] if r['kind']=='section_definition')
        self.assertEqual(section['name'], '"Main"')
        self.assertEqual(section['scope_names'], ['"Group"'])
        self.assertTrue(all(r['macro_expanded'] is False and r['semantic_acceptance'] is False for r in result['declarations']))

    def test_missing_end_is_error_not_success(self):
        result, _ = self.parse('Function missing\nDetailPrint "x"\n')
        self.assertTrue(result['has_parse_error'])
        self.assertTrue(result['parse_errors'])

    def test_runtime_failure_and_invalid_json_are_explicit(self):
        with mock.patch.object(syntax_index.subprocess, 'run', side_effect=FileNotFoundError):
            result = syntax_index.isolated_rust_index(Path('fixture.nsi'), 'fixture.nsi', '--nsis-worker')
        self.assertEqual(result['parse_errors'][0]['kind'], 'parser_runtime_unavailable')
        self.assertTrue(result['native_parser_failure'])

    def test_wrong_wasm_hash_fails_before_require(self):
        if not shutil.which('node'):
            self.skipTest('Node unavailable')
        with tempfile.TemporaryDirectory() as td:
            base=Path(td);(base/'web-tree-sitter').mkdir();(base/'web-tree-sitter/tree-sitter.cjs').write_text('throw new Error("EXECUTED");')
            src=base/'fixture.nsi';src.write_text('Function real\nFunctionEnd\n')
            env=dict(os.environ,SUFLYOR_RESEARCH_NSIS_WASM=td)
            result=subprocess.run(['node',str(Path(syntax_index.__file__).with_name('nsis_worker.cjs')),str(src),'fixture.nsi'],env=env,text=True,capture_output=True)
            self.assertNotEqual(result.returncode,0)
            self.assertIn('hash mismatch',result.stderr)
            self.assertNotIn('EXECUTED',result.stderr)


if __name__=='__main__':
    unittest.main()
