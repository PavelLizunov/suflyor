"""Native declaration/direct-call navigation fixtures; no compiler/ABI proof."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import native_ffi_inventory as ffi


class NativeFfiFixtures(unittest.TestCase):
    def rust(self, data):
        try:
            return ffi.rust_edges(data, "fixture.rs", {"bridge"})
        except ModuleNotFoundError:
            self.skipTest("pinned Rust grammar unavailable")

    def test_prototypes_not_calls_comments_strings_or_macro_token_trees(self):
        data = b'''// bridge(ptr)
extern "C" { fn bridge(ptr: *mut u8) -> i32; }
fn caller() { let text = "bridge(ptr)"; log!(bridge(ptr)); }
'''
        rows = self.rust(data)
        self.assertEqual(len(rows["foreign_declarations"]), 1)
        self.assertEqual(rows["foreign_declarations"][0]["foreign_abi_source"], 'extern "C"')
        self.assertEqual(rows["direct_calls"], [])

    def test_direct_qualified_and_closure_call_ranges(self):
        data = '// пример\nfn caller() { unsafe { bridge(ptr); crate::m::bridge(ptr); let x = || bridge(ptr); } }'.encode()
        rows = self.rust(data)
        calls = rows["direct_calls"]
        self.assertEqual(len(calls), 3)
        self.assertEqual([r["callee_source"] for r in calls], ["bridge", "crate::m::bridge", "bridge"])
        self.assertEqual([r["inside_closure"] for r in calls], [False, False, True])
        for row in calls:
            self.assertEqual(row["containing_function"]["name"], "caller")
            self.assertIn("bridge", data[row["start_byte"]:row["end_byte"]].decode())
            self.assertEqual(row["start_line"], 2)

    def test_bare_argument_reference_is_not_a_direct_call(self):
        rows = self.rust(b'fn f() { register(bridge); let ptr = bridge; }')
        self.assertEqual(rows["direct_calls"], [])
        self.assertEqual(len(rows["argument_references"]), 1)
        self.assertEqual(rows["argument_references"][0]["kind"], "bare_argument_identifier_not_resolved_callback")

    def test_nonforeign_signature_not_classified_as_foreign(self):
        rows = self.rust(b'trait T { fn bridge(); } fn bridge() {}')
        self.assertEqual(rows["foreign_declarations"], [])
        self.assertEqual(rows["direct_calls"], [])

    def test_c_exports_exclude_static_and_objc_selector_methods(self):
        data = b'''static int helper(void) { return 1; }
int bridge(void *p) { return helper(); }
@interface C : NSObject
- (void)method;
@end
@implementation C
- (void)method {}
@end
'''
        try:
            rows = ffi.c_exports(data, "fixture.m")
        except ModuleNotFoundError:
            self.skipTest("pinned Objective-C grammar unavailable")
        self.assertEqual([r["name"] for r in rows], ["bridge"])
        self.assertEqual(rows[0]["signature_source"], "int bridge(void *p)")

    def test_pointer_return_export_declarator_and_invalid_c(self):
        try:
            rows = ffi.c_exports(b'char *copy_name(void) { return 0; }', "fixture.c")
        except ModuleNotFoundError:
            self.skipTest("pinned C grammar unavailable")
        self.assertEqual(rows[0]["name"], "copy_name")
        with self.assertRaises(ValueError):
            ffi.c_exports(b'int broken( {', "fixture.c")

    def test_invalid_rust_is_not_success(self):
        with self.assertRaises(ValueError):
            self.rust(b'extern "C" { fn bridge( ; }')

    def test_validator_acceptance_and_source_drift_fail_closed(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td);path = "fixture.c";raw = b'int bridge(void) { return 0; }';digest = hashlib.sha256(raw).hexdigest()
            (root/path).write_bytes(raw)
            snapshot = root/'docs/agent-map/reconciliation/snapshot.json';snapshot.parent.mkdir(parents=True)
            snapshot.write_text(json.dumps({"source_commit":"frozen","source_files":[{"path":path,"sha256":digest}]}))
            report = {"source_commit":"frozen","inputs":{path:{"sha256":digest,"bytes":len(raw)}},"symbols":[{"name":"bridge","abi_accepted":False,"semantic_acceptance":False,"native_export_ranges":[{"name":"bridge","path":path,"start_byte":0,"end_byte":len(raw),"start_line":1,"end_line":1}],"rust_foreign_declarations":[],"rust_direct_calls":[],"rust_argument_references":[]}],"semantic_acceptance":False,"complete_project_coverage":False}
            self.assertEqual(ffi.validate(root, report), [])
            report["symbols"][0]["abi_accepted"] = True
            self.assertIn("accepted_symbol:bridge", ffi.validate(root, report))
            (root/path).write_bytes(b'changed')
            self.assertIn("source_hash:fixture.c", ffi.validate(root, report))


if __name__ == '__main__':
    unittest.main()
