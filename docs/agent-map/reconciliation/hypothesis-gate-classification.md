# Original CI C04/C05: bounded Git classification evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_gate_classification_hypotheses.py) use temporary Git repositories and frozen script source. They do not execute the PowerShell gate, hooks, or GitHub Actions. C04/C05 remain hypotheses.

## C04 — missing base narrows the push diff

[Push base](<../../../scripts/git-gate-native.ps1#L42-L55>) falls back to `HEAD~1` when `origin/master` cannot be resolved. A temporary three-commit history contains Rust followed by Markdown. `HEAD~1...HEAD` lists only `README.md` and classifies as docs; the full range lists both files and classifies as targeted.

This demonstrates the precondition, not a native hook execution or an actual protected-branch bypass.

## C05 — name-only output is quoted

The same [diff invocation](<../../../scripts/git-gate-native.ps1#L50-L70>) uses `--name-only` without `-z`, then only replaces backslashes. Git prints `quote".md` as `"quote\".md"`. The quoted token does not match a docs suffix, while the `-z` form preserves the real `.md` name and does.

The fixture therefore shows a parser boundary. It does not prove the PowerShell pipeline misclassifies every unusual filename or execute the native gate.

## Limits

No `powershell.exe`, pre-commit hook, remote push, or CI job was run. Original statuses and 39/75/5 remain unchanged.
