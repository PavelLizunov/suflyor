# Original memory C05/C08: bounded grounding and load evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_memory_grounding_hypotheses.py) model frozen logic in Python. They do not execute Rust, read owner memory, or send prompts. C05/C08 remain hypotheses.

## C05 — order survives omission and clipping

[grounded_in_order](<../../../overlay-backend/src/memory/normalize.rs#L410-L419>) advances after each match, so intermediate words may disappear while reversed order fails. A 240-character clip drops the tail mechanically; it does not preserve a semantic boundary.

## C08 — full load has no fact dedup

[Context load](<../../../overlay-backend/src/memory/context_builder.rs#L170-L190>) requests all active default items with limit `-1`. [Ranking](<../../../overlay-backend/src/memory/context_builder.rs#L140-L165>) scores and clones matches, while formatting contains no dedup step. Question normalization is whitespace and case only, so reordered questions remain distinct.

## Limits

No Rust test, owner memory database, prompt, or performance measurement was run. Original statuses and 39/75/5 remain unchanged.
