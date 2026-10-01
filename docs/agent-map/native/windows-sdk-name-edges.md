# Selected Windows SDK name-edge candidates

[Machine inventory](windows-sdk-name-edges.json): four frozen adapters — native screen, singleton lifecycle, win32 facade and tray — yield **180 SDK import records /121 direct or qualified call syntax candidates**. Every row `resolved_symbol:false`, every inventory `semantic_acceptance:false`. **Not** Windows API graph/exhaustive adapters/compiler cfg/native behavior acceptance.

Rust CST handles nested use lists and rename aliases to namespace strings; direct unqualified callees matched to file-level imports, fully windows-qualified calls separately labelled. Member calls/comments/string/macro token trees not counted. Lexical shadowing/scope/cfg/module/type/trait/function-pointer ambiguity unresolvable here; Windows tuple-type constructors can appear as calls, not all records are OS functions. POSIX/test syntax in facade retained, not Windows runtime reachability.

Source hashes/byte-line ranges/container function preserved. Seven fixtures exercise alias/nesting/unresolved local shadowing, macro/comment/member exclusions, UTF8/closure ranges, invalid syntax, source GDI deselection/cleanup/partial scanline conditions, WDA readback/restore order and tray lifetime/context. No SDK install/Rust compile/Win32 call/input/clipboard/window change.

```bash
PYTHONPATH=.campaign-state/parser-site-stable python3 -B \
  docs/agent-map/operations/windows_sdk_edges.py --output docs/agent-map/native/windows-sdk-name-edges.json
python3 -B docs/agent-map/operations/windows_sdk_edges.py \
  --output docs/agent-map/native/windows-sdk-name-edges.json --validate
```

Read generated files before replacement under file-observation policy. Binding 0.25.2/Rust grammar 0.24.2 unchanged; worker isolated 60s. [Manual contract](../features/windows-capture-tray-and-sdk-ownership.md) separates observed code/order from native ownership/visual/fault scenarios. Backend audio/JobObject/credentials and all indirect callers still outside this selected SDK census; no original claim status promotion or completeness percentage.
