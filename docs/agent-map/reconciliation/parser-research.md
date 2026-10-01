# Reliable symbol extraction: proportional search-first record

## Requirement and constraints

Replace historical regex-only symbol candidates with reproducible language-aware declaration ranges and explicit parse errors/cfg/macro limits. Use an isolated research tool, not production dependencies or a newly provisioned Rust SDK on DSH. No dependency installed or native build performed in this search pass.

## Current installed facts

Observed by import/command discovery: Python 3.12 standard `ast` and `tomllib` available; tree_sitter, tree_sitter_languages, tree_sitter_rust and tree_sitter_language_pack absent. tree-sitter CLI, ctags, rust-analyzer, rustfmt and Cargo absent in this control-plane session. Repo search found no owning existing general symbol extractor; historical generator is regex and still marked heuristic.

## Live external evidence

Sources were fetched as data, not instructions. Registry/upstream observations are not executed parser compatibility tests.

- [Python Tree-sitter registry](https://pypi.org/pypi/tree-sitter/json): latest observed 0.26.0, Python >=3.10, Linux wheels for supported Python/platforms. [Upstream license](https://raw.githubusercontent.com/tree-sitter/py-tree-sitter/master/LICENSE) MIT.
- [Rust grammar registry](https://pypi.org/pypi/tree-sitter-rust/json): 0.24.2, Python >=3.9, MIT, Linux abi3 wheels; [upstream grammar](https://github.com/tree-sitter/tree-sitter-rust) parses Rust syntax. It does not provide rustc type resolution or macro expansion.
- [Multi-language pack registry](https://pypi.org/pypi/tree-sitter-language-pack/json): observed 1.20.0, MIT, Python >=3.10, tree-sitter>=0.23, Linux manylinux_2_34 wheels. [Current upstream README](https://github.com/xberg-io/tree-sitter-language-pack) lists Rust/Python/Slint/Swift/C/Bash and other grammars and on-demand fetching. Main README may differ from the observed release; verify the exact pinned package catalog/grammar revisions before use, not assume every advertised language ships unchanged.
- [syn registry](https://crates.io/api/v1/crates/syn): observed latest 3.0.6, MIT OR Apache-2.0, Rust parser/full/visit features. A Rust-only AST helper on an authorized native worker could give stronger Rust item detail but still not automatic macro/cfg/type acceptance; no Cargo implementation is authorized on DSH by this research.
- [Slint compiler parser](https://github.com/slint-ui/slint/blob/master/internal/compiler/parser.rs): official syntax node definitions for properties/callbacks/components/expressions and byte ranges; license is Slint's multi-license scheme, not assumed MIT. Repo currently declares Slint 1.17; pin matching version if integrating rather than importing mutable master code.

## Decision

**Verdict: Compose — Python stdlib AST/TOML plus pinned Tree-sitter grammars for declaration navigation, with Slint parser/grammar validation separately.** This is an implementation direction, not a completed installation or accurate full-project index. Prefer selected standalone grammars initially to a broad runtime-downloading pack; use the pack only after exact catalog/digest/ABI/redistribution verification shows justified coverage.

Why not a custom regex: known failures include multiline signatures, impl scope leakage, nested enum payloads, unsupported Swift/Objective-C/PowerShell/shell/NSIS and symbols inside comments/strings. AST/syntax trees address structure but not reachability or semantic review by themselves.

## Required small acceptance slice

Before dispatching a whole-repo extraction:

1. Freeze parser/grammar versions and wheel SHA/platform; isolate local research environment without touching app manifests/providers.
2. Parse Rust nested enum/impl/generics/multiline/attributes/string/comment fixtures, representative Slint callbacks/properties and Swift/Objective-C seams. Report ERROR/missing nodes instead of treating a file as complete.
3. For each item retain exact raw source range, normalized name/kind, signature, parent/cfg/test flag, unresolved macro-generated-symbol status and source hash.
4. Prove scopes/signatures/fields against manually read reference cases and native parser where applicable; use syntax extraction for navigation, not complete function semantics.
5. Account for all file/language policies, generated/vendor/experiments/legacy protected scopes. A grammar missing for a language stays unparsed; no zero-symbol completeness claim.
6. Compare reviewed item inventory against prior candidates and per-file requirements; do not equate successful parser tree or covered input bytes with every-line code understanding.

Parser setup is useful remaining work, not a blocked condition. No package, grammar runtime download or DSH plugin was installed by this search.
