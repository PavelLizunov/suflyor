# Frozen syntax index: Rust CST and Python AST

This is a reproducible declaration/navigation index, **not semantic line review, macro expansion, evaluated cfg, compiler type resolution or native acceptance**. Historical regex records remain separately available with explicit unverified labels.

## Measured current scope

- Frozen baseline: `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`.
- [File policies/results](files.json): 840 baseline tracked non-map paths.
- 224 selected files syntax-parsed: 220 Rust, four Python, zero reported parse errors/native worker failures in the accepted pinned run.
- 74 files explicitly unsupported (including Slint, Swift/Objective-C, PowerShell/Bash/NSIS); 22 protected legacy/vendor paths excluded; 520 nonselected data/doc paths.
- [Declaration/node records](declarations.jsonl): 13,961 total, including 7,692 unexpanded macro invocations, 3,477 Rust function bodies, 68 Rust signature-only functions, fields/types/variants/constants/impl/module items and Python declarations. **13,961 is not a function count.**
- Parent IDs/scope, exact byte/line ranges, signature text and attributes are retained; macro bodies are not expanded or copied as invented generated functions. Some signature/location limits follow CST/AST, not every semantic caller.

## Parser provenance and failed attempt

[Package provenance](../reconciliation/parser-install-provenance.json) pins Tree-sitter Python 0.25.2 and Rust grammar 0.24.2 from exact wheel SHA with embedded MIT licenses inspected. Python AST uses the running 3.12 stdlib. Research dependencies live only in ignored `.campaign-state/parser-site-stable`, not app manifests/global Python/DSH runtime.

[Failed attempt](../reconciliation/parser-failed-attempt.json) preserves 0.26.0 native crashes despite successful fixtures. One-process-per-file isolation makes crash/timeout/invalid output explicit rather than accepting an empty file. Alternate 0.25.2 parsed all selected snapshot files with no observed errors; this is compatibility evidence of tested snapshot, not a root-cause fix of upstream binding.

## Reproduce

Linux x86_64 CPython 3.12 only for this wheel lock. Do not install a Rust SDK or build application on DSH.

```bash
python3 -m pip install --only-binary=:all: --no-deps --require-hashes \
  --target .campaign-state/parser-site-stable \
  -r docs/agent-map/operations/parser-requirements-linux-cp312.txt
PYTHONPATH=.campaign-state/parser-site-stable python3 -B docs/agent-map/operations/syntax_index.py
PYTHONPATH=.campaign-state/parser-site-stable python3 -B -m unittest discover \
  -s docs/agent-map/operations -p 'test_*.py' -v
# Source-range/hash/parent validation needs no native parser dependency:
python3 -B docs/agent-map/operations/syntax_index.py --validate
```

Dependency setup needs network unless cached wheels exist. Generated output must be read before replacement under agent file-observation policy. A different Python/platform needs its own verified wheel selection; do not bypass hash/ABI pin errors.

## Checks and remaining work

14 parser/index fixtures cover Rust multiline/generics/tuple args/enum payload/impl/trait scope, comments/strings/unexpanded macro, cfg/test inheritance, UTF8 ranges, parse errors, Python nested/async/decorators, source drift, excluded/unsupported policies and native worker crash/timeout receipts. Full research suite passed 62 checks with pinned wheels loaded; without those wheels Rust-specific fixtures skip and that is **not a parser PASS**.

Saved index validation checks exact source hashes, spans/signature prefixes, parent IDs, per-file and total counts. Remaining: Slint/native/script grammars, configuration schema/all-caller feature coverage, macro/cfg awareness and manually verified semantics. Never turn successful syntax parse/input byte coverage into a 100% review claim.
