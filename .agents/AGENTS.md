# Agent Skill Guidelines (`.agents/`)

Local guide for DSH agents maintaining and invoking project-owned skills stored in `.agents/skills/`.

## Mandatory Skill Invocations

Project skills are mandatory procedural requirements for specific tasks. Agents must load and follow the corresponding skill before completing the work:

- **`slint-mcp-ui-audit`**: Mandatory after any `.slint` edit or Rust change affecting visible UI, layout, text, control states, Settings tabs, overlay windows, or release presentation. Must be completed before committing UI work or declaring a UI task complete.
- **`source-command-release`**: Mandatory whenever building, verifying, publishing, or cleaning up a Suflyor Release Candidate (RC) or stable release under the repository release policy.

## Procedural Sources of Truth

- The `.agents/skills/<skill-name>/SKILL.md` files are the canonical, procedural sources of truth for domain-specific execution steps.
- Do not duplicate full skill procedures, scripts, or step-by-step checklists into higher-level `AGENTS.md` files; reference skills by name and purpose.
- Maintain skill files directly within `.agents/skills/` when procedures evolve, ensuring frontmatter metadata (`name`, `description`) accurately reflects activation triggers.

## Repository Skill Catalog

- **`slint-mcp-ui-audit`** (`.agents/skills/slint-mcp-ui-audit/SKILL.md`): End-to-end visual and functional verification procedure via the embedded Slint MCP server (`--features ui-mcp`), baseline screenshot comparisons, Settings tab coverage, and global hotkey smoke testing.
- **`source-command-release`** (`.agents/skills/source-command-release/SKILL.md`): Full release execution workflow, including version sync, testing gate tiers, RC vs stable release approval policies, installer artifact verification, and mandatory post-release cleanup.
