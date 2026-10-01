#!/usr/bin/env python3
"""Frozen-source declaration index. Syntax evidence only, never semantic review."""
import argparse
import ast
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

BASELINE = "a10c356af05a5832a14ea06a5d0cb6c49694e3f1"
RUST_VERSIONS = {"tree-sitter": "0.25.2", "tree-sitter-rust": "0.24.2"}
RUST_KINDS = {
    "function_item", "function_signature_item", "struct_item", "enum_item",
    "enum_variant", "trait_item", "impl_item", "type_item", "const_item",
    "static_item", "mod_item", "macro_definition", "macro_invocation",
    "field_declaration", "associated_type", "foreign_mod_item",
}
NO_DESCEND = {"macro_definition", "macro_invocation"}
UNSUPPORTED = {".slint", ".ps1", ".psm1", ".sh", ".swift", ".m", ".c", ".h", ".nsi", ".cmd", ".js", ".ts"}
EXCLUDED_PREFIXES = (".claude/", ".codex/", "vendor/")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def safe_source(root, relative):
    rel = Path(relative)
    if rel.is_absolute() or ".." in rel.parts:
        raise ValueError("unsafe source path")
    path = root / rel
    if not path.is_file() or path.is_symlink():
        raise ValueError("source unavailable or symlink")
    return path


def rust_parser():
    from importlib.metadata import version
    for package, expected in RUST_VERSIONS.items():
        if version(package) != expected:
            raise RuntimeError(f"Parser version mismatch: {package}")
    from tree_sitter import Language, Parser
    import tree_sitter_rust
    return Parser(Language(tree_sitter_rust.language()))


def rust_index(data, path, parser=None):
    parser = parser or rust_parser()
    tree = parser.parse(data)
    declarations, errors = [], []

    def text(node):
        return data[node.start_byte:node.end_byte].decode("utf-8") if node else None

    def attributes(node):
        found = []
        sibling = node.prev_named_sibling
        while sibling and sibling.type in {"attribute_item", "line_comment", "block_comment"}:
            if sibling.type == "attribute_item":
                found.append(text(sibling))
            sibling = sibling.prev_named_sibling
        return list(reversed(found))

    def walk(node, scopes, inherited):
        if node.is_error or node.is_missing:
            errors.append({"kind": "ERROR" if node.is_error else "MISSING", "node_type": node.type, "start_byte": node.start_byte, "end_byte": node.end_byte, "start_line": node.start_point.row + 1, "end_line": node.end_point.row + 1})
        local = attributes(node) if node.type in RUST_KINDS else []
        effective = inherited + local
        next_scopes = scopes
        if node.type in RUST_KINDS:
            name = text(node.child_by_field_name("name"))
            if node.type == "impl_item":
                trait, target = text(node.child_by_field_name("trait")), text(node.child_by_field_name("type"))
                name = f"{trait} for {target}" if trait else target
            if node.type == "macro_invocation":
                name = text(node.child_by_field_name("macro"))
            body = node.child_by_field_name("body")
            stop = body.start_byte if body and node.type not in {"macro_definition", "macro_invocation"} else node.end_byte
            identifier = f"{path}:{node.start_byte}:{node.type}"
            record = {
                "id": identifier, "path": path, "language": "rust", "kind": node.type,
                "name": name, "parent_ids": [item[0] for item in scopes], "scope_names": [item[1] for item in scopes],
                "start_byte": node.start_byte, "end_byte": node.end_byte,
                "start_line": node.start_point.row + 1, "end_line": node.end_point.row + 1,
                "signature_source": None if node.type == "macro_invocation" else data[node.start_byte:stop].decode("utf-8").rstrip(),
                "attributes": local, "inherited_attributes": inherited,
                "cfg_attributes": [a for a in effective if "cfg" in a],
                "test_conditional": any(a.strip() == "#[test]" or re.search(r"\bcfg(?:_attr)?\s*\([^\n]*\btest\b", a) for a in effective),
                "macro_expanded": False, "semantic_acceptance": False,
            }
            visibility = next((text(c) for c in node.named_children if c.type == "visibility_modifier"), None)
            record["visibility_source"] = visibility or "implicit/private_or_inherited"
            if node.type in {"function_item", "function_signature_item"}:
                record["parameters_source"] = text(node.child_by_field_name("parameters"))
                record["return_type_source"] = text(node.child_by_field_name("return_type"))
            if node.type == "enum_variant" and body:
                record["payload_source"] = text(body)
            if node.type == "field_declaration":
                record["type_source"] = text(node.child_by_field_name("type"))
            declarations.append(record)
            if node.type in {"impl_item", "mod_item", "struct_item", "enum_item", "trait_item", "function_item", "foreign_mod_item", "enum_variant"}:
                next_scopes = scopes + [(identifier, name or node.type)]
        if node.type not in NO_DESCEND:
            for child in node.children:
                if child.is_named or child.is_missing:
                    walk(child, next_scopes, effective)

    walk(tree.root_node, [], [])
    return {"declarations": declarations, "parse_errors": errors, "has_parse_error": tree.root_node.has_error}


def python_index(data, path):
    try:
        text = data.decode("utf-8")
        tree = ast.parse(text, filename=path, type_comments=True)
    except (UnicodeDecodeError, SyntaxError) as exc:
        return {"declarations": [], "parse_errors": [{"kind": type(exc).__name__, "start_line": getattr(exc, "lineno", None), "detail": "invalid UTF8 or Python syntax; no exception/source text exported"}], "has_parse_error": True}
    lines = data.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))

    def start(node):
        return offsets[node.lineno - 1] + node.col_offset

    def end(node):
        return offsets[node.end_lineno - 1] + node.end_col_offset

    declarations = []

    def walk(node, scopes):
        next_scopes = scopes
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            begin, finish = start(node), end(node)
            stop = start(node.body[0]) if node.body else finish
            record = {"id": f"{path}:{begin}:{type(node).__name__}", "path": path, "language": "python", "kind": type(node).__name__, "name": node.name, "parent_ids": [x[0] for x in scopes], "scope_names": [x[1] for x in scopes], "start_byte": begin, "end_byte": finish, "start_line": node.lineno, "end_line": node.end_lineno, "signature_source": data[begin:stop].decode("utf-8").rstrip(), "decorators_source": [data[start(d):end(d)].decode("utf-8") for d in node.decorator_list], "semantic_acceptance": False}
            declarations.append(record)
            next_scopes = scopes + [(record["id"], node.name)]
        for child in ast.iter_child_nodes(node):
            walk(child, next_scopes)

    walk(tree, [])
    return {"declarations": declarations, "parse_errors": [], "has_parse_error": False}


def isolated_rust_index(source, path, mode="--rust-worker"):
    # Native bindings can fail outside Python exceptions. Preserve failure per file.
    try:
        result = subprocess.run([sys.executable, "-B", str(Path(__file__).resolve()), mode, str(source.resolve()), "--source-label", path], capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        return {"declarations": [], "parse_errors": [{"kind": "parser_timeout", "seconds": 30}], "has_parse_error": True, "native_parser_failure": True}
    if result.returncode:
        return {"declarations": [], "parse_errors": [{"kind": "parser_process_failure", "exit_code": result.returncode}], "has_parse_error": True, "native_parser_failure": True}
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"declarations": [], "parse_errors": [{"kind": "invalid_parser_receipt"}], "has_parse_error": True, "native_parser_failure": True}


def build(root, snapshot, parser=None, isolate=False):
    from polyglot_syntax import GRAMMARS, parse
    root = root.resolve()
    frozen = json.loads(snapshot.read_text(encoding="utf-8"))
    if frozen["source_commit"] != BASELINE:
        raise ValueError("unsupported source baseline")
    portability_path = snapshot.parent / "source-portability.json"
    forms = {r["path"]: r for r in json.loads(portability_path.read_text())["entries"]} if portability_path.is_file() else {}
    files, symbols = [], []
    for item in frozen["source_files"]:
        path = item["path"]
        extension = Path(path).suffix.lower()
        row = {"path": path, "frozen_sha256": item["sha256"], "extension": extension, "semantic_acceptance": False}
        if path.startswith(EXCLUDED_PREFIXES):
            row.update(status="excluded_protected_or_vendor", declaration_count=0)
            files.append(row)
            continue
        source = safe_source(root, path)
        data = source.read_bytes()
        actual = sha(data)
        proof = forms.get(path, {})
        if actual != item["sha256"] and not (proof.get("source_commit") == BASELINE and proof.get("frozen_worktree_sha256") == item["sha256"] and proof.get("git_blob_sha256") == actual):
            raise ValueError(f"frozen source drift: {path}")
        row.update(actual_sha256=actual, bytes=len(data), lines=len(data.splitlines()), source_role="experiment" if path.startswith("experiments/") else ("test" if "/tests/" in path else "source_or_support"))
        if extension == ".rs":
            result = isolated_rust_index(source, path) if isolate else rust_index(data, path, parser)
        elif extension == ".py":
            result = python_index(data, path)
        elif extension in GRAMMARS:
            if isolate:
                result = isolated_rust_index(source, path, "--polyglot-worker")
            else:
                declarations, errors = parse(data, path, actual, GRAMMARS[extension][0])
                result = {"declarations": declarations, "parse_errors": errors, "has_parse_error": bool(errors)}
        else:
            row.update(status="unsupported_language" if extension in UNSUPPORTED else "non_selected_data_or_document", declaration_count=0)
            files.append(row)
            continue
        row.update(status="parser_process_failure" if result.get("native_parser_failure") else ("parse_error" if result["has_parse_error"] else "syntax_parsed"), declaration_count=len(result["declarations"]), parse_errors=result["parse_errors"], cfg_evaluated=False, macros_expanded=False)
        for declaration in result["declarations"]:
            declaration["source_sha256"] = actual
            declaration["file_has_parse_error"] = result["has_parse_error"]
            symbols.append(declaration)
        files.append(row)
    from collections import Counter
    return {"schema_version": 1, "source_commit": BASELINE, "parser_versions": {**RUST_VERSIONS, **{"tree-sitter-" + language: version for language, version in GRAMMARS.values()}}, "python_ast": f"{sys.version_info.major}.{sys.version_info.minor}", "files": files, "status_counts": dict(Counter(f["status"] for f in files)), "declaration_count": len(symbols), "scope_limits": ["Rust/Python/Slint/Swift/Objective-C/C/PowerShell/Bash explicit syntax only", "local/member declarations included; syntax errors retain partial navigation, not success", "macros not expanded; cfg not evaluated; types/call edges not resolved", "unsupported languages not interpreted as zero-symbol proof", "protected legacy/vendor are excluded, not reviewed", "no semantic line coverage or independent/native acceptance"], "complete_project_coverage": False}, symbols


def validate_artifacts(root, report, declarations):
    """Validate source hash/range/parent seams without parsing or semantic acceptance."""
    errors = []
    ids = {row["id"] for row in declarations}
    if len(ids) != len(declarations):
        errors.append("duplicate_declaration_ids")
    if report.get("source_commit") != BASELINE or report.get("complete_project_coverage") is not False:
        errors.append("invalid_index_acceptance_boundary")
    files = {row["path"]: row for row in report["files"]}
    seen_counts = {}
    source_data = {}
    for row in declarations:
        path = row["path"]
        if path not in source_data:
            source_data[path] = safe_source(root, path).read_bytes()
        data = source_data[path]
        begin, finish = row["start_byte"], row["end_byte"]
        if not 0 <= begin < finish <= len(data):
            errors.append("range:" + row["id"])
        if row["source_sha256"] != sha(data):
            errors.append("source_hash:" + row["id"])
        if not 1 <= row["start_line"] <= row["end_line"] <= len(data.splitlines()) + 1:
            errors.append("line_range:" + row["id"])
        if row.get("semantic_acceptance") is not False or any(parent not in ids for parent in row["parent_ids"]):
            errors.append("parent_or_acceptance:" + row["id"])
        if row.get("signature_source"):
            raw = data[begin:finish].decode("utf-8")
            if not raw.startswith(row["signature_source"]):
                errors.append("signature_source:" + row["id"])
        seen_counts[path] = seen_counts.get(path, 0) + 1
    for path, row in files.items():
        if row["declaration_count"] != seen_counts.get(path, 0):
            errors.append("file_count:" + path)
        if row["status"] == "syntax_parsed" and row.get("parse_errors"):
            errors.append("masked_parse_error:" + path)
        if row["status"] in {"unsupported_language", "excluded_protected_or_vendor"} and seen_counts.get(path):
            errors.append("invented_unsupported_symbols:" + path)
    if report["declaration_count"] != len(declarations):
        errors.append("total_count_mismatch")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--validate", action="store_true", help="Validate saved syntax ranges/source hashes without native parser dependencies")
    parser.add_argument("--rust-worker", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--polyglot-worker", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--source-label", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.rust_worker:
        print(json.dumps(rust_index(args.rust_worker.read_bytes(), args.source_label or "source.rs"), ensure_ascii=False))
        return
    if args.polyglot_worker:
        from polyglot_syntax import GRAMMARS, parse
        data = args.polyglot_worker.read_bytes()
        label = args.source_label or args.polyglot_worker.name
        declarations, errors = parse(data, label, sha(data), GRAMMARS[Path(label).suffix][0])
        print(json.dumps({"declarations": declarations, "parse_errors": errors, "has_parse_error": bool(errors)}, ensure_ascii=False))
        return
    root = args.root.resolve()
    output = args.output or root / "docs/agent-map/syntax"
    if args.validate:
        report = json.loads((output / "files.json").read_text(encoding="utf-8"))
        symbols = [json.loads(line) for line in (output / "declarations.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
        errors = validate_artifacts(root, report, symbols)
        print(json.dumps({"declarations": len(symbols), "files": len(report["files"]), "errors": errors, "semantic_acceptance": False}))
        if errors:
            raise SystemExit(1)
        return
    rust_parser()  # Verify installed pins before the isolated worker campaign.
    report, symbols = build(root, root / "docs/agent-map/reconciliation/snapshot.json", isolate=True)
    output.mkdir(parents=True, exist_ok=True)
    (output / "files.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output / "declarations.jsonl").write_text("".join(json.dumps(row, ensure_ascii=False) + "\n" for row in symbols), encoding="utf-8")
    print(json.dumps({"files": len(report["files"]), "status_counts": report["status_counts"], "declarations": len(symbols), "complete_project_coverage": False}))


if __name__ == "__main__":
    main()
