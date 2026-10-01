#!/usr/bin/env python3
"""Frozen configuration/UI/resource navigation; never semantic/native acceptance."""
import argparse
import hashlib
import io
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath
from xml.etree import ElementTree

import syntax_index
import polyglot_syntax

CONFIG = "overlay-backend/src/config.rs"
PO = "slint-experiment/translations/ru/LC_MESSAGES/slint-replay.po"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def source_text(data, node):
    return data[node.start_byte:node.end_byte].decode("utf-8") if node else None


def position(data, node):
    return {"start_byte": node.start_byte, "end_byte": node.end_byte,
            "start_line": data.count(b"\n", 0, node.start_byte) + 1,
            "end_line": data.count(b"\n", 0, max(node.start_byte, node.end_byte - 1)) + 1}


def literal(raw):
    """JSON-compatible Slint subset only; unknown escape/interpolation stays unresolved."""
    try:
        return json.loads(raw)
    except (ValueError, TypeError):
        return None


def config_fields(data):
    root = syntax_index.rust_parser().parse(data).root_node
    if root.has_error:
        raise ValueError("config parse error")
    declarations = syntax_index.rust_index(data, CONFIG)["declarations"]
    fields = [r for r in declarations if r["kind"] == "field_declaration" and r["scope_names"] == ["Config"]]
    default_sites = {}
    functions = {r["name"]: {"path": CONFIG, "start_line": r["start_line"], "end_line": r["end_line"]}
                 for r in declarations if r["kind"] == "function_item"}

    def walk(n, default_fn=False):
        if n.type == "function_item":
            default_fn = source_text(data, n.child_by_field_name("name")) == "defaults"
        if default_fn and n.type == "field_initializer":
            name = source_text(data, n.child_by_field_name("field"))
            default_sites[name] = position(data, n)
        for child in n.named_children:
            walk(child, default_fn)
    walk(root)
    rows = []
    for r in fields:
        attrs = r["attributes"]
        # Identify helper tokens using the Rust attribute CST text; no default evaluation.
        helpers = []
        for attr in attrs:
            if 'default = "' in attr:
                name = attr.split('default = "', 1)[1].split('"', 1)[0]
                helpers.append({"name": name, "source": functions.get(name)})
        rows.append({"name": r["name"], "type_source": r["type_source"], "serde_attributes": attrs,
                     "serde_default_helpers": helpers,
                     "defaults_initializer_source_range": default_sites.get(r["name"]),
                     "derived_default_evaluated": False, "default_value": "not_evaluated",
                     **{k: r[k] for k in ("start_byte", "end_byte", "start_line", "end_line")}})
    return rows


def ui_nodes(data, parser):
    root = parser.parse(data).root_node
    if root.has_error:
        raise ValueError("Slint parse error")
    rows = []
    kinds = {"property", "callback", "callback_event", "function_definition",
             "import_statement", "tr", "image_call", "component_definition", "global_definition"}

    def walk(n, scopes):
        name_node = n.child_by_field_name("name")
        name = source_text(data, name_node)
        if n.type in kinds:
            row = {"kind": n.type, "name": name, "scope_names": scopes, **position(data, n)}
            if n.type in {"tr", "image_call", "import_statement"}:
                strings = [c for c in n.named_children if c.type == "string_value"]
                selected = n.child_by_field_name("message") if n.type == "tr" else strings[0] if strings else None
                raw = source_text(data, selected)
                if n.type == "tr":
                    row["has_context_or_plural"] = any(n.child_by_field_name(field) is not None for field in ("context", "pipe", "percent"))
                row.update(literal_source=raw, literal_value=literal(raw),
                           literal_status="decoded_json_subset" if literal(raw) is not None else "unresolved")
            else:
                row["type_source"] = source_text(data, n.child_by_field_name("type"))
            if n.type == "callback_event":
                row["name"] = next((source_text(data, c) for c in n.named_children if c.type == "simple_identifier"), None)
            rows.append(row)
        child_scopes = scopes + [name] if n.type in {"component_definition", "global_definition"} else scopes
        if n.type not in {"comment", "string_value"}:
            for child in n.named_children:
                walk(child, child_scopes)
    walk(root, [])
    return rows


def po_catalog(data):
    import babel
    from babel.messages.catalog import Catalog
    from babel.messages.pofile import PoFileParser
    from babel.messages import pofile
    if babel.__version__ != "2.10.3":
        raise RuntimeError("Babel must match researched 2.10.3")
    if sha(Path(pofile.__file__).read_bytes()) != "372c53da4adfb537594db564a8eefea0612d40ad2c392c99dd83c4ced33c8bc0":
        raise RuntimeError("PO reader source provenance mismatch")

    class CaptureCatalog(Catalog):
        def __init__(self):
            self.messages_seen = []
            super().__init__(locale="ru", charset="utf-8")

        def __setitem__(self, key, message):
            if message.id:
                self.messages_seen.append(message)
            super().__setitem__(key, message)

    catalog = CaptureCatalog()
    PoFileParser(catalog, ignore_obsolete=True, abort_invalid=True).parse(io.BytesIO(data))
    rows = []
    for m in catalog.messages_seen:
        strings = list(m.string) if isinstance(m.string, tuple) else [m.string]
        rows.append({"msgid": list(m.id) if isinstance(m.id, tuple) else m.id,
                     "msgstr": strings if isinstance(m.string, tuple) else m.string,
                     "context": m.context, "flags": sorted(m.flags), "start_line": m.lineno,
                     "fuzzy": "fuzzy" in m.flags, "empty_translation": any(not s for s in strings)})
    return rows


def catalog_match(row, keys):
    if row["has_context_or_plural"]:
        return {"catalog_status": "unresolved_context_or_plural"}
    if row["literal_value"] is None:
        return {"catalog_status": "unresolved_literal"}
    matched = keys.get((None, row["literal_value"]), [])
    status = "missing" if not matched else "duplicate" if len(matched) > 1 else "unresolved_plural_catalog" if isinstance(matched[0]["msgid"], list) else "fuzzy_or_empty" if matched[0]["fuzzy"] or matched[0]["empty_translation"] else "exact_member"
    return {"catalog_status": status, "catalog_lines": [r["start_line"] for r in matched]}


def resolve_local(path, value, files, standard=False):
    if value is None:
        return {"status": "unresolved_literal"}
    if standard and value == "std-widgets.slint":
        return {"status": "external_standard_library"}
    if "://" in value or value.startswith("/") or "\\" in value:
        return {"status": "nonrelative_or_external"}
    parts = list(PurePosixPath(path).parent.parts)
    for p in PurePosixPath(value).parts:
        if p == "..":
            if not parts:
                return {"status": "outside_repository"}
            parts.pop()
        elif p != ".":
            parts.append(p)
    target = "/".join(parts)
    return {"target": target, "status": "frozen_target" if target in files else "missing_frozen_target"}


def build(root):
    snapshot = json.loads((root / "docs/agent-map/reconciliation/snapshot.json").read_text())
    files = {r["path"]: r for r in snapshot["source_files"]}
    inputs = {}

    def read(path):
        data = syntax_index.safe_source(root, path).read_bytes()
        if sha(data) != files[path]["sha256"]:
            raise ValueError("frozen source drift: " + path)
        inputs[path] = {"sha256": sha(data), "bytes": len(data)}
        return data

    config = config_fields(read(CONFIG))
    catalog = po_catalog(read(PO))
    keys = {}
    for row in catalog:
        key = (row["context"], row["msgid"][0] if isinstance(row["msgid"], list) else row["msgid"])
        keys.setdefault(key, []).append(row)
    parser = polyglot_syntax.parser_for("slint")
    ui = []
    assets = []
    for path in sorted(files):
        if path.startswith("slint-experiment/ui/") and path.endswith(".slint"):
            for row in ui_nodes(read(path), parser):
                row["path"] = path
                row["source_sha256"] = inputs[path]["sha256"]
                if row["kind"] == "tr":
                    row.update(catalog_match(row, keys))
                if row["kind"] in {"image_call", "import_statement"}:
                    row["resource"] = resolve_local(path, row["literal_value"], files, row["kind"] == "import_statement")
                ui.append(row)
        if path.startswith("slint-experiment/assets/") and not path.endswith(".md"):
            data = read(path)
            row = {"path": path, **inputs[path], "kind": Path(path).suffix}
            if path.endswith(".svg"):
                node = ElementTree.fromstring(data)
                row.update(svg_root=node.tag, svg_attributes=dict(sorted(node.attrib.items())),
                           svg_tags=sorted({n.tag.split("}")[-1] for n in node.iter()}))
            assets.append(row)
    import_edges = {}
    for row in ui:
        if row["kind"] == "import_statement" and row["resource"]["status"] == "frozen_target":
            import_edges.setdefault(row["path"], []).append(row["resource"]["target"])
    reachable = set()
    pending = ["slint-experiment/ui/index.slint"]
    while pending:
        current = pending.pop()
        if current in reachable:
            continue
        reachable.add(current)
        pending.extend(import_edges.get(current, []))
    used = {r["literal_value"] for r in ui if r["kind"] == "tr" and r["literal_value"] is not None}
    return {"schema_version": 1, "source_commit": snapshot["source_commit"],
            "tools": {"tree-sitter": "0.25.2", "tree-sitter-rust": "0.24.2", "slint_revision": polyglot_syntax.SLINT_REVISION, "Babel": "2.10.3", "XML": "stdlib ElementTree"},
            "inputs": inputs, "config_fields": config, "ui_nodes": ui, "catalog": catalog,
            "ui_import_reachable_from_index": sorted(reachable),
            "ui_not_reachable_from_index": sorted(p for p in inputs if p.startswith("slint-experiment/ui/") and p not in reachable),
            "catalog_unreferenced_msgids": [r["msgid"] for r in catalog if isinstance(r["msgid"], str) and r["msgid"] not in used],
            "assets": assets, "semantic_acceptance": False, "complete_project_coverage": False,
            "limits": ["not evaluated defaults/Serde or all Config consumers", "not resolved UI Rust callbacks/setters", "JSON-compatible literal subset only", "catalog membership is not translation quality or compiler bundling", "asset existence/XML attributes are not native embedding or rendering"]}


def validate(root, report):
    errors = []
    if report.get("semantic_acceptance") is not False or report.get("complete_project_coverage") is not False:
        errors.append("acceptance_boundary")
    data = {}
    for path, item in report["inputs"].items():
        raw = syntax_index.safe_source(root, path).read_bytes()
        data[path] = raw
        if sha(raw) != item["sha256"] or len(raw) != item["bytes"]:
            errors.append("source_drift:" + path)
    seen = set()
    for row in report["ui_nodes"]:
        path = row["path"]
        raw = data[path]
        begin, finish = row["start_byte"], row["end_byte"]
        ident = (path, row["kind"], begin, finish)
        if ident in seen:
            errors.append("duplicate_node:" + path)
        seen.add(ident)
        if not 0 <= begin < finish <= len(raw):
            errors.append("byte_range:" + path)
        if row["source_sha256"] != sha(raw):
            errors.append("node_hash:" + path)
        if not 1 <= row["start_line"] <= row["end_line"] <= len(raw.splitlines()) + 1:
            errors.append("line_range:" + path)
        if row.get("literal_source") and row["literal_source"] not in raw[begin:finish].decode("utf-8"):
            errors.append("literal_range:" + path)
    names = [r["name"] for r in report["config_fields"]]
    if len(set(names)) != len(names):
        errors.append("config_field_duplicates")
    snapshot = json.loads((root / "docs/agent-map/reconciliation/snapshot.json").read_text())
    frozen = {r["path"]: r["sha256"] for r in snapshot["source_files"]}
    if report["source_commit"] != snapshot["source_commit"]:
        errors.append("baseline_mismatch")
    for path, item in report["inputs"].items():
        if item["sha256"] != frozen.get(path):
            errors.append("snapshot_hash:" + path)
    for row in report["config_fields"]:
        if row["default_value"] != "not_evaluated" or row["derived_default_evaluated"] is not False:
            errors.append("evaluated_default:" + row["name"])
        if not 0 <= row["start_byte"] < row["end_byte"] <= len(data[CONFIG]):
            errors.append("config_range:" + row["name"])
    for row in report["assets"]:
        if sha(data[row["path"]]) != row["sha256"]:
            errors.append("asset_hash:" + row["path"])
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--validate", action="store_true")
    args = parser.parse_args()
    if args.validate:
        errors = validate(args.root.resolve(), json.loads(args.output.read_text(encoding="utf-8")))
        print(json.dumps({"errors": errors, "semantic_acceptance": False}))
        raise SystemExit(bool(errors))
    if not args.worker:
        result = subprocess.run([sys.executable, "-B", str(Path(__file__).resolve()), "--worker", "--root", str(args.root.resolve()), "--output", str(args.output.resolve())], capture_output=True, text=True, timeout=60)
        if result.returncode:
            raise RuntimeError("isolated schema parser failed: " + str(result.returncode) + " " + result.stderr[-2000:])
        print(result.stdout, end="")
        return
    report = build(args.root.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"config_fields": len(report["config_fields"]), "ui_nodes": len(report["ui_nodes"]), "catalog_entries": len(report["catalog"]), "assets": len(report["assets"]), "semantic_acceptance": False}))


if __name__ == "__main__":
    main()
