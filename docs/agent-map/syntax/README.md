# Frozen syntax navigation: language-aware, not semantic acceptance

This reproducible declaration/navigation index is **not semantic line review, macro expansion, evaluated cfg, compiler type resolution or native acceptance**. Historical regex records remain separate and unverified.

## Measured round-5 scope

- Frozen baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- [File policies/results](files.json): 840 baseline tracked non-map paths.
- 290 files syntax-parsed without reported errors: 220 Rust, four Python, 25 Slint, five Swift, eight Objective-C, one C, ten Bash and 17 PowerShell.
- Seven additional PowerShell files have explicit `parse_error` receipts and partial nodes; **not** counted as syntax success. Native PowerShell parser was not run, so this is not a verdict on script validity.
- One unsupported NSIS file; 22 protected legacy/vendor paths excluded; 520 nonselected data/doc paths. Exclusions retain source hashes, not review credit.
- [Declaration/node records](declarations.jsonl): **16,243 total, not a function count**. Unchanged Rust/Python subtotal 13,961 includes 7,692 unexpanded macro invocations. Added 2,282 navigation records: 1,563 Slint (including properties/callback events/imports), 301 Swift, 384 Objective-C, three C, 25 PowerShell, six Bash. C/Objective-C local/member declarations are included; partial PowerShell files retain `file_has_parse_error: true`.
- Exact byte/line ranges, signature text, parent IDs/scopes and applicable attributes retained. Caller reachability, overload/selector dispatch, native ABI correctness, extension/type resolution and macro-generated functions are unresolved.

## Parser provenance

[Original package provenance](../reconciliation/parser-install-provenance.json) pins Tree-sitter **binding 0.25.2** and Rust grammar 0.24.2. [Failed attempt](../reconciliation/parser-failed-attempt.json) preserves rejected binding 0.26.0 crashes; never replay it.

[Additional grammar provenance](../reconciliation/parser-polyglot-provenance.json) records wheel hashes/platform/licenses and exact Slint Linux shared-library digest. [Search/decision/limits](../reconciliation/parser-polyglot-research.md) distinguishes grammar versions from binding versions, source catalog provenance from native build attestation, and grammar errors from source errors. Python AST remains running 3.12 stdlib. All binaries/dependencies remain in ignored research targets, not app manifests or global/DSH runtime.

## Reproduce

Linux x86_64 CPython 3.12 only for this evidence. No Rust/Swift SDK or application build on DSH.

```bash
python3 -m pip install --only-binary=:all: --no-deps --require-hashes \
  --target .campaign-state/parser-site-stable \
  -r docs/agent-map/operations/parser-requirements-linux-cp312.txt
python3 -m pip install --only-binary=:all: --no-deps --require-hashes \
  --target .campaign-state/language-site-locked \
  -r docs/agent-map/operations/parser-polyglot-requirements-linux.txt
# Obtain the immutable Slint release asset in parser-polyglot-provenance.json.
# Verify archive hash, select only its libtree_sitter_slint.so member, verify library hash.
# Do not install language-pack, invoke dynamic fetches, or extract arbitrary archive paths.
export SUFLYOR_RESEARCH_SLINT_GRAMMAR="$PWD/.campaign-state/slint-grammar/libtree_sitter_slint.so"
export PYTHONPATH=.campaign-state/parser-site-stable:.campaign-state/language-site-locked
python3 -B docs/agent-map/operations/syntax_index.py
python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py' -v
# Source hash/range/parent validation needs no parser dependency:
python3 -B docs/agent-map/operations/syntax_index.py --validate
```

Setup needs network unless artifacts are cached. Read generated files before replacing them under file-observation policy. A different platform/Python needs separate verified artifacts; do not bypass ABI/version/hash failures. Each native file is parsed in a subprocess with a 30s timeout, nonzero/crash/invalid receipt explicit. Slint library hash is checked before ctypes and its lifetime retained.

## Checks and remaining work

71 current research tests pass with all pinned grammars supplied: prior 48 recovery/SQL/source checks plus 23 syntax/index fixtures (15 Rust/Python/index and eight polyglot). Fixtures without grammars may skip: **not parser PASS**. Missing anonymous delimiter tokens are now traversed in Rust and all added CSTs. Saved index validation checks source hashes/ranges/signature prefixes/parent IDs/counts.

Remaining: NSIS parsing; resolving seven PowerShell grammar errors with trusted native parsing or a verified better grammar; startup/wizard/health/diagnostics/config/translation/assets/all-caller feature coverage; macro/cfg awareness and manually reviewed semantics. Successful parse or input byte coverage never establishes a 100% review.
