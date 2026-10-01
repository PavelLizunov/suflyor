"""Canonical PowerShell parser-only fixtures, not Windows/runtime/action tests."""
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import powershell_parser


class CanonicalPowerShellFixtures(unittest.TestCase):
    def parse(self, source):
        if not os.environ.get('SUFLYOR_RESEARCH_POWERSHELL'):
            self.skipTest('verified portable PowerShell runtime unavailable')
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/'fixture.ps1';path.write_text(source,encoding='utf-8')
            result=powershell_parser.parse(path,'fixture.ps1')
            self.assertFalse(result.get('native_parser_failure'),result)
            self.assertFalse(result['script_executed'])
            return result,source.encode()

    def test_valid_size_suffix_native_args_and_treesitter_gaps(self):
        result,_=self.parse('function F { [math]::Round(10 / 1MB, 2); & git.exe for-each-ref --format="%(refname)"; [StringComparison]::OrdinalIgnoreCase }')
        self.assertFalse(result['has_parse_error'])
        self.assertEqual([r['name'] for r in result['declarations']],['F'])

    def test_nested_functions_class_properties_methods_enum_scopes(self):
        source='''function Outer {
 function Inner { param($x); $x }
}
class C {
 [string] $Value
 [int] Get() { return 1 }
}
enum E { One; Two }
'''
        result,_=self.parse(source);self.assertFalse(result['has_parse_error'])
        rows=result['declarations'];names=[r['name'] for r in rows]
        for name in ('Outer','Inner','C','Value','Get','E','One','Two'):self.assertIn(name,names)
        inner=next(r for r in rows if r['name']=='Inner');self.assertEqual(inner['scope_names'],['Outer'])
        get=next(r for r in rows if r['name']=='Get');self.assertIn('C',get['scope_names'])

    def test_here_string_embedded_csharp_comments_not_functions_or_execution(self):
        source='''# function Fake {}
$text = @"
function Fake2 {}
using System;
"@
function Real { throw "DO-NOT-EXECUTE" }
throw "INPUT-MUST-NOT-RUN"
'''
        result,_=self.parse(source);self.assertFalse(result['has_parse_error'])
        self.assertEqual([r['name'] for r in result['declarations']],['Real'])

    def test_utf16_utf8_emoji_crlf_ranges(self):
        source='# Пример 😀\r\nfunction Real { "строка" }\r\n'
        result,data=self.parse(source);row=result['declarations'][0]
        self.assertEqual(row['start_line'],2)
        self.assertEqual(data[row['start_byte']:row['end_byte']].decode(),'function Real { "строка" }')
        self.assertFalse(row['semantic_acceptance'])

    def test_utf8_bom_matches_parsefile_and_preserves_raw_offsets(self):
        source='\ufeff# header\n[CmdletBinding()]\nparam([int]$Value = 1)\nfunction Real { $Value }\n'
        result,data=self.parse(source)
        self.assertFalse(result['has_parse_error'])
        row=result['declarations'][0]
        self.assertEqual(data[row['start_byte']:row['end_byte']].decode(),'function Real { $Value }')
        self.assertEqual(row['start_line'],4)

    def test_invalid_input_reports_error_id_and_extent(self):
        result,_=self.parse('function Broken {')
        self.assertTrue(result['has_parse_error']);self.assertTrue(result['parse_errors'])
        self.assertIn('error_id',result['parse_errors'][0])

    def test_runtime_absence_and_wrong_hash_fail_closed(self):
        with mock.patch.dict(os.environ,{},clear=True):
            result=powershell_parser.parse(Path('fixture.ps1'),'fixture.ps1')
        self.assertTrue(result['native_parser_failure'])
        with tempfile.TemporaryDirectory() as td:
            with mock.patch.dict(os.environ,{'SUFLYOR_RESEARCH_POWERSHELL':td}):
                result=powershell_parser.parse(Path('fixture.ps1'),'fixture.ps1')
        self.assertEqual(result['parse_errors'][0]['kind'],'canonical_runtime_unavailable_or_hash_mismatch')


if __name__=='__main__':
    unittest.main()
