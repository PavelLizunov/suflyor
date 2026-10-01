# Frozen native bridge export/foreign declaration/direct-call inventory

[Inventory](inventory.json) maps **37 nonstatic C ABI definitions** across seven production macOS bridges to **37 Rust foreign declarations** and **57 direct CST call occurrences**. 182 frozen inputs (seven C/Objective-C plus 175 Rust production-directory sources) hashed before parse. No unmatched export/declaration/direct-call names in this selected set. Not an ABI/linker/compiler/cfg/reachability or native behavior pass.

## Counted scope

| Bridge | Nonstatic C ABI definitions |
|---|---:|
| [Mic](<../../../overlay-backend/native/macos/mic_capture.m>) | 11 |
| [System audio](<../../../overlay-backend/native/macos/system_capture.m>) | 6 |
| [Process memory](<../../../overlay-backend/native/macos/process_memory.c>) | 1 |
| [Clipboard](<../../../slint-experiment/src/native/macos/clipboard.m>) | 5 |
| [Screen/OCR](<../../../slint-experiment/src/native/macos/screen.m>) | 8 |
| [Status](<../../../slint-experiment/src/native/macos/status.m>) | 2 |
| [Window](<../../../slint-experiment/src/native/macos/window.m>) | 4 |

Each symbol has full native/Rust signature, precise ranges and direct-call containing-function ranges; call record flags enclosing closure. Foreign declaration and call syntaxes distinguish prototypes from execution expressions. Test/cfg branches are syntactically present, not evaluated. Macro token trees excluded rather than regex-counted as calls. Nonforeign trait signatures are not C ABI declarations.

Selectors/static C helpers, experiment bridges, Windows SDK wrappers/calls, external compiler-generated aliases, function-pointer dispatch, callback lifetime/closure alias resolution remain outside this name-based inventory. One symbol record is not a feature/function-acceptance record. No header generation/SDK/Clang/Rust compilation or application execution.

## Reproduce and verify

[Existing parser pins/setup](<../syntax/README.md>) supply Tree-sitter binding 0.25.2, Rust 0.24.2, Objective-C 3.0.2 and C 0.24.1 in ignored targets. No new dependency or global/production install. Full census is one isolated subprocess/60s timeout; parse/crash/nonzero fails instead of manufacturing successful file receipts.

```bash
export PYTHONPATH=.campaign-state/parser-site-stable:.campaign-state/language-site-locked
python3 -B docs/agent-map/operations/native_ffi_inventory.py --output docs/agent-map/native/inventory.json
# Saved validation itself needs no native parser:
python3 -B docs/agent-map/operations/native_ffi_inventory.py --output docs/agent-map/native/inventory.json --validate
```

Read generated artifact before replacement under file-observation policy. Eight fixtures cover comments/strings/macro exclusion, foreign versus trait prototypes, direct/qualified/closure calls, pointer-return C declarator/static/selector filtering, unresolved callback argument identifier, malformed syntax, hash/range/acceptance boundaries. Missing grammars may skip fixtures, never an accepted parser run.

[Manual ownership/thread/permission contract](<../features/native-macos-ffi-and-ownership.md>) records source chains beyond symbol matching, including system Pending detach and screenshot timeout late-image concern. No native reproductions or original-Grok classification promotion. Every other language/native/semantic gap stays explicit.
