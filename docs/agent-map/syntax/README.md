# Frozen syntax navigation: language-aware, not semantic acceptance

This reproducible declaration/navigation index is **not semantic line review, macro expansion, evaluated cfg, compiler type resolution or native acceptance**. Historical regex records remain separate and unverified.

## Measured round-5 scope

- Frozen baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- [File policies/results](files.json): 840 baseline tracked non-map paths.
- 298 files syntax-parsed without reported errors: 220 Rust, four Python, 25 Slint, five Swift, eight Objective-C, one C, ten Bash, 24 PowerShell and one NSIS.
- All selected PowerShell now use canonical System.Management.Automation.Language.Parser 7.4.13 AST; prior seven Tree-sitter grammar errors preserved in [historical receipts](../reconciliation/powershell-tree-sitter-errors.json), not source bugs. [Parser research](../reconciliation/powershell-parser-research.md) records BOM decoding correction and native/compatibility limits.
- No unsupported selected snapshot languages or source parse/process errors in this pinned run. 22 protected legacy/vendor paths excluded; 520 nonselected data/doc paths. Exclusions retain source hashes, not review credit.
- [Declaration/node records](declarations.jsonl): **16,255 total, not a function count**. Unchanged Rust/Python subtotal 13,961 includes 7,692 unexpanded macro invocations. Added 2,294 navigation records: 1,563 Slint (including properties/callback events/imports), 301 Swift, 384 Objective-C, three C, 25 PowerShell, six Bash and 12 NSIS (six preprocessor definitions/two sections/four labels). C/Objective-C local/member declarations are included; PowerShell AST functions are not proof of Windows script/module/command execution. All other 16,230 language records preserved unchanged by canonical PS replacement.
- Exact byte/line ranges, signature text, parent IDs/scopes and applicable attributes retained. Caller reachability, overload/selector dispatch, native ABI correctness, extension/type resolution and macro-generated functions are unresolved.

## Parser provenance

[Original package provenance](../reconciliation/parser-install-provenance.json) pins Tree-sitter **binding 0.25.2** and Rust grammar 0.24.2. [Failed attempt](../reconciliation/parser-failed-attempt.json) preserves rejected binding 0.26.0 crashes; never replay it.

[Additional grammar provenance](../reconciliation/parser-polyglot-provenance.json) records wheel hashes/platform/licenses and exact Slint Linux shared-library digest. [Search/decision/limits](../reconciliation/parser-polyglot-research.md) distinguishes grammar versions from binding versions, source catalog provenance from native build attestation, and grammar errors from source errors. Python AST remains running 3.12 stdlib. All binaries/dependencies remain in ignored research targets, not app manifests or global/DSH runtime.

[NSIS WASM provenance](../reconciliation/nsis-parser-provenance.json) and [research/reproduction](../reconciliation/nsis-parser-research.md) pin prebuilt npm grammar 0.4.1, web-tree-sitter 0.25.10 and existing Node 22.23.2. No npm install/scripts, NSIS compiler or Python binding upgrade. Set `SUFLYOR_RESEARCH_NSIS_WASM` to the hash-verified ignored package directory before regeneration; absence/hash/runtime mismatch becomes explicit parser failure.

[Canonical PowerShell provenance](../reconciliation/powershell-parser-provenance.json) pins official Linux-x64 portable runtime 7.4.13 and complete file manifest. Set `SUFLYOR_RESEARCH_POWERSHELL` to its verified ignored target; NoProfile/NonInteractive task-owned helper reads/parses only, never executes repo scripts. No SDK/global installation. Windows PowerShell/platform behavior remains unaccepted.

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

116 current research tests pass with all pinned grammars/NSIS WASM/PO reader/canonical PS runtime supplied: prior 109 plus seven canonical PS fixtures. Syntax/index fixture subtotal 36 (15 Rust/Python/index, eight polyglot, six NSIS, seven canonical PS). Fixtures without grammars may skip: **not parser PASS**. Missing anonymous delimiter tokens are now traversed in Rust and all added CSTs. Saved index validation checks source hashes/ranges/signature prefixes/parent IDs/counts.

Remaining: compiler/platform semantics beyond parsed syntax, all remaining startup/config/UI/native consumer/caller feature coverage; macro/cfg awareness and manually reviewed semantics. Successful parse or input byte coverage never establishes a 100% review.
