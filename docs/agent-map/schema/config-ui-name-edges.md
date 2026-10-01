# Rust Config/Slint name-edge candidates — unresolved navigation

[Machine inventory](config-ui-name-edges.json) uses pinned Rust CST over 175 frozen Rust sources. It collects **2,951 candidates**: 1,232 member expressions whose names match 84 Config fields, plus 1,719 member-call names matching generated Slint setters/getters/callback registration/invocation names. **Type/receiver/cfg/alias resolution false for every row**; no percentage, semantic or complete consumer graph acceptance.

Records carry path/raw byte-line span, receiver syntax kind/simple identifier (not arbitrary expression text), containing function/range, closure flag, direct assignment-LHS flag, immediate parent kind and exact candidate UI declaration ranges. No user configs/private endpoints/comments/default expression values copied. Source hashes and schema-inventory hash checked before generation; validation rejects drift/name/range/count/false-resolution boundaries.

## Ambiguity and zero matches

- 176 Slint method-name candidates match more than one component declaration; no guessed owner.
- `value`, `model`, `set_text` etc may belong to unrelated Rust types. Exact receiver identifier `cfg`/`c`/`win` isn't type proof.
- `auto_export_on_quit` has no selected member-expression candidate; source declaration/default initializer exist. **Not declared dead/unused universally**: macros/external/generated code and unresolved borrowing/aliases require separate inspection. Targeted whole tracked Rust search also found only declaration/initializer, recorded with scope in [manual setting chains](../features/ui-setting-consumers-and-save-boundaries.md).
- Macro token trees excluded, not expanded. cfg(test) expressions syntactically included; compiler cfg/visibility/module `include!` and test execution not evaluated. Method calls named like Config fields are not field reads. Top-level struct initializer entries are not consumer edges here.

## Reproduce

[Existing parser setup](../syntax/README.md) supplies Tree-sitter binding 0.25.2/Rust 0.24.2. Whole census isolated subprocess/60s, no Rust compilation or application/script execution.

```bash
PYTHONPATH=.campaign-state/parser-site-stable python3 -B \
  docs/agent-map/operations/config_ui_edges.py --output docs/agent-map/schema/config-ui-name-edges.json
python3 -B docs/agent-map/operations/config_ui_edges.py \
  --output docs/agent-map/schema/config-ui-name-edges.json --validate
```

Read generated output before replacement under file-observation policy. Seven fixtures cover nonConfig collision, receiver/write shape, method-versus-field/generated-name candidates, comments/strings/macros, UTF8/closure ranges, invalid input and actual source ordering for language/scheme/opacity/monitor. [Manual contract](../features/ui-setting-consumers-and-save-boundaries.md) separately resolves four selected principal chains; no save fault/visual/native/independent acceptance.
