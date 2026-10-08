# Documentation taxonomy and navigation guide (`docs/AGENTS.md`)

This guide defines the documentation taxonomy, update discipline, privacy constraints, and verification requirements for files maintained under `docs/`.

## 1. Authoritative vs. Historical Artifacts

Documentation in this project is categorized into two operational tiers:

### Operational and current-reference documents
These files have specific ownership; verify their scope and date before relying on them:
- `docs/CODEX_HANDOFF.md`: the live state file (owner decision, 2026-10-08). It describes the branch or worktree it names. Verify `git status` and `git log` first; it does not describe every checkout.
- `docs/state-and-plan.md`: historical context-recovery record. It is no longer updated (owner decision, 2026-10-08).
- `docs/AGENT_TASKS.md`: self-contained task queue with acceptance criteria and branch names.
- `docs/architecture.md`: developer overview. It may lag production code; code, manifests, and nested `AGENTS.md` win on conflicts.
- `docs/memory-architecture.md`: proposed memory ADR. It is authoritative for intended future phases only; current code and migrations decide what is implemented.
- `docs/read-aloud-status.md`: subsystem status/reference for TTS and OCR.
- `docs/winbrat-recovery.md`: mandatory operational guide for Windows worker build/test recovery.
- `docs/REVIEW_AGENT_PROMPT.md`: standard prompt for independent review.

### Historical provenance and milestone context
Historical planning and migration blueprints are evidence. Preserve completed artifacts; correct an active document only when the task owns it. Completed audit reports, retest checklists, release notes, release evidence and archive pages are not kept in the working tree; see section 6:
- `docs/goal-*.md`: Task charters and goal specifications for scoped deliverables. A charter whose release has shipped is completed history (section 6).
- `docs/retest-*.html` & `docs/archive-*.html`: Golden-rule tester checklists and acceptance evidence for published releases. Section 6 decides which ones stay in the tree.
- `docs/release-notes-v*.md` & `docs/release-evidence-v*/`: Release notes and visual acceptance artifacts for releases. Section 6 decides which ones stay in the tree.
- `docs/PHASE-*.md`, `docs/PLAN-*.md`, `docs/MIGRATION-*.md`, `docs/ADR-*.md`: Design records, architecture decision records, and migration cut plans (e.g., Phase 7 Tauri-to-Slint cut). Section 6 decides which completed ones stay in the tree.

---

## 2. Naming Families in `docs/`

| Prefix / Pattern | Category | Status | Maintenance Rule |
|------------------|----------|--------|------------------|
| `CODEX_HANDOFF.md` | Live state file | Live | Update when the task owns the branch or worktree it names. Verify against Git. |
| `state-and-plan.md` | Context-recovery history | Historical | Not updated. Live state is in `CODEX_HANDOFF.md`. |
| `HISTORY-INDEX.md` | Index of completed history removed from the tree | Live index | One row per removed item (section 6). |
| `AGENT_TASKS.md` | Agent Task Queue | **Authoritative** | Claim open tasks `[~]` and mark finished items `[x]`. |
| `goal-*.md` | Deliverable Charter | Living (active) / Historical (done) | Create for multi-step feature/refactor charters; state scope & done criteria. |
| `retest-*.html` | Tester Checklist | Historical Evidence | Copy `retest-template.html` to `retest-v<version>-<topic>.html` prior to release. Past ones leave the tree (section 6). |
| `audit-YYYY-MM-DD-*/` | Audit evidence (working copy during a task) | Not kept in the tree after the task | Create the folder for the run; remove it from the tree when the task closes and index it (section 6). |
| `release-notes-v*.md` | Release Notes | Historical Record | Create when preparing release publications. Past ones leave the tree (section 6). |
| `architecture.md` / `*-architecture.md` | System / subsystem reference | Current or proposed as labelled | Keep current overviews in sync; never present a proposed phase as implemented. |
| `PHASE-*.md` / `PLAN-*.md` / `MIGRATION-*.md` | Blueprint / Migration Plan | Historical Record | Do not edit past plans; write a new plan document for new architectural phases. |
| `ADR-*.md` | Architecture Decision Record | Historical Record | Append new decision records sequentially; do not edit accepted past ADRs. |

---

## 3. Documentation Update Discipline

1. **Session Entry:** Inspect Git first. Read `docs/CODEX_HANDOFF.md` when the current task resumes the branch/worktree named there.
2. **Work Completion:** Update `docs/CODEX_HANDOFF.md` only when the task owns the branch or worktree it names. Do not overwrite another active worktree's handoff with unrelated branch information.
3. **Task Scope & Charters:** Reference or write a `docs/goal-<name>.md` charter for multi-step tasks. Keep scope strictly bounded to the charter.
4. **Release Verification:** Before publishing, create a release retest checklist (`docs/retest-v<version>-<topic>.html`) from `docs/retest-template.html` and fill in its per-change items.
5. **Preservation of History:** Completed history leaves the working tree only under section 6, and is never rewritten in place. Active goal charters may be corrected by the task that owns them; prefer a superseding document for material historical changes.

---

## 4. Privacy & Security Rules

Documentation, logs, and audit reports MUST strictly prevent secret and sensitive data leakage:
- **No API Keys or Credentials:** Never write, log, or commit live Groq API keys (`groq_api_key`), AI bearer tokens (`ai_bearer`), or credentials from `%APPDATA%\suflyor\config.json`.
- **No Private Prep Files:** `nini-context-backup.txt` and similar personal notes must remain gitignored and never committed to documentation.
- **Redact Local System Paths:** Use `%USERPROFILE%` or `~` instead of exposing real Windows/macOS user paths (`C:\Users\<user>\...`). Apply `redact_user_home` logic to log outputs.
- **Redact Private Network Data:** Mask LAN addresses, private endpoints, and user-specific hostnames in logs, screenshots, and public docs. Repository-approved worker aliases may appear in internal operational instructions without addresses or credentials.

---

## 5. Verification & Docs Gate

Documentation, plans, and non-executable markdown/HTML text files use the **Docs Gate**:
- **Gate Classifier:** The native classifier is `scripts/git-gate-native.ps1`; DSH may perform the read-only docs check directly.
- **Scope:** Validate formatting and trailing whitespace without building Rust binaries.
- **Verification Commands:** Before commit run `git diff --cached --check` for the complete staged change; after commit run `git show --check <SHA>`.

---

## 6. Completed history lives in git

Completed history is evidence of a past run: audit reports, retest checklists, release notes, release evidence, archive pages, and goal charters, plans, post-mortems and dated reviews whose work has shipped. It is not kept in the working tree. Each item stays in git at a recorded revision, and that revision is how it is read. This follows the pattern of the VPNRouter repository.

Kept in the tree, even when finished:

- the retest checklists, release notes and release evidence of the latest stable release and of the prerelease line in progress, so the release gate can find the retest it needs;
- the retest template;
- anything a tracked file other than `docs/HISTORY-INDEX.md` still refers to: code, comments, tests, scripts, workflows or other documents.

Rules for moved items:

1. While a task runs, its evidence may sit in its folder or file (for example `docs/audit-YYYY-MM-DD-<task>/`).
2. When the task closes, the task removes the item in its own commit and adds one row to `docs/HISTORY-INDEX.md`: the path and the revision that still holds it. That revision is the last commit before the removal that is reachable from master or a pushed branch.
3. Read a removed item with `git show <revision>:<path>`. List a removed folder with `git ls-tree -r --name-only <revision> <folder>/`. Find the removing commit with `git log --diff-filter=D --oneline -- <path>`. Use the full revision SHA, not a short one.
4. A revision must stay reachable: it must be on a pushed branch, a tag, or master. Do not rewrite history to hide evidence.
5. Removing an item from the tree does not remove it from git. Sensitive content is handled under section 4, not by this rule.
6. Screenshots, clips and other files inside a folder follow the same rule.

A goal charter or plan is completed when the release it targets has shipped and it has no unchecked item. An active charter, a plan not yet carried out and an ADR that is still in force stay; section 3 applies to them.
