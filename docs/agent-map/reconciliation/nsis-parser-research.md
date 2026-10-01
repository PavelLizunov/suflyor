# NSIS parser extension: bounded search-first and evidence

**Verdict: Extend — pinned prebuilt tree-sitter-nsis WASM grammar and web-tree-sitter runtime, reuse existing syntax index schema/isolation.** No handwritten regex parser, NSIS compiler, SDK, npm lifecycle scripts, production dependencies or installer execution.

## New registry evidence

The prior [polyglot search](parser-polyglot-research.md) found no PyPI NSIS package or pinned language-pack entry; that bounded search was not proof parser absence. Direct [npm registry](https://registry.npmjs.org/tree-sitter-nsis/0.4.1) now provides grammar 0.4.1 with `tree-sitter-nsis.wasm`; [web-tree-sitter 0.25.10](https://registry.npmjs.org/web-tree-sitter/0.25.10) supplies ready CJS+WASM runtime. Both MIT licences read, registry tarball SHA512 integrity and SHA256 matched, selected files hashed. [Provenance](nsis-parser-provenance.json) records immutable registry/package/file identities.

Use existing Node **22.23.2** only for this evidence; Python Tree-sitter binding remains **0.25.2**, not rejected 0.26.0. WASM grammar ABI15 smoke-tested. Runtime/grammar loaded files hash-checked before require/WASM execution. All dependencies in ignored task-local cache; source parser.c/grammar metadata may be inspected but not compiled. WASM does not execute NSIS script commands.

## Observed frozen-source result

Installer source parses without ERROR/missing nodes. Twelve navigation records: six `!define` directives, two sections and four labels. Zero function declarations in this particular file is grammar evidence for selected syntax, not zero installer behavior/functions from macro/include expansion. Uninstall's MessageBox/RMDir/Delete actions are not “tested safe” by a parse.

All unchanged 16,243 prior syntax nodes preserved; adding 12 NSIS nodes gives 16,255. File scope becomes 291 successful parses and seven partial PowerShell grammar-error files, no unsupported selected snapshot languages. 22 protected/vendor and 520 nonselected exclusions remain. Syntax success does not establish semantic audit percent/native acceptance.

## Tests and limits

Six new fixtures cover UTF16→UTF8 byte ranges (Cyrillic and emoji), comments/strings fake definitions, section/macro/label parent scopes, variables/preprocessor, missing-end errors, unavailable runtime and hash failure **before** loading JS. Node grammar macro bodies are syntax only, `macro_expanded: false`; cfg/conditional syntax retained, never evaluated. Parser runs isolated 30s; nonzero/crash/invalid receipt explicit.

Pinned cache reproduction: download the two exact tarballs from provenance, verify SHA512 integrity **and** SHA256, safely extract regular members under own package directories (reject absolute/traversal/symlinks), set `SUFLYOR_RESEARCH_NSIS_WASM` to directory containing `web-tree-sitter/` and `tree-sitter-nsis/`. No `npm install`, package scripts or online runtime fetch.

```bash
export SUFLYOR_RESEARCH_NSIS_WASM="$PWD/.campaign-state/nsis-wasm"
# Other grammar setup/PYTHONPATH as in syntax/README.md.
node docs/agent-map/operations/nsis_worker.cjs scripts/slint-installer.nsi scripts/slint-installer.nsi
python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py'
python3 -B docs/agent-map/operations/syntax_index.py --validate
```

Remaining: seven PowerShell grammar-error files, full semantic/macro/include/compiler/native validation, remaining Config/UI/Windows/indirect callers, original hypotheses and independent acceptance. No raw reports touched, original 39/75/5 claim counts unchanged.
