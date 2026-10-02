# Original memory C01/C02: bounded filter and budget evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_memory_budget_hypotheses.py) model frozen string logic in Python. They do not execute Rust, read owner memory, or send prompts. C01/C02 remain hypotheses.

## C01 — the denylist is substring-only and ask-scoped

[Instruction filter](<../../../overlay-backend/src/memory/context_builder.rs#L26-L44>) lowercases text and searches a fixed needle list. It matches `IGNORE`, but not a split `ig nore` or a `SYSTEM:` role marker. [Summary formatter](<../../../overlay-backend/src/memory/summary_ref.rs#L147-L164>) emits approved text without calling that filter.

## C02 — budgets stop later lines, not the whole prompt

[Ask block](<../../../overlay-backend/src/memory/context_builder.rs#L51-L85>) checks the projected size only when `used > 0`. The model admits initial maximum-length lines and then stops before consuming all eight. [Merge](<../../../overlay-backend/src/memory/context_builder.rs#L97-L107>) concatenates base and block without `MAX_BLOCK_CHARS`. Character counts are not model-token counts.

## Limits

No Rust unit test, owner memory database, transcript, or model prompt was run. Original statuses and 39/75/5 remain unchanged.
