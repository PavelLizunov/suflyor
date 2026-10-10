# History index

Completed history removed from the working tree: audit reports, retest checklists, release notes, release evidence and archive pages. The rule is in `docs/AGENTS.md`, section 6.

Read a removed report with `git show <revision>:<path>`. List a removed folder with `git ls-tree -r --name-only <revision> <folder>/`. Find the removing commit with `git log --diff-filter=D --oneline -- <path>`. Use the full revision SHA.

Every row below names the master commit 8a38baccc38ec317693be4dde376e958e8f3c242, the last master commit that holds these files. The "Removed in" column names the batch of the 2026-10-08 hygiene pass; the removing commit is found with the `git log` command above.

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
| docs/archive-panel-retest-v0.22.2.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/archive-panel-test-report.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/archive-panel-visual-checklist.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-evidence-v0.35.3/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-evidence-v0.36.0/ | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.23.0.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.34.0.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.35.3.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.36.0.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.36.1-rc.1.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.36.1-rc.2.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.36.1-rc.3.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.38.0-rc.1.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/release-notes-v0.38.0-rc.2.md | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-context-window-2026-07-30.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-selection-2026-07-03.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-selection-r2-2026-07-03.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-selection-r3-2026-07-03.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-transcript-stars-r4-2026-07-03.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.23.0-fixes.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.24.0.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.25.1-fixes.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.26.0-fixes.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.27.0.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.29.0.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.30-F-fix.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.30-phase1.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.30-player.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.34.0-release.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.35.1.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.35.2-fixes.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.1.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.10.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.11.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.12.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.13.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.14.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.15.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.2.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.3.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.4.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.5.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.6.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.7.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.8.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.36.1-rc.9.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.10.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.11.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.12.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.13.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.14.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.16.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.2.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.3.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.4.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.5.1.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.5.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.6.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.7.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.8.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0-rc.9.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.37.0.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.38.0-rc.1-stt.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
| docs/retest-v0.38.0-rc.3-macos-audio.html | 8a38baccc38ec317693be4dde376e958e8f3c242 | batch 2 |
