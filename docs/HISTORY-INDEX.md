# History index

Completed history removed from the working tree: audit reports, retest checklists, release notes, release evidence and archive pages. The rule is in `docs/AGENTS.md`, section 6.

Read a removed report with `git show <revision>:<path>`. List a removed folder with `git ls-tree -r --name-only <revision> <folder>/`. Find the removing commit with `git log --diff-filter=D --oneline -- <path>`. Use the full revision SHA.

The revision used here is the master commit 8a38baccc38ec317693be4dde376e958e8f3c242. The removal commits are on the local branch hygiene/2026-10-08 and are not pushed yet. Until they are pushed, master is the only reachable copy that holds these files.

| Path before removal | Revision that holds it | Removed in |
|---|---|---|
| docs/audit-2026-07-31-results/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-02-docs/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-memory-structure/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-readme-screenshots/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-taskbar-exclusion/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-taskbar-windows/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-ui-language-notices/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-03-ui-visual-method/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-07-rc2-deep-lock/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-09-ai-providers/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-10-codex-real-provider/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-12-codex-reasoning-vision/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-15-macos-gate0a/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-15-macos-gate0b/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-15-rc16-regressions/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-29-macos-model-tps/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-29-runtime-decomposition/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-08-29-tile-selection/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-09-02-stt-question-boundary/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/audit-2026-09-06-astra-icons/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/follow-up-audit-v0.8.4.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
| docs/qwen-parallel-audit-2026-07-29.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 1 |
