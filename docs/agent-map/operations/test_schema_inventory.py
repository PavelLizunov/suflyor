"""Schema/navigation fixtures, not production compiler/i18n/UI verification."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import schema_inventory as inventory


class SchemaFixtures(unittest.TestCase):
    def parser(self):
        try:
            return inventory.polyglot_syntax.parser_for("slint")
        except ModuleNotFoundError:
            self.skipTest("pinned Slint library unavailable")

    def test_ui_comments_escaped_utf8_types_and_resource_ranges(self):
        data = '''// @tr("fake")
export component C inherits Window {
 in-out property <string> value: "пример";
 callback done(string);
 done(value) => { root.value = value; }
 Text { text: @tr("Line\\n{}", value); }
 Image { source: @image-url("../assets/icons/ai.svg"); }
}
'''.encode()
        rows = inventory.ui_nodes(data, self.parser())
        tr = [r for r in rows if r["kind"] == "tr"]
        self.assertEqual(len(tr), 1)
        self.assertEqual(tr[0]["literal_value"], "Line\n{}")
        self.assertEqual(tr[0]["scope_names"], ["C"])
        self.assertEqual(data[tr[0]["start_byte"]:tr[0]["end_byte"]].decode(), '@tr("Line\\n{}", value)')
        prop = next(r for r in rows if r["kind"] == "property")
        self.assertEqual(prop["name"], "value")
        self.assertEqual(prop["type_source"], "string")
        self.assertEqual(prop["start_line"], 3)

    def test_context_plural_and_dynamic_interpolation_fail_closed(self):
        parser = self.parser()
        for expression in ('@tr("ctx" => "Hello")', '@tr("one" | "many" % root.count)'):
            with self.subTest(expression=expression):
                rows = inventory.ui_nodes(('export component C inherits Window { text: '+expression+'; }').encode(), parser)
                row = next(r for r in rows if r["kind"] == "tr")
                self.assertTrue(row["has_context_or_plural"])
        self.assertIsNone(inventory.literal('"value\\{root.count}"'))
        self.assertIsNone(inventory.literal('"escape\\q"'))
        self.assertEqual(inventory.literal('"Пример"'), "Пример")

    def test_catalog_match_does_not_report_unresolved_literals_as_missing(self):
        row = {"has_context_or_plural": False, "literal_value": None}
        self.assertEqual(inventory.catalog_match(row, {})["catalog_status"], "unresolved_literal")
        row["literal_value"] = "one"
        entry = {"msgid": ["one", "many"], "start_line": 1, "fuzzy": False, "empty_translation": False}
        self.assertEqual(inventory.catalog_match(row, {(None, "one"): [entry]})["catalog_status"], "unresolved_plural_catalog")
        self.assertEqual(inventory.catalog_match(row, {})["catalog_status"], "missing")
        row["has_context_or_plural"] = True
        self.assertEqual(inventory.catalog_match(row, {})["catalog_status"], "unresolved_context_or_plural")

    def test_invalid_ui_is_not_a_success_inventory(self):
        with self.assertRaises(ValueError):
            inventory.ui_nodes(b'export component C inherits Window {', self.parser())

    def test_po_duplicate_entries_not_hidden_by_catalog_merge(self):
        data = b'msgid "Hello"\nmsgstr "First"\n\nmsgid "Hello"\nmsgstr "Second"\n'
        rows = inventory.po_catalog(data)
        self.assertEqual([r["msgstr"] for r in rows], ["First", "Second"])
        self.assertEqual([r["start_line"] for r in rows], [1, 4])

    def test_po_multiline_context_plural_fuzzy_empty_obsolete(self):
        data = b'''#, fuzzy
msgctxt "dialog"
msgid "Hello "
"world"
msgstr ""

msgid "one"
msgid_plural "many"
msgstr[0] "one"
msgstr[1] "two"
msgstr[2] "three"

#~ msgid "old"
#~ msgstr "old"
'''
        rows = inventory.po_catalog(data)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["msgid"], "Hello world")
        self.assertEqual(rows[0]["context"], "dialog")
        self.assertTrue(rows[0]["fuzzy"])
        self.assertTrue(rows[0]["empty_translation"])
        self.assertEqual(rows[1]["msgid"], ["one", "many"])
        self.assertEqual(len(rows[1]["msgstr"]), 3)

    def test_invalid_po_and_version_mismatch_fail_closed(self):
        from babel.messages.pofile import PoFileError
        with self.assertRaises(PoFileError):
            inventory.po_catalog(b'msgid "A"\ninvalid "B"\n')
        import babel
        with mock.patch.object(babel, "__version__", "unknown"):
            with self.assertRaises(RuntimeError):
                inventory.po_catalog(b'msgid "A"\nmsgstr "B"\n')

    def test_local_resource_resolution_does_not_open_paths_outside_snapshot(self):
        files = {"slint-experiment/assets/icon.png": {}}
        path = "slint-experiment/ui/c.slint"
        self.assertEqual(inventory.resolve_local(path, "../assets/icon.png", files)["status"], "frozen_target")
        self.assertEqual(inventory.resolve_local(path, "missing.png", files)["status"], "missing_frozen_target")
        for value, expected in (("../../../secret", "outside_repository"),
                                ("/tmp/icon", "nonrelative_or_external"),
                                (None, "unresolved_literal"),
                                ("std-widgets.slint", "external_standard_library")):
            self.assertEqual(inventory.resolve_local(path, value, files, True)["status"], expected)

    def test_validator_rejects_acceptance_drift_ranges_and_false_defaults(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            path = inventory.CONFIG
            raw = b"pub struct Config { pub a: bool }"
            target = root / path
            target.parent.mkdir(parents=True)
            target.write_bytes(raw)
            snap = root / "docs/agent-map/reconciliation/snapshot.json"
            snap.parent.mkdir(parents=True)
            snap.write_text(json.dumps({"source_commit": "frozen", "source_files": [{"path": path, "sha256": hashlib.sha256(raw).hexdigest()}]}))
            report = {"source_commit": "frozen", "semantic_acceptance": False, "complete_project_coverage": False,
                      "inputs": {path: {"sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}},
                      "ui_nodes": [], "config_fields": [{"name": "a", "default_value": "not_evaluated", "derived_default_evaluated": False, "start_byte": 20, "end_byte": 31}], "assets": []}
            self.assertEqual(inventory.validate(root, report), [])
            report["semantic_acceptance"] = True
            report["config_fields"][0]["default_value"] = True
            report["config_fields"][0]["end_byte"] = 1000
            errors = inventory.validate(root, report)
            self.assertIn("acceptance_boundary", errors)
            self.assertIn("evaluated_default:a", errors)
            self.assertIn("config_range:a", errors)
            target.write_bytes(b"changed")
            self.assertIn("source_drift:" + path, inventory.validate(root, report))

    def test_config_struct_and_function_defaults_are_distinct_unevaluated_sources(self):
        data = b'''#[derive(Default)] #[serde(default)]
pub struct Config { #[serde(default = "helper")] pub value: String, pub enabled: bool }
impl Config { pub fn defaults() -> Self { Self { value: "PRIVATE-ENDPOINT".into(), enabled: true } } }
fn helper() -> String { String::new() }
'''
        try:
            rows = inventory.config_fields(data)
        except ModuleNotFoundError:
            self.skipTest("pinned Rust parser unavailable")
        self.assertEqual([r["name"] for r in rows], ["value", "enabled"])
        self.assertEqual(rows[0]["serde_default_helpers"][0]["name"], "helper")
        self.assertIsNotNone(rows[0]["serde_default_helpers"][0]["source"])
        self.assertTrue(all(r["defaults_initializer_source_range"] for r in rows))
        self.assertNotIn("PRIVATE-ENDPOINT", json.dumps(rows))
        self.assertTrue(all(r["default_value"] == "not_evaluated" for r in rows))


if __name__ == "__main__":
    unittest.main()
