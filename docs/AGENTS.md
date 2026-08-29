# Documentation taxonomy and navigation guide (`docs/AGENTS.md`)

This guide defines the documentation taxonomy, update discipline, privacy constraints, and verification requirements for files maintained under `docs/`.

## 1. Authoritative vs. Historical Artifacts

Documentation in this project is categorized into two operational tiers:

### Operational and current-reference documents
These files have specific ownership; verify their scope and date before relying on them:
- `docs/CODEX_HANDOFF.md`: handoff for the branch/worktree named inside it. Read it when resuming that work, but verify `git status` and `git log` first; it does not describe every checkout.
- `docs/state-and-plan.md`: context-recovery history that points to the active handoff. Older sections are intentionally historical.
- `docs/AGENT_TASKS.md`: self-contained task queue with acceptance criteria and branch names.
- `docs/architecture.md`: developer overview. It may lag production code; code, manifests, and nested `AGENTS.md` win on conflicts.
- `docs/memory-architecture.md`: proposed memory ADR. It is authoritative for intended future phases only; current code and migrations decide what is implemented.
- `docs/read-aloud-status.md`: subsystem status/reference for TTS and OCR.
- `docs/winbrat-recovery.md`: mandatory operational guide for Windows worker build/test recovery.
- `docs/REVIEW_AGENT_PROMPT.md`: standard prompt for independent review.

### Historical provenance and milestone context
Historical planning, release evidence, migration blueprints, and audit snapshots are evidence. Preserve completed artifacts; correct an active document only when the task owns it:
- `docs/goal-*.md`: Task charters and goal specifications for past or scoped deliverables (e.g., `goal-teratts-rc17.md`, `goal-quality-2026-07-10.md`).
- `docs/retest-*.html` & `docs/archive-*.html`: Golden-rule tester checklists and acceptance evidence for published releases.
- `docs/audit-YYYY-MM-DD-*/` & `docs/*audit*.md`: Visual evidence, screenshot audit bundles, design reviews, and performance audit reports.
- `docs/release-notes-v*.md` & `docs/release-evidence-v*/`: Release notes and visual acceptance artifacts for past releases.
- `docs/PHASE-*.md`, `docs/PLAN-*.md`, `docs/MIGRATION-*.md`, `docs/ADR-*.md`: Historical design records, architecture decision records, and migration cut plans (e.g., Phase 7 Tauri-to-Slint cut).

---

## 2. Naming Families in `docs/`

| Prefix / Pattern | Category | Status | Maintenance Rule |
|------------------|----------|--------|------------------|
| `CODEX_HANDOFF.md` | Branch/worktree handoff | Scope-bound | Update only for the in-flight work it names. Verify against Git. |
| `state-and-plan.md` | Context-recovery history | Mixed current/historical | Keep its pointer to the active handoff accurate when that work is owned. |
| `AGENT_TASKS.md` | Agent Task Queue | **Authoritative** | Claim open tasks `[~]` and mark finished items `[x]`. |
| `goal-*.md` | Deliverable Charter | Living (active) / Historical (done) | Create for multi-step feature/refactor charters; state scope & done criteria. |
| `retest-*.html` | Tester Checklist | Historical Evidence | Copy `retest-template.html` to `retest-v<version>-<topic>.html` prior to release. |
| `audit-YYYY-MM-DD-*/` | Audit Evidence Bundle | Historical Evidence | Create date-stamped folder for visual/functional audit runs; store screenshots & README. |
| `release-notes-v*.md` | Release Notes | Historical Record | Create when preparing release publications. |
| `architecture.md` / `*-architecture.md` | System / subsystem reference | Current or proposed as labelled | Keep current overviews in sync; never present a proposed phase as implemented. |
| `PHASE-*.md` / `PLAN-*.md` / `MIGRATION-*.md` | Blueprint / Migration Plan | Historical Record | Do not edit past plans; write a new plan document for new architectural phases. |
| `ADR-*.md` | Architecture Decision Record | Historical Record | Append new decision records sequentially; do not edit accepted past ADRs. |

---

## 3. Documentation Update Discipline

1. **Session Entry:** Inspect Git first. Read `docs/CODEX_HANDOFF.md` when the current task resumes the branch/worktree named there.
2. **Work Completion:** Update `docs/state-and-plan.md` or `docs/CODEX_HANDOFF.md` only when the task owns that shared operational state. Do not overwrite another active worktree's handoff with unrelated branch information.
3. **Task Scope & Charters:** Reference or write a `docs/goal-<name>.md` charter for multi-step tasks. Keep scope strictly bounded to the charter.
4. **Release Verification:** Create a release retest checklist (`docs/retest-v<version>-<topic>.html`) from `docs/retest-template.html` before publishing.
5. **Preservation of History:** Do not rewrite or delete completed audit, retest, or release evidence. Active goal charters may be corrected by the task that owns them; prefer a superseding document for material historical changes.

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
