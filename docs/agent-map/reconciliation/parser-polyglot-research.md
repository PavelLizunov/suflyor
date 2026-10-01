# Native/UI/script syntax extension — round 5

## Decision and tested scope

**Verdict: Extend — reuse the frozen Python index and Tree-sitter binding 0.25.2 with selected prebuilt grammars.** No custom regex parser, production dependency, SDK, Cargo/Swift build, application execution or model dispatch. Bindings/grammars are task-local and ignored; all records remain syntax navigation, not semantic acceptance.

[Wheel metadata and binary provenance](parser-polyglot-provenance.json) and [hash lock](../operations/parser-polyglot-requirements-linux.txt) preserve Bash 0.25.1, Swift 0.7.3, Objective-C 3.0.2, C 0.24.1 and PowerShell 0.26.4. The PowerShell **grammar** version is not the rejected Tree-sitter **binding** 0.26.0. Each selected wheel's exact bytes/hash and embedded MIT license were inspected; offline `--require-hashes --no-deps --only-binary` installation was exercised in a new ignored target.

## Slint search-first route

Standalone [Slint PyPI lookup](https://pypi.org/pypi/tree-sitter-slint/json) and [npm registry lookup](https://registry.npmjs.org/tree-sitter-slint/latest) returned 404; [upstream releases](https://api.github.com/repos/slint-ui/tree-sitter-slint/releases) were empty. The older language-pack 0.13.0 wheel was inspected, not installed: its 170 shared grammar libraries/catalog did not contain Slint. Do not substitute it based on current README advertising.

The immutable [language-pack v1.20.0 grammar catalog](https://raw.githubusercontent.com/xberg-io/tree-sitter-language-pack/v1.20.0/sources/language_definitions.json) names Slint revision `f0c59d1507a4221b521a772296f7b3adf87b44ed`. Its [Linux x86_64 parsers asset](https://github.com/xberg-io/tree-sitter-language-pack/releases/download/v1.20.0/parsers-linux-x86_64.tar.zst) has GitHub SHA256 `f72a6cc06efdc70785ebd283ecfae23e650f12211887349898594c78757d0f5b`; downloaded bytes matched. Only `libtree_sitter_slint.so` is loaded, with separate SHA256 `64d95623b895a714c80026747945876f6ec5c7e51520f1632640fa0f068f942c` checked **before** ctypes. The library exports `tree_sitter_slint`, ABI 15 works with binding 0.25.2. The [pinned MIT license](https://raw.githubusercontent.com/slint-ui/tree-sitter-slint/f0c59d1507a4221b521a772296f7b3adf87b44ed/LICENSES/MIT.txt) was read. Catalog provenance is not a reproducible native-build attestation.

Do not install the modern pack or invoke runtime downloads; reproduce the exact release-asset selection and supply its library path explicitly through `SUFLYOR_RESEARCH_SLINT_GRAMMAR`. Full package and grammar binaries are not committed.

## Observed acceptance limits

- All 25 selected Slint, five Swift, eight Objective-C, one C and ten Bash snapshot files parse with no reported errors; 17/24 selected PowerShell files parse. Seven PowerShell files retain ERROR spans and partial declaration nodes flagged `file_has_parse_error: true`.
- Parser errors are **not proof that PowerShell source is invalid**. For example the grammar rejects `1MB` in `scripts/build-slint-release.ps1:134`. Native System.Management.Automation parsing was not run; PowerShell is not installed here. No automatic source rewrite/sanitization/error suppression.
- NSIS remains the one unsupported snapshot file. [PyPI NSIS lookup](https://pypi.org/pypi/tree-sitter-nsis/json) returned 404 and the inspected pinned language-pack catalog contains no NSIS entry. This is only a bounded negative search, not proof no existing parser exists; investigate native NSIS parser/tokenizer without executing installer or inventing regex completeness.
- Declaration fixtures cover nested shell/PowerShell functions, heredoc/comment/string non-declarations, Swift types/method return-name ambiguity/init/enum case lists, Objective-C selectors/pointer typedefs/property nodes, C multi-declarators/UTF8/preprocessor context, Slint component/global/property/callback event/function/import scopes and malformed input. Missing anonymous punctuation tokens are now traversed in both Rust and polyglot CSTs.
- C/Objective-C local/member declarations are included. Slint callback events/import statements and unexpanded Rust macros are navigation records, not functions or resolved caller edges. Swift extensions, dynamic selectors, includes, preprocessors, macros, cfg and platform semantics need separate manual/compiler evidence.
- One native subprocess per file, 30s timeout, nonzero/crash/invalid receipt preserved. Grammar absence/version/hash mismatch fails closed; missing grammar fixtures may skip and are not an accepted parser run.

## Search limitation

The configured web-search endpoint returned HTTP 402 (insufficient balance); no provider/settings changes were made. Direct PyPI/GitHub registry/upstream fetches supplied the evidence above. This did not block local inspection/implementation.
