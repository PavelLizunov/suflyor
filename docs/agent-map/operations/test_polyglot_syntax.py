"""Native/script CST fixtures. No source execution, SDK or compiler acceptance."""
import hashlib
import unittest
from pathlib import Path

import polyglot_syntax as poly
import syntax_index


class PolyglotFixtures(unittest.TestCase):
    def parse(self, language, source):
        data = source.encode("utf-8")
        suffix = next(s for s, (lang, _) in poly.GRAMMARS.items() if lang == language)
        path = "fixture" + suffix
        try:
            rows, errors = poly.parse(data, path, hashlib.sha256(data).hexdigest(), language)
        except ModuleNotFoundError:
            self.skipTest("pinned grammar unavailable: " + language)
        self.assertFalse(errors)
        ids = {r["id"] for r in rows}
        self.assertEqual(len(ids), len(rows))
        for r in rows:
            self.assertTrue(data[r["start_byte"]:r["end_byte"]].decode().startswith(r["signature_source"]))
            self.assertTrue(all(p in ids for p in r["parent_ids"]))
            self.assertFalse(r["semantic_acceptance"])
        return rows

    def test_bash_nested_multiline_comment_heredoc(self):
        rows = self.parse("bash", '''# fake() { :; }
outer()
{
  text="function fake2 { :; }"
  cat <<'END'
function fake3 { :; }
END
  function inner { :; }
}
''')
        self.assertEqual([r["name"] for r in rows], ["outer", "inner"])
        self.assertEqual(rows[1]["parent_ids"], [rows[0]["id"]])

    def test_swift_types_methods_return_type_and_case_list(self):
        rows = self.parse("swift", '''// func fake() {}
public struct Box<T> {
 let value: T
 init(value: T) { self.value = value }
 func get() -> T { value }
}
enum E { case one, two(Int) }
''')
        names = [r["name"] for r in rows]
        for name in ("Box", "value", "init", "get", "E", "one", "two"):
            self.assertIn(name, names)
        self.assertNotIn("T", names)
        get = next(r for r in rows if r["name"] == "get")
        self.assertEqual(get["scope_names"], ["Box"])
        self.assertIn("-> T", get["signature_source"])

    def test_objc_selector_ffi_and_pointer_typedef(self):
        rows = self.parse("objc", '''@interface C : NSObject
- (void)send:(id)x other:(int)y;
@end
@implementation C
- (void)send:(id)x other:(int)y {}
@end
typedef void (*Callback)(void);
int bridge(void *p) { return 0; }
''')
        selectors = [r for r in rows if r["kind"].startswith("method_")]
        self.assertEqual([r["name"] for r in selectors], ["send:other:", "send:other:"])
        self.assertTrue(all(r["scope_names"] == ["C"] for r in selectors))
        self.assertIn("Callback", [r["name"] for r in rows])
        self.assertIn("bridge", [r["name"] for r in rows])
        self.assertNotIn("p", [r["name"] for r in rows])

    def test_powershell_functions_nested_and_class_members(self):
        rows = self.parse("powershell", '''# function Fake {}
function Test-One([string]$Value) {
 function Nested { param($x); $x }
 $s="function Fake2 {}"
}
class C { [int] Get() { return 1 } }
''')
        names = [r["name"] for r in rows]
        for name in ("Test-One", "Nested", "C", "Get"):
            self.assertIn(name, names)
        self.assertNotIn("Fake", names)
        self.assertNotIn("Fake2", names)
        nested = next(r for r in rows if r["name"] == "Nested")
        self.assertEqual(nested["scope_names"], ["Test-One"])

    def test_c_conditional_pointer_multiple_declarations_utf8(self):
        rows = self.parse("c", '''// UTF8: пример
#ifdef FLAG
typedef void (*Callback)(void);
int a, b;
int bridge(void *p) { return 0; }
#endif
''')
        names = [r["name"] for r in rows]
        for name in ("Callback", "a", "b", "bridge"):
            self.assertIn(name, names)
        self.assertNotIn("p", names)
        self.assertTrue(all(r["cfg_attributes"] == ["#ifdef FLAG"] for r in rows))
        self.assertEqual(next(r for r in rows if r["name"] == "bridge")["start_line"], 5)

    def test_slint_multiline_callbacks_properties_scope_and_strings(self):
        rows = self.parse("slint", '''import { Window } from "std-widgets.slint";
// callback fake();
export component C inherits Window {
 in-out property <string> title: "callback fake2();";
 callback done(string);
 done(value) => { root.title = value; }
 function test(x: int) -> int { return x; }
}
export global G { in-out property <int> v; }
''')
        names = [r["name"] for r in rows]
        for name in ("C", "title", "done", "test", "G", "v"):
            self.assertIn(name, names)
        self.assertEqual(names.count("done"), 2)
        self.assertIn('"std-widgets.slint"', names)
        self.assertNotIn("fake", names)
        self.assertNotIn("fake2", names)
        self.assertEqual(next(r for r in rows if r["name"] == "v")["scope_names"], ["G"])

    def test_errors_are_not_success_receipts(self):
        for language, source in (("c", "int f( {"), ("bash", "f() {"),
                                 ("swift", "struct C {"), ("objc", "@interface C"),
                                 ("powershell", "function F {")):
            with self.subTest(language=language):
                try:
                    _, errors = poly.parse(source.encode(), "fixture", "fixture", language)
                except ModuleNotFoundError:
                    self.skipTest("pinned grammar unavailable: " + language)
                self.assertTrue(errors)

    def test_version_mismatch_is_fail_closed(self):
        from unittest.mock import patch
        with patch.object(poly, "version", return_value="0.26.0"):
            with self.assertRaisesRegex(RuntimeError, "0.25.2"):
                poly.parser_for("bash")


if __name__ == "__main__":
    unittest.main()
