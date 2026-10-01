# Canonical PowerShell syntax corroboration — parser-only

**Verdict: Extend — use official System.Management.Automation.Language.Parser AST instead of rewriting/sanitizing seven Tree-sitter grammar-error sources.** No handwritten PowerShell parser, SDK/.NET SDK installation, global runtime change, native builds or script execution.

## Runtime and safety

Official [PowerShell 7.4.13 release](https://github.com/PowerShell/PowerShell/releases/tag/v7.4.13) supplies Linux x64 portable runtime. [Release asset](https://github.com/PowerShell/PowerShell/releases/download/v7.4.13/powershell-7.4.13-linux-x64.tar.gz) is 72,482,320 bytes, SHA256 `59e5df675dacbfe45374c32c1bf2480168a33243423c9b19252d0476fd1b748c`, matched GitHub asset digest. Safe regular-file extraction under ignored task-local cache; MIT licence and bundled third-party notice file inspected. [Provenance/623-file manifest](powershell-parser-provenance.json) preserves asset and every extracted runtime file hash; Python checks all manifest hashes before starting pwsh.

This is a self-contained interpreter/parser runtime, **not an SDK/global PowerShell installation**. Existing libc/ICU/SSL/z libraries sufficed; nothing provisioned globally. Task-owned worker only runs with NoLogo/NoProfile/NonInteractive, telemetry/update opt-out, exact version 7.4.13. It reads source and calls Parser::ParseInput, traverses AST/tokens, never dot-sources/Invoke-Expression/Add-Type/builds/gates the input. Fixture input contains throws/here-string C# and is not executed. Runtime binaries/modules/tarball remain ignored, not shipped production artifacts.

## Result and preserved counterevidence

Previous [Tree-sitter ERROR receipts](powershell-tree-sitter-errors.json) preserved separately for all seven affected paths and original binding/grammar versions. Canonical parser now accepts all 24 selected PowerShell files, retains 25 source function definitions matching prior names/lines, and resolves the seven grammar errors **without changing any source bytes**. No embedded C# execution or validation; here-strings are opaque data.

Initial canonical trial falsely reported errors in a UTF8-BOM screenshot helper: naive ReadAllBytes/GetString passed U+FEFF into ParseInput. Independent call to canonical ParseFile consumed BOM and returned no errors; helper now mirrors that decoding and adds BOM length back to raw-byte extents. Explicit BOM fixture protects this integration correction. No source finding was promoted from the bad trial.

Index uses FunctionDefinitionAst/TypeDefinitionAst/FunctionMemberAst/PropertyMemberAst with exact UTF16→UTF8/BOM-safe byte ranges and ancestor scopes. Other script blocks/commands/variables/embedded languages/cfg-like conditions aren't silently counted as declarations. AST parser acceptance does not prove Windows PowerShell 5.1 compatibility, command/module existence, build/cleanup safety or native behavior.

Current syntax result: 298 successes across Rust/Python/Slint/Swift/Objective-C/C/Bash/PowerShell/NSIS, zero source parse/native-process errors/unsupported selected languages, 22 protected/vendor exclusions and 520 nonselected. Still 16,255 records, **not function count/all-semantic acceptance**. 16,230 non-PowerShell nodes unchanged; only PS declaration schema/parser identity/ranges replaced. Macros/cfg/include/type/dynamic callers remain unresolved.

## Fixtures and reproduction

Seven canonical fixtures: native argument/operators/1MB size suffix, nested/class-method/enum scopes, comments/here-string fake declarations with never-executed input throws, emoji/Cyrillic/CRLF spans, UTF8 BOM advanced-function ParseFile parity, malformed input with ErrorId/ranges and runtime absence/hash failures. Per-file subprocess 30s; nonzero/invalid receipt explicit. Helpers do not print source/error chains that could contain sensitive endpoints.

```bash
# Download exact portable asset above, verify digest and safely extract regular members only.
# Runtime and complete manifest must match; no pwsh/npm/module install or update.
export SUFLYOR_RESEARCH_POWERSHELL="$PWD/.campaign-state/powershell-parser"
# Original grammar/WASM/PO setup from syntax/README.md.
python3 -B -m unittest discover -s docs/agent-map/operations -p 'test_*.py'
python3 -B docs/agent-map/operations/syntax_index.py
python3 -B docs/agent-map/operations/syntax_index.py --validate
```

Retain original Tree-sitter grammar to reproduce its limitations/fixtures; do not rename grammar error evidence to confirmed source bug. Zero parse errors is bounded grammar/AST evidence, not a completion percentage; original 39/75/5 statuses, independent/native/compiler acceptance unchanged.
