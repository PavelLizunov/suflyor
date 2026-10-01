"""Pinned CST navigation for native/script sources; not compiler or runtime proof."""
import importlib
import hashlib
import os
from pathlib import Path
from importlib.metadata import version

def line_at(data, offset):
    return data.count(b"\n", 0, offset) + 1


def decl_id(path, kind, begin, end):
    return f"{path}:{begin}:{end}:{kind}"

SLINT_DIGEST = "64d95623b895a714c80026747945876f6ec5c7e51520f1632640fa0f068f942c"
SLINT_REVISION = "f0c59d1507a4221b521a772296f7b3adf87b44ed"
_LIBRARIES = []  # Keep native grammar libraries alive for all parser/tree lifetimes.
GRAMMARS = {
    ".slint": ("slint", SLINT_REVISION),
    ".sh": ("bash", "0.25.1"),
    ".swift": ("swift", "0.7.3"),
    ".m": ("objc", "3.0.2"),
    ".c": ("c", "0.24.1"),
    ".ps1": ("powershell", "0.26.4"),
    ".psm1": ("powershell", "0.26.4"),
}
KINDS = {
    "slint": {"component_definition", "global_definition", "struct_definition", "enum_definition",
              "property", "callback", "callback_event", "function_definition", "import_statement"},
    "bash": {"function_definition"},
    "swift": {"class_declaration", "protocol_declaration", "function_declaration",
              "init_declaration", "deinit_declaration", "property_declaration",
              "typealias_declaration", "enum_entry", "subscript_declaration"},
    "objc": {"class_interface", "class_implementation", "protocol_declaration",
             "category_interface", "category_implementation", "method_declaration",
             "method_definition", "property_declaration", "function_definition",
             "type_definition", "struct_specifier", "enum_specifier", "enumerator",
             "declaration", "preproc_def", "preproc_function_def"},
    "c": {"function_definition", "type_definition", "struct_specifier",
          "enum_specifier", "enumerator", "declaration", "preproc_def", "preproc_function_def"},
    "powershell": {"function_statement", "class_statement", "enum_statement",
                   "class_method_definition", "class_property_definition", "enum_member"},
}
# String/heredoc/embedded source is data, not independently parsed declarations.
NO_DESCEND = {"comment", "string", "string_value", "raw_string_literal", "line_string_literal",
              "multi_line_string_literal", "heredoc_body", "here_string_literal",
              "expandable_string_literal", "verbatim_string_literal"}


def parser_for(language):
    from tree_sitter import Language, Parser
    if version("tree-sitter") != "0.25.2":
        raise RuntimeError("research binding must stay pinned to 0.25.2")
    if language == "slint":
        import ctypes
        configured = os.environ.get("SUFLYOR_RESEARCH_SLINT_GRAMMAR")
        if not configured:
            raise ModuleNotFoundError("SUFLYOR_RESEARCH_SLINT_GRAMMAR must name pinned Linux x86_64 library")
        path = Path(configured)
        if hashlib.sha256(path.read_bytes()).hexdigest() != SLINT_DIGEST:
            raise RuntimeError("Slint grammar hash mismatch")
        library = ctypes.CDLL(str(path.resolve()))
        library.tree_sitter_slint.restype = ctypes.c_void_p
        capsule = ctypes.pythonapi.PyCapsule_New
        capsule.restype = ctypes.py_object
        capsule.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p]
        grammar = Language(capsule(library.tree_sitter_slint(), b"tree_sitter.Language", None))
        _LIBRARIES.append(library)
        return Parser(grammar)
    expected = next(v for lang, v in GRAMMARS.values() if lang == language)
    if version("tree-sitter-" + language) != expected:
        raise RuntimeError("unexpected grammar version: " + language)
    return Parser(Language(importlib.import_module("tree_sitter_" + language).language()))


def text(data, node):
    return data[node.start_byte:node.end_byte].decode("utf-8") if node else ""


def declarator_name(node):
    """Follow C declarator shape, never mistake a parameter for the declared name."""
    if node is None:
        return None
    if node.type in {"identifier", "type_identifier", "field_identifier"}:
        return node
    inner = node.child_by_field_name("declarator")
    if inner:
        return declarator_name(inner)
    if node.type == "parenthesized_declarator":
        return next((found for child in node.named_children
                     if (found := declarator_name(child)) is not None), None)
    return None


def names_for(data, node, language):
    if language == "slint" and node.type == "import_statement":
        # One edge per statement; signature retains the complete imported-name list.
        return [text(data, c) for c in node.named_children if c.type == "string_value"][:1]
    if language == "slint" and node.type == "callback_event":
        return [text(data, c) for c in node.named_children if c.type == "simple_identifier"][:1]
    if language == "powershell":
        if node.type == "class_method_definition":
            container = next((c for c in node.named_children if c.type == "class_method_signature"), node)
        else:
            container = node
        wanted = "function_name" if node.type == "function_statement" else "simple_name"
        return [text(data, child) for child in container.named_children if child.type == wanted][:1]
    if language == "objc" and node.type == "property_declaration":
        declaration = next((c for c in node.named_children if c.type == "struct_declaration"), None)
        if declaration:
            return [text(data, c.named_children[0]) for c in declaration.named_children
                    if c.type == "struct_declarator" and c.named_children]
    if language == "objc" and node.type in {"method_declaration", "method_definition"}:
        # Direct identifiers are selector segments; identifiers inside parameters are not.
        segments = [text(data, c) for c in node.named_children if c.type == "identifier"]
        has_args = any(c.type == "method_parameter" for c in node.named_children)
        return [("".join(s + ":" for s in segments) if has_args else "".join(segments))]
    if language == "objc" and node.type in {"class_interface", "class_implementation",
                                             "category_interface", "category_implementation",
                                             "protocol_declaration"}:
        return [text(data, c) for c in node.named_children if c.type == "identifier"][:1]
    if language in {"c", "objc"} and node.type in {"declaration", "type_definition", "function_definition"}:
        return [text(data, found) for d in node.children_by_field_name("declarator")
                if (found := declarator_name(d)) is not None]
    if language == "swift" and node.type in {"deinit_declaration", "subscript_declaration"}:
        return ["deinit" if node.type == "deinit_declaration" else "subscript"]
    candidates = node.children_by_field_name("name")
    if language == "swift" and node.type in {"function_declaration", "init_declaration"}:
        # Swift uses the name field again for return type; the first is the function.
        candidates = candidates[:1]
    return [text(data, c) for c in candidates]


def parse(data, path, source_hash, language, parser=None):
    parser = parser if parser is not None else parser_for(language)
    root = parser.parse(data).root_node
    symbols, errors = [], []

    def walk(node, parents, scope, conditional):
        if node.type == "ERROR" or node.is_missing:
            errors.append({"kind": "missing" if node.is_missing else "ERROR",
                           "node_type": node.type, "start_byte": node.start_byte,
                           "end_byte": node.end_byte, "start_line": line_at(data, node.start_byte),
                           "end_line": line_at(data, node.end_byte)})
        if node.type in NO_DESCEND:
            return
        conditional = conditional + ([text(data, node).splitlines()[0]]
                                      if node.type.startswith("preproc_if") else [])
        child_parents, child_scope = parents, scope
        if node.type in KINDS[language]:
            names = names_for(data, node, language)
            # Anonymous aggregates still retain a range and explicit anonymous name.
            if not names:
                names = ["<anonymous>" if node.type in {"struct_specifier", "enum_specifier"}
                         else "<unnamed:" + node.type + ">"]
            body = node.child_by_field_name("body")
            if body is None:
                body = next((c for c in node.named_children
                             if c.type in {"compound_statement", "script_block", "imperative_block"}), None)
            end = body.start_byte if body else node.end_byte
            signature = data[node.start_byte:end].decode("utf-8").rstrip()
            new_ids = []
            for index, name in enumerate(names):
                # Multiple names can share one syntax span (e.g. Swift enum case list).
                ident = decl_id(path, node.type + ":" + str(index), node.start_byte, node.end_byte)
                row = {"id": ident, "path": path, "source_sha256": source_hash,
                       "language": language, "parser": "tree-sitter-" + language,
                       "kind": node.type, "name": name, "start_byte": node.start_byte,
                       "end_byte": node.end_byte, "start_line": line_at(data, node.start_byte),
                       "end_line": line_at(data, max(node.start_byte, node.end_byte - 1)),
                       "signature_source": signature, "parent_ids": parents,
                       "scope_names": scope, "attributes": [], "cfg_attributes": conditional,
                       "test_conditional": path.startswith("suflyor-mlx/Tests/"),
                       "macro_expanded": False, "semantic_acceptance": False}
                symbols.append(row)
                new_ids.append(ident)
            child_parents = parents + new_ids
            child_scope = scope + names
        # Missing delimiters are often anonymous CST tokens, not named_children.
        for child in node.children:
            if child.is_named or child.is_missing:
                walk(child, child_parents, child_scope, conditional)

    walk(root, [], [], [])
    return symbols, errors
