# Original memory C04/C07: bounded matching evidence

**Baseline:** `a10c356af05a5832a14ea06a5d0cb6c49694e3f1`. Six [fixtures](../operations/test_memory_matching_hypotheses.py) model frozen string logic with pure Python. They do not execute Rust tests, query owner memory, or build prompts. C04/C07 remain hypotheses.

## C04 — four-character root is intentionally broad

[words_match](<../../../overlay-backend/src/memory/normalize.rs#L394-L402>) accepts a shared prefix of `min(len_a, len_b, 4)`. The model confirms `проверили` and `провалили` match, while `код` does not match `кот`. A three-character word still requires its whole prefix, so inflection such as `код/кода` matches. This is the stated residual, not a semantic-antonym execution.

## C07 — long terms drop one character, Latin terms have no minimum

[term_in_tokens](<../../../overlay-backend/src/memory/summary_ref.rs#L118-L126>) uses `term[0..n-1]` once a term reaches five characters. Thus `альфа` matches `альфе`, while a four-character term still requires exact equality. [Latin detection](<../../../overlay-backend/src/memory/summary_ref.rs#L80-L103>) accepts any ASCII alphanumeric token inside Cyrillic text, including `in`; no length guard appears in that condition.

## Limits

No Rust unit test, owner memory database, transcript, or prompt injection was run. Original statuses and 39/75/5 remain unchanged.
