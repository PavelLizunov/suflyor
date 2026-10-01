#!/usr/bin/env python3
"""Frozen native bridge declaration/direct-call census, not linker/ABI acceptance."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import polyglot_syntax
import syntax_index

BRIDGES = {
    "slint-experiment/src/native/macos/clipboard.m",
    "slint-experiment/src/native/macos/screen.m",
    "slint-experiment/src/native/macos/status.m",
    "slint-experiment/src/native/macos/window.m",
    "overlay-backend/native/macos/mic_capture.m",
    "overlay-backend/native/macos/system_capture.m",
    "overlay-backend/native/macos/process_memory.c",
}


def text(data, node):
    return data[node.start_byte:node.end_byte].decode() if node else None


def span(node):
    return {"start_byte": node.start_byte, "end_byte": node.end_byte,
            "start_line": node.start_point.row + 1,
            "end_line": node.end_point.row + 1}


def c_exports(data, path):
    language = "c" if path.endswith(".c") else "objc"
    root = polyglot_syntax.parser_for(language).parse(data).root_node
    if root.has_error:
        raise ValueError("native parse error: " + path)
    rows = []
    # Only top-level nonstatic C ABI definitions, never Objective-C selectors/helpers.
    for node in root.named_children:
        if node.type != "function_definition":
            continue
        storage = [text(data, c) for c in node.children if c.type == "storage_class_specifier"]
        if "static" in storage:
            continue
        name_node = polyglot_syntax.declarator_name(node.child_by_field_name("declarator"))
        if name_node is None:
            raise ValueError("unresolved export declarator")
        body = node.child_by_field_name("body")
        rows.append({"path": path, "name": text(data, name_node), "signature_source": data[node.start_byte:body.start_byte].decode().rstrip(), **span(node)})
    return rows


def rust_edges(data, path, symbols, parser=None):
    root = (parser or syntax_index.rust_parser()).parse(data).root_node
    if root.has_error:
        raise ValueError("Rust parse error: " + path)
    foreign, calls, callback_refs = [], [], []

    def walk(node, foreign_abi=None, container=None):
        if node.type in {"macro_definition", "macro_invocation", "line_comment", "block_comment", "string_literal", "raw_string_literal"}:
            return
        if node.type == "foreign_mod_item":
            modifier = next((c for c in node.named_children if c.type == "extern_modifier"), None)
            foreign_abi = text(data, modifier)
        if node.type == "function_item":
            container = {"name": text(data, node.child_by_field_name("name")), **span(node)}
        if foreign_abi is not None and node.type == "function_signature_item":
            name = text(data, node.child_by_field_name("name"))
            if name in symbols:
                foreign.append({"path": path, "name": name, "foreign_abi_source": foreign_abi,
                                "signature_source": text(data, node), **span(node)})
        if node.type == "call_expression":
            function = node.child_by_field_name("function")
            name_node = function.child_by_field_name("name") if function and function.type == "scoped_identifier" else function
            name = text(data, name_node)
            if name in symbols and function.type in {"identifier", "scoped_identifier"}:
                calls.append({"path": path, "name": name, "callee_source": text(data, function),
                              "containing_function": container,
                              "inside_closure": any(p.type == "closure_expression" for p in ancestors(node)), **span(node)})
        if node.type == "identifier" and text(data, node) in symbols:
            parent = node.parent
            if parent and parent.type == "arguments":
                callback_refs.append({"path": path, "name": text(data, node), "kind": "bare_argument_identifier_not_resolved_callback", "containing_function": container, **span(node)})
        for child in node.named_children:
            walk(child, foreign_abi, container)

    walk(root)
    return {"foreign_declarations": foreign, "direct_calls": calls, "argument_references": callback_refs}


def ancestors(node):
    current = node.parent
    while current:
        yield current
        current = current.parent


def build(root):
    snapshot = json.loads((root / "docs/agent-map/reconciliation/snapshot.json").read_text())
    frozen = {r["path"]: r for r in snapshot["source_files"]}
    inputs = {}

    def read(path):
        data = syntax_index.safe_source(root, path).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != frozen[path]["sha256"]:
            raise ValueError("source drift: " + path)
        inputs[path] = {"sha256": digest, "bytes": len(data)}
        return data

    exports = [row for path in sorted(BRIDGES) for row in c_exports(read(path), path)]
    symbols = {r["name"] for r in exports}
    foreign, calls, refs = [], [], []
    parser = syntax_index.rust_parser()
    for path in sorted(frozen):
        if path.endswith(".rs") and path.startswith(("overlay-backend/src/", "slint-experiment/src/", "suflyor-tts/src/", "suflyor-teratts/src/", "suflyor-wsola/src/")):
            rows = rust_edges(read(path), path, symbols, parser)
            foreign.extend(rows["foreign_declarations"])
            calls.extend(rows["direct_calls"])
            refs.extend(rows["argument_references"])
    pairs = []
    for name in sorted(symbols):
        pairs.append({"name": name, "native_export_ranges": [r for r in exports if r["name"] == name],
                      "rust_foreign_declarations": [r for r in foreign if r["name"] == name],
                      "rust_direct_calls": [r for r in calls if r["name"] == name],
                      "rust_argument_references": [r for r in refs if r["name"] == name],
                      "abi_accepted": False, "semantic_acceptance": False})
    return {"schema_version": 1, "source_commit": snapshot["source_commit"], "inputs": inputs,
            "symbols": pairs, "semantic_acceptance": False, "complete_project_coverage": False,
            "limits": ["symbol-name matching only; no linker cfg/type/ABI/struct layout proof", "production macOS C/Objective-C bridges only; experiments and Windows SDK graph excluded", "macro token trees not expanded; aliases/dynamic dispatch/function pointers unresolved", "calls inside cfg(test) syntax included, not evaluated or executed"]}


def validate(root, report):
    errors = []
    if report.get("semantic_acceptance") is not False or report.get("complete_project_coverage") is not False:
        errors.append("acceptance_boundary")
    snapshot = json.loads((root / "docs/agent-map/reconciliation/snapshot.json").read_text())
    hashes = {r["path"]: r["sha256"] for r in snapshot["source_files"]}
    if report["source_commit"] != snapshot["source_commit"]:
        errors.append("baseline")
    data = {}
    for path, row in report["inputs"].items():
        raw = syntax_index.safe_source(root, path).read_bytes();data[path] = raw
        if hashlib.sha256(raw).hexdigest() != row["sha256"] or row["sha256"] != hashes.get(path):
            errors.append("source_hash:" + path)
    seen = set()
    for row in report["symbols"]:
        if row["abi_accepted"] is not False or row["semantic_acceptance"] is not False:
            errors.append("accepted_symbol:" + row["name"])
        for key in ("native_export_ranges", "rust_foreign_declarations", "rust_direct_calls", "rust_argument_references"):
            for ref in row[key]:
                raw = data[ref["path"]];a,b=ref["start_byte"],ref["end_byte"]
                ident = (key, ref["path"], a, b)
                if ident in seen:errors.append("duplicate_range:" + ref["path"])
                seen.add(ident)
                if not 0 <= a < b <= len(raw):errors.append("byte_range:" + ref["path"])
                if ref["name"] != row["name"] or row["name"] not in raw[a:b].decode():errors.append("name_range:" + row["name"])
                if not 1 <= ref["start_line"] <= ref["end_line"] <= len(raw.splitlines())+1:errors.append("line_range:" + ref["path"])
    return errors


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root",type=Path,default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--worker",action="store_true",help=argparse.SUPPRESS)
    parser.add_argument("--validate",action="store_true")
    args=parser.parse_args()
    if args.validate:
        errors=validate(args.root.resolve(),json.loads(args.output.read_text()));print(json.dumps({"errors":errors,"abi_accepted":False}));raise SystemExit(bool(errors))
    if not args.worker:
        result=subprocess.run([sys.executable,"-B",str(Path(__file__).resolve()),"--worker","--root",str(args.root.resolve()),"--output",str(args.output.resolve())],text=True,capture_output=True,timeout=60)
        if result.returncode:raise RuntimeError("native census process failure: "+str(result.returncode)+" "+result.stderr[-2000:])
        print(result.stdout,end="");return
    report=build(args.root.resolve());args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
    print(json.dumps({"symbols":len(report["symbols"]),"inputs":len(report["inputs"]),"foreign_declarations":sum(len(r['rust_foreign_declarations']) for r in report['symbols']),"direct_calls":sum(len(r['rust_direct_calls']) for r in report['symbols']),"abi_accepted":False}))


if __name__=="__main__":
    main()
