# Original CI C08/C10: bounded cleanup and workflow evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_ci_cleanup_gate_hypotheses.py) inspect frozen NSIS and workflow source. They do not run Actions, branch protection, or file deletion. C08/C10 remain hypotheses.

## C08 — recursive cleanup has no reparse guard

[Uninstall cleanup](<../../../scripts/slint-installer.nsi#L148-L160>) asks for confirmation, then recursively removes three user trees. The selected section contains no file-attribute or reparse-tag test before `RMDir /r`. This is a source precondition, not a junction traversal or deletion test.

## C10 — required gate does not include advisory jobs

[Gate job](<../../../.github/workflows/ci.yml#L146-L167>) needs only `changes` and `rust`. It passes when Rust is not needed, and requires Rust success only when the selector says `true`. [Validate](<../../../.github/workflows/ci.yml#L169-L178>) is a separate job without `needs`, and macOS is also outside the gate dependency list.

The workflow calls the external docs classifier and does not interpolate a PR title or body into shell. A self-modifying workflow bypass remains a repository-policy hypothesis: no branch protection or Actions execution was tested.

## Limits

No installer, filesystem mutation, GitHub Actions job, or protection-rule inspection was run. Original statuses and 39/75/5 remain unchanged.
