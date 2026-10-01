"""Pinned Rust CST/Python AST declaration fixtures, not compiler acceptance."""
import hashlib
import json
import tempfile
import unittest
from unittest import mock
from pathlib import Path

import syntax_index

try:
    PARSER = syntax_index.rust_parser()
    RUST_AVAILABLE = True
except ModuleNotFoundError:
    PARSER = None
    RUST_AVAILABLE = False


@unittest.skipUnless(RUST_AVAILABLE, "pinned Rust parser wheels unavailable; this is not a parser PASS")
class RustSyntaxFixtures(unittest.TestCase):
    def parse(self, text):
        return syntax_index.rust_index(text.encode("utf-8"), "fixture.rs", PARSER)

    def test_multiline_signature_with_nested_tuple_and_generics(self):
        text = "pub async fn sample<T: Clone>(\n tuple: (usize, Option<T>),\n callback: impl Fn(T) -> (T, usize),\n) -> Result<T, Error> where T: Send { callback(todo!()).0 }"
        r = self.parse(text)
        self.assertFalse(r["has_parse_error"])
        fn = next(row for row in r["declarations"] if row["name"] == "sample")
        self.assertIn("where T: Send", fn["signature_source"])
        self.assertIn("callback: impl Fn(T) -> (T, usize)", fn["parameters_source"])
        self.assertEqual(fn["return_type_source"], "Result<T, Error>")

    def test_nested_enum_payload_keeps_variants_and_fields_separate(self):
        r = self.parse("pub enum E { A, B { x: usize, y: usize }, C(String) }")
        variants = [x for x in r["declarations"] if x["kind"] == "enum_variant"]
        self.assertEqual([x["name"] for x in variants], ["A", "B", "C"])
        self.assertEqual([x["name"] for x in r["declarations"] if x["kind"] == "field_declaration"], ["x", "y"])
        self.assertIn("E", variants[1]["scope_names"])

    def test_impl_scope_and_trait_signature_do_not_leak_into_free_fn(self):
        r = self.parse("trait T { fn read(&self); } impl T for One { fn read(&self) {} } fn free() {}")
        funcs = [x for x in r["declarations"] if x["kind"].startswith("function_")]
        self.assertEqual(funcs[-1]["name"], "free")
        self.assertEqual(funcs[-1]["scope_names"], [])
        self.assertEqual(funcs[1]["scope_names"], ["T for One"])
        self.assertEqual(funcs[0]["kind"], "function_signature_item")

    def test_comments_strings_and_macro_tokens_not_invented_functions(self):
        r = self.parse('// fn fake() {}\nconst S: &str = "fn imaginary() {}"; macro_rules! make { () => {fn generated() {}} } make!(); fn real() {}')
        self.assertEqual([x["name"] for x in r["declarations"] if x["kind"] == "function_item"], ["real"])
        self.assertEqual(len([x for x in r["declarations"] if x["kind"] == "macro_invocation"]), 1)
        self.assertTrue(all(not x["macro_expanded"] for x in r["declarations"]))

    def test_cfg_attributes_and_test_inheritance(self):
        r = self.parse('#[cfg(windows)]\npub fn prod() {}\n#[cfg(test)] mod tests { #[test] fn sample() {} }')
        prod = next(x for x in r["declarations"] if x["name"] == "prod")
        test = next(x for x in r["declarations"] if x["name"] == "sample")
        self.assertFalse(prod["test_conditional"])
        self.assertEqual(prod["cfg_attributes"], ["#[cfg(windows)]"])
        self.assertTrue(test["test_conditional"])
        self.assertIn("#[cfg(test)]", test["inherited_attributes"])

    def test_invalid_rust_explicit_parse_error_not_complete(self):
        r = self.parse("fn broken( { let x = ;")
        self.assertTrue(r["has_parse_error"])
        self.assertTrue(r["parse_errors"])

    def test_missing_anonymous_delimiter_is_reported(self):
        result = self.parse("struct X { a: i32")
        self.assertTrue(result["has_parse_error"])
        self.assertTrue(any(e["kind"] == "MISSING" for e in result["parse_errors"]))

    def test_utf8_bytes_and_exact_source_ranges(self):
        text = '// Пример\npub fn correct() -> u32 { 1 }'
        data = text.encode()
        r = syntax_index.rust_index(data, "fixture.rs", PARSER)
        fn = next(x for x in r["declarations"] if x["name"] == "correct")
        self.assertEqual(data[fn["start_byte"]:fn["end_byte"]].decode(), "pub fn correct() -> u32 { 1 }")
        self.assertEqual(fn["start_line"], 2)


class PythonSyntaxFixtures(unittest.TestCase):
    def test_async_nested_class_decorators_and_comments(self):
        text = '# def imaginary(): pass\nclass One:\n    @staticmethod\n    async def real(x: tuple[int, str]) -> str:\n        def nested():\n            return "def not_a_fn():"\n        return x[1]\n'
        r = syntax_index.python_index(text.encode(), "fixture.py")
        self.assertFalse(r["has_parse_error"])
        self.assertEqual([x["name"] for x in r["declarations"]], ["One", "real", "nested"])
        self.assertEqual(r["declarations"][1]["scope_names"], ["One"])
        self.assertEqual(r["declarations"][1]["decorators_source"], ["staticmethod"])
        self.assertEqual(r["declarations"][2]["scope_names"], ["One", "real"])

    def test_invalid_python_is_explicit_error(self):
        r = syntax_index.python_index(b"def broken(:", "fixture.py")
        self.assertTrue(r["has_parse_error"])
        self.assertEqual(r["declarations"], [])

    def test_utf8_offsets_are_bytes(self):
        text = '# текст\ndef real():\n    return "строка"\n'
        data = text.encode()
        row = syntax_index.python_index(data, "fixture.py")["declarations"][0]
        self.assertEqual(data[row["start_byte"]:row["end_byte"]].decode(), 'def real():\n    return "строка"')

    def test_frozen_drift_rejected_before_python_parse(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "one.py").write_text("def one(): pass\n")
            snapshot = root / "snapshot.json"
            snapshot.write_text(json.dumps({"source_commit": syntax_index.BASELINE, "source_files": [{"path": "one.py", "sha256": "wrong"}]}))
            with self.assertRaisesRegex(ValueError, "frozen source drift"):
                syntax_index.build(root, snapshot)

    def test_protected_vendor_and_unsupported_language_are_not_zero_symbol_proof(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            (root / "one.cmd").write_text("func one() {}\n")
            snapshot = root / "snapshot.json"
            snapshot.write_text(json.dumps({"source_commit": syntax_index.BASELINE, "source_files": [{"path": "one.cmd", "sha256": hashlib.sha256((root / "one.cmd").read_bytes()).hexdigest()}, {"path": ".claude/private.py", "sha256": "not_read"}]}))
            report, symbols = syntax_index.build(root, snapshot)
            self.assertEqual([x["status"] for x in report["files"]], ["unsupported_language", "excluded_protected_or_vendor"])
            self.assertFalse(report["complete_project_coverage"])
            self.assertEqual(symbols, [])

class NativeIsolationFixtures(unittest.TestCase):
    def test_native_crash_is_receipt_not_accepted_empty_index(self):
        completed = mock.Mock(returncode=-11, stdout="", stderr="native details not published")
        with mock.patch.object(syntax_index.subprocess, "run", return_value=completed):
            r = syntax_index.isolated_rust_index(Path("fixture.rs"), "fixture.rs")
        self.assertTrue(r["native_parser_failure"])
        self.assertTrue(r["has_parse_error"])
        self.assertEqual(r["parse_errors"][0]["exit_code"], -11)
        self.assertEqual(r["declarations"], [])

    def test_timeout_and_invalid_receipt_are_explicit(self):
        with mock.patch.object(syntax_index.subprocess, "run", side_effect=syntax_index.subprocess.TimeoutExpired("parser", 30)):
            r = syntax_index.isolated_rust_index(Path("fixture.rs"), "fixture.rs")
        self.assertEqual(r["parse_errors"][0]["kind"], "parser_timeout")
        with mock.patch.object(syntax_index.subprocess, "run", return_value=mock.Mock(returncode=0, stdout="not-json")):
            r = syntax_index.isolated_rust_index(Path("fixture.rs"), "fixture.rs")
        self.assertEqual(r["parse_errors"][0]["kind"], "invalid_parser_receipt")


if __name__ == "__main__":
    unittest.main()
