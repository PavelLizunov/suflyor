# Original memory C03/C06: bounded negation counting and unconditional recency fallback evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_memory_negation_recency_confirmed.py) inspect frozen personal-memory rewrite validation and context ranking sources. They do not execute AI prompt completion or query live user databases. C03 and C06 remain confirmed mechanisms.

## C03 — Russian-only negation particles and count-based validation

In `normalize.rs`, [NEGATIONS](<../../../overlay-backend/src/memory/normalize.rs#L375-L386>) defines a hardcoded list of Cyrillic particles:
`["не", "нет", "ни", "нельзя", "без", "никак", "никогда", "ничего", "никто", "никакой"]`.
It contains no English negative particles (`not`, `never`, `without`, `cannot`).
In [validate_rewrite](<../../../overlay-backend/src/memory/normalize.rs#L493-L510>), polarity preservation is enforced exclusively by counting:
`if negation_words(f).len() != negation_words(span).len() { return false; }`.
Any rewrite that inverts polarity using English terms or swaps negative scope without changing particle counts passes this gate.

## C06 — query term length threshold and unconditional recency fallback

In `context_builder.rs`, [query_terms](<../../../overlay-backend/src/memory/context_builder.rs#L106-L115>) filters question tokens using `t.chars().count() >= 4`, discarding short acronyms and names under 4 characters (such as `Go`, `S3`, or 2-3 character entity tags).
When no terms match or the query produces zero relevant hits, [rank_by_relevance](<../../../overlay-backend/src/memory/context_builder.rs#L140-L155>) returns `None`.
In [context_for_meeting](<../../../overlay-backend/src/memory/context_builder.rs#L176-L190>), if `rank_by_relevance` returns `None`, it unconditionally executes:
`None => format_memory_block(&items)`.
This fallback injects the newest memory items directly into the prompt context, regardless of whether they have any topical relation to the user's ask.

## Limits

No synthetic prompt poisoning attacks were sent to LLMs, and no private memory leaks were tested in live UI sessions. Original statuses in `candidates.json` remain `confirmed`.
